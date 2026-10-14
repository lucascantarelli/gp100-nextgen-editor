//! Coorte de DIAGNÓSTICO e ciclo de SESSÃO — o que não lê nem escreve preset:
//! `Conectar` (reconexão, #150), `SyncNow` (adianta o read-back, #164) e o log
//! de fio (`LogSession`, `LogStop`, `LogPath`).
//!
//! O log de fio é **decorador transparente** no transporte
//! (`LoggingTransport`): ligar/desligar remonta a Session por
//! `into_transport`/`Session::new` sem reiniciar o device. O caminho ativo mora
//! no ACTOR ([`Estado::log_path`]) e não no transporte — o `WireLogger` guarda
//! o arquivo, não o nome dele, e quem pergunta é a tela (`device_log_path`).

use std::sync::mpsc;
use std::time::Instant;

use gp100_core::transport::DeviceTransport;

use super::backends::DeviceBackend;
use super::estado::{emite, Cadencia, Estado};
use super::tipos::Backend;
use super::tipos::{SyncEvent, SyncStatus};

/// `device_conectar` — reabre o transporte no MESMO objeto (ADR-4).
pub(super) fn atende_conectar<T: DeviceBackend>(
    estado: &mut Estado<T>,
    reply: mpsc::Sender<Result<(), String>>,
    cad: &Cadencia<'_>,
) {
    let Cadencia { sink, debounce } = *cad;
    let Estado {
        backend,
        transporte,
        session,
        sync,
        sync_detail,
        debounced,
        proximo_refresh,
        ..
    } = estado;
    // Reconexão no MESMO transporte (ADR-4): com sessão viva o open é refeito
    // por cima (device sumiu e voltou — o backlog D7 é descartado, como no
    // remount do Info); sem sessão é o PRIMEIRO open (app que subiu sem
    // aparelho). No build de campo o `ProcuraAparelho` re-enumera as portas — e,
    // achando o aparelho, o estado sobe de `Desligado` para `Real` (o open do
    // `RealDevice` SÓ sucede com o aparelho presente, então a promoção é
    // evidência, não chute).
    let mut t = match session.take() {
        Some(s) => s.into_transport(),
        None => match transporte.take() {
            Some(t) => t,
            None => {
                let _ = reply.send(Err("sem transporte para reconectar".into()));
                return;
            }
        },
    };
    match t.open() {
        Ok(()) => {
            if *backend == Backend::Desligado {
                *backend = Backend::Real;
                eprintln!("[device] aparelho conectado pelo device_conectar");
            }
            *session = Some(gp100_core::session::Session::new(t));
            // #164: reconectou mas ainda não leu nada — `Inativo` (nunca
            // "sincronizado" sem prova); o read-back do debounce confirma em
            // seguida.
            *sync = SyncStatus::Inativo;
            sync_detail.clear();
            *debounced = Some(Instant::now() + debounce);
            emite(
                sink,
                SyncEvent {
                    status: sync.as_str(),
                    changed: false,
                    detail: String::new(),
                },
            );
            let _ = reply.send(Ok(()));
        }
        Err(e) => {
            *transporte = Some(t);
            *sync = SyncStatus::Offline;
            *sync_detail = format!("aparelho não conectou: {e}");
            *debounced = None;
            *proximo_refresh = None;
            emite(
                sink,
                SyncEvent {
                    status: sync.as_str(),
                    changed: false,
                    detail: sync_detail.clone(),
                },
            );
            let _ = reply.send(Err(format!("aparelho não conectou: {e}")));
        }
    }
}

/// `device_sync_now` — adianta o read-back sem esperar o refresh de cadência.
///
/// Só adianta o prazo no RELÓGIO da fila: o braço do read-back roda no próximo
/// passo do loop, com os MESMOS caminhos de falha (DeviceGone → Offline,
/// decode → Falhou) — sem caminho paralelo de leitura.
pub(super) fn atende_sync_now<T: DeviceBackend>(
    estado: &mut Estado<T>,
    reply: mpsc::Sender<Result<(), String>>,
) {
    estado.debounced = Some(Instant::now());
    let _ = reply.send(Ok(()));
}

/// `device_log_start` — liga o log de fio no caminho pedido.
pub(super) fn atende_log_session<T: DeviceBackend>(
    estado: &mut Estado<T>,
    path: String,
    reply: mpsc::Sender<Result<bool, String>>,
) {
    let Estado {
        session, log_path, ..
    } = estado;
    let Some(s) = session.take() else {
        let _ = reply.send(Err("session do actor ausente".into()));
        return;
    };
    let mut t = s.into_transport();
    let r = t.enable_log(std::path::Path::new(&path));
    *session = Some(gp100_core::session::Session::new(t));
    // So um log que ABRIU passa a ser o ativo: um caminho que nao pode ser
    // criado nao pode aparecer na tela como "gravando".
    if r.is_ok() {
        *log_path = Some(path);
    }
    let _ = reply.send(r.map(|()| true));
}

/// `device_log_stop` — desliga o log (a sessão segue; só o log para).
pub(super) fn atende_log_stop<T: DeviceBackend>(
    estado: &mut Estado<T>,
    reply: mpsc::Sender<Result<bool, String>>,
) {
    let Estado {
        session, log_path, ..
    } = estado;
    let Some(s) = session.take() else {
        let _ = reply.send(Err("session do actor ausente".into()));
        return;
    };
    let mut t = s.into_transport();
    t.logger = None;
    *session = Some(gp100_core::session::Session::new(t));
    *log_path = None;
    let _ = reply.send(Ok(true));
}

/// `device_log_path` — em que arquivo o log está gravando agora.
pub(super) fn atende_log_path<T: DeviceBackend>(
    estado: &mut Estado<T>,
    reply: mpsc::Sender<Option<String>>,
) {
    // Clonado: o estado continua no actor, a resposta é do chamador (a tela
    // precisa do nome, não da posse dele).
    let _ = reply.send(estado.log_path.clone());
}
