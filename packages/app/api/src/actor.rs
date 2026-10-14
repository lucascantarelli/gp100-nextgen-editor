//! DeviceActor — dono ÚNICO da `Session` (D8 do ADR-6).
//!
//! A FSM é `&mut self` (D1/D8: consumidor único do stream IN); compartilhar
//! `Mutex<Session>` com a UI permitiria interleave de transações entre
//! commands. O actor resolve: UMA thread possui a `Session` e processa uma
//! fila `mpsc` de requisições na ordem de chegada; cada command envia a
//! requisição + um canal de resposta e fica bloqueado no `recv` (o command
//! síncrono do Tauri roda fora da main thread — ADR-3 preservado).
//!
//! Posse: `Session::new(transport)` TOMA o transporte por valor (ADR-4/6) —
//! o actor guarda `Option<Session<MockDevice>>` e, para o snapshot de
//! `device_info`, devolve o transporte com `into_transport()` e remonta a
//! Session (o backlog D7 de observação é descartado — não há transação em
//! curso; operações novas chegam vazias pela fila serializada).
//!
//! Progresso do boot: o command passa um `mpsc::Sender<BootProgress>`
//! (Send) na requisição; o actor fecha um callback em volta do sender e o
//! `device_boot` encaminha cada beat como evento Tauri `device://progress`
//! (a UI mostra barra — nunca trava). Callback por REFERÊNCIA não atravessa
//! a fila (não é Send); canal é o padrão seguro.
//!
//! Inventário: o actor usa o DEFAULT da `Session` (0..198). O inventário
//! da captura (pp corrente primeiro + duplicação do 0x0100) é artefato de
//! REPLAY — o app não reproduz quirks de captura, apenas o script.

//!
//! **Layout do módulo (#243):** este arquivo é o HANDLE (a API que a thread do
//! Tauri usa) e o spawn. A thread em si vive em [`relogio`] (o loop com prazos
//! e o read-back), o estado que ela possui em [`estado`], a tradução
//! `Request`→`Session` nos módulos de coorte ([`leitura`], [`escrita`],
//! [`boot`], [`diagnostico`]), o contrato da fila em [`fila`] e os tipos do
//! IPC em [`tipos`]. O tamanho de cada arquivo é orçamento declarado em
//! `docs/dev/ARCHITECTURE.md` §4.1 (a cobrança automática do Rust é a #248).

use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

use gp100_core::pedalboard::BoardView;
use gp100_core::session::{
    BootProgress, BootReport, GlobalBruto, IrUploadReport, SnapToneUploadReport,
};

mod backends;
mod boot;
mod diagnostico;
mod escrita;
mod estado;
mod fila;
mod leitura;
mod relogio;
mod tipos;

#[cfg(test)]
mod tests;

pub use backends::{DesconectadoDevice, DeviceBackend};
// Os dois tipos do build de campo seguem a feature do transporte: sem
// `real-device` não existe `RealDevice` para o alias prometer (ADR-5/#150).
#[cfg(feature = "real-device")]
pub use backends::{AppDevice, ProcuraAparelho};
pub use relogio::{READBACK_DEBOUNCE, REFRESH_INTERVAL};
// Reexporta o que o crate consome fora do módulo (`commands.rs`, `lib.rs`).
// `PresetEntry`/`SyncStatus` NÃO entram: `mod actor` é privado em `lib.rs`, então
// um `pub use` que ninguém no crate usa é import não usado (o compilador acusa) —
// e o caminho deles nunca foi alcançável de fora. Vivem em [`tipos`] e o campo
// `PresetLibrary::entries` continua utilizável como sempre.
pub use tipos::{Backend, DeviceSnapshot, DumpReport, PresetLibrary, SyncEvent};

use estado::Estado;
use fila::Request;

