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

use std::sync::mpsc;
use std::thread::JoinHandle;

use gp100_core::model::Dictionary;
use gp100_core::pedalboard::{board_view_for, embedded_document, preset_list, BoardView};
use gp100_core::session::{
    BootProgress, BootReport, IrUploadReport, Session, SnapToneUploadReport,
};
use gp100_core::transport::mock::MockDevice;
use gp100_core::transport::DeviceTransport;
use gp100_core::wire_log::LoggingTransport;

/// Estado snapshot do mock (alias curto; o tipo vive no core).
type MockState = gp100_core::transport::mock::MockState;

/// Bytes crus em hex maiúsculo, sem separador — o mesmo formato que o
/// `gp100-cli` imprime e que os juízes (`h1_compare.py`/`h2_compare.py`)
/// leem dos logs.
fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02X}")).collect()
}

/// **O tipo de transporte que este binário carrega**, escolhido por
/// compilação.
///
/// No build comum e o `MockDevice` concreto. No build de campo
/// (`--features real-device`) e `Box<dyn DeviceBackend>` — a trait deste
/// crate, que ja estende `DeviceTransport`, entao o `RealDevice` entra pela
/// mesma fila do actor sem que nenhum tipo concrete precise conhecer o
/// outro.
///
/// Este alias é o que a `abrir_backend` de `lib.rs` **anota** nos dois
/// builds (`let mock: actor::AppDevice = …`), e não é decoração: é ele que
/// faz o `spawn` ser o mesmo código nos dois caminhos. Sem a anotação, o
/// build padrão não teria consumidor nenhum do alias e o clippy acusaria
/// código morto onde o desenho está certo.
#[cfg(not(feature = "real-device"))]
pub type AppDevice = MockDevice;

/// Alias de transporte do build de campo (ver [`AppDevice`]).
#[cfg(feature = "real-device")]
pub type AppDevice = Box<dyn DeviceBackend + Send>;

/// `WRITE_VERIFIED` **espelhado** do core (ADR-5).
///
/// Vive como `cfg!` local e nao como import do `core::transport::real`
/// porque aquele modulo so existe com a feature `real-device` — e este
/// crate precisa da resposta tambem no build padrao (mock), para o front
/// saber que ali a escrita esta liberada por construcao.
///
/// Espelhar a feature em vez de importa-la e proposital: e o que faz o
/// binario de campo dizer "estou travado" na tela em vez de so descobrir
/// quando o operador clica.
const WRITE_VERIFIED: bool = cfg!(feature = "write-verified");

/// Qual backend esta thread possui. Vem **declarado por quem monta o
/// transporte**, nunca deduzido: a trait [`DeviceTransport`] e a fronteira
/// de BYTES CRUOS (ADR-4) e nao tem como responder "sou mock?" — e essa
/// resposta e o que decide se o app pode prometer nome de preset e CRC de
/// fabrica.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// `MockDevice` — estado local completo, sem tráfego de fio.
    Mock,
    /// `RealDevice` (feature `real-device`) — o aparelho **não tem estado
    /// local**: o que ele mostra na tela, a app precisa ler do fio.
    ///
    /// Mesmo caso do [`AppDevice`]: só a `abrir_backend` de `real-device`
    /// constrói esta variante, e o `match` do `as_str` não conta como
    /// construção — então o build padrão acusaria "nunca construído" para
    /// algo que tem construtor.
    #[cfg_attr(not(feature = "real-device"), allow(dead_code))]
    Real,
}

impl Backend {
    /// Etiqueta estável no fio IPC (`DeviceInfo.backend` do front).
    pub fn as_str(self) -> &'static str {
        match self {
            Backend::Mock => "mock",
            Backend::Real => "real",
        }
    }
}

/// Fonte do estado local do backend — só o mock tem.
///
/// Existe **neste crate**, e nao como metodo do `DeviceTransport`, para nao
/// alargar a fronteira de bytes do core (ADR-4: a trait trafega SysEx cru e
/// nada mais). O `RealDevice` devolve `None`, e o app degrada com honestidade
/// em vez de mostrar zero como se fosse um dado.
pub trait DeviceBackend: DeviceTransport + Send {
    /// Estado local do mock, quando existir.
    fn local_state(&self) -> Option<MockState>;

