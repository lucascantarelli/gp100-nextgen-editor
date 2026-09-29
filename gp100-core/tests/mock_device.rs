//! Testes de CONTRATO da M0.5 — o DoD do ROADMAP: "diálogo completo
//! mock↔codec numa sessão sintética: boot → scan 198 pp → set param → save
//! → upload IR 2 slots, sem timeouts". O diálogo é dirigido como a FSM
//! (M0.6) vai dirigir: `DeviceTransport` cru + helpers do codec/golden.

use std::time::Duration;

use gp100_core::codec::{ir_begin, ir_chunk, meta_block, op_payload, set_param, set_param_parse};
use gp100_core::golden::{decode_envelope, GoldenFile};
use gp100_core::transport::{DeviceTransport, MockDevice, TransportError};

const T: Duration = Duration::from_millis(3000); // ADR-3

fn open_mock() -> MockDevice {
    let mut m = MockDevice::new().expect("mock cria (all.prst + dicionário embedados)");
    m.open().expect("abre");
    m
}

/// Numa sessão real o boot começa com o device FALANDO (pushes não
/// solicitados). O mock os entrega via `queue_push_template` (exemplo
/// congelado do golden) e `recv_raw` os entrega FIFO (D1) — sem timeout.
#[test]
fn boot_prologo_de_pushes_sem_timeout() {
    let mut m = open_mock();
    m.queue_push_template(0x12, [0x12, 0x00, 0x00, 0x01], None)
        .expect("push status (t21)");
    m.queue_push_template(0x12, [0x13, 0x00, 0x00, 0x00], None)
        .expect("push dump de boot (t1)");
    m.queue_push_template(0x12, [0x13, 0x01, 0x00, 0x01], None)
        .expect("push meta6 (t5)");
    m.queue_push_template(0x12, [0x12, 0x00, 0x10, 0x02], Some(75))
        .expect("push tabela IRs 75B (t3, by-len)");
    m.queue_push_template(0x12, [0x12, 0x00, 0x10, 0x12], None)
        .expect("push setlist 44B (t17)");

    for _ in 0..5 {
        let msg = m.recv_raw(T).expect("push chega sem timeout");
        let (_, _, payload) = decode_envelope(&msg).expect("envelope válido");
        assert!(!payload.is_empty());
    }
    // esgotado: timeout tipado (nada a ver com erro de protocolo)
    assert!(matches!(
        m.recv_raw(Duration::from_millis(10)),
        Err(TransportError::RecvTimeout { .. })
    ));
}

/// READ de tabela de IRs (func 11 em `12001002`) → push 75B (D1; shape
/// §13.12: [slot] + nome 0xFF nibble-exp + CRC de fábrica do .prst).
#[test]
fn read_tabela_irs_75b_com_crc_de_fabrica() {
    let mut m = open_mock();
    // req do template t4: var1 = página 0
    let golden = GoldenFile::embedded().expect("golden");
    let req = golden
        .request_template(0x11, &[0x12, 0x00, 0x10, 0x02], 1)
        .expect("t4")
        .build_request(&[0x00])
        .expect("req de leitura");
    m.send_raw(&req).expect("envia READ");
    let msg = m.recv_raw(T).expect("tabela chega");
    let (func, addr, payload) = decode_envelope(&msg).expect("envelope");
    assert_eq!((func, addr), (0x12, [0x12, 0x00, 0x10, 0x02]));
    assert_eq!(payload.len(), 75, "tabela 75B (by-len escolhido)");
    assert_eq!(payload[0], 0x00, "página/slot ecoado");
    // nome vazio = nibble 0xF (0xFF por byte real): 32 bytes 0xFF
    assert!(
        payload[1..33].iter().all(|&b| b == 0x0F),
        "nome vazio nibble-expandido (estado: sem IR importado)"
    );
}

