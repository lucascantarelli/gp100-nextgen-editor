//! Estado vivo do actor — o que a thread DONA da `Session` guarda entre duas
//! requisições (ADR-6/D8: posse única; nunca uma `Mutex<Session>`).
//!
//! Três grupos: (a) a sessão e o transporte, incluindo o caminho de
//! reconexão do app que subiu sem aparelho (#150); (b) os caches do scan
//! (#155 §5.1/§5.2 — `pp → nome` e `pp → BoardView`), que moram no ACTOR e não
//! na `Session` de propósito: `device_snapshot` remonta a Session a cada
//! `Info`/`Library` e os dados do boot têm de sobreviver; (c) o relógio da
//! convergência (#164) — os prazos do read-back, de quem o [`super::relogio`]
//! é dono.
//!
//! Aqui também ficam os dois auxiliares que o resto do actor compartilha —
//! [`device_snapshot`] (take/remount da Session) e [`emite`] (o evento
//! `device://sync`) —, além de [`marca`], a classificação do desfecho de uma
//! ESCRITA (era o `macro_rules! marca` do arquivo único).

use std::collections::BTreeMap;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use gp100_core::pedalboard::BoardView;
use gp100_core::session::Session;
use gp100_core::transport::DeviceTransport;
use gp100_core::wire_log::LoggingTransport;

use super::backends::DeviceBackend;
use super::tipos::{Backend, DeviceSnapshot, SyncEvent, SyncStatus, WRITE_VERIFIED};

/// Destinatário dos eventos `device://sync` (#164) — registrado pelo `run()`
/// depois do build do Tauri. Sem sink os eventos são descartados (o estado
/// continua consultável pelo `info`).
pub(super) type SyncSink = Arc<Mutex<Option<mpsc::Sender<SyncEvent>>>>;

/// Cadência da convergência (#164) — o par que TODA escrita marcada carrega:
/// o `debounce` do read-back pós-escrita e o [`SyncSink`] por onde o evento
/// `device://sync` sai.
///
/// O par anda junto de propósito: marcar o estado sem avisar a tela (ou o
/// inverso) é justamente o desencontro que a convergência existe para evitar —
/// com um valor só, nenhum braço da fila consegue metade da decisão.
#[derive(Clone, Copy)]
pub(super) struct Cadencia<'a> {
    /// Destinatário do evento `device://sync` (pode não ter quem escuta).
    pub(super) sink: &'a SyncSink,
    /// Prazo do read-back agendado depois de uma escrita aceita (§13, ADR-15).
    pub(super) debounce: Duration,
}

/// Tudo o que a thread do actor possui enquanto atende a fila.
pub(super) struct Estado<T: DeviceBackend> {
    /// Backend declarado por quem montou o transporte (ADR-4: a trait de bytes
    /// não responde "sou mock?").
    pub(super) backend: Backend,
    /// Motivo humano do estado `Desligado` (#150) — o texto que a UI mostra.
    pub(super) motivo: String,
    /// Transporte guardado quando NÃO há sessão (open falhou / app sem
    /// aparelho): é ele que o `device_conectar` reabre (ADR-4).
    pub(super) transporte: Option<LoggingTransport<T>>,
    /// A sessão viva, quando o open sucedeu.
    pub(super) session: Option<Session<LoggingTransport<T>>>,
    /// `pp → nome` decodificado do scan (#155 §5.1).
    pub(super) nomes: BTreeMap<u16, String>,
    /// `pp → BoardView` do scan (#155 §5.2).
    pub(super) boards: BTreeMap<u16, BoardView>,
    /// Estado de convergência app ↔ device (#164).
    pub(super) sync: SyncStatus,
    /// Detalhe humano de `Falhou`/`Offline` (vazio nos outros estados).
    pub(super) sync_detail: String,
    /// Prazo do debounce do read-back pós-escrita (#164).
    pub(super) debounced: Option<Instant>,
    /// Prazo do refresh periódico do preset corrente (#164).
    pub(super) proximo_refresh: Option<Instant>,
    /// Caminho do log de fio ATIVO — estado do actor, não do transporte (o
    /// `WireLogger` guarda o arquivo, não o nome dele; quem pergunta é a tela).
    pub(super) log_path: Option<String>,
}

