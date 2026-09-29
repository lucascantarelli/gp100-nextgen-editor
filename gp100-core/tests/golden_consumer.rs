//! Testes de CONTRATO do consumidor do golden — ROADMAP M0.3.
//!
//! Ângulo caixa-preta. Propriedade central provada para TODOS os 40
//! templates: extrair as vars do exemplo e reconstruir reproduz o exemplo
//! byte a byte (build_request e extract são inversos exatos).
//!
//! ⚠️ Convenção do golden (build_golden.py): `example.hex`/`request_hex`/
//! `response_hex` são os PAYLOADS (data), NÃO o SysEx completo — os bytes de
//! envelope testados aqui são reconstruídos com build_request e conferidos
//! contra as constantes do §13.1.

use gp100_core::golden::{hex_decode, GoldenFile};

/// O golden embutido carrega com 40 templates (DoD M0.3).
#[test]
fn embedded_golden_loads_40_templates() {
    let g = GoldenFile::embedded().expect("golden válido");
    assert_eq!(g.templates().len(), 40, "40 templates (baseline v1.0)");
    let (mut w, mut p, mut r) = (0, 0, 0);
    for t in g.templates() {
        match t.template_type.as_str() {
            "write" => w += 1,
            "push" => p += 1,
            "req" => r += 1,
            other => panic!("tipo inesperado: {other}"),
        }
    }
    assert_eq!((w, p, r), (22, 10, 8));
}

/// PROPRIEDADE FUNDAMENTAL para os 40 templates: exemplo (payload) → extract
/// → build == exemplo (byte a byte).
#[test]
fn build_and_extract_are_exact_inverses_for_all_templates() {
    let g = GoldenFile::embedded().expect("golden válido");
    let (mut checked_req, mut checked_write, mut checked_push) = (0, 0, 0);

    for t in g.templates() {
        let label = t
            .semantic()
            .map(|s| s.chars().take(40).collect::<String>())
            .unwrap_or_else(|| format!("{}|{}", t.template_type, t.addr_out().unwrap_or("?")));
        match t.template_type.as_str() {
            "req" => {
                // lado OUT
                let req = hex_decode(t.example.request_hex.as_deref().expect("request_hex"))
                    .expect("hex do request");
                let pat = t.request_pattern().expect("req tem request_payload");
                let vars = pat
                    .extract_vars(&req)
                    .unwrap_or_else(|| panic!("request não casa o próprio template: {label}"));
                let rebuilt = t.build_request(&vars.concat()).expect("build");
                assert_eq!(
                    rebuilt.len(),
                    8 + 1 + 4 + req.len() + 1,
                    "envelope completo: {label}"
                );
                assert_eq!(&rebuilt[13..13 + req.len()], &req[..], "payload: {label}");
                assert_eq!(rebuilt.last(), Some(&0xF7), "EOX: {label}");
                assert_eq!(
                    &rebuilt[..8],
                    &[0xF0, 0x21, 0x25, 0x7F, 0x47, 0x50, 0x2D, 0x64]
                );
                assert_eq!(
                    format!("{:02x}", rebuilt[8]),
                    t.func_out().unwrap(),
                    "func no envelope"
                );

                // lado IN: resposta do exemplo casa o padrão de resposta
                let rsp = hex_decode(t.example.response_hex.as_deref().expect("response_hex"))
                    .expect("hex da resposta");
                assert!(
                    t.matches_response(&rsp).is_some(),
                    "resposta do exemplo não casa: {label}"
                );
                checked_req += 1;
            }
            "write" => {
                let req = hex_decode(t.example.hex.as_deref().expect("hex")).expect("hex");
                let pat = t.request_pattern().expect("write tem request_payload");
                let vars = pat
                    .extract_vars(&req)
                    .unwrap_or_else(|| panic!("write não casa o próprio template: {label}"));
                let rebuilt = t.build_request(&vars.concat()).expect("build");
                assert_eq!(&rebuilt[13..13 + req.len()], &req[..], "{label}");
                assert_eq!(rebuilt.last(), Some(&0xF7));
                checked_write += 1;
            }
            "push" => {
                let data = hex_decode(t.example.hex.as_deref().expect("hex")).expect("hex");
                assert!(
                    t.matches_response(&data).is_some(),
                    "push não casa o próprio exemplo: {label}"
                );
                assert!(
                    t.build_request(&[]).is_err(),
                    "push NÃO deve ter build_request"
                );
                checked_push += 1;
            }
            other => panic!("tipo inesperado: {other}"),
        }
    }
    assert_eq!((checked_req, checked_write, checked_push), (8, 22, 10));
}

/// Consistência dos ENDPOINTS do golden: func/addr dos lados são hex válido;
/// para `req`, example é do lado OUT e o IN existe; addr tem 8 dígitos.
#[test]
fn endpoints_are_coherent() {
    let g = GoldenFile::embedded().expect("golden válido");
    for t in g.templates() {
        let (f, a) = (t.func_out().unwrap_or(""), t.addr_out().unwrap_or(""));
        {
            if !a.is_empty() {
                assert_eq!(a.len(), 8, "addr OUT 8 dígitos: {a}");
                hex_decode(a).expect("addr OUT hex");
                assert!(f == "11" || f == "12", "func OUT é 11/12: {f}");
            }
        }
        let (f, a) = (t.func_in().unwrap_or(""), t.addr_in().unwrap_or(""));
        {
            if !a.is_empty() {
                assert_eq!(a.len(), 8, "addr IN 8 dígitos: {a}");
                hex_decode(a).expect("addr IN hex");
                assert!(f == "11" || f == "12", "func IN é 11/12: {f}");
            }
        }
        // semântica anotada em todos (build_golden anota por endereço)
        assert!(
            t.semantic().is_some(),
            "template sem semântica: {}",
            t.addr_out().unwrap_or("?")
        );
    }
}

