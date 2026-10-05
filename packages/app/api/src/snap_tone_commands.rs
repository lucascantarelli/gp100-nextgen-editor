//! Commands do gestor de SnapTone/NAM (issue #25) — casca fina sobre o
//! `gp100-library` e sobre o `DeviceActor`.
//!
//! **Dois mundos, duas portas.** A biblioteca é um arquivo local (vida da
//! #26): lista, importa, renomeia, atribui slot e apaga — passam pelo
//! `Mutex<Library>` e não tocam o device. O ENVIO do modelo ao aparelho passa
//! pelo [`crate::actor::DeviceActor`], como todo tráfego de fio (D8).
//!
//! **Nenhum DTO mora aqui.** Os tipos que cruzam a ponte IPC — `ToneRow`,
//! `Tone`, `ToneBoard` e o relatório de envio do core — já são `serde` nos
//! crates que os definem, e cada um tem o contrato do fio testado no CI das
//! três plataformas. Um `ToneDto` reescrito aqui passaria por revisão (o
//! campo é o mesmo, o nome é o mesmo) e falharia só em tempo de execução, no
//! `invoke`, quando o `savedAt` chegasse `undefined` para a tela.
//!
//! **O `.clo` chega como BYTES, não como caminho.** O webview do Tauri não tem
//! acesso ao disco (o projeto não usa plugin de `fs`) e o `invoke` trafega
//! JSON: o `<input type="file">` da UI lê o arquivo e manda os bytes, que o
//! serde transporta como array de números. Passar caminho exigiria um plugin
//! novo e uma permissão para ler o disco inteiro do usuário — mais poder do
//! que esta tela precisa.
//!
//! **A conversão do `.nam` não acontece aqui.** As strings do Valeton Suite
//! mostram o caminho (`Choose a nam file to open it` → `*.nam` → conversão →
//! `/name.clo`) e o motor NAM mora no exe da Valeton (`BLOCKERS.md`). O app
//! importa o `.clo` JÁ CONVERTIDO; se alguém tentar importar um `.nam`, os
//! bytes entram e o aparelho recusa — a UI não deve fingir que converte o que
//! não converte.

use tauri::State;

use crate::commands::AppState;

/// A lista do gestor + o estado dos slots (uma leitura só — ver
/// [`gp100_library::ToneBoard`]).
///
/// # Erros
/// String de erro se o banco falhar (o front mostra no banner com retry).
#[tauri::command]
pub fn tone_board(state: State<'_, AppState>) -> Result<gp100_library::ToneBoard, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.tone_board().map_err(|e| e.to_string())
}

/// Números do gestor (rodapé).
///
/// # Erros
/// String de erro se o banco falhar.
#[tauri::command]
pub fn tone_stats(state: State<'_, AppState>) -> Result<gp100_library::ToneBoard, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.tone_board().map_err(|e| e.to_string())
}

/// Importa um `.clo` (bytes já convertidos pelo Suite) e, se o dono tiver,
/// o `nam_output_wav.wav` que o Suite renderiza ao lado dele.
///
/// `preview` nulo é o caso normal: o dono tem o modelo e não pediu o áudio. É
/// por isso que ele é um argumento à parte e não um campo obrigatório — e por
/// isso que o botão de tocar no A/B fica desabilitado COM o motivo, em vez de
/// sumir.
///
/// O `saved_at` é montado AQUI porque o shell é o que tem relógio — o crate de
/// biblioteca não tem, pela mesma razão do `library_export`.
///
/// # Erros
/// String de erro com o motivo da recusa (nome vazio, modelo de 0 bytes,
/// banco cheio). A recusa é sempre ANTES de gravar: a importação é
/// tudo-ou-nada em transação.
#[tauri::command]
pub fn tone_import(
    state: State<'_, AppState>,
    name: String,
    model: Vec<u8>,
    preview: Option<Vec<u8>>,
) -> Result<gp100_library::ToneRow, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    let linha = lib
        .import_tone(
            &name,
            &model,
            preview.as_deref(),
            &crate::library_commands::agora_iso(),
        )
        .map_err(|e| e.to_string())?;
    Ok(linha)
}

/// Grava (ou apaga) só o áudio de preview de um tom — o dono tem o `.clo` e
/// exporta o WAV do Suite depois.
///
/// Comando separado do import porque os dois arquivos têm vida própria: o
/// `.clo` é o que vai para o aparelho, o WAV é só o A/B. Reescrever o modelo
/// para trocar o áudio seria fazer o aparelho receber 2,7 KB por causa de uma
/// gravação que não mudou.
///
/// # Erros
/// String de erro se o tom não existir ou o banco falhar.
#[tauri::command]
pub fn tone_set_preview(
    state: State<'_, AppState>,
    id: String,
    preview: Option<Vec<u8>>,
) -> Result<(), String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.grava_preview(&id, preview.as_deref())
        .map_err(|e| e.to_string())
}

