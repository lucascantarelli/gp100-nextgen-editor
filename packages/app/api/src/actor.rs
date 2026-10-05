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

/// Estado snapshot do mock (alias curto; o tipo vive no core).
type MockState = gp100_core::transport::mock::MockState;

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
        reply: mpsc::Sender<Result<MockState, String>>,
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
    /// Encerra a thread do actor (drop do `DeviceActor`).
    Shutdown,
}

/// Snapshot do estado via take/remount da Session (mesma semântica do
/// `Info` — mock: sem tráfego de fio). Helper interno do Library.
fn device_snapshot(session: &mut Option<Session<MockDevice>>) -> MockState {
    match session.take() {
        Some(s) => {
            let transport = s.into_transport();
            let st = transport.state().clone();
            *session = Some(Session::new(transport));
            st
        }
        None => MockState::load().unwrap_or_else(|_| MockState {
            preset_count: 0,
            current_pp: 0,
            current_name: String::new(),
            current_pp_type: 4,
            ir_crcs: [0; 20],
            set_params: Default::default(),
            snap_tone_model: Vec::new(),
            snap_tone_transfers: 0,
            snap_tone_acks: 0,
            rejected: 0,
        }),
    }
}

/// Handle do actor: clonável para múltiplos commands (a fila serializa).
#[derive(Clone)]
pub struct DeviceActor {
    tx: mpsc::Sender<Request>,
    handle: std::sync::Arc<std::sync::Mutex<Option<JoinHandle<()>>>>,
}

impl DeviceActor {
    /// Spawn do actor com o device (M1: mock — política ADR-4/ADR-5).
    /// Falha de pânico na thread (bug do core) vira erro de canal — nunca
    /// derruba o app.
    pub fn spawn(device: MockDevice) -> Self {
        let (tx, rx) = mpsc::channel::<Request>();
        let handle = std::thread::spawn(move || {
            let mut device = device;
            if let Err(e) = device.open() {
                eprintln!("[device] mock.open falhou: {e}");
                return;
            }
            let mut session = Some(Session::new(device));
            while let Ok(req) = rx.recv() {
                match req {
                    Request::Info { reply } => match session.take() {
                        Some(s) => {
                            let transport = s.into_transport();
                            let st = transport.state().clone();
                            session = Some(Session::new(transport));
                            let _ = reply.send(Ok(st));
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
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
                            let state = device_snapshot(&mut session); // corrente
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
                    Request::Shutdown => break,
                }
            }
        });
        Self {
            tx,
            handle: std::sync::Arc::new(std::sync::Mutex::new(Some(handle))),
        }
    }

    /// Snapshot do estado (mock: sem tráfego de fio — mesma semântica do
    /// `info` do CLI).
    ///
    /// # Erros
    /// String de erro se a thread do actor morreu (pânico do core).
    pub fn info(&self) -> Result<MockState, String> {
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

    /// Boot completo via actor (2297 transações no inventário default
    /// 0..198) e o resultado chega ao chamador pelo canal de resposta.
    #[test]
    fn boot_via_actor_completa_e_responde() {
        let mock = MockDevice::new().expect("mock montado");
        let actor = DeviceActor::spawn(mock);
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
        let actor = DeviceActor::spawn(mock);
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
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"));
        let st = actor.info().expect("snapshot");
        assert_eq!(st.preset_count, 99);
        assert_eq!(st.current_name, "It's GP100");
        let report = actor.boot(None).expect("boot após info");
        assert_eq!(report.transactions, 2297);
        actor.shutdown();
    }

    /// Shutdown: depois dele, requisições novas falham com erro limpo
    /// (string) — nunca pânico no command.
    #[test]
    fn shutdown_depois_falha_limpo() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"));
        actor.shutdown();
        assert!(actor.info().is_err(), "info pós-shutdown = erro, não panic");
    }

    /// Tabela via actor: 20 slots, nomes ASCII (vazio = 0xFF no fio).
    #[test]
    fn list_user_irs_via_actor() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"));
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
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"));
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
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"));
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
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"));
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
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"));
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
        let actor = DeviceActor::spawn(mock);

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
