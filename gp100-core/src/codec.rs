//! codec — codificação/decodificação do fio GP-100 (ROADMAP M0.4).
//!
//! **Regras de largura/endian (ADR-1, evidência no golden/§13):** pp/PG/CRC
//! e endereços **BE**; `effectCode` e float de valor **LE**; payloads de
//! objeto **nibble-expandidos** (hi primeiro). Este módulo materializa essas
//! regras em primitivas + helpers semânticos com EVIDÊNCIA DIRETA nas
//! fixtures P4:
//!
//! - [`set_param`] — knob da UI (§13.11): payload 20B nibble-exp =
//!   `[effectCode u32 LE][ctrl u8][00][f32 LE]`; addr `10 [slot 1..9] 00 02`.
//!   Vetor: fixture `knobs.jsonl` linha 1 = slot 3, code `0700006e`, ctrl 0,
//!   valor 15.0 (`Bog RedM`).
//! - [`meta_block`] — metadados do save (§13.12): 5 writes
//!   (`11000000` zeros4+pp BE+zeros2+nome ASCII 12B · `11000004` 20B zeros ·
//!   `11000005` ppType BE · `11000007` 50B zeros · `12000002` zeros4+pp+zeros2).
//!   Vetor: `save.jsonl` (S4 "It's GP100", pp 0, type 4).
//! - [`op`] — ops `00020000` `[4..5]`= op u16 BE (0 sai/1 entra, §13.12).
//! - [`ir_begin`]/[`ir_chunk`]/[`ir_chunk_ack`] — upload de IR (§13.7):
//!   begin cru `00 [slot] 00 00 01 00 00 0a` (ANTES dos chunks); chunk 33B
//!   `[slot][idx u16 BE]+30B nibble` (15B reais por chunk; último idx dup —
//!   regra da FSM M0.6, não do codec); ACK `[slot][idx][01]`.
//!   Vetores: `ir.jsonl` (592 chunks + 592 ACKs, final idx 0x0226 slot 1).
//!
//! **Não fica aqui:** regras de SEQUÊNCIA da FSM (ordem dos writes, quirk de
//! re-leitura, índice de página com salto) — isso é o M0.6. O codec é puro:
//! mesmo input → mesmos bytes. Consistência codec ↔ golden é provada em
//! `tests/codec_wire.rs` (os writes semânticos também existem como templates).

use crate::golden::hex_decode;
use crate::{ProtocolError, SYSEX_EOX, SYSEX_HEADER};

/// Converte addr hex de 8 dígitos em `[u8; 4]` (helper local sobre o golden).
fn addr4(hex: &str) -> Result<[u8; 4], ProtocolError> {
    let b = hex_decode(hex)?;
    if b.len() != 4 {
        return Err(crate::ProtocolError::InvalidShape {
            expected: "addr com 4 bytes".into(),
            got: format!("hex '{hex}' decodifica {} bytes", b.len()),
        });
    }
    Ok([b[0], b[1], b[2], b[3]])
}

/// Monta o SysEx COMPLETO (header §13.1 + func + addr 4B BE + payload + F7).
fn envelope(func: u8, addr: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(SYSEX_HEADER.len() + 1 + 4 + payload.len() + 1);
    out.extend_from_slice(&SYSEX_HEADER);
    out.push(func);
    out.extend_from_slice(&addr);
    out.extend_from_slice(payload);
    out.push(SYSEX_EOX);
    out
}

/// Nibble-expande (hi primeiro): `0x6E → [0x06, 0x0E]`. Regra ADR-1 dos
/// payloads de objeto (§13.11/§13.12/§13.7).
pub fn nibble_expand(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 2);
    for &b in data {
        out.push(b >> 4);
        out.push(b & 0x0F);
    }
    out
}

