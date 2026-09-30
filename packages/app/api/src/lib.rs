//! gp100-ui — backend desktop (Tauri 2) do GP-100 NextGen Editor.
//!
//! **Papel:** ponte IPC entre o front (`ui/`) e o
//! [gp100-core]. O crate conhece commands, actor e DTOs; **toda regra de
//! protocolo fica no core** (R1). Na M1 o único backend é o `MockDevice` —
//! nenhum byte sai para hardware (política ADR-4/ADR-5); a feature
//! `real-device` (espelhada do core) entra como build de campo pós-H1.
//!
//! **DeviceActor:** o actor é o dono ÚNICO da `Session` (D8 do
//! ADR-6) — commands enfileiram requisições; o boot emite progresso via
//! evento `device://progress`. Ver `actor.rs`.
//!
//! **Windows:** o crate é compilado com toolchain **MSVC** (RUSTUP_TOOLCHAIN
//! nos workflows; canal "stable" portável em `rust-toolchain.toml`;
//! ADR-7/ADR-8) — o Tauri 2 não suporta windows-gnu, que morre com
//! STATUS_ACCESS_VIOLATION no build script.

mod actor;
mod commands;

use gp100_core::transport::mock::MockDevice;

/// Boot do app Tauri: registra estado + commands (invocado pelo `main`).
///
/// # Erros
/// Propaga falha de setup/runtime do Tauri (janela/recursos/assets) — o
/// binário encerra com exit ≠ 0.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mock = MockDevice::new()?;
    let actor = actor::DeviceActor::spawn(mock);
    let result = tauri::Builder::default()
        .manage(commands::AppState {
            actor: actor.clone(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::device_info,
            commands::device_boot,
            commands::list_user_irs,
            commands::pending_pushes,
        ])
        .run(tauri::generate_context!());
    // Ciclo de vida: o actor roda até o app fechar — shutdown explícito
    // (o handle é Clone; Drop em clone derrubaria o actor alheio).
    actor.shutdown();
    result.map_err(Into::into)
}
