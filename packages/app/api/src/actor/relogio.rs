//! O relógio do actor: a fila com PRAZOS e o read-back bilateral (#164).
//!
//! Sem prazo o `recv` é bloqueante como sempre foi (nenhum poll ocioso,
//! nenhum sleep — a thread só acorda para fila ou prazo). Com prazo, os dois
//! vencimentos — o debounce da rajada de escrita e o refresh periódico — são
//! despachados por aqui, e o roteamento das `Request` para os módulos de
//! coorte ([`super::leitura`], [`super::escrita`], [`super::boot`],
//! [`super::diagnostico`]) mora no `match` deste loop.
//!
//! **O read-back relê as 9 páginas pelo caminho do BOOT**
//! (`Session::reload_paginas_correntes`: open → pg0 + PG 0..7 → págs 1..8,
//! §13.10 — tudo `WireKind::Read`, NENHUMA escrita nova) e reconcilia o cache
//! do actor: **o aparelho vence, sempre**. `SyncEvent.changed` só é `true`
//! quando as páginas divergem do cache.
//!
//! Este é o caminho mais quente depois da própria escrita: a latência do
//! `SetParam` inclui o debounce (dívida da #239). Não aumente o trabalho
//! dentro do loop sem pensar na fila.

use std::collections::BTreeMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use gp100_core::model::Dictionary;
use gp100_core::pedalboard::BoardView;
use gp100_core::session::Session;
use gp100_core::wire_log::LoggingTransport;

use super::backends::DeviceBackend;
use super::estado::{emite, Cadencia, Estado, SyncSink};
use super::fila::Request;
use super::tipos::{SyncEvent, SyncStatus};
use super::{boot, diagnostico, escrita, leitura};

/// Debounce do read-back pós-escrita (#164): uma RAJADA de knob (N giros
/// seguidos) vira UMA releitura das 9 páginas. 800 ms — medido contra o
/// custo real no fio (ADR-15: RTT mediano 3,05 ms no H1 → 9 páginas ≈ 27 ms;
/// o debounce custa ~3% do fio numa rajada de 2 s).
pub const READBACK_DEBOUNCE: Duration = Duration::from_millis(800);

/// Cadência do refresh periódico do preset corrente (#164): 10 s ≈ 0,9 tx/s
/// (≈3% do fio pelo RTT mediano do H1). É a ÚNICA janela em que o app vê
/// mudança feita no PEDAL: o knob físico não emite SysEx (ADR-15 — poll,
/// não notificação espontânea).
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(10);

/// Falha classificada do read-back (#164): `Caiu` = o transporte morreu
/// (DeviceGone — vira `Offline` e o guarda para reconexão); `Outro` = erro
/// de protocolo/decode (vira `Falhou` sem derrubar a sessão).
///
/// A separação existe para o front: só `Caiu` tira o app de "sessão viva".
enum FalhaLeitura {
    /// O transporte caiu (DeviceGone) — o aparelho sumiu do fio.
    Caiu(String),
    /// Decode/dicionário falhou — a sessão segue viva.
    Outro(String),
}

/// O read-back em si (#164): relê as 9 páginas do preset corrente pelo caminho
/// do boot e atualiza os caches do actor. `Ok(changed)` diz se o `BoardView`
/// mudou em relação ao cache — é o que o front usa para não repintar à toa.
fn reconciliar<T: DeviceBackend>(
    session: &mut Option<Session<LoggingTransport<T>>>,
    nomes: &mut BTreeMap<u16, String>,
    boards: &mut BTreeMap<u16, BoardView>,
) -> Result<bool, FalhaLeitura> {
    let s = session
        .as_mut()
        .ok_or_else(|| FalhaLeitura::Outro("sem sessão com aparelho".into()))?;
    let pp = s.current_pp();
    let pags = s.reload_paginas_correntes().map_err(|e| match e {
        gp100_core::ProtocolError::DeviceGone { why } => FalhaLeitura::Caiu(why),
        outro => FalhaLeitura::Outro(outro.to_string()),
    })?;
    let dec =
        gp100_core::preset_pages::decode(&pags).map_err(|e| FalhaLeitura::Outro(e.to_string()))?;
    let dict = Dictionary::from_json(gp100_core::model::DICTIONARY_JSON)
        .map_err(|e| FalhaLeitura::Outro(e.to_string()))?;
    let off = gp100_core::preset_pages::Offsets::carregado()
        .map_err(|e| FalhaLeitura::Outro(e.to_string()))?;
    let novo = dec
        .slots(pp, &dict, &off)
        .map_err(|e| FalhaLeitura::Outro(e.to_string()))?;
    if let Ok(nome) = dec.nome() {
        nomes.insert(pp, nome.to_string());
    }
    let changed = boards.get(&pp).is_none_or(|b| b != &novo);
    boards.insert(pp, novo);
    Ok(changed)
}

