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

use gp100_core::transport::mock::{MockDevice, MockFault};

/// Plano de falha do shell lido do ambiente — gancho de teste/e2e do #48.
///
/// `GP100_DEBUG_FAULT=die-after:<n>`: o device MOCK "cai" depois de `n`
/// transmissões (todo `send_raw`/`recv_raw`/`open` seguinte devolve
/// [`TransportError::DeviceGone`](gp100_core::transport::TransportError)) —
/// prova ponta-a-ponta o cenário de USB removido no meio da sessão: shell →
/// actor → command → UI (o front, sozinho, só sabia simulá-lo por
/// localStorage). Valor ausente/malformado = backend saudável (default).
///
/// **Exclusivo do backend MOCK:** o transporte real (`real-device`) não lê
/// este env — a política de hardware (ADR-4/ADR-5) segue intocada.
///
/// [`TransportError::DeviceGone`]: gp100_core::transport::TransportError::DeviceGone
fn debug_fault_from_env() -> Option<MockFault> {
    let raw = std::env::var("GP100_DEBUG_FAULT").ok()?;
    let fault = parse_debug_fault(&raw);
    if fault.is_none() {
        eprintln!("GP100_DEBUG_FAULT ignorado (esperado `die-after:<n>`): {raw}");
    }
    fault
}

/// Parser puro do gancho de falha (testável sem mutar o ambiente global).
///
/// Aceita `die-after:<n>` com espaços acidentais (trim); devolve `None` para
/// qualquer outra forma (nunca pânico — env de debug não derruba o app).
fn parse_debug_fault(raw: &str) -> Option<MockFault> {
    let n = raw.trim().strip_prefix("die-after:")?;
    n.trim().parse::<u32>().ok().map(MockFault::DieAfter)
}

/// Boot do app Tauri: registra estado + commands (invocado pelo `main`).
///
/// # Erros
/// Propaga falha de setup/runtime do Tauri (janela/recursos/assets) — o
/// binário encerra com exit ≠ 0.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut mock = MockDevice::new()?;
    if let Some(fault) = debug_fault_from_env() {
        eprintln!("GP100_DEBUG_FAULT armado: {fault:?} (backend mock)");
        mock = mock.with_fault(fault);
    }
    let actor = actor::DeviceActor::spawn(mock);
    let result = tauri::Builder::default()
        .manage(commands::AppState {
            actor: actor.clone(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::device_info,
            commands::device_board,
            commands::device_preset_library,
            commands::device_select_preset,
            commands::device_set_param,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// O gancho aceita a forma canônica (e tolera espaços do shell).
    #[test]
    fn parse_debug_fault_aceita_die_after() {
        assert_eq!(
            parse_debug_fault("die-after:0"),
            Some(MockFault::DieAfter(0))
        );
        assert_eq!(
            parse_debug_fault("die-after:300"),
            Some(MockFault::DieAfter(300))
        );
        assert_eq!(
            parse_debug_fault("  die-after:42  "),
            Some(MockFault::DieAfter(42))
        );
    }

    /// Qualquer outra forma é ignorada (backend saudável) — env de debug
    /// nunca derruba o app nem arma algo inesperado.
    #[test]
    fn parse_debug_fault_rejeita_forma_desconhecida() {
        for raw in [
            "",
            "die-after:",
            "die-after:abc",
            "die-after:-1",
            "other:1",
            "300",
        ] {
            assert_eq!(parse_debug_fault(raw), None, "raw={raw:?}");
        }
    }
}
