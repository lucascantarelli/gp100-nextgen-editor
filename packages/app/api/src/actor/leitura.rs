//! Coorte de LEITURA — as `Request` que só perguntam (`Info`, `Board`,
//! `Library`, `Globals`, `ListIrs`, `DrainPushes`, `Dump`).
//!
//! Regra que vale para todas: nada aqui inventa dado. Onde o aparelho não tem
//! estado local (o `MockState` é só do mock), o palco vem do CACHE do scan
//! (#155 §5.1/§5.2) e a ausência de cache vira ERRO honesto (#150 + spec §6) —
//! projetar o `all.prst` no palco do aparelho desenharia uma cadeia que não
//! está gravada nele. Fonte: skill `app-dev` + `docs/referencias/REAL_DEVICE_GAP.md`.

use std::sync::mpsc;

use gp100_core::model::Dictionary;
use gp100_core::pedalboard::{board_view_for, embedded_document, preset_list, BoardView};
use gp100_core::session::{inventario_do_aparelho, GlobalBruto, Session};

use super::backends::DeviceBackend;
use super::estado::{device_snapshot, Estado};
use super::tipos::{hex, Backend, DeviceSnapshot, DumpReport, PresetEntry, PresetLibrary};

/// `device_info` — o que o app pode AFIRMAR sobre o device (por backend).
pub(super) fn atende_info<T: DeviceBackend>(
    estado: &mut Estado<T>,
    reply: mpsc::Sender<Result<DeviceSnapshot, String>>,
) {
    let Estado {
        backend,
        motivo,
        session,
        nomes,
        sync,
        sync_detail,
        ..
    } = estado;
    let mut snap = device_snapshot(session, *backend, nomes);
    if *backend == Backend::Desligado {
        snap.detail = motivo.clone();
    }
    snap.sync = sync.as_str();
    snap.sync_detail = sync_detail.clone();
    let _ = reply.send(Ok(snap));
}

/// `device_board` — o palco do preset (slots/arquétipos/knobs).
pub(super) fn atende_board<T: DeviceBackend>(
    estado: &mut Estado<T>,
    pp: Option<u16>,
    reply: mpsc::Sender<Result<BoardView, String>>,
) {
    let Estado {
        backend,
        session,
        boards,
        ..
    } = estado;
    // #155 §5.2: o palco vem do CACHE do scan, nunca de um select novo — ler
    // OUTRO pp não pode mudar o preset que o pedal está mostrando.
    //
    // Sem cache (boot não rodou, ou pp fora do inventário) cai na projeção pura
    // do `all.prst`, o caminho de sempre. Com cache mas sem decode, ERRO: spec
    // §6 — um `BoardView` com chute é pior que erro, porque o palco desenha e o
    // usuário acredita.
    let alvo = pp.or_else(|| session.as_ref().map(|s| s.current_pp()));
    let r = (|| -> Result<BoardView, String> {
        // **O palco vem do CACHE do scan (#155 §5.2)** — as páginas que o boot
        // já leu, decodificadas. Nunca de um `select` novo: ler outro pp não
        // pode mudar o preset que o pedal está mostrando.
        if let Some(a) = alvo {
            if let Some(b) = boards.get(&a) {
                return Ok(b.clone());
            }
            if let Some(pags) = session.as_ref().and_then(|s| s.preset_state(a)) {
                let dict = Dictionary::from_json(gp100_core::model::DICTIONARY_JSON)
                    .map_err(|e| e.to_string())?;
                let off =
                    gp100_core::preset_pages::Offsets::carregado().map_err(|e| e.to_string())?;
                let dec = gp100_core::preset_pages::decode(pags).map_err(|e| e.to_string())?;
                let b = dec.slots(a, &dict, &off).map_err(|e| e.to_string())?;
                boards.insert(a, b.clone());
                return Ok(b);
            }
        }
        // Sem cache no APARELHO: erro honesto (#150 + spec §6) — projetar o
        // `all.prst` no palco do aparelho desenharia uma cadeia que não está
        // gravada nele.
        if *backend == Backend::Real {
            return Err("páginas deste pp não foram lidas no boot — refaça o boot \
                 (o palco do aparelho vem do cache, nunca do artefato)"
                .into());
        }
        // Mock: projeção PURA do artefato (doc embedado + dicionário) — não
        // toca a Session nem o fio.
        let doc = embedded_document().map_err(|e| e.to_string())?;
        let dict =
            Dictionary::from_json(gp100_core::model::DICTIONARY_JSON).map_err(|e| e.to_string())?;
        board_view_for(&doc, &dict, pp).map_err(|e| e.to_string())
    })();
    let _ = reply.send(r);
}

