//! gp100-ui — backend desktop (Tauri 2) do GP-100 NextGen Editor.
//!
//! **Papel:** ponte IPC entre o front (`ui/`) e o
//! [gp100-core]. O crate conhece commands, actor e DTOs; **toda regra de
//! protocolo fica no core** (R1).
//!
//! **Backend por compilação (não por env).** O build comum fala com o
//! `MockDevice` (política ADR-4/ADR-5 — nenhum byte sai para hardware sem o
//! build de campo dizer que pode); o build `--features real-device` abre o
//! `RealDevice` e cai no mock se o aparelho não estiver ligado. Ver
//! [`abrir_backend`] e `docs/REAL_DEVICE_GAP.md`.
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
mod export_commands;
mod gain_commands;
mod ir_commands;
mod library_commands;
mod snap_tone_commands;

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

/// **O log de fio automático vale só no aparelho.**
///
/// No mock não existe fio: o arquivo encheria com o tráfego de um device que
/// não é o do operador e enterraria o incidente real (assert do firmware) em
/// ruído. Sem aparelho aberto, o backend que o app carrega é o mock — e a
/// política de hardware do ADR-4/ADR-5 já diz que ali não há o que registrar.
fn log_automatico(backend: actor::Backend) -> bool {
    matches!(backend, actor::Backend::Real)
}

