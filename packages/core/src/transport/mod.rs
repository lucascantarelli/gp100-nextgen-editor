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

/// O QUE O FRAME FAZ NO DEVICE — a classificacao que a politica de escrita consome.
///
/// **POR QUE ISTO É PARÂMETRO E NÃO DEDUÇÃO.** A primeira versão deste
/// gate tentou classificar pelo byte FUNC (a tabela do §13.2 diz `0x11` =
/// READ REQUEST, `0x12` = WRITE) e isso está **errado**: o bloco de
/// páginas `13 01 00 02` (abrir) e `13 01 00 04` (avançar PG) são
/// *leituras* que o device responde com uma página de 196B, e ambas viajam
/// com FUNC `0x12`. Um gate por FUNC recusaria o `boot()` inteiro — que é o
/// caminho de LEITURA do H1 — e o resultado seria um gate que barra a
/// leitura, que é o oposto do que se quer.
///
/// Por isso a classificação é **declarada por quem conhece a semântica**
/// (a [`Session`](crate::Session), via o codec/golden) e não inferida pelo
/// transporte: a trait continua sendo a fronteira de BYTES CRUOS
/// (ADR-4), e a semântica fica do lado do codec, como ADR-4 exige.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireKind {
    /// Pedido cujo efeito é o device responder com dados.
    ///
    /// Inclui os dois formatos de leitura que existem no fio: `FUNC 0x11`
    /// sem payload (tabelas de IR, nomes, setlist) **e** `FUNC 0x12` com
    /// payload que abre/avança página (§13.10 — "req pg0..7 → pág1..8").
    Read,
    /// Frame que **muta** o estado do device.
    ///
    /// Os 3 fluxos do gate H2 (§13.11 knob, §13.12 save, §13.7 upload de
    /// IR) e o keepalive de boot `12/00020001` (§13.12: "keepalive/ping no
    /// boot", OUT sem resposta). Nada com `Write` passa num device real
    /// sem a feature `write-verified` (ADR-5).
    Write,
}