/// Despacho por by-len: `12|12001002` recebe payloads de tamanhos distintos —
/// resync de save (4B `[01][02][idx][01]`, §13.12), tabela de tipos (75B,
/// nibble-exp c/ `0f` de padding) e ACK de chunk de IR (4B, do `req` §13.7).
/// Cada comprimento casa com o sub-padrão certo.
#[test]
fn by_len_dispatch_on_same_endpoint() {
    let g = GoldenFile::embedded().expect("golden válido");
    let cands = g.for_response("12", "12001002");
    assert!(
        cands.len() >= 2,
        "pelo menos push (by-len) e req (ACK) no endereço"
    );

    // sub-len 4 do PUSH: [01][02][var][01] (resync de save, §13.12)
    let resync: &[u8] = &[0x01, 0x02, 0x08, 0x01];
    let (t_resync, vars) = g
        .match_response("12", "12001002", resync)
        .expect("resync 4B despacha");
    assert_eq!(t_resync.template_type, "push");
    assert_eq!(vars, vec![&resync[2..3]], "var do resync = idx");

    // sub-len 75 do PUSH: tabela de tipos (exemplo real do golden)
    let push = cands
        .iter()
        .copied()
        .find(|t| t.template_type == "push")
        .expect("push 12001002");
    let table = hex_decode(push.example.hex.as_deref().expect("exemplo 75B")).expect("hex");
    assert_eq!(table.len(), 75);
    let (t_tab, _) = g
        .match_response("12", "12001002", &table)
        .expect("tabela 75B despacha");
    assert_eq!(t_tab.template_type, "push");

    // req de leitura do MESMO endereço: ACK de chunk de IR [var 3][01] (§13.7)
    let ack = hex_decode(
        cands
            .iter()
            .copied()
            .find(|t| t.template_type == "req")
            .expect("req 12001002")
            .example
            .response_hex
            .as_deref()
            .expect("response_hex"),
    )
    .expect("hex");
    assert_eq!(ack.len(), 4);
    let (t_ack, av) = g
        .match_response("12", "12001002", &ack)
        .expect("ACK despacha");
    assert_eq!(t_ack.template_type, "req");
    assert_eq!(av.concat(), &ack[..3]);
}

/// build_request monta o ENVELOPE completo: header + func + addr(BE) + data + F7.
#[test]
fn build_request_produces_full_sysex() {
    let g = GoldenFile::embedded().expect("golden válido");
    // write de keepalive: 12|00020001, payload const 00000000 (sem vars)
    let t = g.for_request("12", "00020001").expect("template keepalive");
    let sysex = t.build_request(&[]).expect("sem vars");
    assert_eq!(sysex.len(), 8 + 1 + 4 + 4 + 1);
    assert_eq!(
        &sysex[..8],
        &[0xF0, 0x21, 0x25, 0x7F, 0x47, 0x50, 0x2D, 0x64]
    );
    assert_eq!(sysex[8], 0x12);
    assert_eq!(&sysex[9..13], &[0x00, 0x02, 0x00, 0x01]); // addr u32 BE
    assert_eq!(&sysex[13..17], &[0x00, 0x00, 0x00, 0x00]); // payload const
    assert_eq!(sysex[17], 0xF7);
}

/// build_request é STRICT: faltam vars = erro; vars em excesso = erro;
/// push não tem request.
#[test]
fn build_request_is_strict() {
    let g = GoldenFile::embedded().expect("golden válido");
    // select de preset (11|13010000): payload [pp hi][pp lo]
    let t = g.for_request("11", "13010000").expect("template select");
    assert!(t.build_request(&[]).is_err(), "faltam vars");
    assert!(
        t.build_request(&[0x01, 0x00, 0x99]).is_err(),
        "vars em excesso"
    );
    let ok = t.build_request(&[0x01, 0x00]).expect("2 vars");
    assert_eq!(&ok[13..15], &[0x01, 0x00]);

    // push (não tem request) → erro
    let push = g
        .templates()
        .iter()
        .find(|t| t.template_type == "push")
        .unwrap();
    assert!(push.build_request(&[]).is_err());
}

/// extract_vars devolve segmentos POSICIONAIS (ordem dos var no padrão) —
/// provado no request real da página de preset: 12|13010004, 5B
/// [pp hi][pp lo][pg hi][pg lo][01].
#[test]
fn extract_vars_are_positional() {
    let g = GoldenFile::embedded().expect("golden válido");
    let t = g.for_request("12", "13010004").expect("template página");
    let pat = t.request_pattern().expect("request_payload");
    let wire = [0x01, 0x00, 0x00, 0x00, 0x01];
    let vars = pat.extract_vars(&wire).expect("casa");
    assert!(!vars.is_empty());
    let rebuilt = t.build_request(&vars.concat()).expect("rebuild");
    assert_eq!(&rebuilt[13..], &[0x01, 0x00, 0x00, 0x00, 0x01, 0xF7][..]);
}

/// hex_decode: aceita caixa alta/baixa, rejeita ímpar e não-hex.
#[test]
fn hex_decode_strict() {
    assert_eq!(hex_decode("0Abf").unwrap(), vec![0x0A, 0xBF]);
    assert!(hex_decode("abc").is_err()); // ímpar
    assert!(hex_decode("zz").is_err()); // não-hex
}