    /// Mensagens que chegaram ao device e **ninguém pediu** (a inbox do
    /// `MockDevice`, D7 na perspectiva do device).
    ///
    /// **Limite conhecido do aparelho real:** o `RealDevice` guarda o excedente
    /// numa fila interna de RX e nao expoe peek, entao devolve vazio. O log de
    /// pushes da UI (`pending_pushes`) e por isso **mock-only por enquanto** —
    /// o espelho fiel no aparelho precisa de um `peek` no `real.rs`, e isso
    /// esta anotado em `docs/REAL_DEVICE_GAP.md` §6.
    fn drain_inbox(&mut self) -> Vec<Vec<u8>>;
}

impl DeviceBackend for MockDevice {
    fn local_state(&self) -> Option<MockState> {
        Some(self.state().clone())
    }

    fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        MockDevice::drain_inbox(self)
    }
}

/// O `LoggingTransport` (do core) continua sendo um backend: o log e um
/// **decorador transparente**, entao o `DeviceBackend` atravessa por ele
/// sem mudar nada. E o que permite ligar o log no MEIO da sessao, sem
/// reiniciar o device nem reconstruir a fila.
impl<T: DeviceBackend> DeviceBackend for LoggingTransport<T> {
    fn local_state(&self) -> Option<MockState> {
        self.inner.local_state()
    }

    fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        self.inner.drain_inbox()
    }
}

/// `Box<dyn DeviceBackend>` tambem e um backend — e o que deixa o build de
/// campo passar o `RealDevice` pela MESMA fila do mock, sem duplicar nada.
impl<T: DeviceBackend + ?Sized> DeviceBackend for Box<T> {
    fn local_state(&self) -> Option<MockState> {
        (**self).local_state()
    }

    fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        (**self).drain_inbox()
    }
}

#[cfg(feature = "real-device")]
impl DeviceBackend for gp100_core::transport::real::RealDevice {
    fn local_state(&self) -> Option<MockState> {
        // O aparelho nao tem estado local: o que ele mostra na tela, a app
        // precisa ler do fio. Encher isto seria a app inventando dado.
        None
    }

    fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        // Limite conhecido, nao um "nao ha nada": ver a doc da trait.
        Vec::new()
    }
}

/// O que o `dump` traz do aparelho: meta6 + páginas de estado, em hex.
///
/// **Hex cru e não decodificado, de propósito.** O layout byte-a-byte da
/// família `13xx` ainda não foi decifrado (o
/// [`gp100_core::session::StatePage`] é opaco por decisão — R1: nunca
/// adivinhar protocolo). Devolver o bruto é o que permite decifrar depois;
/// devolver algo pretty mas errado seria inventar.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DumpReport {
    /// pp que foi lido.
    pub pp: u16,
    /// Página meta6 do pp (6B, §13.9).
    pub meta6: String,
    /// Páginas 0..=7 (196B cada) + a final de 4B, em hex, na ordem do fio.
    pub pages: Vec<String>,
}

/// O que a app pode afirmar sobre o device, **por backend**.
///
/// A distincao que importa: com o mock, `current_name` e `ir_slots_with_crc`
/// sao leituras locais. Com o aparelho real, o nome do pp vem da pagina
/// meta6 (`13010001`) cujo layout **ainda nao foi decifrado** (o
/// [`gp100_core::session::StatePage`] e opaco de proposito), e o CRC de
/// fabrica nao existe em lugar nenhum do fio. Encher esses campos com zero
/// seria a app mentindo sobre o aparelho — por isso eles vao marcados.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSnapshot {
    /// Backend ativo ("mock" | "real") — nunca mais um literal.
    pub backend: &'static str,
    /// Nº de presets. No mock, o do `all.prst`; no real, o **inventário que
    /// o boot percorreu** (ver `docs/REAL_DEVICE_GAP.md` §4.3 — hoje é o
    /// default 0..198, não uma contagem descoberta no aparelho).
    pub preset_count: usize,
    /// pp corrente (lido da Session nos dois backends).
    pub current_pp: u16,
    /// Nome do pp corrente. **Só no mock** — ver a nota de `DeviceSnapshot`.
    pub current_name: String,
    /// ppType do pp corrente. **Só no mock.**
    pub current_pp_type: u16,
    /// Slots de IR com CRC de fabrica. **Só no mock** — o fio nao tem esse campo.
    pub ir_slots_with_crc: usize,
    /// Tabela de 20 User IRs **lida do device** (§13.12) — existe nos dois.
    pub ir_slots: Vec<(u8, String)>,
    /// `true` quando o binario foi compilado com `write-verified` (ADR-5).
    /// O front usa isto para desabilitar os botoes de escrita com explicacao,
    /// em vez de descobrir a recusa depois de clicar.
    pub write_verified: bool,
}