/// Inverso de [`nibble_expand`]: repara os bytes reais.
///
/// # Erros
/// [`ProtocolError::InvalidShape`] se algum byte > 0x0F (payload não é
/// nibble-expandido) ou o comprimento é ímpar. Strict de propósito:
/// mascarar bits fora do nibble esconderia corrupção.
pub fn nibble_collapse(data: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    if !data.len().is_multiple_of(2) {
        return Err(ProtocolError::InvalidShape {
            expected: "comprimento par (pares de nibble)".into(),
            got: format!("len={}", data.len()),
        });
    }
    let mut out = Vec::with_capacity(data.len() / 2);
    for pair in data.as_chunks::<2>().0 {
        if pair[0] > 0x0F || pair[1] > 0x0F {
            return Err(ProtocolError::InvalidShape {
                expected: "bytes de nibble (0x00..=0x0F)".into(),
                got: format!("par {:02x} {:02x}", pair[0], pair[1]),
            });
        }
        out.push((pair[0] << 4) | pair[1]);
    }
    Ok(out)
}

/// Payload do SET de parâmetro (§13.11): 20B nibble-exp de
/// `[effectCode u32 LE][ctrl u8][00][f32 LE]`.
///
/// # Erros
/// [`ProtocolError::InvalidShape`] se `slot` fora de 1..=9 (cadeia de 9).
pub fn set_param_payload(
    slot: u8,
    code: u32,
    ctrl: u8,
    value: f32,
) -> Result<Vec<u8>, ProtocolError> {
    if !(1..=9).contains(&slot) {
        return Err(ProtocolError::InvalidShape {
            expected: "slot 1..=9 (posição na cadeia, §13.11)".into(),
            got: format!("slot={slot}"),
        });
    }
    let mut real = Vec::with_capacity(10);
    real.extend_from_slice(&code.to_le_bytes()); // effectCode u32 LE
    real.push(ctrl);
    real.push(0x00); // separador constante nas 92 amostras
    real.extend_from_slice(&value.to_le_bytes()); // f32 LE
    Ok(nibble_expand(&real))
}

/// SysEx COMPLETO do set-param: `… | 12 | 10 [slot] 00 02 | payload | F7`.
pub fn set_param(slot: u8, code: u32, ctrl: u8, value: f32) -> Result<Vec<u8>, ProtocolError> {
    let payload = set_param_payload(slot, code, ctrl, value)?;
    Ok(envelope(
        0x12,
        addr4(&format!("10{slot:02x}0002"))?,
        &payload,
    ))
}

/// Decodifica o payload de set-param: devolve `(code, ctrl, value)`.
///
/// # Erros
/// [`ProtocolError::InvalidShape`] se não são 20 nibbles ou o separador
/// `[5]` não é 0x00 (invariante das 92 amostras).
pub fn set_param_parse(payload: &[u8]) -> Result<(u32, u8, f32), ProtocolError> {
    let real = nibble_collapse(payload)?;
    if real.len() != 10 || real[5] != 0x00 {
        return Err(ProtocolError::InvalidShape {
            expected: "10 bytes reais com [5]==00 (§13.11)".into(),
            got: format!(
                "len={} b5={:02x}",
                real.len(),
                real.get(5).copied().unwrap_or(0)
            ),
        });
    }
    let code = u32::from_le_bytes([real[0], real[1], real[2], real[3]]);
    let value = f32::from_le_bytes([real[6], real[7], real[8], real[9]]);
    Ok((code, real[4], value))
}

/// Um write de objeto: `(addr 4B BE, payload)`. Forma canônica dos blocos
/// que o codec gera (meta do save, IR) e que a FSM envia em sequência.
pub type WireWrite = ([u8; 4], Vec<u8>);

/// Nome ASCII em 12B (trunc + pad `\0`), forma do `11000000` (§13.12).
///
/// # Erros
/// [`ProtocolError::InvalidShape`] se o nome tem bytes não-ASCII imprimíveis.
fn name_field(name: &str) -> Result<[u8; 12], ProtocolError> {
    if !name.bytes().all(|b| (0x20..=0x7E).contains(&b)) {
        return Err(ProtocolError::InvalidShape {
            expected: "nome ASCII imprimível (fio guarda ASCII cru, §13.12)".into(),
            got: name.chars().take(20).collect(),
        });
    }
    let mut out = [0u8; 12];
    for (i, b) in name.bytes().take(12).enumerate() {
        out[i] = b;
    }
    Ok(out)
}

