//! gp100-core — núcleo do GP-100 NextGen Editor (RE do Valeton Suite).
//!
//! **Arquitetura:** este crate concentra TODA a lógica de protocolo e presets;
//! a UI (packages/app) e o CLI ([gp100-cli](../../gp100-cli)) apenas orquestram.
//! Pipeline de módulos:
//!
//! ```text
//! model → preset → golden → codec
//!      → transport (trait DeviceTransport + MockDevice)
//!      → session  (FSM com replay de analysis/fixtures/)
//! ```
//!
//! **Fontes de verdade** — regra de ouro R1: NUNCA adivinhar protocolo.
//! - [docs/protocol_golden.json](../../docs/protocol_golden.json) — especificação
//!   executável (40 templates; Baseline v1.0 com sha256 no §13 do PROTOCOL.md);
//! - [docs/PROTOCOL.md](../../docs/PROTOCOL.md) §13 — narrativa de campo do fio
//!   (SysEx Valeton; envelope em §13.1);
//! - [docs/DECISIONS.md](../../docs/DECISIONS.md) — ADR-1..5 pré-assinados
//!   (endian/nibble, `ProtocolError`, timeout 3s, trait `DeviceTransport`,
//!   `WRITE_VERIFIED`);
//! - [analysis/fixtures/](../../analysis/fixtures) — replay por fase
//!   (boot/knobs/save/ir), paridade travada no pytest.
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
/// (paginação, NÃO fim de mensagem). A regra do codec é **trim no 1º
/// `F7`** — já aplicada pelos loaders Python (`build_golden.py`) e, portanto,
/// nas fixtures do P4 consumidas pelos testes de replay.
pub const SYSEX_EOX: u8 = 0xF7;

/// Erro de PROTOCOLO tipado (ADR-2 de `docs/DECISIONS.md`): toda falha de
/// framing/timeout/FSM desta crate é esta enum — nunca `unwrap`/`panic!`.
///
/// Erros de I/O do transporte ficarão num `TransportError` separado (ADR-4); variantes novas entram aqui por decisão registrada (novo ADR).
#[derive(Debug, Clone, thiserror::Error)]
pub enum ProtocolError {
    /// Mensagem fora do formato esperado: comprimento errado, segmentos
    /// `const` divergentes, payload incoerente. Cobrí inclusive o
    /// truncamento do ring buffer do proxy (SEM-HDR) — que é **dado
    /// conhecido** das capturas, não bug de decode (fixtures já excluem).
    #[error("shape inválido: esperado {expected}, obtido {got}")]
    InvalidShape {
        /// Descrição do formato esperado (human-readable, com evidência).
        expected: String,
        /// O que de fato chegou (hex/len/mensagem de parser).
        got: String,
    },

    /// Resposta não chegou dentro da janela da transação (ADR-3: 3000 ms
    /// por transação, mesmo valor do pairing do golden).
    #[error("timeout de {timeout_ms} ms na transação {addr}")]
    Timeout {
        /// Janela aplicada, em milissegundos (diagnóstico).
        timeout_ms: u64,
        /// Endereço (func+addr) da transação que estourou.
        addr: String,
    },

    /// Resposta/ACK chegou com conteúdo fora do esperado pela FSM:
    /// ecos errados, status incoerente, ACK de chunk inválido (§13.7).
    #[error("ack inesperado na transação {addr}: {got}")]
    UnexpectedAck {
        /// Endereço (func+addr) da transação.
        addr: String,
        /// O que de fato chegou (hex curto).
        got: String,
    },
}

/// model — dicionário de algoritmos/controles.
pub mod model;

/// preset — arquivo `.prst` com round-trip byte-idêntico (regra R4).
pub mod preset;

/// golden — consumidor da especificação executável.
pub mod golden;

/// codec — codificação do fio: nibble + helpers semânticos.
pub mod codec;

/// transport — fronteira de I/O com o device: trait `DeviceTransport`
/// (ADR-4; implementadores: `MockDevice`/`RealDevice`).
pub mod transport;

/// session — FSM de sessão (ADR-6 rev.3: assinaturas D1–D8
/// travadas; replay byte-a-byte das fixtures prova a FSM).
pub mod session;

/// pedalboard — projeção de preset → view de board (dados puros p/ a UI
/// do app: arquétipos por família, knobs do dicionário, biblioteca).
pub mod pedalboard;

// Os testes dos herdados do P2 (header/envelope §13.1) usam SÓ API pública,
// então vivem como contratos caixa-preta em `tests/wire_envelope.rs` (regra
// da skill rust-practices: unitário dentro de `src/` só para internals).
