//! Testes de CONTRATO do envelope de fio (PROTOCOL.md §13.1) — os 3 testes
//! unitários herdados do P2 em `lib.rs`, movidos para caixa-preta: usam
//! exclusivamente API pública (`SYSEX_HEADER`, `SYSEX_EOX`, golden embutido),
//! então o lugar deles é `tests/` (regra da skill rust-practices: só API
//! pub → `tests/`; internals → `#[cfg(test)]` dentro de `src/`).

use gp100_core::{SYSEX_EOX, SYSEX_HEADER};

/// O cabeçalho em hex maiúsculo bate com o §13.1 do PROTOCOL.md (e com o
/// golden — prova A explica 100% das mensagens IN com este envelope).
#[test]
fn sysex_header_matches_golden() {
    assert_eq!(
        SYSEX_HEADER
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>(),
        "F021257F47502D64"
    );
}

/// Forma do envelope mínimo de leitura: HEADER + FUNC + ADDR(u32 BE) + EOX.
///
/// Usa um request REAL do golden: READ da tabela de nomes `11000008`
/// (§13.12) — payload de request vazio. O ADDR é u32 BIG-ENDIAN
/// (ADR-1: pp/PG/endereços BE no fio).
#[test]
fn minimal_read_request_shape() {
    let mut msg = Vec::from(SYSEX_HEADER);
    msg.push(0x11); // FUNC: READ request (§13.1)
    msg.extend_from_slice(&0x1100_0008u32.to_be_bytes()); // ADDR 4B BE
    msg.push(SYSEX_EOX);

    assert_eq!(msg.len(), 8 + 1 + 4 + 1, "header + func + addr + eox");
    assert_eq!(&msg[8..9], &[0x11]);
    assert_eq!(&msg[9..13], &[0x11, 0x00, 0x00, 0x08]);
    assert_eq!(msg[13], SYSEX_EOX);
}

/// Smoke caixa-preta do crate compilado como EXTERNO (a diferença real do
/// antigo `smoke` `2+2` do P2): o golden EMBUTIDO carrega e expõe templates
/// pela API pública — prova linkage + `include_str!` do ponto de vista de
/// um consumidor qualquer do gp100-core.
#[test]
fn embedded_golden_smoke() {
    let golden = gp100_core::golden::GoldenFile::embedded().expect("golden embutido carrega");
    assert!(
        !golden.templates().is_empty(),
        "golden embedded deve expor templates"
    );
}
