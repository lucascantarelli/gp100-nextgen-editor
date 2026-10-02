//! transport — fronteira com o device (ADR-4 de
//! `docs/DECISIONS.md`).
//!
//! **Este módulo é a ÚNICA fronteira de I/O do gp100-core** (ADR-4): a trait
//! [`DeviceTransport`] trafega **bytes crus** — framing do SysEx, nibble e
//! semântica moram no [`crate::codec`] e no golden, nunca aqui.
//! [`Session`] (ADR-6) é genérica sobre a trait; o replay das fixtures
//! não conhece hardware.
//!
//! Implementadores previstos:
//! - `MockDevice` (**default**) — responde conforme o golden; o
//!   contrato de comportamento é D1–D8 do ADR-6 (fila IN por endpoint, ACK
//!   por chunk de IR, pushes de boot; **sem** resync pós-save — D3);
//! - `RealDevice` (midir/WinMM/ALSA) **atrás da feature `real-device`**,
//!   desabilitada por default (política de hardware, ADR-5: escrita real
//!   guardada por `WRITE_VERIFIED`).
//!
//! **Contrato de mensagem** (armadilha de captura, §13.1/knowledge):
//! [`DeviceTransport::recv_raw`] devolve **UMA mensagem completa** — de `F0`
//! até o **1º `F7`**. O trim no 1º F7 (cauda stale dos buffers MIM_LONGDATA
//! do WinMM) é responsabilidade do transporte REAL (watchlist H1 item 2 do
//! ADR-6: buffers de IN ≥ 4KB; SysEx repartido desincroniza D1). O mock
//! devolve as filas verbatim — e `F7` embutido no MEIO de resposta longa
//! (paginação) NUNCA é fim de mensagem: quem fatia é o driver, não a FSM.

pub mod mock;

pub use mock::{MockDevice, MockFault, MockState};

use std::time::Duration;

/// Erro de TRANSPORTE (ADR-4): distinto do [`crate::ProtocolError`] por
/// decisão de ADR-2 — falha de I/O/ciclo de vida não polui o erro de
/// protocolo. `Clone` + `Debug` para acompanhar o `ProtocolError` na
/// superfície da lib (sem `unwrap`/`panic!`).
#[derive(Debug, Clone, thiserror::Error)]
pub enum TransportError {
    /// O device/transporte não está aberto para a operação pedida.
    #[error("transporte fechado (opere open() antes)")]
    Closed,

    /// A abertura do device falhou (device ausente, permissão, busy).
    #[error("falha ao abrir o device: {why}")]
    OpenFailed {
        /// Detalhe humano do motivo (do SO/driver).
        why: String,
    },

    /// O envio falhou (driver rejeitou o buffer, device desconectado).
    #[error("falha no envio: {why}")]
    SendFailed {
        /// Detalhe humano do motivo (do SO/driver).
        why: String,
    },

    /// O device SUMIU no meio da sessão (desconexão física: USB removido,
    /// driver reportou erro definitivo). Distinto de [`SendFailed`]
    /// (transitório) — a FSM/H1 decide diferente para cada um.
    /// SEMÂNTICA DE RECONECTA (decisão de projeto): após `DeviceGone` o
    /// transporte fica no estado FECHADO; chamar [`open`](DeviceTransport::open)
    /// de novo no MESMO objeto reconecta (o handle WinMM/midir é reaberto;
    /// não é preciso recriar o transporte). Já `open()` com o device
    /// fisicamente AUSENTE é [`OpenFailed`] (device ausente), não este erro.
    #[error("device sumiu no meio da sessão: {why}")]
    DeviceGone {
        /// Detalhe humano do motivo (código do SO/driver).
        why: String,
    },

    /// Nenhuma mensagem chegou dentro da janela pedida. A JANELA é da
    /// chamada (ADR-3 usa 3s por transação na FSM); aqui só reportamos.
    #[error("recepção excedeu {timeout_ms} ms")]
    RecvTimeout {
        /// Janela aplicada, em milissegundos (diagnóstico).
        timeout_ms: u64,
    },
}