/// O bloco de METADADOS do save (§13.12): os 5 writes `(addr, payload)` NA
/// ORDEM capturada (save.jsonl S4/S2). `pp` e `pp_type` vêm do `.prst`
/// (ppID/ppType — provado pela prova D do validate_golden).
///
/// # Erros
/// [`ProtocolError::InvalidShape`] se o nome não é ASCII (ver [`name_field`]).
pub fn meta_block(pp: u16, pp_type: u16, name: &str) -> Result<Vec<WireWrite>, ProtocolError> {
    let nb = name_field(name)?;
    // 11000000: zeros4 + pp u16 BE + zeros2 + nome 12B
    let mut m0 = vec![0u8; 4];
    m0.extend_from_slice(&pp.to_be_bytes());
    m0.extend_from_slice(&[0, 0]);
    m0.extend_from_slice(&nb);
    // 12000002: zeros4 + pp u16 BE + zeros2 (fecha o bloco)
    let mut m5 = vec![0u8; 4];
    m5.extend_from_slice(&pp.to_be_bytes());
    m5.extend_from_slice(&[0, 0]);
    Ok(vec![
        (addr4("11000000")?, m0),
        (addr4("11000004")?, vec![0u8; 20]), // reservado
        (addr4("11000005")?, {
            // ppType u32: BE u16 + zeros2
            let mut v = pp_type.to_be_bytes().to_vec();
            v.extend_from_slice(&[0, 0]);
            v
        }),
        (addr4("11000007")?, vec![0u8; 50]), // reservado
        (addr4("12000002")?, m5),
    ])
}

/// Payload da op `00020000` (§13.12): `[4..5]` = nº da op u16 BE
/// (0 = sair/commit, 1 = entrar modo edição; capturado ×2 cada no S4).
/// Evidência byte-exata da fixture `save.jsonl`: `0000000000010000` →
/// bytes `[4..5]` = `00 01` = 1 em **BE** (LE daria 256 — nunca observado;
/// ADR-1: campos de endereçamento/globais são BE).
pub fn op_payload(op: u8) -> Vec<u8> {
    let mut out = vec![0u8; 4];
    out.extend_from_slice(&(op as u16).to_be_bytes());
    out.extend_from_slice(&[0, 0]);
    out
}

/// Payload do BEGIN/reserva de upload de IR (§13.7): cru,
/// `00 [slot] 00 00 01 00 00 0a` — vem ANTES dos chunks e NÃO é commit.
///
/// # Erros
/// [`ProtocolError::InvalidShape`] se `slot` >= 20 (tabela de 20 User IRs).
pub fn ir_begin_payload(slot: u8) -> Result<Vec<u8>, ProtocolError> {
    if slot >= 20 {
        return Err(ProtocolError::InvalidShape {
            expected: "slot 0..=19 (20 User IRs, §13.12)".into(),
            got: format!("slot={slot}"),
        });
    }
    Ok(vec![0x00, slot, 0x00, 0x00, 0x01, 0x00, 0x00, 0x0A])
}

/// SysEx COMPLETO do BEGIN (`12 | 10050001 | payload | F7`).
pub fn ir_begin(slot: u8) -> Result<Vec<u8>, ProtocolError> {
    Ok(envelope(0x12, addr4("10050001")?, &ir_begin_payload(slot)?))
}

/// Payload do CHUNK de IR (§13.7): `[slot][idx u16 BE] + 30B nibble-exp`
/// de `data` (15 bytes reais do blob de IR por chunk).
///
/// # Erros
/// [`ProtocolError::InvalidShape`] se `data` != 15B ou slot >= 20.
pub fn ir_chunk_payload(slot: u8, idx: u16, data: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    if slot >= 20 {
        return Err(ProtocolError::InvalidShape {
            expected: "slot 0..=19 (20 User IRs)".into(),
            got: format!("slot={slot}"),
        });
    }
    if data.len() != 15 {
        return Err(ProtocolError::InvalidShape {
            expected: "15 bytes reais de blob por chunk (30 nibbles no fio)".into(),
            got: format!("{} bytes", data.len()),
        });
    }
    let mut out = Vec::with_capacity(33);
    out.push(slot);
    out.extend_from_slice(&idx.to_be_bytes());
    out.extend(nibble_expand(data));
    Ok(out)
}