/// Roda a thread do actor até o `Shutdown` (ou até a fila morrer). Fila com
/// RELÓGIO: os prazos do read-back (debounce da rajada) e do refresh periódico
/// dividem o `recv` com a fila.
pub(super) fn roda<T: DeviceBackend + 'static>(
    mut estado: Estado<T>,
    rx: mpsc::Receiver<Request>,
    debounce: Duration,
    intervalo: Duration,
    sink: SyncSink,
) {
    // O par da convergência (#164) viaja junto para os braços de escrita: uma
    // escrita que marca o estado sem avisar a tela é metade da decisão.
    let cad = Cadencia {
        sink: &sink,
        debounce,
    };
    loop {
        let agora = Instant::now();
        let debounce_venceu = estado.debounced.is_some_and(|t| t <= agora);
        let mut refresh_venceu = estado.proximo_refresh.is_some_and(|t| t <= agora);
        // Rajada em curso: o refresh espera a rajada fechar — um read-back
        // cobre os dois (9 páginas lidas uma vez só).
        if refresh_venceu && !debounce_venceu && estado.debounced.is_some() {
            estado.proximo_refresh = Some(agora + intervalo);
            refresh_venceu = false;
        }
        if debounce_venceu || refresh_venceu {
            estado.debounced = None;
            if refresh_venceu {
                estado.proximo_refresh = Some(agora + intervalo);
            }
            let r = reconciliar(&mut estado.session, &mut estado.nomes, &mut estado.boards);
            let ev = match r {
                Ok(changed) => {
                    estado.sync = SyncStatus::Sincronizado;
                    estado.sync_detail.clear();
                    SyncEvent {
                        status: estado.sync.as_str(),
                        changed,
                        detail: String::new(),
                    }
                }
                Err(FalhaLeitura::Caiu(why)) => {
                    estado.sync = SyncStatus::Offline;
                    estado.sync_detail = format!("aparelho desconectado: {why}");
                    // A sessão FICA viva com o transporte morto (ADR-4: retry
                    // devolve DeviceGone tipado rápido, e o `device_conectar`
                    // reabre por cima — contrato do
                    // `device_morrendo_no_meio_da_sessao`).
                    estado.debounced = None;
                    estado.proximo_refresh = None;
                    SyncEvent {
                        status: estado.sync.as_str(),
                        changed: false,
                        detail: estado.sync_detail.clone(),
                    }
                }
                Err(FalhaLeitura::Outro(msg)) => {
                    estado.sync = SyncStatus::Falhou;
                    estado.sync_detail = msg.clone();
                    SyncEvent {
                        status: estado.sync.as_str(),
                        changed: false,
                        detail: msg,
                    }
                }
            };
            emite(&sink, ev);
            continue;
        }
        // Prazo mais próximo (ou fila, quando não há prazo).
        let espera = [estado.debounced, estado.proximo_refresh]
            .into_iter()
            .flatten()
            .map(|t| t.saturating_duration_since(agora))
            .min();
        let req = match espera {
            None => match rx.recv() {
                Ok(r) => r,
                Err(_) => break,
            },
            Some(d) => match rx.recv_timeout(d) {
                Ok(r) => r,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            },
        };
        // Roteamento por coorte: cada módulo traduz o seu grupo de `Request`
        // para a `Session` (leitura / escrita / boot / diagnóstico).
        match req {
            Request::SyncNow { reply } => diagnostico::atende_sync_now(&mut estado, reply),
            Request::Info { reply } => leitura::atende_info(&mut estado, reply),
            Request::Conectar { reply } => diagnostico::atende_conectar(&mut estado, reply, &cad),
            Request::Board { pp, reply } => leitura::atende_board(&mut estado, pp, reply),
            Request::Library { reply } => leitura::atende_library(&mut estado, reply),
            Request::Select { pp, reply } => escrita::atende_select(&mut estado, pp, reply, &cad),
            Request::SetParam {
                slot,
                code,
                ctrl,
                value,
                reply,
            } => escrita::atende_set_param(&mut estado, slot, code, ctrl, value, reply, &cad),
            Request::Globals { reply } => leitura::atende_globals(&mut estado, reply),
            Request::SetGlobal {
                addr,
                payload,
                reply,
            } => escrita::atende_set_global(&mut estado, addr, payload, reply),
            Request::Boot { progress, reply } => {
                boot::atende_boot(&mut estado, progress, reply, &cad, intervalo)
            }
            Request::ListIrs { reply } => leitura::atende_list_irs(&mut estado, reply),
            Request::DrainPushes { reply } => leitura::atende_drain_pushes(&mut estado, reply),
            Request::UploadSnapTone { slot, model, reply } => {
                escrita::atende_upload_snap_tone(&mut estado, slot, model, reply)
            }
            Request::UploadIr { slot, blob, reply } => {
                escrita::atende_upload_ir(&mut estado, slot, blob, reply)
            }
            Request::Save {
                pp,
                pp_type,
                name,
                reply,
            } => escrita::atende_save(&mut estado, pp, pp_type, name, reply, &cad),
            Request::Dump { pp, reply } => leitura::atende_dump(&mut estado, pp, reply),
            Request::LogSession { path, reply } => {
                diagnostico::atende_log_session(&mut estado, path, reply)
            }
            Request::LogStop { reply } => diagnostico::atende_log_stop(&mut estado, reply),
            Request::LogPath { reply } => diagnostico::atende_log_path(&mut estado, reply),
            Request::Shutdown => break,
        }
    }
}