impl WireKind {
    /// Nome humano da operação, para a mensagem de recusa.
    pub fn as_str(self) -> &'static str {
        match self {
            WireKind::Read => "leitura",
            WireKind::Write => "escrita",
        }
    }
}

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

    /// Escrita recusada pela política de hardware (ADR-5): o frame é
    /// mutante e este transporte não tem `WRITE_VERIFIED`.
    ///
    /// **ERRO TIPADO, e não string.** A recusa é a ÚNICA prova de que a
    /// política funciona: se fosse um `SendFailed` genérico, um operador
    /// veria "falha no envio" e não saberia que o motivo foi a trava. E o
    /// runbook de campo (H2) precisa distinguir "a trava me protegeu" de
    /// "o driver recusou" — são decisões opostas.
    #[error(
        "escrita bloqueada (ADR-5/WRITE_VERIFIED): {op} precisa de um binário \
         compilado com --features real-device,write-verified"
    )]
    WriteBlocked {
        /// Endereço do frame recusado (addr 4B em hex), para diagnóstico.
        op: String,
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

    /// Transmite o envelope SysEx completo, **classificado** pelo que ele
    /// faz no device.
    ///
    /// `kind` não é decoração: é o que a política de escrita do ADR-5
    /// consome. Um [`WireKind::Write`] num device real sem a feature
    /// `write-verified` é recusado **antes de tocar no driver** — o byte
    /// não sai da máquina.
    ///
    /// Por que o parâmetro é obrigatório e não tem `default`: com um
    /// `default`, o próximo `send_raw(&frame)` de um fluxo de escrita
    /// novo compila e manda bytes de mutação sem passar pela trava. O
    /// compilador é a trava secundária; a primária é o `WireKind::Write`
    /// no código.
    ///
    /// # Erros
    /// [`TransportError::Closed`] (não aberto),
    /// [`TransportError::SendFailed`] (driver/SO recusou) ou
    /// [`TransportError::WriteBlocked`] (escrita sem `write-verified`).
    fn send_raw(&mut self, data: &[u8], kind: WireKind) -> Result<(), TransportError>;

    /// Espera UMA mensagem de entrada até `timeout` e a devolve crua.
    ///
    /// # Erros
    /// [`TransportError::RecvTimeout`] se nada chegar na janela;
    /// [`TransportError::Closed`] se o transporte não estiver aberto.
    fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError>;

    /// Este transporte deixa ESCREVER? — a pergunta que o `boot()` faz antes
    /// do keepalive (ADR-5, decisão do owner de 06/10).
    ///
    /// **Por que existe.** O script de boot termina no ping `12/00020001`,
    /// que é `WireKind::Write`. Com a trava fechada o transporte o recusaria
    /// e o boot inteiro falharia no ÚLTIMO frame — ou seja, um build de
    /// LEITURA não teria leitura. O `Session::boot` pergunta aqui e **omite**
    /// o ping quando a resposta é `false`, em vez de mandar e ser barrado.
    /// (A alternativa que ficou registrada em `tests/write_gate.rs` era
    /// exigir `write-verified` no passo B5 do H1.)
    ///
    /// **Por que a pergunta vai ao transporte e não ao `cfg!(feature)`.** A
    /// trava é da build, mas quem sabe se o byte passa é o objeto que vai
    /// enviá-lo — e o `MockDevice` vive numa build SEM a feature e mesmo
    /// assim permite escrita (ADR-5: "mock SEMPRE permite", senão os replays
    /// das fixtures não existiriam). Perguntar ao `cfg` no core pularia o
    /// keepalive do mock e mudaria o boot de 2297 transações para 2295.
    ///
    /// `true` (default) = os `Write` saem; `false` = este transporte
    /// recusaria qualquer um deles.
    fn permite_escrita(&self) -> bool {
        true
    }

    /// Este transporte fala com o **aparelho físico**? — a pergunta que a
    /// trava de faixa do `pp` (#132) faz antes de deixar um `select` sair.
    ///
    /// **Por que existe.** O espaço de `pp` do GP-100 é MEDIDO (captura S1:
    /// `0x0000..=0x0062` + `0x0100..=0x0162`, 198 pps) e mandar um `select`
    /// fora dele é o tipo de coisa que asserta o firmware
    /// (`Drivers/audio/audio.c:912`, `PresetNum < TOTAL_PA` — issue #132).
    /// O mock, em contrapartida, aceita qualquer `pp` por desenho: ele
    /// DERIVA de `all.prst` e os testes varrem pps que o aparelho não tem.
    /// Então a recusa é do aparelho — o mock mantém o comportamento de
    /// sempre (mesma lógica da face A da #126, com o `select`).
    ///
    /// **Por que a pergunta vai ao transporte e não ao `cfg!(feature)`.**
    /// Mesma razão de [`permite_escrita`](Self::permite_escrita): quem sabe
    /// com quem a sessão conversa é o objeto que vai enviar o frame — o
    /// `MockDevice` vive na MESMA build que o `RealDevice` (os testes do
    /// app misturam os dois), e um falso-aparelho de teste precisa poder
    /// dizer "sou aparelho" para exercitar a trava.
    ///
    /// `true` = [`Session::select_preset`](crate::session::Session::select_preset)
    /// confere o `pp` no inventário ANTES do frame (#132/ADR-12); `false`
    /// (default) = sem trava, como sempre.
    fn e_aparelho(&self) -> bool {
        false
    }
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
    fn send_raw(&mut self, data: &[u8], kind: WireKind) -> Result<(), TransportError> {
        (**self).send_raw(data, kind)
    }
    fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        (**self).recv_raw(timeout)
    }
    fn permite_escrita(&self) -> bool {
        (**self).permite_escrita()
    }
    fn e_aparelho(&self) -> bool {
        (**self).e_aparelho()
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
    fn send_raw(&mut self, data: &[u8], kind: WireKind) -> Result<(), TransportError> {
        (**self).send_raw(data, kind)
    }
    fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        (**self).recv_raw(timeout)
    }
    fn permite_escrita(&self) -> bool {
        (**self).permite_escrita()
    }
    fn e_aparelho(&self) -> bool {
        (**self).e_aparelho()
    }
}

/// `real` — `RealDevice` (H1): USB-MIDI real via midir (WinMM/ALSA/CoreMIDI),
/// ATRÁS da feature `real-device` (default OFF — política de hardware,
/// ADR-4/ADR-5; leitura real = H1, escrita real = pós-H2 WRITE_VERIFIED).
#[cfg(feature = "real-device")]
pub mod real;
