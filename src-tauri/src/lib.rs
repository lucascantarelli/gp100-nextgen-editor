//! gp100-ui — backend desktop (Tauri 2) do GP-100 NextGen Editor.
//!
//! **Papel (docs/UI_PLAN.md §2):** ponte IPC entre o front (`ui/`) e o
//! [gp100-core]. O crate conhece commands e DTOs; **toda regra de protocolo
//! fica no core** (R1). Na M1.0 (spike) o único backend é o
//! [`MockDevice`](gp100_core::transport::mock::MockDevice) — nenhum byte sai
//! para hardware (política ADR-4/ADR-5); a feature `real-device` (espelhada
//! do core) entra como build de campo pós-H1.
//!
//! **Contrato com o front:** os commands devolvem os mesmos shapes de
//! `ui/src/ipc/types.ts` (espelho manual; drift é pego pelo teste do command
//! contra o mock real). Na M1.1 o DeviceActor (D8: consumidor único da
//! Session) assume — aqui a prova é do caminho IPC ponta-a-ponta.
//!
//! **Windows-gnu:** o pin do workspace é o ambiente de dev/campo; o crate
//! compila e é testado nele (ver workflow CI, job `ui-rust`). Riscos de
//! ABI (WebView2 via webview2-com) são cobertos pelo build do binário.

use std::sync::Mutex;

use gp100_core::transport::mock::MockDevice;
use serde::Serialize;
use tauri::State;

/// Estado da aplicação: o device dono dos dados de device (D8 — consumidor
/// único). M1.0: o mock vive atrás de `Mutex` e NENHUM command abre transação
/// (só leitura de estado snapshot) — o actor da M1.1 substitui o Mutex sem
/// mudar o contrato dos commands.
pub struct AppState {
    device: Mutex<MockDevice>,
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
    /// imprime — fonte: [`MockState`](gp100_core::transport::mock::MockState)).
    fn from_mock(state: &gp100_core::transport::mock::MockState) -> Self {
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

/// `device_info` — estado do device para o front (M1.0: mock, sem tráfego).
///
/// # Erros
/// Se o mutex estiver envenenado (panic em outro command — não deve ocorrer:
/// leitura pura), devolve string de erro ao front (ScreenState.error com retry).
#[tauri::command]
pub fn device_info(state: State<'_, AppState>) -> Result<DeviceInfo, String> {
    let device = state.device.lock().map_err(|e| e.to_string())?;
    Ok(DeviceInfo::from_mock(&device.state()))
}

/// Boot do app Tauri: registra estado + commands (invocados pelo `main`).
///
/// # Erros
/// Propaga falha de setup do Tauri (janela/recursos) — o binário encerra.
pub fn run() -> tauri::Result<()> {
    let mock = MockDevice::new()
        .map_err(|e| tauri::Error::Setup(e.to_string()))?;
    tauri::Builder::default()
        .manage(AppState {
            device: Mutex::new(mock),
        })
        .invoke_handler(tauri::generate_handler![device_info])
        .run(tauri::generate_context!())
        .map_err(|e| {
            // `run` consome o builder; o erro de runtime é fatal no binário.
            eprintln!("[!] runtime do Tauri encerrou com erro: {e}");
            e
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O DTO do command é derivado do MOCK REAL (nunca de valores inventados):
    /// mesmo estado que o CLI `info` imprime — R1 também nos DTOs.
    #[test]
    fn device_info_deriva_do_mock_real() {
        let mock = MockDevice::new().expect("mock montado (R4 travado no build)");
        let info = DeviceInfo::from_mock(&mock.state());
        assert_eq!(info.backend, "mock");
        assert_eq!(info.preset_count, 99);
        assert_eq!(info.current_pp, 0x0000);
        assert_eq!(info.current_name, "It's GP100");
        assert_eq!(info.ir_slots_with_crc, 20);
        // ppType do 1º preset — vem do .prst, o teste fixa o valor do mock.
        assert_eq!(info.current_pp_type, 4);
    }

    /// O serde em camelCase é o CONTRATO do fio IPC (ui/src/ipc/types.ts):
    /// se alguém renomear campo, o JSON diverge do TS — este teste quebra.
    #[test]
    fn device_info_serializa_camelcase() {
        let mock = MockDevice::new().expect("mock montado");
        let info = DeviceInfo::from_mock(&mock.state());
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
}