/// Nome do arquivo do log de fio de UMA execução do app.
///
/// **Por que carimbo, e não nome fixo.** O `WireLogger` **trunca** o arquivo ao
/// abrir. Com nome fixo, o primeiro boot depois de um incidente apagaria
/// exatamente o log que interessa — e a recuperação de um aparelho assertado é
/// um power-cycle, ou seja, o app É reiniciado no meio da investigação. Um
/// arquivo por execução preserva a sessão que quebrou.
///
/// O carimbo sai do ISO só com dígitos (`2026-10-06T16:04:05Z` →
/// `wire-20261006160405.jsonl`): `:` é inválido em nome de arquivo no Windows,
/// e a ordem lexicográfica continua sendo a cronológica.
fn nome_wire_log(agora_iso: &str) -> String {
    let stamp: String = agora_iso.chars().filter(char::is_ascii_digit).collect();
    format!("wire-{stamp}.jsonl")
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

/// **O backend que este binário vai falar.** Escolha por COMPILAÇÃO, nao por
/// runtime: o build comum e o `MockDevice` (política ADR-4/ADR-5 — nenhum
/// byte sai para hardware sem o build de campo dizer que pode), e o build
/// `--features real-device` tenta o aparelho primeiro e cai no mock se ele
/// nao estiver ligado.
///
/// **POR QUE O `GP100_BACKEND` NAO ESCOLHE.** Um env que troca mock por
/// aparelho faria o build distribuível trocar de comportamento conforme a
/// maquina — e um `.exe` que hoje responde 99 presets passaria a responder o
/// que estiver na USB, sem ninguem pedir. O mock nao e um modo de depuracao
/// e um **backend**: ele e o que garante que abrir o app nunca escreve no
/// hardware de surpresa. O CLI ja faz a mesma escolha (`--real` exige
/// `--i-know-what-im-doing`; ver `packages/cli/src/main.rs`).
///
/// O `GP100_DEBUG_FAULT` continua mock-only (e o `if` abaixo deixa isso
/// explicito em vez de silencioso).
///
/// # Erros
/// Falha de I/O na abertura do backend; o binário encerra em vez de abrir uma
/// janela que depois mente sobre o device.
fn abrir_backend() -> Result<abrir_backend::Escolha, Box<dyn std::error::Error>> {
    #[cfg(feature = "real-device")]
    {
        match gp100_core::transport::real::RealDevice::new() {
            Ok(real) => {
                eprintln!("[device] RealDevice aberto — build de campo");
                return Ok(abrir_backend::Escolha {
                    actor: actor::DeviceActor::spawn(
                        Box::new(real) as actor::AppDevice,
                        actor::Backend::Real,
                    ),
                    backend: actor::Backend::Real,
                });
            }
            Err(e) => {
                eprintln!("[device] aparelho nao abriu ({e}); caindo no MOCK");
            }
        }
    }
    #[cfg(not(feature = "real-device"))]
    {
        let _ = debug_fault_from_env();
    }

    // O mock é montado TIPADO (é ele que arma a falha de debug, e `with_fault`
    // é método do `MockDevice` concreto) e só DEPOIS convertido no transporte
    // que este build carrega.
    //
    // A conversão é `actor::como_app_device`, e não uma anotação de tipo: anotar
    // `AppDevice` aqui compila no build comum — onde o alias É o `MockDevice` —
    // e quebra o build de campo, onde ele é `Box<dyn DeviceBackend + Send>` e o
    // mock precisa ir para o heap. Uma anotação não faz essa coerção; foi o
    // E0308 que a primeira run com `ui-rust` de verdade mostrou nos dois SOs.
    let mut mock = MockDevice::new()?;
    if let Some(fault) = debug_fault_from_env() {
        eprintln!("GP100_DEBUG_FAULT armado: {fault:?} (backend mock)");
        mock = mock.with_fault(fault);
    }
    let mock = actor::como_app_device(mock);
    Ok(abrir_backend::Escolha {
        actor: actor::DeviceActor::spawn(mock, actor::Backend::Mock),
        backend: actor::Backend::Mock,
    })
}

/// Resultado de [`abrir_backend`] — isolado num modulo para os dois `cfg`
/// manterem o MESMO tipo de retorno (o `cfg` some com o ramo, nao com a
/// assinatura).
mod abrir_backend {
    use crate::actor::DeviceActor;

    /// O actor ja aberto, com o backend que ele possui.
    pub(super) struct Escolha {
        /// Thread dona unica da `Session` (D8 do ADR-6).
        pub actor: DeviceActor,
        /// Qual backend o actor possui — **declarado por quem o montou**, e não
        /// deduzido depois. É o que decide se o log de fio automático vale: só
        /// o aparelho tem fio para registrar.
        pub backend: crate::actor::Backend,
    }
}

/// Boot do app Tauri: registra estado + commands (invocado pelo `main`).
///
/// # Erros
/// Propaga falha de setup/runtime do Tauri (janela/recursos/assets) — o
/// binário encerra com exit ≠ 0.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let escolha = abrir_backend()?;
    let backend = escolha.backend;
    let actor = escolha.actor;

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
            commands::device_save_preset,
            commands::device_dump_preset,
            commands::device_log_session,
            commands::device_log_stop,
            commands::device_preview,
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
            library_commands::library_versions,
            library_commands::library_version,
            library_commands::library_diff,
            library_commands::library_restore_version,
            library_commands::library_restore_knob,
            snap_tone_commands::tone_board,
            snap_tone_commands::tone_stats,
            snap_tone_commands::tone_import,
            snap_tone_commands::tone_set_preview,
            snap_tone_commands::tone_get,
            snap_tone_commands::tone_do_slot,
            snap_tone_commands::tone_rename,
            snap_tone_commands::tone_assign_slot,
            snap_tone_commands::tone_delete,
            snap_tone_commands::tone_send,
            ir_commands::ir_board,
            ir_commands::ir_stats,
            ir_commands::ir_import,
            ir_commands::ir_get,
            ir_commands::ir_do_slot,
            ir_commands::ir_rename,
            ir_commands::ir_assign_slot,
            ir_commands::ir_delete,
            ir_commands::ir_send,
            export_commands::preset_export_json,
            export_commands::preset_export_tone_sheet,
            export_commands::preset_import_json,
            gain_commands::preset_gain_report,
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

    // **LOG DE FIO AUTOMÁTICO NO CAMPO (06/10/2026).** Em 06/10 o app de campo
    // assertou o firmware do GP-100 (`CODE:PresetNum < TOTAL_PA`,
    // `Drivers/audio/audio.c:912`) e **não havia um único frame gravado**: o log
    // só existia se o operador abrisse o painel de diagnóstico e clicasse o
    // toggle — e ninguém clica num toggle durante um incidente. A causa acabou
    // sendo atribuída por LEITURA DE CÓDIGO (dedução), não por registro do fio.
    //
    // Daqui em diante o build de campo grava sozinho, e este ponto é ANTES do
    // primeiro command do front: o `select` da abertura automática
    // (`useStage` → `openPreset(0)`) também entra no arquivo. Ligar o log aqui
    // não reinicia o device (o `LoggingTransport` é decorador: `actor.rs`).
    //
    // Falha de log NÃO derruba a sessão — a policy é a mesma do
    // `LoggingTransport`: a sessão vale mais que o log. O operador é avisado no
    // `stderr` (e o caminho é impresso quando liga).
    if log_automatico(backend) {
        match app
            .path()
            .app_data_dir()
            .map(|dir| dir.join(nome_wire_log(&library_commands::agora_iso())))
        {
            Ok(caminho) => match actor.log_session(caminho.to_string_lossy().as_ref()) {
                Ok(_) => eprintln!("[log] wire log de campo: {}", caminho.display()),
                Err(e) => eprintln!("[log] o log de fio nao ligou ({e}); sessao sem log"),
            },
            Err(e) => eprintln!("[log] sem diretorio de dados para o log de fio: {e}"),
        }
    }

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

    /// O carimbo do log tira o que o Windows recusa em nome de arquivo e
    /// mantém a ordem cronológica na lexicográfica (precisa para achar "a
    /// sessão que quebrou" numa pasta com várias).
    #[test]
    fn nome_do_log_de_fio_e_ordenavel_e_sem_dois_pontos() {
        let n = nome_wire_log("2026-10-06T16:04:05Z");
        assert_eq!(n, "wire-20261006160405.jsonl");
        assert!(
            !n.contains(':'),
            "dois-pontos e invalido em nome no Windows"
        );
    }

    /// **O log automático é do aparelho, e só dele.** No mock não existe fio
    /// para registrar; um arquivo ali seria ruído por cima do incidente real.
    #[test]
    fn log_automatico_so_no_backend_real() {
        assert!(log_automatico(actor::Backend::Real));
        assert!(!log_automatico(actor::Backend::Mock));
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