/// SysEx COMPLETO do chunk (`12 | 12001002 | payload | F7`).
pub fn ir_chunk(slot: u8, idx: u16, data: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    Ok(envelope(
        0x12,
        addr4("12001002")?,
        &ir_chunk_payload(slot, idx, data)?,
    ))
}

/// Decodifica o payload de chunk: `(slot, idx, data_reais)` — colapsa os
/// 30 nibbles de volta aos 15 bytes do blob.
///
/// # Erros
/// [`ProtocolError::InvalidShape`] se o payload não tem 33B.
pub fn ir_chunk_parse(payload: &[u8]) -> Result<(u8, u16, Vec<u8>), ProtocolError> {
    if payload.len() != 33 {
        return Err(ProtocolError::InvalidShape {
            expected: "payload de 33B ([slot][idx BE][30 nibbles])".into(),
            got: format!("{} bytes", payload.len()),
        });
    }
    let slot = payload[0];
    let idx = u16::from_be_bytes([payload[1], payload[2]]);
    let data = nibble_collapse(&payload[3..])?;
    Ok((slot, idx, data))
}

/// Payload do ACK de chunk (IN): `[slot][idx u16 BE][01]` — eco + flag ok.
pub fn ir_chunk_ack_payload(slot: u8, idx: u16) -> [u8; 4] {
    [slot, (idx >> 8) as u8, (idx & 0xFF) as u8, 0x01]
}

/// Envelopa um WRITE na forma canônica [`WireWrite`] → frame completo
/// (§13.1): `HEADER | 12 | ADDR(4B) | DATA | F7`. PONTO ÚNICO de montagem
/// de envelope de write (ADR-6: binários não montam SysEx; a FSM e o
/// dry-run do CLI imprimem o que ESTA função produz).
pub fn write_frame(addr: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut m = Vec::with_capacity(SYSEX_HEADER.len() + 5 + payload.len() + 1);
    m.extend_from_slice(&SYSEX_HEADER);
    m.push(0x12);
    m.extend_from_slice(addr);
    m.extend_from_slice(payload);
    m.push(SYSEX_EOX);
    m
}

