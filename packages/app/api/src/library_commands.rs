//! Commands de biblioteca (issue #26) — casca fina sobre o `gp100-library`.
//!
//! **Por que este arquivo existe separado do `commands.rs`.** O `generate_handler!`
//! importa o macro oculto `__cmd__<nome>` (ver o cabeçalho de `commands.rs`):
//! comando e handler não podem morar no mesmo módulo do `lib.rs`. E há uma
//! razão melhor — os commands de **device** e os de **biblioteca** têm cycle of
//! vida diferente. O device vem do `DeviceActor` (fila serializada, dono
//! único); a biblioteca é um arquivo local que não tem device nenhum. Misturar
//! os dois num arquivo só economiza dez linhas e embaralha a política.
//!
//! **Nenhuma regra de armazenamento mora aqui.** Buscar, gravar, apagar,
//! importar e exportar são chamadas de uma linha ao crate. O que é decisão
//! (versão do esquema, modo do import, seed) vive no `gp100-library` e é
//! testado lá, nas três plataformas da matriz (ADR-9).

use gp100_library::{ImportMode, SearchQuery};
use serde::Serialize;
use tauri::State;

use crate::commands::AppState;

/// Estado da biblioteca pronto para o front (o que a UI mostra no rodapé da
/// biblioteca: "99 de fábrica, 3 meus, esquema v2").
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStats {
    /// Total de registros.
    pub total: i64,
    /// Registros de fábrica.
    pub factory: i64,
    /// Registros de usuário.
    pub user: i64,
    /// `PRAGMA user_version` do arquivo.
    pub schema: u32,
    /// Versão do SQLite embutido.
    pub sqlite: String,
}

/// Executa a busca da biblioteca.
///
/// # Erros
/// String de erro se o banco não abrir/migrar (o front mostra o banner com retry).
#[tauri::command]
pub fn library_search(
    state: State<'_, AppState>,
    text: Option<String>,
    pp: Option<u16>,
    pp_type: Option<u16>,
) -> Result<Vec<gp100_library::PresetRow>, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    let q = SearchQuery {
        text,
        pp,
        pp_type,
        bank: None,
        limit: Some(SearchQuery::LIMITE_PADRAO),
    };
    lib.search(&q).map_err(|e| e.to_string())
}

/// Números da biblioteca para o rodapé.
///
/// # Erros
/// String de erro se o banco falhar ao ser consultado.
#[tauri::command]
pub fn library_stats(state: State<'_, AppState>) -> Result<LibraryStats, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    let total = lib.count().map_err(|e| e.to_string())?;
    let factory = lib
        .factory_presets()
        .map(|v| v.len())
        .map_err(|e| e.to_string())?;
    Ok(LibraryStats {
        total,
        factory: factory as i64,
        user: total - factory as i64,
        schema: lib.schema_version().map_err(|e| e.to_string())?,
        sqlite: gp100_library::Library::sqlite_version().to_string(),
    })
}

/// Grava (ou regrava) um patch de usuário.
///
/// # Erros
/// String de erro se o registro violar o esquema do banco.
#[tauri::command]
pub fn library_save(
    state: State<'_, AppState>,
    preset: gp100_library::Preset,
) -> Result<(), String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.upsert(&preset).map_err(|e| e.to_string())
}

/// Apaga pelo id.
///
/// # Erros
/// String de erro se o banco falhar.
#[tauri::command]
pub fn library_delete(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.delete(&id).map_err(|e| e.to_string())
}

/// Importa um arquivo de biblioteca.
///
/// # Erros
/// String de erro com o motivo da recusa (formato desconhecido, versão futura,
/// JSON truncado). A recusa é sempre antes de gravar.
#[tauri::command]
pub fn library_import(
    state: State<'_, AppState>,
    json: String,
    replace: bool,
) -> Result<gp100_library::ImportReport, String> {
    let mut lib = state.library.lock().map_err(|e| e.to_string())?;
    let modo = if replace {
        ImportMode::Replace
    } else {
        ImportMode::Insert
    };
    lib.import_json(&json, modo).map_err(|e| e.to_string())
}

/// Exporta a biblioteca para JSON versionado.
///
/// # Erros
/// String de erro se a serialização falhar.
#[tauri::command]
pub fn library_export(state: State<'_, AppState>) -> Result<String, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    // O instante vem do shell, não do crate: o crate não tem relógio, e um
    // export determinístico (mesmo banco → mesmo JSON) é o que permite
    // comparar dois exports.
    let agora = agora_iso();
    lib.export_json(&agora).map_err(|e| e.to_string())
}

/// ISO-8601 (UTC, segundos) do agora, sem dependência de `chrono`.
///
/// O envelope de export carrega o instante como metadado, e um metadado com
/// formato errado é pior do que ausente: alguém vai tentar abrir o arquivo em
/// outra ferramenta e ler "unix-1780000000". A conversão dias→data civil é a de
/// Hinnant (era civilGregorian); 15 linhas evitam uma dependência a mais no
/// crate que já tem uma dependência nativa (`rusqlite`).
pub(crate) fn agora_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dias = (secs / 86_400) as i64;
    let resto = secs % 86_400;
    let (h, mi, s) = (resto / 3600, (resto % 3600) / 60, resto % 60);

    // dias a partir de 1970-01-01 -> ano/mês/dia (algoritmo de Hinnant).
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let ano = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dia = doy - (153 * mp + 2) / 5 + 1;
    let mes = if mp < 10 { mp + 3 } else { mp - 9 };
    let ano = if mes <= 2 { ano + 1 } else { ano };

    format!("{ano:04}-{mes:02}-{dia:02}T{h:02}:{mi:02}:{s:02}Z")
}