/// Scan de presets (§13.10): select `13010000` (write, muda estado) →
/// req de abertura (t6, caso CONGELADO) e req de scan (t13) respondem com
/// pp BE do ESTADO; página de abertura (t8) sai com 196B.
#[test]
fn scan_de_presets_responde_com_pp_do_estado() {
    let mut m = open_mock();
    let golden = GoldenFile::embedded().expect("golden");

    // select pp 0x002A (write 11/13010000, [pp BE]) — sem resposta (D3)
    let req = golden
        .request_template(0x11, &[0x13, 0x01, 0x00, 0x00], 2)
        .expect("t9")
        .build_request(&[0x00, 0x2A])
        .expect("req select");
    m.send_raw(&req).expect("select");
    assert_eq!(m.state().current_pp, 0x002A, "estado muda com o select");

    // abertura congelada (t6: const 010001 em 13010002) → meta6 CONST do
    // golden (idêntico nas 4 sessões — o pp NÃO entra aqui; evidência S1)
    let req = golden
        .request_template(0x12, &[0x13, 0x01, 0x00, 0x02], 0)
        .expect("t6 (0 vars = congelado)")
        .build_request(&[])
        .expect("req t6");
    m.send_raw(&req).expect("abre");
    let msg = m.recv_raw(T).expect("meta6");
    let (_, _, p) = decode_envelope(&msg).expect("envelope");
    assert_eq!(
        p,
        &[0x01, 0x00, 0x0C, 0x1C, 0x01, 0x40],
        "meta6 = exemplo congelado"
    );

    // scan (t13: const 000001 em 13020002) → página 196B. EVIDÊNCIA: a
    // resposta do scan NÃO começa com pp (exemplo congelado começa com
    // consts 0000…) — só o TAMANHO é shape legítimo do mock aqui.
    let req = golden
        .request_template(0x12, &[0x13, 0x02, 0x00, 0x02], 0)
        .expect("t13")
        .build_request(&[])
        .expect("req t13");
    m.send_raw(&req).expect("scan");
    let msg = m.recv_raw(T).expect("página scan");
    let (_, _, p) = decode_envelope(&msg).expect("envelope");
    assert_eq!(p.len(), 196, "página de scan 196B (shape do mock)");

    // leitura de página (t8: mixed 5B em 13010004) → by-len 196B
    let req = golden
        .request_template(0x12, &[0x13, 0x01, 0x00, 0x04], 3)
        .expect("t8 (3 vars)")
        .build_request(&[0x00, 0x00, 0x01])
        .expect("req t8");
    m.send_raw(&req).expect("lê página");
    let msg = m.recv_raw(T).expect("página 196B");
    let (_, _, p) = decode_envelope(&msg).expect("envelope");
    assert_eq!(p.len(), 196);
}

/// SET de parâmetro (§13.11, D3/D4): fire-and-forget — SEM resposta; o
/// estado registra o par (validado contra o dicionário pelo mock).
#[test]
fn set_param_e_fire_and_forget_sem_readback() {
    let mut m = open_mock();
    let sysex = set_param(3, 0x0700_006e, 0, 15.0).expect("knob Bog RedM @ 15.0");
    m.send_raw(&sysex).expect("write aceito");
    // NENHUMA resposta (drena rápido e não vem nada)
    assert!(matches!(
        m.recv_raw(Duration::from_millis(20)),
        Err(TransportError::RecvTimeout { .. })
    ));
    let (code, ctrl, value) = set_param_parse(&sysex[13..33]).expect("roundtrip codec");
    assert_eq!(
        m.state().set_params.get(&((code >> 24) as u8, ctrl)),
        Some(&(code, value)),
        "par registrado no estado do device"
    );
}

/// SAVE (§13.12 re-derivado, D3): meta_block + ciclo de ops da S4 — todos
/// aceitos SEM NENHUMA resposta; o mock NÃO emite resync/burst (quirk
/// fechado no §13.7). O nome troca no estado (meta write).
#[test]
fn save_e_fire_and_forget_sem_nenhum_in() {
    let mut m = open_mock();
    for (addr, payload) in meta_block(0, 4, "It's GP100").expect("meta") {
        let msg = {
            let mut v = Vec::from(gp100_core::SYSEX_HEADER);
            v.push(0x12);
            v.extend_from_slice(&addr);
            v.extend_from_slice(&payload);
            v.push(gp100_core::SYSEX_EOX);
            v
        };
        m.send_raw(&msg).expect("write de meta aceito");
    }
    // ops: op0 ×2 → op1 ×2 (ciclo da S4, §13.12 re-derivado)
    for op in [0u8, 0, 1, 1] {
        let msg = {
            let mut v = Vec::from(gp100_core::SYSEX_HEADER);
            v.push(0x12);
            v.extend_from_slice(&[0x00, 0x02, 0x00, 0x00]);
            v.extend_from_slice(&op_payload(op));
            v.push(gp100_core::SYSEX_EOX);
            v
        };
        m.send_raw(&msg).expect("op aceita");
    }
    assert_eq!(m.state().current_name, "It's GP100", "meta atualiza nome");
    assert_eq!(m.state().current_pp_type, 4);
    // D3: o mock NÃO emite nada pós-save (janela curta esgota em timeout)
    assert!(matches!(
        m.recv_raw(Duration::from_millis(50)),
        Err(TransportError::RecvTimeout { .. })
    ));
}

