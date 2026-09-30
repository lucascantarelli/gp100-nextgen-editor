//! real — `RealDevice` (feature `real-device`, gate H1): o transporte
//! USB-MIDI REAL para a pedaleira, via **midir** (WinMM no Windows — o mesmo
//! stack que o proxy instrumenta; ALSA/CoreMIDI nos demais SOs).
//!
//! **Política de hardware (não negociável — ADR-4/ADR-5, VISION §7, regra 8 do
//! `core-dev`):** o transporte entrega bytes (papel da trait ADR-4); o USO de
//! escrita real fica nas camadas de política — CLI (dupla confirmação) e gate
//! H2 (`WRITE_VERIFIED`, BLOCKERS §4). Nada aqui bypassa o ADR-6: a FSM é a
//! mesma do mock e o H1 é SÓ LEITURA por roteiro.
//!
//! **Decisões de implementação (evidências):**
//! - **RX por callback → fila compartilhada:** o callback do midir roda no
//!   thread de MIDI e NUNCA pode bloquear — só empurra os bytes crus numa
//!   `Arc<Mutex<VecDeque>>` (O(1)); `recv_raw` drena a fila com timeout ADR-3
//!   (3s por transação) e poll de 2ms (latência de entrega ≤2ms por mensagem;
//!   SysEx chega no ritmo do device, não do host).
//! - **Trim no 1º `F7` já na entrada** (knowledge.md, armadilha de captura):
//!   buffers longos podem trazer cauda stale, e `F7` no MEIO = paginação, NÃO
//!   fim de mensagem. O contrato do `recv_raw` (P2) é devolver UMA mensagem
//!   `F0..F7` — o trim é do transporte, a Session continua intacta.
//! - **Despacho de porta por NOME contendo "gp-100"** (case-insensitive): o
//!   WinMM não expõe VID/PID (VID_84EF/PID_0021 vêm do `.inf` do driver —
//!   BLOCKERS #7); nome é o que a API dá e vem do driver ("GP-100 MIDI").
//! - **SysEx completo em um send** (`midiOutLongMsg` no WinMM — o que o proxy
//!   loga como `out_long`); NUNCA fatiar SysEx em eventos de 3 bytes.
//! - **Reconexão no MESMO objeto** (ADR-4): `open()` após `close()`/`DeviceGone`
//!   recria IN+OUT; soltar as conexões (RAII) fecha as portas.
//! - **Sem filtro de mensagens:** o midir por default ignora NADA (docs 0.9) e
//!   o nosso protocolo é 100% SysEx — qualquer filtro futuro é bug silencioso.
//!
//! **Limitações conhecidas (revisão de revalidação, 29/09 — documentadas, não
//! corrigidas por serem indistinguíveis sem campo):**
//! - Desconexão física NO MEIO da sessão NÃO vira [`TransportError::DeviceGone`]
//!   dedicado: no TX ela aparece como `SendFailed` (erro do WinMM), no RX como
//!   `RecvTimeout` (fila vazia ≠ silêncio do device). Mapear para `DeviceGone`
//!   agora seria chute (R1); o H1 é read-only e o checklist já trata timeout
//!   pós-repetição como logística/R3. Observado o comportamento real em campo,
//!   o mapeamento vira patch com evidência.
//! - `recv_raw` faz poll de 2ms: latência de entrega por mensagem ≤2ms (SysEx
//!   chega no ritmo do device); se o live mode (M3) exigir menos, revisitar
//!   com evento/waker — novo ADR na ocasião.
//!
//! **Dependências de sistema por SO (feature `real-device`):** Windows = WinMM
//! (sistema, nada extra); macOS = CoreMIDI (framework de sistema); **Linux =
//! ALSA via `alsa-sys`, que compila C e exige `pkg-config` + `libasound2-dev`
//! (Debian/Ubuntu) — capturado pela CI multi-OS (job gp100-cli/ubuntu).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};

use super::{DeviceTransport, TransportError};
use crate::SYSEX_EOX;

/// Substring do nome de porta que identifica a pedaleira (fw "GP-100 MIDI";
/// WinMM não dá VID/PID — BLOCKERS #7). Comparação em minúsculas.
const PORT_MATCH: &str = "gp-100";

/// Fila de RX compartilhada com o callback (o midir exige `Send + 'static`).
type RxQueue = Arc<Mutex<VecDeque<Vec<u8>>>>;

/// O transporte real (H1). Ciclo de vida é do CHAMADOR (ADR-4): `open()` →
/// uso → `close()`; `open()` no mesmo objeto reconecta.
pub struct RealDevice {
    /// Nome da porta (diagnóstico/H1_REPORT).
    port_name: Option<String>,
    /// Conexão de saída (TX). `None` = fechado.
    out: Option<MidiOutputConnection>,
    /// Conexão de entrada (RX) — manter viva mantém o stream aberto.
    _in_conn: Option<MidiInputConnection<()>>,
    /// Fila que o callback de RX alimenta (Some = RX ativo).
    queue: Option<RxQueue>,
}