/// Handle do actor: clonável para múltiplos commands (a fila serializa).
#[derive(Clone)]
pub struct DeviceActor {
    tx: mpsc::Sender<Request>,
    handle: std::sync::Arc<std::sync::Mutex<Option<JoinHandle<()>>>>,
    /// Destinatário dos eventos `device://sync` (#164) — registrado pelo
    /// `run()` depois do build do Tauri. Sem sink os eventos são descartados
    /// (o estado continua consultável pelo `info`).
    sync_sink: std::sync::Arc<std::sync::Mutex<Option<mpsc::Sender<SyncEvent>>>>,
}

impl DeviceActor {
    /// Spawn do actor com o device.
    ///
    /// **O parametro e `T: DeviceBackend`, nao `MockDevice`.** Era essa
    /// assinatura concreta que impedia o app de falar com a pedaleira: a
    /// feature `real-device` do crate existia e nao mudava nada, porque
    /// nao havia como *passar* um `RealDevice` para ca. Com o generico, o
    /// build de campo passa `Box<dyn DeviceTransport>` e o build de
    /// desenvolvimento passa o `MockDevice` — o mesmo caminho do CLI
    /// (`--real`), sem duplicar a fila nem a FSM.
    ///
    /// `backend` e **declarado por quem monta** (o `run()` decide pelo
    /// build), porque a trait de bytes nao responde "sou mock?".
    ///
    /// Falha de pânico na thread (bug do core) vira erro de canal — nunca
    /// derruba o app.
    ///
    /// **(#150) no build padrão só os TESTES montam actor com device** (o
    /// runtime sobe `desligado`); no build de campo é o `RealDevice` que
    /// entra por aqui. Mesmo `cfg_attr` da variante `Mock`.
    #[cfg_attr(not(all(test, not(feature = "real-device"))), allow(dead_code))]
    pub fn spawn<T: DeviceBackend + 'static>(device: T, backend: Backend) -> Self {
        Self::spawn_com_motivo(device, backend, String::new())
    }

    /// Spawn do app **sem aparelho** (issue #150): estado [`Backend::Desligado`]
    /// com o motivo que a UI mostra no aviso de conexão. Nenhum dado é servido
    /// — `info` responde honesto e todo tráfego recusa.
    ///
    /// O build de campo usa o [`desligado_procurando`] (transporte que re-
    /// enumera portas); este aqui existe para o build SEM a feature.
    #[cfg_attr(feature = "real-device", allow(dead_code))]
    pub fn desligado(motivo: impl Into<String>) -> Self {
        Self::spawn_com_motivo(DesconectadoDevice, Backend::Desligado, motivo.into())
    }

    /// Spawn do build de campo **sem USB** (issue #150): o transporte procura
    /// o aparelho a cada `device_conectar` — o "Reconectar" da UI tem ação
    /// real (re-enumera as portas MIDI) em vez de desistir para sempre. Ao
    /// conectar, o estado do actor sobe para [`Backend::Real`].
    #[cfg(feature = "real-device")]
    pub fn desligado_procurando(motivo: impl Into<String>) -> Self {
        Self::spawn_com_motivo(
            ProcuraAparelho::default(),
            Backend::Desligado,
            motivo.into(),
        )
    }

    /// Corpo comum do spawn; `motivo` só é lido quando o backend é
    /// [`Backend::Desligado`].
    fn spawn_com_motivo<T: DeviceBackend + 'static>(
        device: T,
        backend: Backend,
        motivo: String,
    ) -> Self {
        Self::spawn_interno(device, backend, motivo, READBACK_DEBOUNCE, REFRESH_INTERVAL)
    }

    /// Spawn com cadências ENCURTADAS (#164) — para os testes do ciclo de
    /// convergência dormirem milissegundos, não segundos. Produção usa as
    /// constantes [`READBACK_DEBOUNCE`]/[`REFRESH_INTERVAL`].
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn spawn_com_cadencia<T: DeviceBackend + 'static>(
        device: T,
        backend: Backend,
        debounce: Duration,
        intervalo: Duration,
    ) -> Self {
        Self::spawn_interno(device, backend, String::new(), debounce, intervalo)
    }

    /// Corpo real do spawn: monta o estado inicial e entrega a thread ao
    /// [`relogio`], que roda a fila com prazos e o read-back (#164).
    fn spawn_interno<T: DeviceBackend + 'static>(
        device: T,
        backend: Backend,
        motivo: String,
        debounce: Duration,
        intervalo: Duration,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<Request>();
        let sync_sink: estado::SyncSink = std::sync::Arc::new(std::sync::Mutex::new(None));
        let sink = std::sync::Arc::clone(&sync_sink);
        let handle = std::thread::spawn(move || {
            // O open acontece ANTES do loop (e o erro não mata a thread — #150:
            // ver `Estado::novo`); daqui para baixo quem manda é o relógio.
            let estado = Estado::novo(device, backend, motivo);
            relogio::roda(estado, rx, debounce, intervalo, sink);
        });
        Self {
            tx,
            handle: std::sync::Arc::new(std::sync::Mutex::new(Some(handle))),
            sync_sink,
        }
    }

    /// Registra o destinatário dos eventos `device://sync` (#164). Quem
    /// registra é o `run()` (reemite para o front); sem sink os eventos são
    /// descartados e o estado continua consultável pelo `info`.
    pub fn set_sync_sink(&self, sink: mpsc::Sender<SyncEvent>) {
        if let Ok(mut g) = self.sync_sink.lock() {
            *g = Some(sink);
        }
    }

    /// Snapshot do estado (o que o app **pode afirmar** sobre o device).
    ///
    /// No mock: nome do pp, ppType e CRC de fabrica sao leituras locais.
    /// No aparelho real: `current_pp` e a tabela de IRs sao **lidos do fio**,
    /// e os campos locais ficam no neutro — a UI mostra "desconhecido", nao
    /// um zero que parece dado.
    ///
    /// # Erros
    /// String de erro se a thread do actor morreu (pânico do core).
    pub fn info(&self) -> Result<DeviceSnapshot, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Info { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Info".to_string())?
    }

    /// Boot completo (§13.10) com progresso opcional pelo canal `progress`
    /// (1 [`BootProgress`] por transação). Bloqueia até o fim do script
    /// (2297 transações no inventário default do MOCK) — o command que chama é
    /// síncrono e roda fora da main thread (ADR-3).
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string (Timeout D6,
    /// InvalidShape D5) ou morte da thread do actor.
    pub fn boot(&self, progress: Option<mpsc::Sender<BootProgress>>) -> Result<BootReport, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Boot {
                progress,
                reply: tx,
            })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Boot".to_string())?
    }

    /// Board do preset (slots/arquétipos/knobs) — projeção pura no core.
    /// `pp = None` = corrente.
    ///
    /// # Erros
    /// String de erro de parse/projeção ou morte da thread do actor.
    pub fn board(&self, pp: Option<u16>) -> Result<BoardView, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Board { pp, reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Board")?
    }

    /// Biblioteca de presets (flight case) + pp corrente.
    ///
    /// # Erros
    /// String de erro de parse ou morte da thread do actor.
    pub fn library(&self) -> Result<PresetLibrary, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Library { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Library")?
    }

    /// Reconecta o transporte do device (issue #150 — command
    /// `device_conectar`): refaz o `open` no MESMO objeto; no build de campo
    /// o `ProcuraAparelho` re-enumera as portas MIDI.
    ///
    /// # Erros
    /// String com o motivo (aparelho ausente) ou morte da thread do actor.
    pub fn conectar(&self) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Conectar { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Conectar")?
    }

    /// Select de preset (§13.10 — write + meta6). Bloqueia até a FSM
    /// completar a transação (mock: 1 ida e volta).
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string.
    pub fn select_preset(&self, pp: u16) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Select { pp, reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Select")?
    }

    /// Set-param do knob (§13.11, fire-and-forget D4). O mock valida o
    /// shape contra o golden e guarda o valor no estado.
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string.
    pub fn set_param(&self, slot: u8, code: u32, ctrl: u8, value: f32) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::SetParam {
                slot,
                code,
                ctrl,
                value,
                reply: tx,
            })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no SetParam")?
    }

    /// Agenda um read-back IMEDIATO do preset corrente (#164 — command
    /// `device_sync_now`, a ação "Reler agora" do chip de sincronização).
    /// Resolve quando o prazo foi agendado; o RESULTADO da leitura (sync/
    /// failed/offline + `changed`) chega pelo evento `device://sync`.
    pub fn sync_now(&self) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::SyncNow { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no SyncNow")?
    }

    /// Registradores globais (§13.13): o bloco medido lido DO device, com o
    /// payload crU por registrador — o decode medido (nibble-par) é do
    /// chamador (`commands::device_globals`).
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string (Timeout D6
    /// se um registrador não responde; InvalidShape para forma fora do
    /// medido) ou morte da thread do actor.
    pub fn globals(&self) -> Result<Vec<GlobalBruto>, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Globals { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Globals")?
    }

    /// Write num registrador global (§13.13.1) — payload pronto (o encode
    /// nibble-par é do chamador). Sem write-verified o TRANSPORTE recusa
    /// antes do driver (ADR-5) e o erro sobe aqui.
    pub fn set_global(&self, addr: [u8; 4], payload: Vec<u8>) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::SetGlobal {
                addr,
                payload,
                reply: tx,
            })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no SetGlobal")?
    }

    /// Tabela dos 20 User IRs (§13.12). Bloqueia até o actor completar as
    /// 20 leituras de tabela (mock: responde por página com eco — D1).
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string.
    pub fn list_user_irs(&self) -> Result<Vec<(u8, String)>, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::ListIrs { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no ListIrs")?
    }

    /// Drena pushes não consumidos do device como hex cru (F0…F7). O
    /// command `pending_pushes` devolve a lista; o `device_boot` reemite
    /// como evento `device://push` para o log da UI.
    ///
    /// # Erros
    /// String de erro se a thread do actor morreu.
    pub fn drain_pushes(&self) -> Result<Vec<String>, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::DrainPushes { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no DrainPushes")?
    }

    /// Envia um modelo de SnapTone ao device (§5).
    ///
    /// Bloqueia até a FSM fechar TODOS os blocos esperando o ACK de 16B de
    /// cada um — dezenas de segundos para um `.clo` de ~2,7 KB, porque o
    /// settle do §4 é 250 ms por operação. É o comportamento correto (§4:
    /// 0,15 s corrompe o stream e loop tight trava a pedaleira) e por isso
    /// quem chama precisa mostrar "enviando…" e esperar o `invoke`.
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string — `Timeout`
    /// (D6) quando um ACK não chega na janela, `InvalidShape` (D5) quando o
    /// ACK vem com outro tamanho ou o slot é inválido.
    pub fn upload_snap_tone(&self, slot: u8, model: &[u8]) -> Result<SnapToneUploadReport, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::UploadSnapTone {
                slot,
                model: model.to_vec(),
                reply: tx,
            })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv()
            .map_err(|_| "actor morreu no upload de SnapTone".to_string())?
    }

    /// Envia um IR de usuário ao aparelho (§13.7): `ir_begin` + chunks de 15B
    /// com ACK por chunk, o último duplicado como marcador de fim.
    ///
    /// **BLOQUEIA por muito tempo.** Um IR de 300 KB são ~20.000 chunks, e cada
    /// um espera o próprio ACK (timeout ADR-3). É a transação mais longa do
    /// app — o comando do Tauri roda fora da main thread (ADR-3), então a
    /// janela continua desenhando enquanto isso.
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string — `Timeout`
    /// (D6) quando um ACK não chega na janela, `InvalidShape` (D5) quando o
    /// blob não é múltiplo de 15B ou o slot está fora de 0..=19, e
    /// `UnexpectedAck` quando o ACK não é o do chunk que foi enviado.
    pub fn upload_ir(&self, slot: u8, blob: &[u8]) -> Result<IrUploadReport, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::UploadIr {
                slot,
                blob: blob.to_vec(),
                reply: tx,
            })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv()
            .map_err(|_| "actor morreu no upload de IR".to_string())?
    }

    /// **Salva o preset no aparelho** (§13.12): 5 writes de meta + 4 ops
    /// (D3). O `set-param` é fire-and-forget, então **sem isto a mudança
    /// morre com a sessão** — é a operação que persiste o que se mexeu.
    ///
    /// Substitui o `save` do `gp100-cli`, que era o único caminho para
    /// isso antes.
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string — nome não
    /// ASCII ou `TransportError::WriteBlocked` no build de leitura (ADR-5).
    pub fn save_preset(&self, pp: u16, pp_type: u16, name: &str) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Save {
                pp,
                pp_type,
                name: name.to_string(),
                reply: tx,
            })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Save".to_string())?
    }

    /// **Lê o preset do aparelho** (§13.9): meta6 + as páginas de estado,
    /// em hex cru. Substitui o `dump-preset` do `gp100-cli`.
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string (timeout,
    /// shape inesperado) ou morte da thread do actor.
    pub fn dump_preset(&self, pp: u16) -> Result<DumpReport, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Dump { pp, reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Dump".to_string())?
    }

    /// **Liga o log de fio da sessao** (schema P4) — o MESMO formato que o
    /// `--log` do CLI e que os juiz (`h1_compare.py`/`h2_compare.py`) leem.
    ///
    /// E o que fecha o ciclo de campo pelo app: sessao no editor, exporta o
    /// `.jsonl`, roda o juiz. Liga e desliga **sem reiniciar o device**.
    ///
    /// # Erros
    /// String com o erro do SO se o arquivo nao puder ser criado.
    pub fn log_session(&self, path: &str) -> Result<bool, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::LogSession {
                path: path.to_string(),
                reply: tx,
            })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv()
            .map_err(|_| "actor morreu no LogSession".to_string())?
    }

    /// Desliga o log de fio (a sessao segue; so o log para).
    ///
    /// # Erros
    /// String de erro se a thread do actor morreu.
    pub fn log_stop(&self) -> Result<bool, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::LogStop { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv()
            .map_err(|_| "actor morreu no LogStop".to_string())?
    }

    /// Em que arquivo o log de fio esta gravando agora (`None` = nenhum).
    ///
    /// E o que deixa a TELA dizer o arquivo sem depender do `stderr` do
    /// processo: desde a #130 o build de campo liga o log sozinho na abertura, e
    /// o caminho e escolhido pelo `run()` (diretorio de dados + carimbo), nao
    /// pelo front.
    ///
    /// # Erros
    /// String de erro se a thread do actor morreu.
    pub fn log_path(&self) -> Result<Option<String>, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::LogPath { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no LogPath".to_string())
    }

    /// Envia Shutdown e agrega a thread (idempotente).
    pub fn shutdown(&self) {
        let _ = self.tx.send(Request::Shutdown);
        if let Ok(mut slot) = self.handle.lock() {
            if let Some(h) = slot.take() {
                let _ = h.join();
            }
        }
    }
}

// SEM impl Drop: o handle é Clone e qualquer clone caindo fora de escopo
// NÃO pode derrubar o actor compartilhado (bug clássico de shutdown por
// Drop em handle clonável — pegaria a thread de teste no meio do uso). O
// shutdown é EXPLÍCITO: o app o tem pelo ciclo da AppState (processo);
// os testes chamam no fim.
