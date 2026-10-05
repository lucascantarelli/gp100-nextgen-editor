//! Commands do laboratório de IRs (issue #24) — casca fina sobre o
//! `gp100-library` e sobre o `DeviceActor`.
//!
//! **Dois mundos, duas portas.** A biblioteca é um arquivo local (vida da
//! #26): lista, importa, renomeia, atribui slot e apaga — passam pelo
//! `Mutex<Library>` e não tocam o device. O ENVIO do `.ir` ao aparelho passa
//! pelo [`crate::actor::DeviceActor`], como todo tráfego de fio (D8).
//!
//! **Nenhum DTO mora aqui.** Os tipos que cruzam a ponte IPC — `IrRow`, `Ir`,
//! `IrBoard` e o `IrUploadReport` do core — já são `serde` nos crates que os
//! definem, e cada um tem o contrato do fio testado no CI das três
//! plataformas. Um `IrDto` reescrito aqui passaria por revisão (o campo é o
//! mesmo, o nome é o mesmo) e falharia só em tempo de execução, no `invoke`,
//! quando o `savedAt` chegasse `undefined` para a tela.
//!
//! **O `.ir` chega como BYTES, não como caminho.** O webview do Tauri não tem
//! acesso ao disco (o projeto não usa plugin de `fs`) e o `invoke` trafega
//! JSON: o `<input type="file">` da UI lê o arquivo e manda os bytes, que o
//! serde transporta como array de números.
//!
//! **O tamanho é recusado aqui, e não só no fio.** §13.7 recusa o pedaço final
//! (strict): o device conta os chunks pelo payload, e um `.ir` com 7 bytes de
//! resto faz o contador divergir do resto do arquivo. A biblioteca já barra
//! na importação, e o upload barra de novo — porque o registro pode ter vindo
//! de um banco mais velho, e o botão desabilitado com o motivo escrito é
//! melhor do que um aparelho meio gravado.

use tauri::State;

use crate::commands::AppState;

/// A lista do laboratório + o estado dos slots (uma leitura só — ver
/// [`gp100_library::IrBoard`]).
///
/// # Erros
/// String de erro se o banco falhar (o front mostra no banner com retry).
#[tauri::command]
pub fn ir_board(state: State<'_, AppState>) -> Result<gp100_library::IrBoard, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.ir_board().map_err(|e| e.to_string())
}

/// Números do laboratório (rodapé).
///
/// # Erros
/// String de erro se o banco falhar.
#[tauri::command]
pub fn ir_stats(state: State<'_, AppState>) -> Result<gp100_library::IrBoard, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.ir_board().map_err(|e| e.to_string())
}

/// Importa um `.ir` escolhido pelo dono.
///
/// O `saved_at` é montado AQUI porque o shell é o que tem relógio — o crate de
/// biblioteca não tem, pela mesma razão do `library_export`.
///
/// # Erros
/// String de erro com o motivo da recusa (nome vazio, arquivo de 0 bytes,
/// tamanho que não é múltiplo de 15B). A recusa é sempre ANTES de gravar: a
/// importação é tudo-ou-nada em transação.
#[tauri::command]
pub fn ir_import(
    state: State<'_, AppState>,
    name: String,
    blob: Vec<u8>,
) -> Result<gp100_library::IrRow, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.import_ir(&name, &blob, &crate::library_commands::agora_iso())
        .map_err(|e| e.to_string())
}

/// Lê UM IR com o conteúdo — o que o envio envia.
/// `null` = não existe mais (o dono apagou em outra janela).
///
/// # Erros
/// String de erro se o banco falhar.
#[tauri::command]
pub fn ir_get(state: State<'_, AppState>, id: String) -> Result<Option<gp100_library::Ir>, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.ir_com_blob(&id).map_err(|e| e.to_string())
}

