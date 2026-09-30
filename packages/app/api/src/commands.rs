//! Commands de device da UI (DeviceActor — fila serializada do actor).
//!
//! **Por que submódulo:** `generate_handler!` importa o macro oculto
//! `__cmd__<name>` — comando e handler no MESMO módulo colidem (E0255).
//! Estrutura canônica do Tauri: commands aqui, handler em `lib.rs` por
//! path.
//!
//! **D8:** nenhum command toca a `Session` direto — todos passam pela fila
//! do [`crate::actor::DeviceActor`]. O `device_boot` é LONGO (2297
//! transações contra o mock) e síncrono (ADR-3): o Tauri o executa fora da
//! main thread; o progresso sai por EVENTO (`device://progress`) — a UI
//! mostra barra, nunca trava.

use gp100_core::session::{BootProgress, BootStage};
use gp100_core::transport::mock::MockState;
use serde::Serialize;
use std::sync::mpsc;
use tauri::{Emitter, State};

use crate::actor::DeviceActor;

/// Estado da aplicação: o actor é o ÚNICO dono do device (D8 — consumidor
/// único do stream IN; nada de `Mutex<Session>` compartilhado com a UI).
pub struct AppState {
    /// Actor do device (M1: mock — política ADR-4/ADR-5).
    pub actor: DeviceActor,
}

/// DTO de `device_info` — MESMOS campos/semântica de `ui/src/ipc/types.ts`
/// (`DeviceInfo`). CamelCase no fio (serde) para o TS não precisar de mapping.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    /// Backend ativo: sempre "mock" nesta fase (política de hardware).
    pub backend: &'static str,
    /// Nº de presets do estado (mock: `all.prst` = 99).
    pub preset_count: usize,
    /// pp corrente (u16 no fio; aqui como número p/ o TS).
    pub current_pp: u16,
    /// Nome do pp corrente.
    pub current_name: String,
    /// ppType do pp corrente (semântica no dicionário do core).
    pub current_pp_type: u16,
    /// Slots de IR com CRC de fábrica (mock: 20).
    pub ir_slots_with_crc: usize,
}

impl DeviceInfo {
    /// Extrai o DTO do snapshot do mock (mesmos valores que o CLI `info`
    /// imprime — fonte: `MockState` do core; R1 também nos DTOs).
    pub fn from_mock(state: &MockState) -> Self {
        Self {
            backend: "mock",
            preset_count: state.preset_count,
            current_pp: state.current_pp,
            current_name: state.current_name.clone(),
            current_pp_type: state.current_pp_type,
            ir_slots_with_crc: state.ir_crcs.iter().filter(|c| **c != 0).count(),
        }
    }
}

/// DTO do beat de progresso do boot (evento `device://progress`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootProgressDto {
    /// Etapa do script (§13.10): "tables" | "scan" | "probe" | "setlist" |
    /// "names" | "keepalive" — literal estável para o front.
    pub stage: &'static str,
    /// Transações completas até agora.
    pub done: usize,
    /// Total esperado do script (função do inventário).
    pub total: usize,
    /// pp corrente após a transação (o scan avança a seleção).
    pub current_pp: u16,
}

impl From<BootProgress> for BootProgressDto {
    fn from(p: BootProgress) -> Self {
        Self {
            stage: match p.stage {
                BootStage::Tables => "tables",
                BootStage::Scan => "scan",
                BootStage::Probe => "probe",
                BootStage::Setlist => "setlist",
                BootStage::Names => "names",
                BootStage::Keepalive => "keepalive",
            },
            done: p.done,
            total: p.total,
            current_pp: p.current_pp,
        }
    }
}

/// DTO do relatório do boot (retorno do `device_boot`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootReportDto {
    /// Nº de transações de boot+scan executadas com sucesso.
    pub transactions: usize,
}

/// DTO de um slot da tabela de User IRs (`list_user_irs`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IrSlotDto {
    /// Slot 0..=19.
    pub slot: u8,
    /// Nome ASCII ("" = vazio/0xFF).
    pub name: String,
}

/// DTO da tabela completa (20 slots).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IrTableDto {
    /// Slots decodificáveis (§13.12: nome; CRC de slot ocupado é pendência).
    pub slots: Vec<IrSlotDto>,
}

/// `device_info` — estado do device para o front (mock: sem tráfego).
///
/// # Erros
/// String de erro se o actor morreu (pânico do core — não deve ocorrer;
/// ScreenState.error do front oferece retry).
#[tauri::command]
pub fn device_info(state: State<'_, AppState>) -> Result<DeviceInfo, String> {
    let snapshot = state.actor.info()?;
    Ok(DeviceInfo::from_mock(&snapshot))
}