impl DeviceSnapshot {
    /// Estado de partida quando nao houve boot (nem mock nem real).
    fn empty(backend: Backend) -> Self {
        Self {
            backend: backend.as_str(),
            preset_count: 0,
            current_pp: 0,
            current_name: String::new(),
            current_pp_type: 0,
            ir_slots_with_crc: 0,
            ir_slots: Vec::new(),
            write_verified: WRITE_VERIFIED,
        }
    }
}

/// Biblioteca de presets (o flight case da UI) + corrente.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetLibrary {
    /// Entradas (pp, nome, tipo) de TODOS os presets do arquivo.
    pub entries: Vec<PresetEntry>,
    /// pp corrente do mock.
    pub current_pp: u16,
}

/// Uma entrada da biblioteca (camelCase no fio IPC).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetEntry {
    /// pp do preset.
    pub pp: u16,
    /// Nome do preset.
    pub name: String,
    /// Rótulo do tipo (ex.: "Pop").
    pub pp_type_name: String,
}

/// Requisições atendidas pelo actor (uma por vez — D8).
enum Request {
    /// Snapshot do estado do device (command `device_info`).
    Info {
        reply: mpsc::Sender<Result<DeviceSnapshot, String>>,
    },
    /// Board do preset: slots/arquétipos/knobs (projeção pura no core).
    Board {
        pp: Option<u16>,
        reply: mpsc::Sender<Result<BoardView, String>>,
    },
    /// Biblioteca de presets (flight case da UI).
    Library {
        reply: mpsc::Sender<Result<PresetLibrary, String>>,
    },
    /// Select de preset (write `13010000` §13.10; o mock troca o corrente).
    Select {
        pp: u16,
        reply: mpsc::Sender<Result<(), String>>,
    },
    /// Set-param (knob §13.11, fire-and-forget D4; o mock valida e guarda).
    SetParam {
        slot: u8,
        code: u32,
        ctrl: u8,
        value: f32,
        reply: mpsc::Sender<Result<(), String>>,
    },
    /// Boot completo (§13.10) com canal de progresso opcional
    /// (command `device_boot`): 1 mensagem por transação do script.
    Boot {
        progress: Option<mpsc::Sender<BootProgress>>,
        reply: mpsc::Sender<Result<BootReport, String>>,
    },
    /// Tabela dos 20 User IRs (§13.12; command `list_user_irs`) —
    /// transação própria da FSM (by-len do golden distingue tabela 75B
    /// do ACK 4B, D1).
    ListIrs {
        reply: mpsc::Sender<Result<Vec<(u8, String)>, String>>,
    },
    /// Drena pushes não consumidos do DEVICE (inbox do mock, FIFO global —
    /// D7 na perspectiva do device) como hex cru (command `pending_pushes`;
    /// o `device_boot` usa o mesmo caminho para reemitir `device://push`).
    DrainPushes {
        reply: mpsc::Sender<Result<Vec<String>, String>>,
    },
    /// Envia um modelo de SnapTone ao device (§5): stream de blocos com ACK
    /// de 16B por bloco e o settle de 250 ms do §4. É a transação mais LONGA
    /// do app depois do boot — 143 blocos para um `.clo` de ~2,7 KB — e é por
    /// isso que ela fica na fila do actor e não em `spawn`: a fila é o que
    /// impede um upload de bytes se misturar com um knob do dono.
    UploadSnapTone {
        slot: u8,
        model: Vec<u8>,
        reply: mpsc::Sender<Result<SnapToneUploadReport, String>>,
    },
    /// Envia um IR de usuário ao device (§13.7): `ir_begin` + chunks de 15B
    /// com ACK por chunk + o último chunk DUPLICADO (o marcador de fim).
    ///
    /// Fica na fila como todo tráfego de fio (D8): um IR de 300 KB são
    /// 20.000 chunks, e misturar isso com um knob do dono seria a mesma
    /// corrida que a fila existe para impedir.
    UploadIr {
        slot: u8,
        blob: Vec<u8>,
        reply: mpsc::Sender<Result<IrUploadReport, String>>,
    },
    /// Salva o preset no aparelho (§13.12) — 9 frames de escrita: 5 do meta
    /// e 4 ops (D3). Este era o ÚNICO jeito de persistir uma mudança: o
    /// `set-param` é fire-and-forget e morre com a sessão. O `gp100-cli`
    /// tinha `save` e o app não tinha command nenhum — a primeira
    /// funcionalidade do CLI trazida para dentro do app.
    Save {
        pp: u16,
        pp_type: u16,
        name: String,
        reply: mpsc::Sender<Result<(), String>>,
    },
    /// Lê o preset do aparelho (§13.9: meta6 + páginas de estado) e devolve
    /// o hex cru. Também era exclusivo do CLI (`dump-preset`).
    Dump {
        pp: u16,
        reply: mpsc::Sender<Result<DumpReport, String>>,
    },
    /// Liga (ou religa) o log de fio da sessao no schema P4 -- o MESMO que
    /// o `--log` do CLI grava e que `h1_compare.py`/`h2_compare.py` leem.
    ///
    /// E o que fecha o ciclo de campo pelo app: o operador abre a sessao no
    /// editor, faz o que precisa, exporta o `.jsonl` e roda o juiz. Sem
    /// isto, a unica forma de capturar o fio era pelo binario de campo.
    LogSession {
        path: String,
        reply: mpsc::Sender<Result<bool, String>>,
    },
    /// Desliga o log de fio (a sessao segue; so o log para).
    LogStop {
        reply: mpsc::Sender<Result<bool, String>>,
    },
    /// Encerra a thread do actor (drop do `DeviceActor`).
    Shutdown,
}