impl<T: DeviceBackend> Estado<T> {
    /// Monta o estado: abre o transporte e fixa o estado inicial de
    /// sincronização. **O open que falha NÃO encerra a thread (#150)** — o app
    /// segue no estado `Desligado` (aviso + Reconectar na tela) em vez de virar
    /// um actor morto cuja única resposta é "não está mais rodando"; o
    /// `device_conectar` refaz o open no MESMO transporte (ADR-4).
    pub(super) fn novo(device: T, backend: Backend, motivo: String) -> Self {
        // O transporte entra embrulhado no LoggingTransport desde o inicio: e um
        // decorador transparente (byte entra, byte sai, `kind` intacto), entao o
        // log pode ser LIGADO no meio da sessao sem que o app precise reiniciar
        // o device nem reconstruir a fila. O `drain_inbox` do backend mora por
        // tras do wrapper.
        let mut logged = LoggingTransport::new(device);
        let mut transporte: Option<LoggingTransport<T>> = None;
        let mut session: Option<Session<LoggingTransport<T>>> = None;
        match logged.open() {
            Ok(()) => session = Some(Session::new(logged)),
            Err(e) => {
                eprintln!(
                    "[device] open do backend {backend:?} falhou: {e} — o app segue sem aparelho (reconectar refaz o open)"
                );
                transporte = Some(logged);
            }
        }
        // Convergência (#164): sem sessão (open falhou / build sem transporte) o
        // estado já nasce Offline com o motivo — o chip nunca mente "conectado".
        let (sync, sync_detail) = if session.is_some() {
            (SyncStatus::Inativo, String::new())
        } else if motivo.is_empty() {
            (
                SyncStatus::Offline,
                "aparelho não conectou (abrir falhou)".to_string(),
            )
        } else {
            (SyncStatus::Offline, motivo.clone())
        };
        Self {
            backend,
            motivo,
            transporte,
            session,
            nomes: BTreeMap::new(),
            boards: BTreeMap::new(),
            sync,
            sync_detail,
            debounced: None,
            proximo_refresh: None,
            log_path: None,
        }
    }
}

/// Snapshot do estado via take/remount da Session (mesma semântica do `Info`).
///
/// **O que muda com o aparelho real:** o `MockState` so existe no mock, e
/// `local_state()` devolve `None` no `RealDevice`. Os campos que vinham dele
/// (nome do pp, ppType, CRC de fabrica) ficam no estado neutro em vez de
/// receber um zero que a UI mostraria como dado — ver [`DeviceSnapshot`].
pub(super) fn device_snapshot<T: DeviceBackend>(
    session: &mut Option<Session<T>>,
    backend: Backend,
    nomes: &BTreeMap<u16, String>,
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
    let pp_corrente = local.as_ref().map_or(current_pp, |m| m.current_pp);
    DeviceSnapshot {
        backend: backend.as_str(),
        // O motivo do Desligado é preenchido no braço do `Info` (quem o
        // conhece é o spawn, não esta função).
        detail: String::new(),
        preset_count: local.as_ref().map_or(0, |m| m.preset_count),
        current_pp: pp_corrente,
        // #155 §5.1: `current_name` vem do CACHE do scan (pg0 decodificada)
        // — é o nome que o aparelho ESTÁ mostrando. O `local` (mock) fica
        // de reserva para quando o boot não rodou; sem as duas fontes,
        // vazio (nunca um nome inventado).
        current_name: nomes
            .get(&pp_corrente)
            .cloned()
            .or_else(|| local.as_ref().map(|m| m.current_name.clone()))
            .unwrap_or_default(),
        current_pp_type: local.as_ref().map_or(0, |m| m.current_pp_type),
        ir_slots_with_crc: local
            .as_ref()
            .map_or(0, |m| m.ir_crcs.iter().filter(|c| **c != 0).count()),
        ir_slots,
        write_verified: WRITE_VERIFIED,
        sync: SyncStatus::Inativo.as_str(),
        sync_detail: String::new(),
    }
}

/// Envia um evento `device://sync` pelo sink registrado (#164). Sem sink
/// (testes que não observam eventos, ou antes do `run()` registrar) o evento
/// é descartado — o estado NÃO depende do sink, ele fica no `info`.
pub(super) fn emite(sink: &SyncSink, ev: SyncEvent) {
    if let Ok(g) = sink.lock() {
        if let Some(tx) = g.as_ref() {
            let _ = tx.send(ev);
        }
    }
}

/// Classificação do desfecho de uma ESCRITA no fio (#164): ok → `Pendente` (o
/// read-back do debounce confirma ou reconcilia); `DeviceGone` → `Offline` +
/// transporte guardado para o `device_conectar`; outro erro → `Falhou` (sessão
/// segue viva).
///
/// Era o `macro_rules! marca` do arquivo único — a mesma decisão, agora
/// chamável pelos quatro braços de escrita (`Select`, `SetParam`, `Save` e o
/// `else` do `Boot`).
pub(super) fn marca<O>(
    r: &Result<O, gp100_core::ProtocolError>,
    sync: &mut SyncStatus,
    sync_detail: &mut String,
    debounced: &mut Option<Instant>,
    proximo_refresh: &mut Option<Instant>,
    cad: &Cadencia<'_>,
) {
    let Cadencia { sink, debounce } = *cad;
    match r {
        Ok(_) => {
            *sync = SyncStatus::Pendente;
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
        }
        Err(e) => {
            if matches!(e, gp100_core::ProtocolError::DeviceGone { .. }) {
                *sync = SyncStatus::Offline;
                *sync_detail = format!("aparelho desconectado: {e}");
                // Sessão viva com transporte morto (ADR-4): retry tipado rápido
                // e reconexão por cima.
                *debounced = None;
                *proximo_refresh = None;
            } else {
                *sync = SyncStatus::Falhou;
                *sync_detail = e.to_string();
            }
            emite(
                sink,
                SyncEvent {
                    status: sync.as_str(),
                    changed: false,
                    detail: sync_detail.clone(),
                },
            );
        }
    }
}