/// Desmonta um write nas partes (§13.1), o inverso exato de [`write_frame`].
pub fn write_parse(frame: &[u8]) -> Result<([u8; 4], &[u8]), ProtocolError> {
    let (func, addr, payload) = crate::golden::decode_envelope(frame)?;
    if func != 0x12 {
        return Err(ProtocolError::InvalidShape {
            expected: "write (func 12)".into(),
            got: format!("func {func:02x}"),
        });
    }
    Ok((addr, payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nibble round-trip com o vetor do knob real (Bog RedM @ 15.0).
    #[test]
    fn nibble_roundtrip() {
        let real = [0x6e, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00, 0x70, 0x41];
        let exp = nibble_expand(&real);
        assert_eq!(exp.len(), 20);
        assert_eq!(exp[..4], [0x06, 0x0E, 0x00, 0x00]);
        assert_eq!(nibble_collapse(&exp).unwrap(), real);
    }

    /// Collapse strict: nybble > 0x0F e comprimento ímpar rejeitados.
    #[test]
    fn nibble_collapse_is_strict() {
        assert!(nibble_collapse(&[0x0F, 0x1F]).is_err());
        assert!(nibble_collapse(&[0x01]).is_err());
    }

    /// set_param: envelope + payload do vetor real da fixture knobs (linha 1).
    #[test]
    fn set_param_vector() {
        let sysex = set_param(3, 0x0700_006e, 0, 15.0).expect("vetor válido");
        assert_eq!(&sysex[9..13], &[0x10, 0x03, 0x00, 0x02]);
        assert_eq!(
            &sysex[13..33],
            &[
                0x06, 0x0e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x07, 0x00, 0x04, 0x01
            ]
        );
        assert_eq!(sysex[33], 0xF7);
        // parse devolve o trio original
        let (code, ctrl, value) = set_param_parse(&sysex[13..33]).expect("parse");
        assert_eq!((code, ctrl), (0x0700_006e, 0));
        assert_eq!(value, 15.0);
    }

    /// set_param rejeita slot fora de 1..=9 (cadeia de 9, §13.11).
    #[test]
    fn set_param_slot_range() {
        assert!(set_param(0, 0, 0, 0.0).is_err());
        assert!(set_param(10, 0, 0, 0.0).is_err());
    }

    /// meta_block: os 5 writes do vetor real ("It's GP100", pp 0, type 4).
    #[test]
    fn meta_block_vector() {
        let block = meta_block(0, 4, "It's GP100").expect("nome ASCII");
        assert_eq!(block.len(), 5);
        let (a0, p0) = &block[0];
        assert_eq!(a0, &addr4("11000000").unwrap());
        assert_eq!(p0.len(), 20);
        assert_eq!(&p0[8..], b"It's GP100\0\0");
        assert_eq!(block[1].1, vec![0u8; 20]);
        assert_eq!(block[2].1, [0x00, 0x04, 0x00, 0x00]);
        assert_eq!(block[3].1, vec![0u8; 50]);
        assert_eq!(block[4].0, addr4("12000002").unwrap());
        assert_eq!(block[4].1, vec![0u8; 8]);
        // nome não-ASCII rejeitado (fio guarda ASCII)
        assert!(meta_block(0, 4, "Café").is_err());
    }

    /// ir_begin nos 2 vetores reais (slots 0 e 1 da fixture ir.jsonl).
    #[test]
    fn ir_begin_vector() {
        assert_eq!(
            ir_begin_payload(0).unwrap(),
            [0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x0A]
        );
        assert_eq!(
            ir_begin_payload(1).unwrap(),
            [0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x0A]
        );
        assert!(ir_begin_payload(20).is_err());
        let sysex = ir_begin(1).expect("ok");
        assert_eq!(&sysex[9..13], &[0x10, 0x05, 0x00, 0x01]);
    }

    /// ir_chunk/ack: vetor do chunk[0] da fixture (slot 0, idx 0) e ACK.
    #[test]
    fn ir_chunk_vector() {
        let data = [
            0x07u8, 0x04, 0x06, 0x05, 0x07, 0x03, 0x07, 0x04, 0x05, 0x0F, 0x06, 0x09, 0x07, 0x02,
            0x05,
        ];
        // (primeiros 15 bytes reais do blob: derivados do próprio fio — ver
        // teste de fixture em tests/codec_wire.rs que usa o chunk original)
        let payload = ir_chunk_payload(0, 0, &data).expect("ok");
        assert_eq!(payload.len(), 33);
        assert_eq!(&payload[..3], &[0x00, 0x00, 0x00]);
        let (slot, idx, back) = ir_chunk_parse(&payload).expect("parse");
        assert_eq!((slot, idx), (0, 0));
        assert_eq!(back, data.to_vec());
        assert_eq!(ir_chunk_ack_payload(0, 0), [0x00, 0x00, 0x00, 0x01]);
        assert!(ir_chunk_payload(20, 0, &data).is_err());
        assert!(ir_chunk_payload(0, 0, &data[..14]).is_err());
    }

    /// op_payload: vetores REAIS das fixtures de save (S4 = ciclo de ops
    /// 0,0 → 1,1 re-derivado; §13.12 + D3 do ADR-6). Fecha o gap registrado
    /// no knowledge (29/09): o BE de `[4..5]` fica travado contra fixture.
    #[test]
    fn op_payload_vector() {
        // op 0 = sair/commit: `0000000000000000` (S4, ×2)
        assert_eq!(
            op_payload(0),
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        // op 1 = entrar modo edição: `0000000000010000` (S4, ×2)
        assert_eq!(
            op_payload(1),
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00]
        );
        // o par [4..5] é o nº da op em BE — LE produziria `01 00` (256 LE ≠ 1)
        assert_eq!(&op_payload(1)[4..6], &[0x00, 0x01]);
        assert_eq!(&op_payload(0)[4..6], &[0x00, 0x00]);
    }
}