/// O IR de um slot, com o conteúdo. `null` = slot vazio.
///
/// Existe separado de `ir_get` porque o clique da tela é "o que está no
/// slot 0?", não "o IR `ia1b2c3d`" — e fazer o front guardar o id do slot seria
/// estado duplicado que o banco já resolve.
///
/// # Erros
/// String de erro se o banco falhar.
#[tauri::command]
pub fn ir_do_slot(
    state: State<'_, AppState>,
    slot: u8,
) -> Result<Option<gp100_library::Ir>, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.ir_do_slot(slot).map_err(|e| e.to_string())
}

/// Renomeia um IR. `false` = não existia.
///
/// # Erros
/// String de erro se o nome for vazio ou o banco falhar.
#[tauri::command]
pub fn ir_rename(state: State<'_, AppState>, id: String, name: String) -> Result<bool, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.renomear_ir(&id, &name).map_err(|e| e.to_string())
}

/// Atribui (ou desliga, com `slot = null`) o slot de um IR.
///
/// `slot = null` é o "tirar do aparelho" da UI: o registro continua na
/// biblioteca, mas volta a ser um IR sem destino.
///
/// # Erros
/// String de erro se o slot estiver fora de 0..=19, se outro IR já ocupa o
/// slot (a mensagem diz QUEM ocupa) ou se o IR não existir.
#[tauri::command]
pub fn ir_assign_slot(
    state: State<'_, AppState>,
    id: String,
    slot: Option<u8>,
) -> Result<(), String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.atribuir_slot_ir(&id, slot).map_err(|e| e.to_string())
}

/// Apaga o IR e o conteúdo. `false` = não existia.
///
/// Apagar da LIBRARIÓRIA não apaga o que já está gravado no aparelho: o
/// `erase` do slot é um comando de fio que o projeto ainda não tem fechado
/// (§13.12). A UI diz isso ao dono no aviso de confirmação — apagar do banco
/// enquanto o aparelho continua com o IR é o estado real, e a tela precisa
/// mostrá-lo em vez de deixar o dono acreditar que o slot ficou livre.
///
/// # Erros
/// String de erro se o banco falhar.
#[tauri::command]
pub fn ir_delete(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.apagar_ir(&id).map_err(|e| e.to_string())
}

/// Envia o IR ao aparelho (§13.7): `ir_begin` + chunks de 15B com ACK por
/// chunk, o último duplicado como marcador de fim.
///
/// **BLOQUEIA por dezenas de segundos** — um IR de 300 KB são ~20.000 chunks,
/// cada um esperando o próprio ACK. O command do Tauri roda fora da main
/// thread, então a janela continua desenhando; a UI mostra "enviando…" e
/// recarrega o quadro quando o `invoke` resolve.
///
/// **Lê o conteúdo do banco e envia, nesta ordem.** O id vem da tela, mas quem
/// decide quais bytes vão para o fio é o registro guardado — enviar o que a UI
/// tem em memória mandaria o IR errado quando a tela está velha.
///
/// # Erros
/// String de erro com o motivo: IR inexistente, sem slot atribuído, ou a falha
/// tipada do core (`Timeout` D6, `InvalidShape` D5, `UnexpectedAck`).
#[tauri::command]
pub fn ir_send(
    state: State<'_, AppState>,
    id: String,
) -> Result<gp100_core::session::IrUploadReport, String> {
    let (slot, blob) = {
        let lib = state.library.lock().map_err(|e| e.to_string())?;
        let ir = lib
            .ir_com_blob(&id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("IR {id} nao existe"))?;
        let slot = ir
            .row
            .slot
            .ok_or_else(|| "o IR nao tem slot atribuido".to_string())?;
        (slot, ir.blob)
        // O banco é liberado AQUI: o envio pode levar dezenas de segundos, e
        // segurar o `Mutex` esse tempo todo travaria toda a biblioteca da UI
        // (busca, importação, o rodapé) só porque um `.ir` está indo para o
        // aparelho.
    };
    state.actor.upload_ir(slot, &blob)
}
