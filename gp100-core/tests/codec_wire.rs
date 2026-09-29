//! Testes de CONTRATO do codec — ROADMAP M0.4: os helpers semânticos devem
//! reproduzir os writes REAIS das fixtures P4 byte a byte. É o elo entre o
//! codec (M0.4) e o que a pedaleira de fato recebeu em campo.

use std::path::PathBuf;

use gp100_core::codec::{
    ir_chunk_ack_payload, ir_chunk_parse, meta_block, nibble_collapse, nibble_expand, set_param,
    set_param_parse,
};

/// Lê uma fixture JSONL (regime de bytes: arquivo do repo, checkout limpo).
fn fixture(name: &str) -> Vec<serde_json::Value> {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // raiz do repo
    p.push("analysis");
    p.push("fixtures");
    p.push(name);
    std::fs::read_to_string(&p)
        .expect("fixture existe (make_fixtures.py)")
        .lines()
        .map(|l| serde_json::from_str(l).expect("JSONL válido"))
        .collect()
}

/// set_param reproduz as 92 writes de knob das fixtures (S3: 89, S1 doc: 3)
/// byte a byte: parse do payload capturado → rebuild → idêntico.
#[test]
fn set_param_matches_all_fixture_writes() {
    let knobs = fixture("knobs.jsonl");
    assert_eq!(knobs.len(), 92, "89 (S3) + 3 (S1 doc)");
    let mut checked = 0;
    for k in &knobs {
        assert_eq!(k["dir"], "out");
        let addr = k["addr"].as_str().expect("addr");
        let payload_hex = k["data"].as_str().expect("data");
        let payload = hex_bytes(payload_hex);

        // extrai slot do addr (10 [slot] 00 02) e decodifica a semântica
        assert_eq!(&addr[..2], "10", "addr de set-param");
        assert_eq!(&addr[4..], "0002");
        let slot: u8 = u8::from_str_radix(&addr[2..4], 16).expect("slot");
        let (code, ctrl, value) = set_param_parse(&payload).expect("payload §13.11");

        // rebuild via codec == bytes capturados (fixture guarda PAYLOAD puro,
        // sem envelope/F7 — ver convenção do build_golden)
        let rebuilt = set_param(slot, code, ctrl, value).expect("rebuild");
        assert_eq!(rebuilt.len(), 8 + 1 + 4 + 20 + 1);
        assert_eq!(
            &rebuilt[13..33],
            &payload[..],
            "knob divergente: {addr} {payload_hex}"
        );
        assert_eq!(rebuilt[8], 0x12);
        assert_eq!(rebuilt[33], 0xF7);
        checked += 1;
    }
    assert_eq!(checked, 92);
}

/// meta_block reproduz as writes de metadados do save (S4 + S2) byte a byte.
#[test]
fn meta_block_matches_fixture_writes() {
    let save = fixture("save.jsonl");
    // S4: "It's GP100" (pp 0, type 4) — os 5 writes do bloco
    let s4: Vec<&serde_json::Value> = save
        .iter()
        .filter(|x| x["s"] == "S4" && x["dir"] == "out")
        .collect();
    let want_addrs = ["11000000", "11000004", "11000005", "11000007", "12000002"];
    for (i, wa) in want_addrs.iter().enumerate() {
        let hit = s4
            .iter()
            .find(|x| x["addr"] == *wa)
            .unwrap_or_else(|| panic!("write {wa} ausente no S4"));
        let captured = hex_bytes(hit["data"].as_str().unwrap());
        // decodifica pp/ppType do PRÓPRIO fio e reconstrói via codec
        let block = meta_block(0, 4, "It's GP100").expect("meta válida");
        let (addr, payload) = &block[i];
        assert_eq!(&hex_of(addr), *wa);
        assert_eq!(payload, &captured, "metadado {wa} divergiu");
    }

    // S2: "Blink OD" (pp 1, type 6) — mesmas regras, outro preset
    let block2 = meta_block(1, 6, "Blink OD").expect("meta válida");
    let (a0, p0) = &block2[0];
    assert_eq!(&hex_of(a0), "11000000");
    let hit0 = save
        .iter()
        .find(|x| x["s"] == "S2" && x["dir"] == "out" && x["addr"] == "11000000")
        .expect("S2 11000000");
    assert_eq!(
        p0,
        &hex_bytes(hit0["data"].as_str().unwrap()),
        "S2 meta completa (pp 1 + 'Blink OD')"
    );
    let hit5 = save
        .iter()
        .find(|x| x["s"] == "S2" && x["dir"] == "out" && x["addr"] == "11000005")
        .expect("S2 11000005");
    assert_eq!(block2[2].1, hex_bytes(hit5["data"].as_str().unwrap()));
}

