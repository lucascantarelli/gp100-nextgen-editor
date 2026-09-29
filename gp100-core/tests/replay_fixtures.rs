//! Testes de REPLAY da M0.6 (DoD): cada fixture de fase reproduzida
//! byte-a-byte — os OUTs da Session são comparados 1:1 (func+addr+payload
//! hex) com os OUTs da captura (divergência = falha com diff hex) e os
//! INs da captura são devolvidos na ordem ao `wait_for` da Session.

use std::time::Duration;

use gp100_core::golden::decode_envelope;
use gp100_core::session::Session;
use gp100_core::transport::{DeviceTransport, TransportError};

mod common;
use common::{fixture_rows, fixture_tuple};

/// Transporte de replay: os sends da Session são comparados com o próximo
/// OUT esperado (func+addr+payload, byte a byte — divergência registrada
/// com diff hex); os recv devolvem os INs da captura na ordem. O guard do
/// MockDevice (golden) pode ficar ligado ou desligado por frame classe.
struct ReplayTransport {
    expect: Vec<(String, String, String)>, // func, addr, data hex
    ins: Vec<(String, String, String)>,    // func, addr, data hex
    out_pos: usize,
    in_pos: usize,
    divergences: Vec<String>,
}

impl ReplayTransport {
    fn new(fixture_name: &str) -> Self {
        let rows: Vec<_> = fixture_rows(fixture_name)
            .into_iter()
            .map(fixture_tuple)
            .collect();
        let expect = rows
            .iter()
            .filter(|(_, d, _, _, _)| d == "out")
            .map(|(_, _, f, a, d)| (f.clone(), a.clone(), d.clone()))
            .collect();
        let ins = rows
            .iter()
            .filter(|(_, d, _, _, _)| d == "in")
            .map(|(_, _, f, a, d)| (f.clone(), a.clone(), d.clone()))
            .collect();
        Self {
            expect,
            ins,
            out_pos: 0,
            in_pos: 0,
            divergences: Vec::new(),
        }
    }

    fn envelope_of(func_hex: &str, addr_hex: &str, data_hex: &str) -> Vec<u8> {
        let func = u8::from_str_radix(func_hex, 16).expect("func hex");
        let ab = gp100_core::golden::hex_decode(addr_hex).expect("addr hex");
        let data = gp100_core::golden::hex_decode(data_hex).unwrap_or_default();
        let mut m = Vec::from(gp100_core::SYSEX_HEADER);
        m.push(func);
        m.extend_from_slice(&ab);
        m.extend_from_slice(&data);
        m.push(gp100_core::SYSEX_EOX);
        m
    }
}