/// UPLOAD de IR em 2 slots (§13.7): begin (reserva) + chunks com ACK POR
/// chunk (D1) + último chunk DUPLICADO (idx 0x226) recebendo 2 ACKs — o
/// marcador de fim é a duplicação em si (quirk fechado). SEM timeouts.
#[test]
fn upload_ir_2_slots_com_ack_por_chunk_e_final_duplicado() {
    let mut m = open_mock();
    let blob = vec![0x5Au8; 15 * 295]; // 295 chunks únicos × 15B
    for slot in [0u8, 1] {
        m.send_raw(&ir_begin(slot).expect("begin"))
            .expect("begin aceito");
        assert!(
            matches!(
                m.recv_raw(Duration::from_millis(10)),
                Err(TransportError::RecvTimeout { .. })
            ),
            "begin não tem resposta"
        );

        // Índices em PÁGINAS de 128 com lacunas (§13.7): 0-127, 256-383,
        // 512-... — o device real NUNCA transmite 128-255 (e assim F7 não
        // aparece cru nos bytes de idx — coerente com o trim no 1º F7).
        // 295 chunks = 128 + 128 + 39 → último idx = 512+38 = 0x0226.
        let idx_of = |i: usize| -> u16 {
            let (page, within) = (i / 128, i % 128);
            (page as u16) * 256 + within as u16
        };
        let chunks = blob.as_chunks::<15>().0;
        for (i, chunk) in chunks.iter().enumerate() {
            let idx = idx_of(i);
            let sysex = ir_chunk(slot, idx, chunk).expect("chunk");
            m.send_raw(&sysex).expect("chunk aceito");
            let ack = m.recv_raw(T).expect("ACK por chunk (sem timeout)");
            let (_, addr, p) = decode_envelope(&ack).expect("envelope");
            assert_eq!(addr, [0x12, 0x00, 0x10, 0x02]);
            assert_eq!(p, &[slot, (idx >> 8) as u8, (idx & 0xFF) as u8, 0x01]);
        }

        // O ÚLTIMO chunk da última página (idx 0x0226 = 550) é enviado 2×
        // — o marcador de fim é a DUPLICAÇÃO em si, com o payload REAL da
        // cauda (§13.7 corrigido): 2 envios, 2 ACKs.
        let last: &[u8] = &chunks[chunks.len() - 1];
        for _ in 0..2 {
            let sysex = ir_chunk(slot, 0x226, last).expect("marcador");
            m.send_raw(&sysex).expect("marcador aceito");
            let ack = m.recv_raw(T).expect("ACK do marcador");
            let (_, _, p) = decode_envelope(&ack).expect("envelope");
            assert_eq!(p, &[slot, 0x02, 0x26, 0x01]);
        }
    }
    assert_eq!(m.state().set_params.len(), 0); // upload não toca set_param
}

/// PUSH INTERCALADO (D7 — o que o mock bem-comportado não pega): no MEIO
/// do upload, o device "fala por conta própria" (`queue_push`); a mensagem
/// entra na FILA GLOBAL e o consumidor (FSM M0.6) precisa FILTRAR — o
/// transporte entrega o push antes do ACK pendente (FIFO global D1).
#[test]
fn push_intercalado_no_meio_do_upload_chega_fora_de_ordem() {
    let mut m = open_mock();
    m.send_raw(&ir_begin(1).expect("begin")).expect("ok");
    let chunk = [0x11u8; 15];
    m.send_raw(&ir_chunk(1, 0, &chunk).expect("chunk"))
        .expect("ok");

    // o device empurra algo (ex.: usuário mexeu na pedal) ENTRE o chunk
    // e o consumo do ACK
    m.queue_push(0x12, [0x12, 0x00, 0x00, 0x01], &[0x01, 0x00, 0x00]);

    // D1 FIFO global: endpoint menor (12000001) sai ANTES do ACK (12001002)
    let first = m.recv_raw(T).expect("1ª msg");
    let (fa, aa, _) = decode_envelope(&first).expect("envelope");
    assert_eq!(
        (fa, aa),
        (0x12, [0x12, 0x00, 0x00, 0x01]),
        "push sai primeiro"
    );
    let second = m.recv_raw(T).expect("2ª msg");
    let (fb, ab, p) = decode_envelope(&second).expect("envelope");
    assert_eq!((fb, ab), (0x12, [0x12, 0x00, 0x10, 0x02]));
    assert_eq!(
        p,
        &[0x01, 0x00, 0x00, 0x01],
        "ACK do chunk vem depois (D7 filtra)"
    );
}

