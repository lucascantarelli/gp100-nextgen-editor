//! Coorte de ESCRITA — as `Request` que mandam algo ao device (`Select`,
//! `SetParam`, `SetGlobal`, `UploadSnapTone`, `UploadIr`, `Save`).
//!
//! Duas regras que valem para todas: (1) **a escrita é privilégio** — a trava
//! mora no TRANSPORTE (ADR-5, feature `write-verified`), então este módulo não
//! recria política nenhuma: ele só chama a `Session`; (2) **escrita aceita não
//! é escrita confirmada** (D4): o desfecho vira `SyncStatus::Pendente` pelo
//! [`super::estado::marca`] e quem confirma é o read-back do
//! [`super::relogio`]. Fonte: skill `app-dev` + `hardware-dev`.

use std::sync::mpsc;

use gp100_core::session::{IrUploadReport, SnapToneUploadReport};

use super::backends::DeviceBackend;
use super::estado::{marca, Cadencia, Estado};

/// Select de preset (write `13010000` §13.10; o mock troca o corrente).
pub(super) fn atende_select<T: DeviceBackend>(
    estado: &mut Estado<T>,
    pp: u16,
    reply: mpsc::Sender<Result<(), String>>,
    cad: &Cadencia<'_>,
) {
    let Estado {
        session,
        sync,
        sync_detail,
        debounced,
        proximo_refresh,
        ..
    } = estado;
    match session.as_mut() {
        Some(s) => {
            let r = s.select_preset(pp);
            marca(&r, sync, sync_detail, debounced, proximo_refresh, cad);
            let _ = reply.send(r.map_err(|e| e.to_string()));
        }
        None => {
            let _ = reply.send(Err("session do actor ausente".into()));
        }
    }
}

/// Set-param (knob §13.11, fire-and-forget D4; o mock valida e guarda).
pub(super) fn atende_set_param<T: DeviceBackend>(
    estado: &mut Estado<T>,
    slot: u8,
    code: u32,
    ctrl: u8,
    value: f32,
    reply: mpsc::Sender<Result<(), String>>,
    cad: &Cadencia<'_>,
) {
    let Estado {
        session,
        sync,
        sync_detail,
        debounced,
        proximo_refresh,
        ..
    } = estado;
    match session.as_mut() {
        Some(s) => {
            let r = s.set_param(slot, code, ctrl, value);
            // Escrita aceita sem resposta do fio (D4): até o read-back, o valor
            // é SÓ do app — o estado fica `Pendente` e a UI pode reconciliar
            // (#164).
            marca(&r, sync, sync_detail, debounced, proximo_refresh, cad);
            let _ = reply.send(r.map_err(|e| e.to_string()));
        }
        None => {
            let _ = reply.send(Err("session do actor ausente".into()));
        }
    }
}

/// Escrita de registrador (`write_register`).
pub(super) fn atende_set_global<T: DeviceBackend>(
    estado: &mut Estado<T>,
    addr: [u8; 4],
    payload: Vec<u8>,
    reply: mpsc::Sender<Result<(), String>>,
) {
    let Estado { session, .. } = estado;
    match session.as_mut() {
        Some(s) => {
            let r = s.write_register(addr, &payload).map_err(|e| e.to_string());
            let _ = reply.send(r);
        }
        None => {
            let _ = reply.send(Err("session do actor ausente".into()));
        }
    }
}

/// Upload de IR (blob em 20 slots) — sem marca de sync: é operação de slot,
/// não de preset corrente.
pub(super) fn atende_upload_ir<T: DeviceBackend>(
    estado: &mut Estado<T>,
    slot: u8,
    blob: Vec<u8>,
    reply: mpsc::Sender<Result<IrUploadReport, String>>,
) {
    let Estado { session, .. } = estado;
    match session.as_mut() {
        Some(s) => {
            let r = s.upload_ir(slot, &blob).map_err(|e| e.to_string());
            let _ = reply.send(r);
        }
        None => {
            let _ = reply.send(Err("session do actor ausente".into()));
        }
    }
}

/// Upload de SnapTone (modelo do app para o slot).
pub(super) fn atende_upload_snap_tone<T: DeviceBackend>(
    estado: &mut Estado<T>,
    slot: u8,
    model: Vec<u8>,
    reply: mpsc::Sender<Result<SnapToneUploadReport, String>>,
) {
    let Estado { session, .. } = estado;
    match session.as_mut() {
        Some(s) => {
            let r = s.upload_snap_tone(slot, &model).map_err(|e| e.to_string());
            let _ = reply.send(r);
        }
        None => {
            let _ = reply.send(Err("session do actor ausente".into()));
        }
    }
}

/// Save do preset (write no aparelho; o mock guarda no arquivo).
pub(super) fn atende_save<T: DeviceBackend>(
    estado: &mut Estado<T>,
    pp: u16,
    pp_type: u16,
    name: String,
    reply: mpsc::Sender<Result<(), String>>,
    cad: &Cadencia<'_>,
) {
    let Estado {
        session,
        sync,
        sync_detail,
        debounced,
        proximo_refresh,
        ..
    } = estado;
    match session.as_mut() {
        Some(s) => {
            let r = s.save_preset(pp, pp_type, &name);
            marca(&r, sync, sync_detail, debounced, proximo_refresh, cad);
            let _ = reply.send(r.map_err(|e| e.to_string()));
        }
        None => {
            let _ = reply.send(Err("session do actor ausente".into()));
        }
    }
}