impl std::fmt::Debug for RealDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealDevice")
            .field("port", &self.port_name)
            .field("open", &self.out.is_some())
            .finish()
    }
}

/// Acha a porta cujo nome contém "gp-100" (case-insensitive) em IN ou OUT.
fn find_port<C: midir::MidiIO>(io: &C) -> Result<(C::Port, String), TransportError> {
    for p in io.ports() {
        if let Ok(name) = io.port_name(&p) {
            if name.to_lowercase().contains(PORT_MATCH) {
                return Ok((p, name));
            }
        }
    }
    Err(TransportError::OpenFailed {
        why: format!(
            "nenhuma porta MIDI com \"{PORT_MATCH}\" (device ligado? driver ok? Suite aberto?)"
        ),
    })
}

impl RealDevice {
    /// Enumera e abre o device real (IN + OUT).
    ///
    /// # Erros
    /// [`TransportError::OpenFailed`] se nenhuma porta casa com "gp-100"
    /// (device desligado, driver ausente ou **Suite aberto** — occupancy),
    /// ou se o midir falhar ao inicializar/conectar.
    pub fn new() -> Result<Self, TransportError> {
        let mut device = Self {
            port_name: None,
            out: None,
            _in_conn: None,
            queue: None,
        };
        device.open()?;
        Ok(device)
    }

    /// Abre o RX: porta IN → callback empurra (trim no 1º F7) na fila.
    fn open_input(&mut self) -> Result<(), TransportError> {
        let input = MidiInput::new("gp100-nextgen-editor (RX)")
            .map_err(|e| TransportError::OpenFailed { why: e.to_string() })?;
        let (port, _name) = find_port(&input)?;
        let queue: RxQueue = Arc::new(Mutex::new(VecDeque::new()));
        let q = Arc::clone(&queue);
        // 3º parâmetro do callback = &mut () (dados compartilhados do midir);
        // nós usamos a queue capturada (Arc) e ignoramos o dado do midir.
        let conn: MidiInputConnection<()> = input
            .connect(
                &port,
                "gp100-session-rx",
                move |_ts_us, bytes, _data| {
                    // Trim no 1º F7 (P2/knowledge): cauda stale fora;
                    // fragmento SEM F7 (ring truncado) entra cru — D5 decide.
                    let msg = match bytes.iter().position(|&b| b == SYSEX_EOX) {
                        Some(i) => &bytes[..=i],
                        None => bytes,
                    };
                    if let Ok(mut guard) = q.lock() {
                        guard.push_back(msg.to_vec());
                    }
                },
                (),
            )
            .map_err(|e| TransportError::OpenFailed { why: e.to_string() })?;
        self._in_conn = Some(conn);
        self.queue = Some(queue);
        Ok(())
    }
}

impl DeviceTransport for RealDevice {
    /// (Re)abre IN+OUT no mesmo objeto (ADR-4: reconexão).
    fn open(&mut self) -> Result<(), TransportError> {
        // TX primeiro (falha rápida se occupancy): porta OUT → conexão.
        let out = MidiOutput::new("gp100-nextgen-editor (TX)")
            .map_err(|e| TransportError::OpenFailed { why: e.to_string() })?;
        let (port, name) = find_port(&out)?;
        let conn = out
            .connect(&port, "gp100-session-tx")
            .map_err(|e| TransportError::OpenFailed { why: e.to_string() })?;
        self.out = Some(conn);
        self.port_name = Some(name);
        // RX depois (callback + fila).
        self.open_input()?;
        Ok(())
    }

    /// Fecha IN+OUT (RAII do midir); um novo `open()` reconecta (ADR-4).
    fn close(&mut self) -> Result<(), TransportError> {
        self.out = None;
        self._in_conn = None;
        self.queue = None;
        self.port_name = None;
        Ok(())
    }

    /// Envia UM frame SysEx completo (`midiOutLongMsg` no WinMM).
    fn send_raw(&mut self, data: &[u8]) -> Result<(), TransportError> {
        let Some(conn) = self.out.as_mut() else {
            return Err(TransportError::Closed);
        };
        conn.send(data)
            .map_err(|e| TransportError::SendFailed { why: e.to_string() })
    }

    /// Drena a fila de RX (FIFO global do D1) até `timeout` (ADR-3).
    fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        let deadline = Instant::now() + timeout;
        loop {
            let Some(q) = self.queue.as_ref() else {
                return Err(TransportError::Closed);
            };
            if let Ok(mut guard) = q.lock() {
                if let Some(msg) = guard.pop_front() {
                    return Ok(msg);
                }
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(TransportError::RecvTimeout {
                    timeout_ms: timeout.as_millis() as u64,
                });
            }
            // Poll de 2ms: latência de entrega ≤2ms sem ocupar a CPU.
            std::thread::sleep(Duration::from_millis(2).min(deadline - now));
        }
    }
}