/// Lê UM tom com o modelo — o que a tela de A/B toca e o envio envia.
/// `null` = não existe mais (o dono apagou em outra janela).
///
/// # Erros
/// String de erro se o banco falhar.
#[tauri::command]
pub fn tone_get(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<gp100_library::Tone>, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.ton_com_modelo(&id).map_err(|e| e.to_string())
}

/// O tom de um slot, com o modelo. `null` = slot vazio.
///
/// Existe separado de `tone_get` porque o clique da tela de A/B é "o que está
/// no slot 3?", não "o tom `t1a2b3c4d`" — e fazer o front guardar o id do slot
/// seria estado duplicado que o banco já resolve.
///
/// # Erros
/// String de erro se o banco falhar.
#[tauri::command]
pub fn tone_do_slot(
    state: State<'_, AppState>,
    slot: u8,
) -> Result<Option<gp100_library::Tone>, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.ton_do_slot(slot).map_err(|e| e.to_string())
}

/// Renomeia um tom. `false` = não existia.
///
/// # Erros
/// String de erro se o nome for vazio ou o banco falhar.
#[tauri::command]
pub fn tone_rename(state: State<'_, AppState>, id: String, name: String) -> Result<bool, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.renomear_tone(&id, &name).map_err(|e| e.to_string())
}

/// Atribui (ou desliga, com `slot = null`) o slot de um tom.
///
/// `slot = null` é o "tirar do aparelho" da UI: o registro continua na
/// biblioteca, mas volta a ser um tom sem destino.
///
/// # Erros
/// String de erro se o slot estiver fora de 1..=5, se outro tom já ocupa o
/// slot (a mensagem diz QUEM ocupa) ou se o tom não existir.
#[tauri::command]
pub fn tone_assign_slot(
    state: State<'_, AppState>,
    id: String,
    slot: Option<u8>,
) -> Result<(), String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.atribuir_slot(&id, slot).map_err(|e| e.to_string())
}

/// Apaga o tom e o modelo. `false` = não existia.
///
/// # Erros
/// String de erro se o banco falhar.
#[tauri::command]
pub fn tone_delete(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.apagar_tone(&id).map_err(|e| e.to_string())
}

/// Envia o tom ao aparelho (§5): stream de blocos com ACK de 16B e o settle
/// de 250 ms entre operações.
///
/// **BLOQUEIA.** Um modelo de ~2,7 KB são 143 blocos e o piso do §4 é 250 ms
/// entre eles — o comando leva dezenas de segundos. O command do Tauri roda
/// fora da main thread, então a janela continua desenhando e respondendo; a UI
/// só precisa mostrar "enviando…" e recarregar o quadro quando o `invoke`
/// resolve. Não há barra de progresso por bloco porque a FSM expõe progresso
/// só no boot (`BootProgress`) e acrescentar um callback por bloco aqui é uma
/// extensão do core que ainda não tem quem use.
///
/// **Lê o modelo do banco e envia, nesta ordem.** O id vem da tela, mas quem
/// decide quais bytes vão para o fio é o registro guardado — enviar o que a UI
/// tem em memória mandaria o tom errado quando a tela está velha.
///
/// # Erros
/// String de erro com o motivo: tom inexistente, sem slot atribuído, ou a
/// falha tipada do core (Timeout D6, InvalidShape D5).
#[tauri::command]
pub fn tone_send(
    state: State<'_, AppState>,
    id: String,
) -> Result<gp100_core::session::SnapToneUploadReport, String> {
    let (slot, model) = {
        let lib = state.library.lock().map_err(|e| e.to_string())?;
        let tom = lib
            .ton_com_modelo(&id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("tom {id} nao existe"))?;
        let slot = tom
            .row
            .slot
            .ok_or_else(|| "o tom nao tem slot atribuido".to_string())?;
        (slot, tom.model)
        // O banco é liberado AQUI: o envio pode levar dezenas de segundos, e
        // segurar o `Mutex` esse tempo todo travaria toda a biblioteca da UI
        // (busca, importação, o rodapé) só porque um `.clo` está indo para o
        // aparelho.
    };
    state.actor.upload_snap_tone(slot, &model)
}
