//! Coorte de BOOT — a `Request::Boot` e o que ela DEIXA no actor.
//!
//! O boot é a única passada longa (2297 transações contra o mock; 2299 contra
//! o aparelho — §13.10) e é dela que sai todo o cache do scan: **§5.1** grava
//! `pp → nome` (pg0 decodificada) e **§5.2** grava `pp → BoardView` NA MESMA
//! passada, AQUI no actor.
//!
//! O cache fica no actor de propósito: `device_snapshot` remonta a Session por
//! `into_transport` a cada `Info`/`Library` e descarta o cache de páginas DELA.
//! Com o palco dependendo só da Session, o primeiro desses calls destruía as
//! páginas do scan e o `device_board` seguinte respondia "páginas deste pp não
//! foram lidas no boot" — o palco morria segundos depois de um boot perfeito
//! (wire log de campo 12/10).
//!
//! O progresso atravessa a fronteira PELO CANAL: callback emprestado não é
//! `Send` e não atravessa a fila, então o command manda um
//! `mpsc::Sender<BootProgress>` e a thread do actor fecha o callback em volta
//! dele (evento `device://progress` — a UI mostra barra, nunca trava).

use std::sync::mpsc;
use std::time::{Duration, Instant};

use gp100_core::model::Dictionary;
use gp100_core::session::{BootProgress, BootReport};

use super::backends::DeviceBackend;
use super::estado::{emite, marca, Cadencia, Estado};
use super::tipos::{SyncEvent, SyncStatus};

/// Boot completo pelo caminho do actor, com o progresso reencaminhado.
pub(super) fn atende_boot<T: DeviceBackend>(
    estado: &mut Estado<T>,
    progress: Option<mpsc::Sender<BootProgress>>,
    reply: mpsc::Sender<Result<BootReport, String>>,
    cad: &Cadencia<'_>,
    intervalo: Duration,
) {
    let Estado {
        session,
        nomes,
        boards,
        sync,
        sync_detail,
        debounced,
        proximo_refresh,
        ..
    } = estado;
    match session.as_mut() {
        Some(s) => {
            // Callback roda NA THREAD do actor: fecha em volta do sender e
            // repassa por coerção para `&mut dyn FnMut` (canal é o padrão
            // seguro — callback emprestado não atravessa a fila).
            let r = match progress {
                Some(tx) => {
                    let mut cb = move |p: BootProgress| {
                        let _ = tx.send(p);
                    };
                    s.boot_with_progress(Some(&mut cb))
                }
                None => s.boot_with_progress(None),
            };
            if r.is_ok() {
                let dict = Dictionary::from_json(gp100_core::model::DICTIONARY_JSON).ok();
                let off = gp100_core::preset_pages::Offsets::carregado().ok();
                for pp in s.cached_pps() {
                    if let Some(pags) = s.preset_state(pp) {
                        if let Ok(pg) = gp100_core::preset_pages::decode(pags) {
                            if let Ok(nome) = pg.nome() {
                                nomes.insert(pp, nome.to_string());
                            }
                        }
                        if let (Some(dict), Some(off)) = (&dict, &off) {
                            if let Ok(dec) = gp100_core::preset_pages::decode(pags) {
                                if let Ok(b) = dec.slots(pp, dict, off) {
                                    boards.insert(pp, b);
                                }
                            }
                        }
                    }
                }
                // #164: o boot LEU tudo — sincronizado de direito, e o refresh
                // periódico passa a armar (10 s; poll é a única janela em que a
                // mudança do PEDAL aparece).
                *sync = SyncStatus::Sincronizado;
                sync_detail.clear();
                *proximo_refresh = Some(Instant::now() + intervalo);
                emite(
                    cad.sink,
                    SyncEvent {
                        status: sync.as_str(),
                        changed: false,
                        detail: String::new(),
                    },
                );
            } else {
                // Boot que falhou é uma escrita que falhou: mesma classificação
                // (o `marca!` do arquivo único usava o debounce da rajada, não a
                // cadência do refresh).
                marca(&r, sync, sync_detail, debounced, proximo_refresh, cad);
            }
            let _ = reply.send(r.map_err(|e| e.to_string()));
        }
        None => {
            let _ = reply.send(Err("session do actor ausente".into()));
        }
    }
}