/// `library_list` — a biblioteca de presets (o flight case da UI).
pub(super) fn atende_library<T: DeviceBackend>(
    estado: &mut Estado<T>,
    reply: mpsc::Sender<Result<PresetLibrary, String>>,
) {
    let Estado {
        backend,
        session,
        nomes,
        ..
    } = estado;
    let r = (|| -> Result<PresetLibrary, String> {
        // App desligado (#150): a biblioteca NÃO é servida do artefato — sem
        // aparelho, o front mostra o aviso de conexão, não uma lista falsa.
        if session.is_none() {
            return Err("session do actor ausente".into());
        }
        // **No APARELHO a biblioteca é o que ele tem (#150).** O inventário é o
        // MEDIDO (captura S1 — ADR-12) e o NOME vem do CACHE do scan (#155 §5.1
        // — pg0 decodificada, custo zero no fio). Sem cache, vazio: honestidade
        // — nunca o dicionário de fábrica emprestado do all.prst, que não diz o
        // que está gravado no slot do dono.
        if *backend == Backend::Real {
            let state = device_snapshot(session, *backend, nomes);
            return Ok(PresetLibrary {
                entries: inventario_do_aparelho()
                    .into_iter()
                    .map(|pp| PresetEntry {
                        pp,
                        name: nomes.get(&pp).cloned().unwrap_or_default(),
                        pp_type_name: String::new(),
                    })
                    .collect(),
                current_pp: state.current_pp,
            });
        }
        let doc = embedded_document().map_err(|e| e.to_string())?;
        let state = device_snapshot(session, *backend, nomes); // corrente
        Ok(PresetLibrary {
            entries: preset_list(&doc)
                .into_iter()
                .map(|e| PresetEntry {
                    pp: e.pp,
                    // #155 §5.1: o nome real do fio (pg0 decodificada) no lugar
                    // do do arquivo — o que muda é o user patch renomeado no
                    // aparelho.
                    name: nomes.get(&e.pp).cloned().unwrap_or(e.name),
                    pp_type_name: e.pp_type_name,
                })
                .collect(),
            current_pp: state.current_pp,
        })
    })();
    let _ = reply.send(r);
}

/// Registradores globais (`read_globals`).
pub(super) fn atende_globals<T: DeviceBackend>(
    estado: &mut Estado<T>,
    reply: mpsc::Sender<Result<Vec<GlobalBruto>, String>>,
) {
    let Estado { session, .. } = estado;
    match session.as_mut() {
        Some(s) => {
            let r = s.read_globals().map_err(|e| e.to_string());
            let _ = reply.send(r);
        }
        None => {
            let _ = reply.send(Err("session do actor ausente".into()));
        }
    }
}

/// Tabela de 20 User IRs (§13.12) — existe nos dois backends.
pub(super) fn atende_list_irs<T: DeviceBackend>(
    estado: &mut Estado<T>,
    reply: mpsc::Sender<Result<Vec<(u8, String)>, String>>,
) {
    let Estado { session, .. } = estado;
    match session.as_mut() {
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
    }
}

/// Backlog de mensagens que chegaram ao device e ninguém pediu (D7 — mock).
pub(super) fn atende_drain_pushes<T: DeviceBackend>(
    estado: &mut Estado<T>,
    reply: mpsc::Sender<Result<Vec<String>, String>>,
) {
    let Estado { session, .. } = estado;
    match session.take() {
        Some(s) => {
            let mut transport = s.into_transport();
            let hexes: Vec<String> = transport
                .drain_inbox()
                .iter()
                .map(|m| m.iter().map(|b| format!("{b:02X}")).collect())
                .collect();
            *session = Some(Session::new(transport));
            let _ = reply.send(Ok(hexes));
        }
        None => {
            let _ = reply.send(Err("session do actor ausente".into()));
        }
    }
}

/// `dump_preset` — meta6 + páginas de estado, em hex cru (por decisão).
pub(super) fn atende_dump<T: DeviceBackend>(
    estado: &mut Estado<T>,
    pp: u16,
    reply: mpsc::Sender<Result<DumpReport, String>>,
) {
    let Estado { session, .. } = estado;
    match session.as_mut() {
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
    }
}