/// D5: frame sem template no golden é RECUSADO com erro (nunca engolir) e
/// contabilizado no estado (diagnóstico de divergência).
#[test]
fn frame_sem_template_e_recusado_d5() {
    let mut m = open_mock();
    let bogus = {
        let mut v = Vec::from(gp100_core::SYSEX_HEADER);
        v.push(0x12);
        v.extend_from_slice(&[0x99, 0x99, 0x99, 0x99]); // endpoint inexistente
        v.extend_from_slice(&[0x00; 8]);
        v.push(gp100_core::SYSEX_EOX);
        v
    };
    assert!(matches!(
        m.send_raw(&bogus),
        Err(TransportError::SendFailed { .. })
    ));
    assert_eq!(m.state().rejected, 1);
}

/// D3/CICLO COMPLETO: boot (pushes) → select/scan → set_param → save →
/// IR 2 slots → esgotado. É a linha do DoD, encadeada numa sessão só.
#[test]
fn dialogo_completo_da_sessao_sintetica_sem_timeouts() {
    let mut m = open_mock();

    // boot: prólogo de pushes
    m.queue_push_template(0x12, [0x12, 0x00, 0x00, 0x01], None)
        .expect("status");
    m.queue_push_template(0x12, [0x13, 0x01, 0x00, 0x01], None)
        .expect("meta6");
    m.queue_push_template(0x12, [0x12, 0x00, 0x10, 0x02], Some(75))
        .expect("tabela");
    for _ in 0..3 {
        m.recv_raw(T).expect("boot sem timeout");
    }

    // scan 198 pp (DoD literal): para cada pp, select + req de scan
    // (t13) — 396 writes + 198 páginas 196B recebidas, sem timeouts
    let golden = GoldenFile::embedded().expect("golden");
    let sel_tpl = golden
        .request_template(0x11, &[0x13, 0x01, 0x00, 0x00], 2)
        .expect("t9")
        .clone();
    let scan_tpl = golden
        .request_template(0x12, &[0x13, 0x02, 0x00, 0x02], 0)
        .expect("t13")
        .clone();
    for i in 0..198u16 {
        let pp_be = i.to_be_bytes();
        let sel = sel_tpl.build_request(&pp_be).expect("select");
        m.send_raw(&sel).expect("select");
        let scan = scan_tpl.build_request(&[]).expect("scan");
        m.send_raw(&scan).expect("scan");
        let msg = m.recv_raw(T).expect("página do scan");
        let (_, _, p) = decode_envelope(&msg).expect("envelope");
        assert_eq!(p.len(), 196, "página 196B no pp {i}");
        assert_eq!(m.state().current_pp, i, "estado segue o select");
    }

    // set_param
    m.send_raw(&set_param(1, 0x0300_0001, 0, 42.0).expect("knob"))
        .expect("set");
    // save
    for (addr, payload) in meta_block(0x0007, 6, "Blink OD").expect("meta") {
        let mut v = Vec::from(gp100_core::SYSEX_HEADER);
        v.push(0x12);
        v.extend_from_slice(&addr);
        v.extend_from_slice(&payload);
        v.push(gp100_core::SYSEX_EOX);
        m.send_raw(&v).expect("meta");
    } // IR slot 0: 1 chunk + o ÚLTIMO chunk duplicado (marcador de fim =
      // a duplicação em si, payload real — §13.7 corrigido)
    m.send_raw(&ir_begin(0).expect("begin")).expect("begin");
    m.send_raw(&ir_chunk(0, 0, &[0x22u8; 15]).expect("chunk"))
        .expect("chunk");
    m.recv_raw(T).expect("ACK");
    for _ in 0..2 {
        m.send_raw(&ir_chunk(0, 0, &[0x22u8; 15]).expect("último dup"))
            .expect("dup");
        m.recv_raw(T).expect("ACK dup");
    }
    // IR slot 1
    m.send_raw(&ir_begin(1).expect("begin")).expect("begin");
    m.send_raw(&ir_chunk(1, 0, &[0x33u8; 15]).expect("chunk"))
        .expect("chunk");
    m.recv_raw(T).expect("ACK");

    // fim: nada pendente (D3 — mock não emite burst de fim de sessão)
    assert!(matches!(
        m.recv_raw(Duration::from_millis(30)),
        Err(TransportError::RecvTimeout { .. })
    ));
    assert_eq!(m.state().current_name, "Blink OD");
    assert_eq!(m.state().rejected, 0, "nenhuma divergência no diálogo");
}