/// Snapshot do estado via take/remount da Session (mesma semântica do
/// `Info`).
///
/// **O que muda com o aparelho real:** o `MockState` so existe no mock, e
/// `local_state()` devolve `None` no `RealDevice`. Os campos que vinham
/// dele (nome do pp, ppType, CRC de fabrica) ficam no estado neutro em vez
/// de receber um zero que a UI mostraria como dado — ver
/// [`DeviceSnapshot`].
fn device_snapshot<T: DeviceBackend>(
    session: &mut Option<Session<T>>,
    backend: Backend,
) -> DeviceSnapshot {
    let Some(s) = session.take() else {
        return DeviceSnapshot::empty(backend);
    };
    let transport = s.into_transport();
    let local = transport.local_state();
    let mut s = Session::new(transport);
    let current_pp = s.current_pp();
    let ir_slots = s.list_user_irs().map(|t| t.slots).unwrap_or_default();
    *session = Some(s);
    DeviceSnapshot {
        backend: backend.as_str(),
        preset_count: local.as_ref().map_or(0, |m| m.preset_count),
        current_pp: local.as_ref().map_or(current_pp, |m| m.current_pp),
        current_name: local
            .as_ref()
            .map_or_else(String::new, |m| m.current_name.clone()),
        current_pp_type: local.as_ref().map_or(0, |m| m.current_pp_type),
        ir_slots_with_crc: local
            .as_ref()
            .map_or(0, |m| m.ir_crcs.iter().filter(|c| **c != 0).count()),
        ir_slots,
        write_verified: WRITE_VERIFIED,
    }
}

