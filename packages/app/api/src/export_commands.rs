//! Commands de EXPORTAÇÃO do preset (issue #114) — casca fina sobre o
//! `gp100-core`.
//!
//! **Por que arquivo próprio.** Três razões, na ordem em que pesam:
//!
//! 1. o `generate_handler!` importa o macro oculto `__cmd__<nome>`, então
//!    comando e handler não podem morar no mesmo módulo do `lib.rs` (a mesma
//!    regra que separou `library_commands.rs` de `commands.rs`);
//! 2. estes commands **não falam com o device** — são projeções puras sobre o
//!    `all.prst` embutido, como o `Request::Board` do actor já era. Não têm
//!    fila, não têm ciclo de vida, não podem mandar byte para o aparelho;
//!    pô-los junto dos `device_*` sugeriria que compartilham essa política;
//! 3. **nenhuma regra de formato mora aqui.** Escolher o preset, recortá-lo,
//!    virar JSON e virar PDF são chamadas de uma linha ao core. O que é
//!    decisão (o schema versionado, o recorte, o layout do PDF) vive no
//!    `gp100-core` e é testado lá, nos 3 SOs da matriz.
//!
//! **O que o app exporta, e por que é o `all.prst`.** O preset de fábrica não
//! tem cadeia guardada no banco (o seed grava `payload: None`: a fonte é o
//! próprio arquivo), então a exportação parte do `all.prst` embutido e recorta
//! o preset pedido. É o mesmo caminho que o `device_board` já usa para
//! desenhar o palco — um caminho só, e a exportação herda a verdade do palco.

use gp100_core::model::{Dictionary, DICTIONARY_JSON};
use gp100_core::pedalboard::{board_view_for, embedded_document, BoardView};
use gp100_core::preset_json;
use gp100_core::tone_sheet;

/// Carrega o documento embutido + o dicionário.
///
/// Os dois vêm de `include_str!`: não há I/O, não há estado e não há falha
/// esperada — os `Result` existem porque a carga valida o insumo (R1) e o
/// app prefere um banner a um panic se o embutido vier corrompido.
fn ferramentas() -> Result<(gp100_core::preset::Document, Dictionary), String> {
    let doc = embedded_document().map_err(|e| e.to_string())?;
    let dict = Dictionary::from_json(DICTIONARY_JSON).map_err(|e| e.to_string())?;
    Ok((doc, dict))
}

/// `preset_export_json` — o preset de fábrica `pp` como `.prst` em JSON
/// versionado (`gp100.preset`), pronto para o dono guardar ou comparar.
///
/// `pp` = `null` → o primeiro preset do arquivo (mesma regra do `device_board`).
///
/// O JSON sai **completo**, layout incluído: é o que faz o arquivo voltar a ser
/// um `.prst` byte a byte em [`preset_import_json`]. O `version` do envelope é o
/// que autoriza um build futuro a recusar o que não sabe ler.
///
/// # Erros
/// String de erro se o `pp` não existir no arquivo de fábrica.
#[tauri::command]
pub fn preset_export_json(pp: Option<u16>) -> Result<String, String> {
    let (doc, _) = ferramentas()?;
    let recorte = doc.apenas_preset(pp).map_err(|e| e.to_string())?;
    preset_json::to_json(&recorte).map_err(|e| e.to_string())
}

/// `preset_export_tone_sheet` — a folha de timbre do preset `pp` em PDF
/// (cadeia de 9 slots + tabela de knobs), para imprimir e levar ao palco.
///
/// Devolve os BYTES do PDF (a UI monta o download). O arquivo é pequeno o
/// bastante para atravessar como array de números: ~5,6 KB para um preset de
/// fábrica, contra os ~500 KB de uma folha do arquivo inteiro.
///
/// # Erros
/// String de erro se o `pp` não existir no arquivo de fábrica.
#[tauri::command]
pub fn preset_export_tone_sheet(pp: Option<u16>) -> Result<Vec<u8>, String> {
    let (doc, dict) = ferramentas()?;
    let recorte = doc.apenas_preset(pp).map_err(|e| e.to_string())?;
    tone_sheet::tone_sheet_pdf(&recorte, &dict).map_err(|e| e.to_string())
}

/// `preset_import_json` — a cadeia de um `.prst` em JSON, pronta para o palco.
///
/// Devolve um `BoardView`, **o mesmo tipo que o `device_board` devolve**: a UI
/// aplica o importado pelo caminho que já existe, sem um segundo formato de
/// cadeia e sem uma segunda renderização. Se o JSON trouxer mais de um preset,
/// vale o primeiro — o que o app exporta é sempre um só.
///
/// **Nada aqui vai para o aparelho.** Importar é ler um arquivo do dono; gravar
/// no GP-100 continua sendo um ato separado, com a trava de escrita do build de
/// campo (ADR-4/ADR-5).
///
/// # Erros
/// String de erro com o motivo do core quando o JSON não é um envelope
/// `gp100.preset` desta versão — a mensagem diz qual versão veio, para o dono
/// entender que o arquivo é de um app mais novo em vez de achar que corrompeu.
#[tauri::command]
pub fn preset_import_json(json: String) -> Result<BoardView, String> {
    let (_, dict) = ferramentas()?;
    let doc = preset_json::from_json(&json).map_err(|e| e.to_string())?;
    board_view_for(&doc, &dict, None).map_err(|e| e.to_string())
}
