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
mod library_commands;

use gp100_core::transport::mock::{MockDevice, MockFault};
use tauri::Manager;

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

/// Abre a biblioteca no diretorio de DADOS do app e semeia a fabrica na
/// primeira execucao (ADR-9, decisao 4).
///
/// # Erros
/// Propaga a falha do SQLite/disco. O chamador decide o que fazer — `run`
/// degrada para uma biblioteca em memoria em vez de derrubar a janela.
fn abrir_biblioteca(app: &tauri::App) -> Result<gp100_library::Library, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("caminho de dados do app: {e}"))?;
    let caminho = dir.join("biblioteca.sqlite");
    let lib = gp100_library::Library::open(&caminho).map_err(|e| e.to_string())?;

    if !lib.tem_fabrica().map_err(|e| e.to_string())? {
        match seed_de_fabrica(&lib) {
            Ok(n) => eprintln!("biblioteca: {n} presets de fabrica semeados"),
            Err(e) => eprintln!("seed de fabrica falhou: {e}"),
        }
    }
    Ok(lib)
}

/// Semeia a fabrica a partir do `all.prst` EMBUTIDO no binario.
///
/// O arquivo e embutido em vez de procurado em disco: o caminho do dado muda a
/// cada tag, e um `include_bytes!` fixo e o que o empacotamento (`tauri.conf`)
/// transporta junto do executavel sem depender de instalador.
///
/// # Erros
/// Falha do parse do `.prst`; quem chama registra e segue (a biblioteca
/// funciona so com os presets do dono).
fn seed_de_fabrica(lib: &gp100_library::Library) -> Result<usize, String> {
    static ALL_PRST: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../files/patches/all.prst"
    ));
    let agora = library_commands::agora_iso();
    lib.seed_factory(ALL_PRST, &agora)
        .map_err(|e| e.to_string())
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

    // `build` ANTES de `manage` porque o caminho do banco vem do proprio Tauri
    // (`app_data_dir`), que so existe depois que o app existe. Com
    // `Builder::run` (o caminho curto) nao dava para abrir o banco antes de
    // registrar o estado que o contem.
    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::device_info,
            commands::device_board,
            commands::device_preset_library,
            commands::device_select_preset,
            commands::device_set_param,
            commands::device_boot,
            commands::list_user_irs,
            commands::pending_pushes,
            library_commands::library_search,
            library_commands::library_stats,
            library_commands::library_save,
            library_commands::library_delete,
            library_commands::library_get,
            library_commands::library_import,
            library_commands::library_export,
        ])
        .build(tauri::generate_context!())?;

    // Biblioteca (#26): um arquivo .sqlite no diretorio de DADOS do app, semeado
    // com os 99 presets de fabrica na primeira execucao. Falha aqui NAO derruba
    // o app: sem biblioteca o editor so nao mostra a lista persistida — cair a
    // janela porque o disco esta cheio seria trocar uma falha visivel por uma
    // tela branca.
    let library = match abrir_biblioteca(&app) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("biblioteca indisponivel, seguindo sem dela: {e}");
            gp100_library::Library::open_in_memory()?
        }
    };
    app.manage(commands::AppState {
        actor: actor.clone(),
        library: std::sync::Mutex::new(library),
    });

    // Laço de eventos do Tauri. `Builder::run` era o atalho que fazia isto e
    // devolvia `Result`; aqui quem inicializa é `build`, e `App::run` não tem
    // o que devolver — o `build` acima ja é o ponto que falha.
    app.run(|_app, _event| {});
    // Ciclo de vida: o actor roda até o app fechar — shutdown explícito
    // (o handle é Clone; Drop em clone derrubaria o actor alheio).
    actor.shutdown();
    Ok(())
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
