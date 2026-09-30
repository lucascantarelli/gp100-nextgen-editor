//! DeviceActor — dono ÚNICO da `Session` (D8 do ADR-6; docs/UI_PLAN.md §2).
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
//! Progresso do boot (M1.1): o command passa um `mpsc::Sender<BootProgress>`
//! (Send) na requisição; o actor fecha um callback em volta do sender e o
//! `device_boot` encaminha cada beat como evento Tauri `device://progress`
//! (a UI mostra barra — nunca trava). Callback por REFERÊNCIA não atravessa
//! a fila (não é Send); canal é o padrão seguro.
//!
//! Inventário: o actor usa o DEFAULT da `Session` (0..198). O inventário
//! da captura (pp corrente primeiro + duplicação do 0x0100) é artefato de
//! REPLAY (M0.6) — o app não reproduz quirks de captura, apenas o script.

use std::sync::mpsc;
use std::thread::JoinHandle;

use gp100_core::session::{BootProgress, BootReport, Session};
use gp100_core::transport::mock::MockDevice;
use gp100_core::transport::DeviceTransport;

/// Estado snapshot do mock (alias curto; o tipo vive no core).
type MockState = gp100_core::transport::mock::MockState;

/// Requisições atendidas pelo actor (uma por vez — D8).
enum Request {
    /// Snapshot do estado do device (command `device_info`).
    Info {
        reply: mpsc::Sender<Result<MockState, String>>,
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
    /// Encerra a thread do actor (drop do `DeviceActor`).
    Shutdown,
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
    /// `info` do CLI M0.7).
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
}