/// A fronteira com o device (ADR-4): **bytes crus, síncrono e bloqueante**.
///
/// Semântica que TODOS os implementadores devem honrar:
/// - [`send_raw`](DeviceTransport::send_raw) transmite o SysEx COMPLETO
///   (envelope `F0 … F7`) — sem recorte, sem reenvio (D6: retry é política
///   de camada acima, nunca do transporte);
/// - [`recv_raw`](DeviceTransport::recv_raw) devolve **uma** mensagem
///   (`F0 … 1º F7`, ver módulo) ou [`TransportError::RecvTimeout`];
/// - `open`/`close` são idempotentes na prática do mock; o erro de estado
///   é [`TransportError::Closed`] (nunca panic).
///
/// O ciclo de vida é do CHAMADOR (ADR-4/ADR-6: `Session::new` NÃO abre; o
/// CLI/UI decide quando ligar e desligar o device). Desconexão física no
/// meio da sessão = [`TransportError::DeviceGone`]; `open()` de novo no
/// mesmo objeto RECONECTA (decisão de projeto, testada em
/// `tests/transport_trait.rs`).
pub trait DeviceTransport {
    /// Abre o device para transação.
    ///
    /// # Erros
    /// [`TransportError::OpenFailed`] se o device não puder ser aberto.
    fn open(&mut self) -> Result<(), TransportError>;

    /// Fecha o device (mensagens em fila podem ser descartadas).
    ///
    /// # Erros
    /// [`TransportError::Closed`] se já estiver fechado.
    fn close(&mut self) -> Result<(), TransportError>;

    /// Transmite o envelope SysEx completo.
    ///
    /// # Erros
    /// [`TransportError::Closed`] (não aberto) ou
    /// [`TransportError::SendFailed`] (driver/SO recusou).
    fn send_raw(&mut self, data: &[u8]) -> Result<(), TransportError>;

    /// Espera UMA mensagem de entrada até `timeout` e a devolve crua.
    ///
    /// # Erros
    /// [`TransportError::RecvTimeout`] se nada chegar na janela;
    /// [`TransportError::Closed`] se o transporte não estiver aberto.
    fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError>;
}

/// `Box<dyn DeviceTransport>` também é um transporte (dispatch por trait
/// object quando o backend só se decide em runtime — CLI de campo H1 e o
/// DeviceActor da M1).
impl<T: DeviceTransport + ?Sized> DeviceTransport for Box<T> {
    fn open(&mut self) -> Result<(), TransportError> {
        (**self).open()
    }
    fn close(&mut self) -> Result<(), TransportError> {
        (**self).close()
    }
    fn send_raw(&mut self, data: &[u8]) -> Result<(), TransportError> {
        (**self).send_raw(data)
    }
    fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        (**self).recv_raw(timeout)
    }
}

/// `&mut T` também é um transporte (a Session toma empréstimo mutável;
/// usado pelo replay das fixtures e pela UI para manter a posse fora).
impl<T: DeviceTransport + ?Sized> DeviceTransport for &mut T {
    fn open(&mut self) -> Result<(), TransportError> {
        (**self).open()
    }
    fn close(&mut self) -> Result<(), TransportError> {
        (**self).close()
    }
    fn send_raw(&mut self, data: &[u8]) -> Result<(), TransportError> {
        (**self).send_raw(data)
    }
    fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        (**self).recv_raw(timeout)
    }
}

/// `real` — `RealDevice` (H1): USB-MIDI real via midir (WinMM/ALSA/CoreMIDI),
/// ATRÁS da feature `real-device` (default OFF — política de hardware,
/// ADR-4/ADR-5; leitura real = H1, escrita real = pós-H2 WRITE_VERIFIED).
#[cfg(feature = "real-device")]
pub mod real;
