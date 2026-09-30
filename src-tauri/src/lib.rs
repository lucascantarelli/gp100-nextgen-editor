//! gp100-ui — backend desktop (Tauri 2) do GP-100 NextGen Editor.
//!
//! **Papel (docs/UI_PLAN.md §2):** ponte IPC entre o front (`ui/`) e o
//! [gp100-core]. O crate conhece commands e DTOs; **toda regra de protocolo
//! fica no core** (R1). Na M1.0 (spike) o único backend é o
//! `MockDevice` — nenhum byte sai para hardware (política ADR-4/ADR-5);
//! a feature `real-device` (espelhada do core) entra como build de campo
//! pós-H1.
//!
//! **Windows:** o crate é compilado com toolchain **MSVC** (pin local em
//! `rust-toolchain.toml`; ADR-7) — o Tauri 2 não suporta windows-gnu, que
//! morre com STATUS_ACCESS_VIOLATION no build script.

mod commands;

use gp100_core::transport::mock::MockDevice;

/// Boot do app Tauri: registra estado + commands (invocado pelo `main`).
///
/// # Erros
/// Propaga falha de setup/runtime do Tauri (janela/recursos/assets) — o
/// binário encerra com exit ≠ 0.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mock = MockDevice::new()?;
    tauri::Builder::default()
        .manage(commands::AppState {
            device: std::sync::Mutex::new(mock),
        })
        .invoke_handler(tauri::generate_handler![
            commands::device_info,
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