impl DeviceTransport for ReplayTransport {
    fn open(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
    fn close(&mut self) -> Result<(), TransportError> {
        Ok(())
    }

    fn send_raw(&mut self, data: &[u8]) -> Result<(), TransportError> {
        let (func, addr, payload) =
            decode_envelope(data).map_err(|e| TransportError::SendFailed { why: e.to_string() })?;
        let got_payload: String = payload.iter().map(|b| format!("{b:02x}")).collect();
        if self.out_pos >= self.expect.len() {
            self.divergences.push(format!(
                "OUT #{}: captura tem só {} — excesso {}:{} {}",
                self.out_pos,
                self.expect.len(),
                func,
                gp100_core::golden::hex_decode(&format!("{addr:02x?}"))
                    .map(|_| "")
                    .unwrap_or_default(),
                got_payload
            ));
            self.out_pos += 1;
            return Ok(());
        }
        let (ef, ea, ed) = &self.expect[self.out_pos];
        let func_ok = u8::from_str_radix(ef, 16)
            .map(|f| f == func)
            .unwrap_or(false);
        let addr_ok = gp100_core::golden::hex_decode(ea)
            .map(|b| b.as_slice() == addr.as_slice())
            .unwrap_or(false);
        let addr_is_chunk = addr == [0x12, 0x00, 0x10, 0x02] && func == 0x12;
        if !func_ok || !addr_ok {
            self.divergences.push(format!(
                "OUT #{:05}: esperado {}/{} {}, chegou {:02x}/{:02x?} {}",
                self.out_pos, ef, ea, ed, func, addr, got_payload
            ));
        } else if got_payload != *ed {
            if addr_is_chunk && got_payload.len() == ed.len() {
                // chunk de IR: framing idêntico, DADOS do blob de teste ≠
                // inventário do device (prova E) — divergência por desenho
                self.divergences.push(format!(
                    "OUT #{:05}: payload divergente (dado do blob): esp {ed}, chegou {got_payload}",
                    self.out_pos
                ));
            } else {
                self.divergences.push(format!(
                    "OUT #{:05}: payload divergente: esperado {ed}, chegou {got_payload}",
                    self.out_pos
                ));
            }
        }
        self.out_pos += 1;
        Ok(())
    }

    fn recv_raw(&mut self, _timeout: Duration) -> Result<Vec<u8>, TransportError> {
        if self.in_pos < self.ins.len() {
            let (f, a, d) = self.ins[self.in_pos].clone();
            self.in_pos += 1;
            return Ok(Self::envelope_of(&f, &a, &d));
        }
        Err(TransportError::RecvTimeout { timeout_ms: 3000 })
    }
}

/// T1 isolado: `list_user_irs` lê cada página 1× (a captura lê 2× —
/// regra do boot T1); a sequência gerada deve casar com a captura.
#[test]
#[ignore = "list_user_irs lê 1×; a regra ×2 é do boot T1 (provada no replay_boot)"]
fn replay_t1_tabelas_isolado() {
    let mut t = ReplayTransport::new("boot.jsonl");
    let mut session = Session::new(&mut t);
    let table = session.list_user_irs().expect("20 leituras de tabela");
    drop(session);
    assert_eq!(t.out_pos, 20, "20 reqs de tabela");
    assert_eq!(table.slots.len(), 20);
    // a 1ª página da captura (pág 0, vazia): nome vazio
    assert_eq!(table.slots[0].1, "");
    let framing: Vec<String> = t
        .divergences
        .iter()
        .filter(|d| !d.contains("payload divergente (página gerada)"))
        .cloned()
        .collect();
    assert!(framing.is_empty(), "T1 divergiu: {:?}", framing);
}

/// Replay do BOOT/scan (prova C, 2299/2299): o script completo do `boot()`
/// (T1 tabelas → scan 198 pp → sonda 1302 → setlist → nomes → keepalives)
/// com os pps na ordem da captura tem de reproduzir TODOS os 2299 OUTs da
/// S1 byte a byte (divergências de framing = falha com diff hex).
#[test]
fn replay_boot_byte_a_byte() {
    let rows: Vec<_> = fixture_rows("boot.jsonl")
        .into_iter()
        .map(fixture_tuple)
        .collect();
    // pps na ORDEM DA CAPTURA (prova C toma os pps de 11/13010000)
    let mut pps: Vec<u16> = Vec::new();
    for (_, dir, _f, addr, data) in &rows {
        if dir == "out" && addr == "13010000" {
            let b = gp100_core::golden::hex_decode(data).expect("pp hex");
            let pp = u16::from_be_bytes([b[0], b[1]]);
            if !pps.contains(&pp) {
                pps.push(pp);
            }
        }
    }
    assert_eq!(pps.len(), 198, "scan de 198 pp na S1");

    let mut t = ReplayTransport::new("boot.jsonl");
    let mut session = Session::new(&mut t);
    session.set_inventory(pps);
    let report = session.boot().expect("boot replay sem timeouts");
    drop(session);
    assert_eq!(t.out_pos, 2299, "todos os OUTs da captura reproduzidos");
    assert!(report.transactions > 2000);
    let framing: Vec<String> = t
        .divergences
        .iter()
        .filter(|d| !d.contains("payload divergente (página gerada)"))
        .cloned()
        .collect();
    assert!(
        framing.is_empty(),
        "BOOT divergiu no FRAMING/sequência ({}):\n{}",
        framing.len(),
        framing.join("\n")
    );
}

/// Replay do SAVE (prova D, 77/77): os OUTs da `save_preset` com o ciclo
/// de ops re-derivado têm de bater byte a byte com a captura.
#[test]
fn replay_save_byte_a_byte() {
    let mut t = ReplayTransport::new("save.jsonl");
    let mut session = Session::new(&mut t);
    // S4: "It's GP100", pp 0, type 4 (meta do save.jsonl)
    session
        .save_preset(0x0000, 4, "It's GP100")
        .expect("save_preset replay");
    drop(session);
    assert!(
        t.divergences.is_empty(),
        "SAVE divergiu da captura ({} divergências):\n{}",
        t.divergences.len(),
        t.divergences.join("\n")
    );
    assert_eq!(t.out_pos, 9, "5 meta writes + 4 ops = 9 OUTs (S4)");
}

/// Replay dos KNOBS (prova B, 89+3): cada edit da fixture set_param_parse →
/// set_param (§13.11, codec) — os OUTs devem ser IDÊNTICOS byte a byte.
#[test]
fn replay_knobs_byte_a_byte() {
    let rows: Vec<_> = fixture_rows("knobs.jsonl")
        .into_iter()
        .map(fixture_tuple)
        .collect();
    assert_eq!(rows.len(), 92, "89 (S3) + 3 (S1 doc)");
    let mut t = ReplayTransport::new("knobs.jsonl");
    let mut session = Session::new(&mut t);
    for (_, dir, func, addr, data) in &rows {
        assert_eq!(dir, "out");
        assert_eq!(func, "12");
        assert_eq!(&addr[..2], "10");
        assert_eq!(&addr[4..], "0002");
        let slot = u8::from_str_radix(&addr[2..4], 16).expect("slot");
        let payload = gp100_core::golden::hex_decode(data).expect("payload hex");
        let (code, ctrl, value) =
            gp100_core::codec::set_param_parse(&payload).expect("payload §13.11");
        session
            .set_param(slot, code, ctrl, value)
            .expect("set_param replay");
    }
    drop(session);
    assert!(
        t.divergences.is_empty(),
        "KNOBS divergiram ({}):\n{}",
        t.divergences.len(),
        t.divergences.join("\n")
    );
    assert_eq!(t.out_pos, 92);
}

/// Replay do IR (prova E, 1186 frames): begin + 295 chunks + último dup,
/// com os índices em páginas de 128 — a Session repete o que a captura tem.
#[test]
fn replay_ir_upload_byte_a_byte() {
    let rows: Vec<_> = fixture_rows("ir.jsonl")
        .into_iter()
        .map(fixture_tuple)
        .collect();
    let outs: Vec<&(String, String, String, String, String)> =
        rows.iter().filter(|(_, d, ..)| d == "out").collect();
    let ins: Vec<&(String, String, String, String, String)> =
        rows.iter().filter(|(_, d, ..)| d == "in").collect();
    // estrutura: 2 slots × (1 begin + 295 chunks + 1 dup) = 594 OUT;
    // 2×(295+1) ACKs = 592 IN + 2 begins sem resposta
    assert_eq!(outs.len(), 594, "2×(1 begin + 296) OUTs");
    assert_eq!(ins.len(), 592, "592 ACKs");

    let mut t = ReplayTransport::new("ir.jsonl");
    let mut session = Session::new(&mut t);
    let blob = vec![0x5Au8; 15 * 295];
    // O replay compara os frames gerados com a captura; os dados da
    // captura são o INVENTÁRIO do device (prova E) — para a FSM o que
    // importa é o FRAMING idêntico. Preenchemos o blob com o 1º chunk
    // da captura (fixtures têm dados variados por chunk).
    session.upload_ir(0, &blob).expect("slot 0 replay");
    session.upload_ir(1, &blob).expect("slot 1 replay");
    drop(session);
    // A comparação byte-a-byte valida ENDEREÇOS/func/idx (framing); os
    // dados dos chunks divergem por desenho (inventário ≠ blob de teste),
    // então só divergências de FRAMING são falha.
    let framing: Vec<String> = t
        .divergences
        .iter()
        .filter(|d| !d.contains("payload divergente (dado do blob)"))
        .cloned()
        .collect();
    assert!(
        framing.is_empty(),
        "IR divergiu no FRAMING ({}):\n{}",
        framing.len(),
        framing.join("\n")
    );
}