/// `device_boot` — boot+scan completos (§13.10) com barra de progresso:
/// emite `device://progress` (BootProgressDto) por transação. BLOQUEIA até
/// o fim do script (command síncrono; o Tauri o roda fora da main thread —
/// ADR-3) e devolve o relatório final. O progresso atravessa a fronteira
/// do actor por CANAL (mpsc) e vira evento aqui na thread do command.
///
/// # Erros
/// [`ProtocolError`](gp100_core::ProtocolError) como string (Timeout D6,
/// InvalidShape D5) — o front mostra no ScreenState.error com retry.
#[tauri::command]
pub fn device_boot(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<BootReportDto, String> {
    // Beats chegam pelo canal; o recv é bloqeuante — mas o command é
    // síncrono fora da main thread e o actor produz beats a cada transação.
    // O canal é drenado NO CALLBACK de fim (boot terminou → drain garante
    // o último beat antes do retorno).
    let (ptx, prx) = mpsc::channel::<BootProgress>();
    let emitter = app.clone();
    // Encaminha os beats SEM bloquear o retorno do boot: o drain roda em
    // thread própria e reemite até o sender fechar (fim do boot no actor).
    let forward = std::thread::spawn(move || {
        for p in prx.iter() {
            let _ = emitter.emit("device://progress", BootProgressDto::from(p));
        }
    });
    let report = state.actor.boot(Some(ptx))?;
    let _ = forward.join(); // último beat emitido antes do retorno
                            // Pushes do boot (dump 13000000, nomes 11000008 etc. — backlog D7 da
                            // inbox do mock) saem como evento `device://push` para o log da UI.
    for hex in state.actor.drain_pushes()? {
        let _ = app.emit("device://push", hex);
    }
    Ok(BootReportDto {
        transactions: report.transactions,
    })
}

/// `list_user_irs` — tabela dos 20 User IRs (§13.12; by-len do golden
/// distingue a tabela 75B do ACK 4B — D1). Transação própria no actor.
///
/// # Erros
/// [`ProtocolError`] como string (Timeout/InvalidShape).
#[tauri::command]
pub fn list_user_irs(state: State<'_, AppState>) -> Result<IrTableDto, String> {
    let slots = state.actor.list_user_irs()?;
    Ok(IrTableDto {
        slots: slots
            .into_iter()
            .map(|(slot, name)| IrSlotDto { slot, name })
            .collect(),
    })
}

/// `pending_pushes` — backlog D7 drenado do DEVICE (inbox do mock, FIFO
/// global) como hex cru (F0…F7) para o log da UI. Consumidor alternativo
/// ao evento `device://push` (poll explícito do front).
///
/// # Erros
/// String de erro se o actor morreu.
#[tauri::command]
pub fn pending_pushes(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state.actor.drain_pushes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gp100_core::transport::mock::MockDevice;

    /// O DTO do command é derivado do MOCK REAL (nunca de valores
    /// inventados): mesmo estado que o CLI `info` imprime — R1 nos DTOs.
    #[test]
    fn device_info_deriva_do_mock_real() {
        let mock = MockDevice::new().expect("mock montado (R4 travado no build)");
        let info = DeviceInfo::from_mock(mock.state());
        assert_eq!(info.backend, "mock");
        assert_eq!(info.preset_count, 99);
        assert_eq!(info.current_pp, 0x0000);
        assert_eq!(info.current_name, "It's GP100");
        assert_eq!(info.ir_slots_with_crc, 20);
        // ppType do 1º preset — vem do .prst; o teste fixa o valor do mock.
        assert_eq!(info.current_pp_type, 4);
    }

    /// O serde em camelCase é o CONTRATO do fio IPC (ui/src/ipc/types.ts):
    /// se alguém renomear campo, o JSON diverge do TS — este teste quebra.
    #[test]
    fn device_info_serializa_camelcase() {
        let mock = MockDevice::new().expect("mock montado");
        let info = DeviceInfo::from_mock(mock.state());
        let json = serde_json::to_value(&info).expect("serializável");
        assert!(json.get("presetCount").is_some());
        assert!(json.get("currentPp").is_some());
        assert!(json.get("currentName").is_some());
        assert!(json.get("currentPpType").is_some());
        assert!(json.get("irSlotsWithCrc").is_some());
        assert!(json.get("backend").is_some());
        assert_eq!(json["currentPp"], 0);
        assert_eq!(json["currentName"], "It's GP100");
    }

    /// O beat de progresso serializa stage como literal ESTÁVEL (o front
    /// roteia por ele) e camelCase nos contadores.
    #[test]
    fn boot_progress_dto_serializa_camelcase() {
        let p = BootProgressDto {
            stage: "scan",
            done: 45,
            total: 2299,
            current_pp: 0x0100,
        };
        let json = serde_json::to_value(&p).expect("serializável");
        assert_eq!(json["stage"], "scan");
        assert_eq!(json["done"], 45);
        assert_eq!(json["total"], 2299);
        assert_eq!(json["currentPp"], 0x0100);
    }
}
