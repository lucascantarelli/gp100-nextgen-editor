//! `preset_gain_report` — o assistente de gain staging (#115) como command.
//!
//! **Por que arquivo próprio, e não dentro do `export_commands.rs`.** Pelo
//! mesmo motivo que separou `library_commands.rs` de `commands.rs`: o
//! `generate_handler!` importa o macro oculto `__cmd__<nome>`, então command e
//! handler não podem morar no módulo do `lib.rs` — e o assistente não é uma
//! exportação. Ele devolve um RELATÓRIO do preset, enquanto os `preset_export_*`
//! devolvem arquivos; juntá-los faria o leitor procurar a folha de timbre no
//! meio do gain staging.
//!
//! **Nenhuma regra de análise mora aqui.** Quem lê o board e estima é o
//! `gp100_core::gain` (função PURA sobre o `BoardView`), testado nos 3 SOs da
//! matriz. Este módulo faz três chamadas de uma linha: o `all.prst` embutido, o
//! dicionário embutido e a projeção do preset pedido.
//!
//! **Nada aqui fala com o device.** É a mesma coorte dos `preset_export_*`: uma
//! projeção pura sobre o arquivo de fábrica. Não há fila, ciclo de vida, nem
//! byte — e o teste que prova isso (`a_analise_nao_muda_o_board_e_nao_manda_um_byte`)
//! vive no core, onde a análise está.

use gp100_core::gain::{relatorio, Relatorio};
use gp100_core::model::{Dictionary, DICTIONARY_JSON};
use gp100_core::pedalboard::{board_view_for, embedded_document};

/// `preset_gain_report` — o relatório de gain staging do preset `pp`.
///
/// `pp` = `null` → o primeiro preset do arquivo (mesma regra do `device_board`).
/// O relatório traz, por módulo da cadeia, as leituras de ganho/nível com a
/// origem de cada número (nome do controle, valor cru e a faixa de onde a
/// posição saiu), o risco declarado da cadeia e a ordem de ajuste sugerida.
/// O método e a LIMITAÇÃO viajam no próprio relatório — a tela mostra os dois.
///
/// # Erros
/// String de erro se o `pp` não existir no arquivo de fábrica.
#[tauri::command]
pub fn preset_gain_report(pp: Option<u16>) -> Result<Relatorio, String> {
    let doc = embedded_document().map_err(|e| e.to_string())?;
    let dict = Dictionary::from_json(DICTIONARY_JSON).map_err(|e| e.to_string())?;
    let board = board_view_for(&doc, &dict, pp).map_err(|e| e.to_string())?;
    Ok(relatorio(&board))
}