/// ir_begin + chunks + ACKs reproduzem o upload completo da fixture (592
/// chunks por slot, último idx duplicado; frame por frame).
#[test]
fn ir_upload_matches_fixture_frame_by_frame() {
    let ir = fixture("ir.jsonl");
    assert_eq!(ir.len(), 1186, "2 BEGIN + 592 chunks + 592 ACKs");

    // BEGINs (slots 0 e 1)
    let begins: Vec<&serde_json::Value> = ir.iter().filter(|x| x["addr"] == "10050001").collect();
    assert_eq!(begins.len(), 2);
    assert_eq!(
        hex_bytes(begins[0]["data"].as_str().unwrap()),
        vec![0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x0A]
    );
    assert_eq!(
        hex_bytes(begins[1]["data"].as_str().unwrap()),
        vec![0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x0A]
    );

    // todos os chunks: parse → rebuild byte-idêntico; ACK ecoa (slot, idx)+01
    let chunks: Vec<&serde_json::Value> = ir
        .iter()
        .filter(|x| x["addr"] == "12001002" && x["dir"] == "out")
        .collect();
    let acks: Vec<&serde_json::Value> = ir
        .iter()
        .filter(|x| x["addr"] == "12001002" && x["dir"] == "in")
        .collect();
    assert_eq!(chunks.len(), 592);
    assert_eq!(acks.len(), 592);

    for (c, a) in chunks.iter().zip(acks.iter()) {
        let payload = hex_bytes(c["data"].as_str().unwrap());
        let (slot, idx, data) = ir_chunk_parse(&payload).expect("chunk válido");
        assert_eq!(data.len(), 15, "15 bytes reais por chunk");

        // rebuild do chunk == bytes capturados (payload puro, 33B)
        let rebuilt = gp100_core::codec::ir_chunk(slot, idx, &data).expect("rebuild");
        assert_eq!(&rebuilt[13..46], &payload[..], "chunk divergiu");

        // ACK capturado == ACK gerado pelo codec (eco + 01)
        let want_ack = ir_chunk_ack_payload(slot, idx);
        assert_eq!(hex_bytes(a["data"].as_str().unwrap()), want_ack.to_vec());
    }

    // regra do FIM (§13.7): último chunk do slot 1 tem idx 0x0226 DUPLICADO
    let last2: Vec<Vec<u8>> = chunks
        .iter()
        .rev()
        .take(2)
        .map(|c| hex_bytes(c["data"].as_str().unwrap()))
        .collect();
    assert_eq!(last2[0], last2[1], "último chunk duplicado (fim do upload)");
    let (_, idx, _) = ir_chunk_parse(&last2[0]).expect("parse");
    assert_eq!(idx, 0x0226);
}

/// Nibble: 30 nibbles do fio = 15 bytes reais (collapse do 1º chunk real
/// devolve 15 bytes; expand volta ao original).
#[test]
fn nibble_roundtrip_on_real_chunk() {
    let ir = fixture("ir.jsonl");
    let chunk = ir
        .iter()
        .find(|x| x["addr"] == "12001002" && x["dir"] == "out")
        .expect("1º chunk");
    let payload = hex_bytes(chunk["data"].as_str().unwrap());
    assert_eq!(payload.len(), 33);
    let real = nibble_collapse(&payload[3..]).expect("30 nibbles");
    assert_eq!(real.len(), 15);
    assert_eq!(
        nibble_expand(&real),
        payload[3..],
        "expand(collapse) == original"
    );
}

// ------------------------------------------------------------ helpers
fn hex_bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

fn hex_of(addr: &[u8; 4]) -> String {
    addr.iter().map(|b| format!("{b:02x}")).collect()
}
