//! gp100-core — núcleo do GP-100 NextGen Editor (RE do Valeton Suite).
//!
//! **Papel no ROADMAP** ([docs/ROADMAP.md](../../docs/ROADMAP.md), Fase M0): este
//! crate concentra TODA a lógica de protocolo e presets; a UI (M1) e o CLI
//! ([gp100-cli](../../gp100-cli), M0.7) apenas orquestram. Pipeline de fases:
//!
//! ```text
//! model (M0.1) → preset (M0.2) → golden (M0.3) → codec (M0.4)
//!      → transport (M0.5: trait DeviceTransport + MockDevice)
//!      → session  (M0.6: FSM com replay de analysis/fixtures/)
//! ```
//!
//! **Fontes de verdade** — regra de ouro R1 do ROADMAP: NUNCA adivinhar protocolo.
//! - [docs/protocol_golden.json](../../docs/protocol_golden.json) — especificação
//!   executável (40 templates; Baseline v1.0 com sha256 no §13 do PROTOCOL.md);
//! - [docs/PROTOCOL.md](../../docs/PROTOCOL.md) §13 — narrativa de campo do fio
//!   (SysEx Valeton; envelope em §13.1);
//! - [docs/DECISIONS.md](../../docs/DECISIONS.md) — ADR-1..5 pré-assinados
//!   (endian/nibble, `ProtocolError`, timeout 3s, trait `DeviceTransport`,
//!   `WRITE_VERIFIED`);
//! - [analysis/fixtures/](../../analysis/fixtures) — replay por fase
//!   (boot/knobs/save/ir, ROADMAP P4), paridade travada no pytest.
//!
//! **Qualidade** (skill [.agents/skills/rust-practices](../../.agents/skills/rust-practices/SKILL.md)):
//! `cargo fmt --check`, `clippy -D warnings` e `cargo test` (que INCLUI os
//! doc-tests — exemplo de documentação que mente quebra o build). Este crate
//! compila com `#![deny(missing_docs)]`: item público sem doc = erro.
//!
//! **Armadilha de host** (knowledge.md): toolchain PINADA em
//! `stable-x86_64-pc-windows-gnu` (`rust-toolchain.toml`) — o host não tem MSVC
//! Build Tools; não trocar de alvo nem apagar esse arquivo.

#![deny(missing_docs)]

/// Cabeçalho fixo de TODO SysEx do GP-100 no fio (PROTOCOL.md §13.1).
///
/// Byte a byte — apenas o que está EVIDENCIADO (R1: sem chute):
///
/// | # | byte | significado |
/// |---|--------|-------------|
/// | 0 | `F0`   | SysEx Start (status MIDI padrão) |
/// | 1–3 | `21 25 7F` | assinatura fixa observada em 100% das mensagens das 4 sessões (semântica de fabricante não confirmada por RE — consultar antes de assumir) |
/// | 4–7 | `47 50 2D 64` | `"GP-d"` em ASCII — assinatura do GP-100 |
///
/// Formato completo do envelope: `HEADER | FUNC(1) | ADDR(4B BE) | DATA | F7`,
/// com FUNC `0x11` = READ request e `0x12` = dados/write (§13.1).
///
/// # Exemplo (doc-test executado por `cargo test`)
///
/// ```
/// // O byte inicial é sempre um SysEx Start...
/// assert_eq!(gp100_core::SYSEX_HEADER[0], 0xF0);
/// // ...e o final do cabeçalho é a assinatura ASCII "GP-d" do GP-100.
/// assert_eq!(&gp100_core::SYSEX_HEADER[4..], &b"GP-d"[..]);
/// ```
pub const SYSEX_HEADER: [u8; 8] = [0xF0, 0x21, 0x25, 0x7F, 0x47, 0x50, 0x2D, 0x64];

/// Fim de mensagem SysEx (EOX, `F7`). TODO envelope termina com este byte.
///
/// ⚠️ **Armadilha de captura** (knowledge.md): os buffers MIM_LONGDATA do proxy
/// chegam com cauda stale e respostas longas contêm `F7` EMBUTIDO no meio
/// (paginação, NÃO fim de mensagem). A regra do codec (M0.4) é **trim no 1º
/// `F7`** — já aplicada pelos loaders Python (`build_golden.py`) e, portanto,
/// nas fixtures do P4 consumidas pelos testes de replay.
pub const SYSEX_EOX: u8 = 0xF7;

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn smoke() {
        assert_eq!(2 + 2, 4);
    }
}