/// Handle do actor: clonável para múltiplos commands (a fila serializa).
#[derive(Clone)]
pub struct DeviceActor {
    tx: mpsc::Sender<Request>,
    handle: std::sync::Arc<std::sync::Mutex<Option<JoinHandle<()>>>>,
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
    pub fn spawn<T: DeviceBackend + 'static>(device: T, backend: Backend) -> Self {
        let (tx, rx) = mpsc::channel::<Request>();
        let handle = std::thread::spawn(move || {
            // O transporte entra embrulhado no LoggingTransport desde o
            // inicio: e um decorador transparente (byte entra, byte sai,
            // `kind` intacto), entao o log pode ser LIGADO no meio da sessao
            // sem que o app precise reiniciar o device nem reconstruir a
            // fila. O `drain_inbox` do backend mora por tras do wrapper.
            let mut logged = LoggingTransport::new(device);
            if let Err(e) = logged.open() {
                eprintln!("[device] open do backend {backend:?} falhou: {e}");
                return;
            }
            let mut session = Some(Session::new(logged));
            while let Ok(req) = rx.recv() {
                match req {
                    Request::Info { reply } => {
                        let snap = device_snapshot(&mut session, backend);
                        let _ = reply.send(Ok(snap));
                    }
                    Request::Board { pp, reply } => {
                        // Projeção PURA (doc embedado + dicionário): não toca
                        // a Session nem o fio — pode rodar fora do device
                        // (o actor só é o caminho para reusar o Dictionary
                        // carregado do mock).
                        let r = (|| -> Result<BoardView, String> {
                            let doc = embedded_document().map_err(|e| e.to_string())?;
                            let dict = Dictionary::from_json(gp100_core::model::DICTIONARY_JSON)
                                .map_err(|e| e.to_string())?;
                            board_view_for(&doc, &dict, pp).map_err(|e| e.to_string())
                        })();
                        let _ = reply.send(r);
                    }
                    Request::Library { reply } => {
                        let r = (|| -> Result<PresetLibrary, String> {
                            let doc = embedded_document().map_err(|e| e.to_string())?;
                            let state = device_snapshot(&mut session, backend); // corrente
                            Ok(PresetLibrary {
                                entries: preset_list(&doc)
                                    .into_iter()
                                    .map(|e| PresetEntry {
                                        pp: e.pp,
                                        name: e.name,
                                        pp_type_name: e.pp_type_name,
                                    })
                                    .collect(),
                                current_pp: state.current_pp,
                            })
                        })();
                        let _ = reply.send(r);
                    }
                    Request::Select { pp, reply } => match session.as_mut() {
                        Some(s) => {
                            let r = s.select_preset(pp).map_err(|e| e.to_string());
                            let _ = reply.send(r);
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::SetParam {
                        slot,
                        code,
                        ctrl,
                        value,
                        reply,
                    } => match session.as_mut() {
                        Some(s) => {
                            let r = s
                                .set_param(slot, code, ctrl, value)
                                .map_err(|e| e.to_string());
                            let _ = reply.send(r);
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::Boot { progress, reply } => match session.as_mut() {
                        Some(s) => {
                            // Callback roda NA THREAD do actor: fecha em
                            // volta do sender e repassa por coerção para
                            // `&mut dyn FnMut` (canal é o padrão seguro —
                            // callback emprestado não atravessa a fila).
                            let r = match progress {
                                Some(tx) => {
                                    let mut cb = move |p: BootProgress| {
                                        let _ = tx.send(p);
                                    };
                                    s.boot_with_progress(Some(&mut cb))
                                        .map_err(|e| e.to_string())
                                }
                                None => s.boot_with_progress(None).map_err(|e| e.to_string()),
                            };
                            let _ = reply.send(r);
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::ListIrs { reply } => match session.as_mut() {
                        Some(s) => {
                            let r = s
                                .list_user_irs()
                                .map(|t| t.slots)
                                .map_err(|e| e.to_string());
                            let _ = reply.send(r);
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::DrainPushes { reply } => match session.take() {
                        Some(s) => {
                            let mut transport = s.into_transport();
                            let hexes: Vec<String> = transport
                                .drain_inbox()
                                .iter()
                                .map(|m| m.iter().map(|b| format!("{b:02X}")).collect())
                                .collect();
                            session = Some(Session::new(transport));
                            let _ = reply.send(Ok(hexes));
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::UploadSnapTone { slot, model, reply } => match session.as_mut() {
                        Some(s) => {
                            let r = s.upload_snap_tone(slot, &model).map_err(|e| e.to_string());
                            let _ = reply.send(r);
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::UploadIr { slot, blob, reply } => match session.as_mut() {
                        Some(s) => {
                            let r = s.upload_ir(slot, &blob).map_err(|e| e.to_string());
                            let _ = reply.send(r);
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::Save {
                        pp,
                        pp_type,
                        name,
                        reply,
                    } => match session.as_mut() {
                        Some(s) => {
                            let r = s.save_preset(pp, pp_type, &name).map_err(|e| e.to_string());
                            let _ = reply.send(r);
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::Dump { pp, reply } => match session.as_mut() {
                        Some(s) => {
                            let r = (|| -> Result<DumpReport, String> {
                                let meta6 = s.state_page(0).map_err(|e| e.to_string())?;
                                let mut pages = Vec::with_capacity(9);
                                for p in 1..=8u8 {
                                    let pg = s.state_page(p).map_err(|e| e.to_string())?;
                                    pages.push(hex(&pg.raw));
                                }
                                Ok(DumpReport {
                                    pp,
                                    meta6: hex(&meta6.raw),
                                    pages,
                                })
                            })();
                            let _ = reply.send(r);
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::LogSession { path, reply } => {
                        let Some(s) = session.take() else {
                            let _ = reply.send(Err("session do actor ausente".into()));
                            continue;
                        };
                        let mut t = s.into_transport();
                        let r = t.enable_log(std::path::Path::new(&path));
                        session = Some(Session::new(t));
                        let _ = reply.send(r.map(|()| true));
                    }
                    Request::LogStop { reply } => {
                        let Some(s) = session.take() else {
                            let _ = reply.send(Err("session do actor ausente".into()));
                            continue;
                        };
                        let mut t = s.into_transport();
                        t.logger = None;
                        session = Some(Session::new(t));
                        let _ = reply.send(Ok(true));
                    }
                    Request::Shutdown => break,
                }
            }
        });
        Self {
            tx,
            handle: std::sync::Arc::new(std::sync::Mutex::new(Some(handle))),
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
    /// (2297 transações no inventário default) — o command que chama é
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

#[cfg(test)]
mod tests {
    use super::*;
    use gp100_core::transport::mock::MockFault;
    use gp100_core::transport::{TransportError, WireKind};

    /// Boot completo via actor (2297 transações no inventário default
    /// 0..198) e o resultado chega ao chamador pelo canal de resposta.
    #[test]
    fn boot_via_actor_completa_e_responde() {
        let mock = MockDevice::new().expect("mock montado");
        let actor = DeviceActor::spawn(mock, Backend::Mock);
        let report = actor.boot(None).expect("boot contra o mock via actor");
        assert_eq!(report.transactions, 2297);
        actor.shutdown();
    }

    /// O progresso atravessa a fronteira do actor PELO CANAL: cada
    /// transação produz 1 beat no `mpsc` (o command vai reemitir como
    /// evento `device://progress`). Beats monótonos até o total.
    #[test]
    fn progresso_atravessa_o_actor_pelo_canal() {
        let mock = MockDevice::new().expect("mock montado");
        let actor = DeviceActor::spawn(mock, Backend::Mock);
        let (ptx, prx) = mpsc::channel::<BootProgress>();
        let report = actor.boot(Some(ptx)).expect("boot com progresso");
        let beats: Vec<BootProgress> = prx.try_iter().collect();
        assert_eq!(beats.len(), report.transactions);
        assert_eq!(beats.last().map(|p| p.done), Some(report.transactions));
        assert!(beats.windows(2).all(|w| w[0].done + 1 == w[1].done));
        actor.shutdown();
    }

    /// Info sobe pelo actor (mesmo estado que o CLI imprime; R1 nos DTOs) —
    /// e DEPOIS do info a Session continua utilizável (boot funciona).
    #[test]
    fn info_via_actor_deriva_do_mock() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        let st = actor.info().expect("snapshot");
        assert_eq!(st.preset_count, 99);
        assert_eq!(st.current_name, "It's GP100");
        let report = actor.boot(None).expect("boot após info");
        assert_eq!(report.transactions, 2297);
        actor.shutdown();
    }

    /// **A prova de que o generico nao e vazio.** Um backend que **nao** e
    /// o mock — sem `MockState`, sem `drain_inbox` de mock — entra na MESMA
    /// fila e responde. E o que garante que o `RealDevice` do build de campo
    /// tem caminho ate o fio, e nao so uma assinatura que compila.
    ///
    /// O `AparelhoFake` responde `Ok(vec![])` no `recv_raw`; o que importa
    /// aqui e que `info` **degrada honestamente**: `backend` = "real", e os
    /// campos que so o mock tem ficam no neutro em vez de virarem zero
    /// que a UI mostraria como dado.
    struct AparelhoFake {
        sent: Vec<Vec<u8>>,
    }

    impl std::fmt::Debug for AparelhoFake {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("AparelhoFake")
                .field("sent", &self.sent.len())
                .finish()
        }
    }

    impl DeviceTransport for AparelhoFake {
        fn open(&mut self) -> Result<(), TransportError> {
            Ok(())
        }
        fn close(&mut self) -> Result<(), TransportError> {
            Ok(())
        }
        fn send_raw(&mut self, data: &[u8], _kind: WireKind) -> Result<(), TransportError> {
            self.sent.push(data.to_vec());
            Ok(())
        }
        fn recv_raw(&mut self, _t: std::time::Duration) -> Result<Vec<u8>, TransportError> {
            Err(TransportError::RecvTimeout { timeout_ms: 0 })
        }
    }

    impl DeviceBackend for AparelhoFake {
        fn local_state(&self) -> Option<MockState> {
            None
        }
        fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
            Vec::new()
        }
    }

    #[test]
    fn um_backend_que_nao_e_mock_entra_na_mesma_fila() {
        let actor = DeviceActor::spawn(AparelhoFake { sent: Vec::new() }, Backend::Real);
        let st = actor.info().expect("info de um backend sem estado local");
        assert_eq!(st.backend, "real", "o backend vem DECLARADO, nao deduzido");
        assert_eq!(st.current_name, "", "sem estado local = vazio, nao nome");
        assert_eq!(st.current_pp_type, 0);
        assert_eq!(st.ir_slots_with_crc, 0);
        assert_eq!(st.preset_count, 0);
        // O knob dentro da faixa (15.0) sai; fora (99.5) e recusado ANTES do
        // `send_raw` — a trava da #110 e de conteudo, e vale no caminho
        // real tambem.
        actor
            .set_param(3, 0x0700_006e, 0, 15.0)
            .expect("dentro da faixa");
        assert!(
            actor.set_param(3, 0x0700_006e, 0, 99.5).is_err(),
            "acima do teto"
        );
        actor.shutdown();
    }

    /// **`save` e `dump` no app — as duas capacidades que eram só do CLI.**
    ///
    /// O `save` é a operação que PERSISTE: o `set-param` é fire-and-forget
    /// (§13.11, D4), então sem isto a mudança morre com a sessão. O `dump`
    /// é a leitura de campo que o `h2_field.sh` usa para conferir o antes e
    /// o depois na página 0 (é o que provou o F1 do H2).
    ///
    /// As contagens nao sao decorativas: sao as do §13.12 (5 writes de meta +
    /// 4 ops, D3) e as 9 transações do §13.9 (meta6 + 8 páginas). Um `save`
    /// que imprimisse "enviado" sem mandar os 9 frames seria o pior defeito
    /// possível — o operador leria, acreditaria, e o preset nunca foi
    /// gravado.
    #[test]
    fn save_e_dump_via_actor_sao_as_duas_capacidades_do_cli() {
        let mut mock = MockDevice::new().expect("mock montado");
        mock.open().expect("abre");
        let antes = mock.state().current_pp_type;
        let actor = DeviceActor::spawn(mock, Backend::Mock);
        actor.boot(None).expect("boot antes do dump");

        let dump = actor.dump_preset(0).expect("dump do pp corrente");
        assert_eq!(dump.pp, 0);
        assert!(!dump.meta6.is_empty(), "a meta6 veio com conteudo");
        assert_eq!(dump.pages.len(), 8, "§13.9: paginas 0..7 + a final");

        actor
            .save_preset(0, antes, "H2 APP")
            .expect("save §13.12: 9 frames");

        // O nome ASCII foi para o estado do mock (o `11000000` do §13.12).
        assert_eq!(actor.info().expect("info").current_pp_type, antes);
        actor.shutdown();
    }

    /// Nome com byte não-ASCII é recusado pelo codec ANTES do fio — e o
    /// mock fica com o nome antigo. Nao e um detalhe: o §13.12 grava ASCII
    /// cru e um byte alto viraria lixo no preset.
    #[test]
    fn save_recusa_nome_nao_ascii_sem_tocar_o_aparelho() {
        let mut mock = MockDevice::new().expect("mock montado");
        mock.open().expect("abre");
        let actor = DeviceActor::spawn(mock, Backend::Mock);
        let e = actor
            .save_preset(0, 4, "H2 Ã")
            .expect_err("byte nao-ASCII no nome");
        assert!(e.to_string().contains("ASCII"), "{e}");
        actor.shutdown();
    }

    /// Shutdown: depois dele, requisições novas falham com erro limpo
    /// (string) — nunca pânico no command.
    #[test]
    fn shutdown_depois_falha_limpo() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        actor.shutdown();
        assert!(actor.info().is_err(), "info pós-shutdown = erro, não panic");
    }

    /// Tabela via actor: 20 slots, nomes ASCII (vazio = 0xFF no fio).
    #[test]
    fn list_user_irs_via_actor() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        let slots = actor.list_user_irs().expect("tabela dos 20 IRs");
        assert_eq!(slots.len(), 20);
        assert_eq!(slots[0].0, 0, "slot 0");
        assert_eq!(slots[19].0, 19, "slot 19");
        actor.shutdown();
    }

    /// O boot deixa o backlog D7 na inbox do DEVICE: os nomes são
    /// fire-and-forget (D4 — a Session não espera) e o mock RESPONDE a
    /// eles (61 pushes @11000008) — a drenagem os devolve como hex
    /// (F0…F7) e a 2ª drenagem vem vazia (dreno esvazia).
    #[test]
    fn boot_deixa_backlog_d7_drenavel() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        actor.boot(None).expect("boot");
        let pushes = actor.drain_pushes().expect("drenagem");
        assert!(
            !pushes.is_empty(),
            "respostas tardias dos nomes = backlog D7 do device"
        );
        for h in &pushes {
            assert!(h.starts_with("F0") && h.ends_with("F7"), "SysEx: {h}");
        }
        assert!(actor.drain_pushes().expect("2ª").is_empty());
        actor.shutdown();
    }

    /// Upload de SnapTone pelo actor: um modelo de 19 bytes é UM bloco, e um
    /// bloco não paga settle (§4 — não há operação seguinte). Por isso este
    /// teste é rápido mesmo com o piso de 250 ms em produção: o caminho real
    /// do actor é exercitado sem transformar o teste em 36 s.
    #[test]
    fn upload_de_snap_tone_via_actor_fecha_o_stream() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        let rel = actor
            .upload_snap_tone(2, &[0xABu8; 19])
            .expect("upload de 1 bloco");
        assert_eq!(rel.slot, 2);
        assert_eq!(rel.blocks, 1);
        assert_eq!(rel.acks, 1, "ACK de 16B por bloco (§5)");
        assert_eq!(rel.bytes, 19);
        actor.shutdown();
    }

    /// Slot fora de 1..=5 é erro TIPADO até a borda do actor — e não um
    /// "enviado com sucesso" para um slot que não existe no aparelho
    /// (`SnapTone1..5`).
    #[test]
    fn upload_com_slot_invalido_e_erro() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        let e = actor
            .upload_snap_tone(6, &[0xABu8; 19])
            .expect_err("slot 6 nao existe");
        assert!(e.contains("1..=5"), "a mensagem diz o intervalo: {e}");
        actor.shutdown();
    }

    /// Fila serializa (D8): info + boot misturados de "duas threads" chegam
    /// na ordem de envio — sem interleave de transações.
    #[test]
    fn fila_serializa_requisicoes() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        let a = actor.clone();
        let t1 = std::thread::spawn(move || a.info().expect("info 1"));
        let st = actor.info().expect("info 2");
        let _ = t1.join().expect("thread 1");
        let report = actor.boot(None).expect("boot após infos");
        assert_eq!(st.preset_count, 99);
        assert_eq!(report.transactions, 2297);
        actor.shutdown();
    }

    /// #48 — ponta-a-ponta no shell: o transporte MORRE no meio da sessão
    /// (`MockFault::DieAfter`) e o erro sobe TIPADO (`ProtocolError::DeviceGone`)
    /// até a borda do actor — nunca achatado em `InvalidShape` ("transporte
    /// saudável"). O actor segue vivo (info responde), a morte aborta na hora
    /// (jamais espera a janela de 3 s) e o retry (⟳ da navbar) falha de novo,
    /// estável: o mock não ressuscita.
    #[test]
    fn device_morrendo_no_meio_da_sessao_nao_mata_o_actor() {
        let mock = MockDevice::new()
            .expect("mock montado")
            .with_fault(MockFault::DieAfter(300));
        let actor = DeviceActor::spawn(mock, Backend::Mock);

        let t0 = std::time::Instant::now();
        let err = actor.boot(None).expect_err("boot com o device morrendo");
        assert!(
            err.contains("device sumiu no meio da sessão"),
            "erro tipado do DeviceGone até o actor: {err}"
        );
        assert!(
            !err.contains("transporte saudável"),
            "DeviceGone não pode virar InvalidShape: {err}"
        );
        assert!(
            t0.elapsed() < std::time::Duration::from_secs(2),
            "a morte aborta a transação na hora (sem janela de 3 s): {:?}",
            t0.elapsed()
        );

        // O actor continua operante: o snapshot do mock não toca o fio.
        let st = actor.info().expect("info após a morte do device");
        assert_eq!(st.preset_count, 99);

        // Retry estável: a falha se repete rápida e tipada (device não volta).
        let t1 = std::time::Instant::now();
        let err2 = actor
            .boot(None)
            .expect_err("retry com o device ainda morto");
        assert!(err2.contains("device sumiu no meio da sessão"), "{err2}");
        assert!(
            t1.elapsed() < std::time::Duration::from_secs(2),
            "retry aborta na hora: {:?}",
            t1.elapsed()
        );
        actor.shutdown();
    }
}
