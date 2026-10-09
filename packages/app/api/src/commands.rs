//! Commands de device da UI (DeviceActor — fila serializada do actor).
//!
//! **Por que submódulo:** `generate_handler!` importa o macro oculto
//! `__cmd__<name>` — comando e handler no MESMO módulo colidem (E0255).
//! Estrutura canônica do Tauri: commands aqui, handler em `lib.rs` por
//! path.
//!
//! **D8:** nenhum command toca a `Session` direto — todos passam pela fila
//! do [`crate::actor::DeviceActor`]. O `device_boot` é LONGO (2299
//! transações contra o mock) e síncrono (ADR-3): o Tauri o executa fora da
//! main thread; o progresso sai por EVENTO (`device://progress`) — a UI
//! mostra barra, nunca trava.

use gp100_core::session::{BootProgress, BootStage};
use serde::Serialize;
use std::sync::mpsc;
use tauri::{Emitter, State};

use crate::actor::{DeviceActor, DeviceSnapshot};
/// Estado da aplicação: o actor é o ÚNICO dono do device (D8 — consumidor
/// único do stream IN; nada de `Mutex<Session>` compartilhado com a UI).
pub struct AppState {
    /// Actor do device (M1: mock — política ADR-4/ADR-5).
    pub actor: DeviceActor,
    /// Biblioteca persistente (#26). É um arquivo local, não tem device: por
    /// isso não passa pelo actor — o actor serializa tráfego de fio, e a
    /// biblioteca é leitura/escrita de disco. O `Mutex` aqui é só oBorrow que o
    /// Tauri exige para um estado compartilhado entre commands; a política do
    /// armazenamento (versão, seed, busca) está no `gp100-library`.
    pub library: std::sync::Mutex<gp100_library::Library>,
}

/// DTO de `device_info` — MESMOS campos/semântica de `ui/src/ipc/types.ts`
/// (`DeviceInfo`). CamelCase no fio (serde) para o TS não precisar de mapping.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    /// Backend ativo: "mock" no build de desenvolvimento, "real" no build
    /// de campo, "none" no app sem aparelho (issue #150). **Nunca mais um
    /// literal** — ver `docs/REAL_DEVICE_GAP.md` §1.
    pub backend: &'static str,
    /// Motivo humano quando `backend` é `"none"` (issue #150): "aparelho não
    /// conectado via USB…" ou "build sem o transporte…". Vazio quando a
    /// sessão está viva — preencher seria inventar aviso.
    pub detail: String,
    /// Nº de presets. No mock, o do `all.prst`; no real, o inventário que
    /// o boot percorreu (que hoje é o default `0..198`, não uma contagem
    /// descoberta no aparelho — `REAL_DEVICE_GAP.md` §4.3).
    pub preset_count: usize,
    /// pp corrente (u16 no fio; aqui como número p/ o TS). Lido da Session
    /// nos dois backends.
    pub current_pp: u16,
    /// Nome do pp corrente. **Vazio com o aparelho real**: vem da página
    /// meta6 (`13010001`), cujo layout ainda não foi decifrado. Vazio é
    /// "a app não sabe"; um nome inventado seria pior.
    pub current_name: String,
    /// ppType do pp corrente (semântica no dicionário do core). **Só no mock.**
    pub current_pp_type: u16,
    /// Slots de IR com CRC de fábrica. **Só no mock** — o fio não expõe CRC.
    pub ir_slots_with_crc: usize,
    /// Tabela dos 20 User IRs lida do device (§13.12) — existe nos dois
    /// backends, e e a unica fonte de verdade de "o que tem no aparelho".
    pub ir_slots: Vec<IrSlotDto>,
    /// Binário compilado com `write-verified` (ADR-5)? O front desabilita os
    /// botões de escrita com este sinal, em vez de o operador descobrir a
    /// recusa so depois de clicar.
    pub write_verified: bool,
}

impl DeviceInfo {
    /// Deriva o DTO do snapshot do actor (R1 também nos DTOs: a fonte e o
    /// que o backend sabe, nunca um literal no command).
    pub fn from_snapshot(s: &DeviceSnapshot) -> Self {
        Self {
            backend: s.backend,
            detail: s.detail.clone(),
            preset_count: s.preset_count,
            current_pp: s.current_pp,
            current_name: s.current_name.clone(),
            current_pp_type: s.current_pp_type,
            ir_slots_with_crc: s.ir_slots_with_crc,
            ir_slots: s
                .ir_slots
                .iter()
                .map(|(slot, name)| IrSlotDto {
                    slot: *slot,
                    name: name.clone(),
                })
                .collect(),
            write_verified: s.write_verified,
        }
    }
}

/// DTO do beat de progresso do boot (evento `device://progress`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootProgressDto {
    /// Etapa do script (§13.10): "tables" | "scan" | "probe" | "setlist" |
    /// "names" | "keepalive" — literal estável para o front.
    pub stage: &'static str,
    /// Transações completas até agora.
    pub done: usize,
    /// Total esperado do script (função do inventário).
    pub total: usize,
    /// pp corrente após a transação (o scan avança a seleção).
    pub current_pp: u16,
}

impl From<BootProgress> for BootProgressDto {
    fn from(p: BootProgress) -> Self {
        Self {
            stage: match p.stage {
                BootStage::Tables => "tables",
                BootStage::Scan => "scan",
                BootStage::Probe => "probe",
                BootStage::Setlist => "setlist",
                BootStage::Names => "names",
                BootStage::Keepalive => "keepalive",
            },
            done: p.done,
            total: p.total,
            current_pp: p.current_pp,
        }
    }
}

/// DTO do relatório do boot (retorno do `device_boot`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootReportDto {
    /// Nº de transações de boot+scan executadas com sucesso.
    pub transactions: usize,
}

/// DTO de um slot da tabela de User IRs (`list_user_irs`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IrSlotDto {
    /// Slot 0..=19.
    pub slot: u8,
    /// Nome ASCII ("" = vazio/0xFF).
    pub name: String,
}

/// DTO da tabela completa (20 slots).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IrTableDto {
    /// Slots decodificáveis (§13.12: nome; CRC de slot ocupado é pendência).
    pub slots: Vec<IrSlotDto>,
}

/// `device_info` — estado do device para o front.
///
/// No mock: leitura local, sem tráfego de fio. No aparelho real: `current_pp`
/// e a tabela de IRs são lidos do dispositivo; o que não tem fonte no fio
/// (nome do pp — meta6 indecifrada —, ppType, CRC de fábrica) volta neutro e
/// a UI mostra desconhecido em vez de zero.
///
/// # Erros
/// String de erro se o actor morreu (pânico do core — não deve ocorrer;
/// ScreenState.error do front oferece retry).
#[tauri::command]
pub fn device_info(state: State<'_, AppState>) -> Result<DeviceInfo, String> {
    let snapshot = state.actor.info()?;
    Ok(DeviceInfo::from_snapshot(&snapshot))
}

/// `device_conectar` — refaz o `open` do transporte do actor (issue #150).
///
/// O app que subiu sem aparelho sobe no estado `Desligado` (`device_info`
/// responde `backend: "none"` + motivo); o clique em "Reconectar" da UI vem
/// para cá. No build de campo o transporte re-enumera as portas MIDI e, ao
/// achar o aparelho, o estado sobe para `Real`; com a sessão viva é no-op
/// (re-open por cima — device sumiu e voltou).
///
/// # Erros
/// String com o motivo (aparelho ausente) ou morte do actor.
#[tauri::command]
pub fn device_conectar(state: State<'_, AppState>) -> Result<(), String> {
    state.actor.conectar()
}

/// `device_board` — board do preset (dados do pedalboard da UI): slots da
/// cadeia, arquétipos por família e knobs do dicionário com os valores do
/// preset. Projeção pura no core, servida pelo actor (D8: um caminho só).
/// `pp` = null → corrente.
#[tauri::command]
pub fn device_board(
    state: State<'_, AppState>,
    pp: Option<u16>,
) -> Result<gp100_core::pedalboard::BoardView, String> {
    state.actor.board(pp)
}

/// `device_preset_library` — a biblioteca (flight case) + pp corrente do
/// mock, servida pelo actor.
#[tauri::command]
pub fn device_preset_library(
    state: State<'_, AppState>,
) -> Result<crate::actor::PresetLibrary, String> {
    state.actor.library()
}

/// `device_select_preset` — select REAL via FSM (§13.10): write `13010000`
/// e meta6; o mock troca o pp corrente (o board/LED recarregam depois).
#[tauri::command]
pub fn device_select_preset(state: State<'_, AppState>, pp: u16) -> Result<(), String> {
    state.actor.select_preset(pp)
}

/// `device_set_param` — knob REAL (§13.11, fire-and-forget D4): o mock
/// valida o shape contra o golden e guarda o valor. O board da UI re-seta
/// o valor local após o Ok.
#[tauri::command]
pub fn device_set_param(
    state: State<'_, AppState>,
    slot: u8,
    code: u32,
    ctrl: u8,
    value: f32,
) -> Result<(), String> {
    state.actor.set_param(slot, code, ctrl, value)
}

/// `device_boot` — boot+scan completos (§13.10) com barra de progresso:
/// emite `device://progress` (BootProgressDto) por transação. BLOQUEIA até
/// o fim do script (command síncrono; o Tauri o roda fora da main thread —
/// ADR-3) e devolve o relatório final. O progresso atravessa a fronteira
/// do actor por CANAL (mpsc) e vira evento aqui na thread do command.
///
/// # Erros
/// [`ProtocolError`](gp100_core::ProtocolError) como string (Timeout D6,
/// InvalidShape D5) — o front mostra no ScreenState.error com retry.
#[tauri::command]
pub fn device_boot(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<BootReportDto, String> {
    // Beats chegam pelo canal; o recv é bloqeuante — mas o command é
    // síncrono fora da main thread e o actor produz beats a cada transação.
    // O canal é drenado NO CALLBACK de fim (boot terminou → drain garante
    // o último beat antes do retorno).
    let (ptx, prx) = mpsc::channel::<BootProgress>();
    let emitter = app.clone();
    // Encaminha os beats SEM bloquear o retorno do boot: o drain roda em
    // thread própria e reemite até o sender fechar (fim do boot no actor).
    let forward = std::thread::spawn(move || {
        for p in prx.iter() {
            let _ = emitter.emit("device://progress", BootProgressDto::from(p));
        }
    });
    let report = state.actor.boot(Some(ptx))?;
    let _ = forward.join(); // último beat emitido antes do retorno
                            // Pushes do boot (dump 13000000, nomes 11000008 etc. — backlog D7 da
                            // inbox do mock) saem como evento `device://push` para o log da UI.
    for hex in state.actor.drain_pushes()? {
        let _ = app.emit("device://push", hex);
    }
    Ok(BootReportDto {
        transactions: report.transactions,
    })
}

/// `list_user_irs` — tabela dos 20 User IRs (§13.12; by-len do golden
/// distingue a tabela 75B do ACK 4B — D1). Transação própria no actor.
///
/// # Erros
/// [`ProtocolError`] como string (Timeout/InvalidShape).
#[tauri::command]
pub fn list_user_irs(state: State<'_, AppState>) -> Result<IrTableDto, String> {
    let slots = state.actor.list_user_irs()?;
    Ok(IrTableDto {
        slots: slots
            .into_iter()
            .map(|(slot, name)| IrSlotDto { slot, name })
            .collect(),
    })
}

/// `device_save_preset` — **salva o preset no aparelho** (§13.12).
///
/// Substitui o `save` do `gp100-cli`, que era o único caminho para isso: o
/// `set-param` é fire-and-forget (§13.11, D4), entao sem o save a mudança
/// morre com a sessão. 9 frames de escrita (5 do meta + 4 ops, D3).
///
/// No build de leitura (sem `write-verified`) o transporte real recusa com
/// `WriteBlocked` **antes do driver** — o operador vê o erro, e o botão
/// ja nasce desabilitado pelo `writeVerified` do `device_info`.
///
/// # Erros
/// String de erro do core (nome não-ASCII, pp fora de faixa, escrita
/// bloqueada) ou morte do actor.
#[tauri::command]
pub fn device_save_preset(
    state: State<'_, AppState>,
    pp: u16,
    pp_type: u16,
    name: String,
) -> Result<(), String> {
    state.actor.save_preset(pp, pp_type, &name)
}

/// `device_dump_preset` — **lê o preset do aparelho** (§13.9): meta6 + as 8
/// páginas de estado, em hex cru.
///
/// O hex é deliberadamente cru e não decodificado: o layout byte-a-byte da
/// família `13xx` ainda não foi decifrado (R1: nunca adivinhar protocolo),
/// e devolver o bruto é o que permite decifrar depois. É este mesmo dump que
/// o `h2_field.sh` usou para provar o F1 do H2 no display.
///
/// # Erros
/// String de erro do core (timeout, shape inesperado) ou morte do actor.
#[tauri::command]
pub fn device_dump_preset(
    state: State<'_, AppState>,
    pp: u16,
) -> Result<crate::actor::DumpReport, String> {
    state.actor.dump_preset(pp)
}

/// `device_log_session` — **liga o log de fio da sessao** (schema P4).
///
/// E o MESMO formato que o `--log` do `gp100-cli` grava e que
/// `scripts/h1_compare.py` / `scripts/h2_compare.py` leem. Liga e desliga
/// **sem reiniciar o device**: o `LoggingTransport` é um decorador
/// transparente que fica no caminho desde o inicio da sessao.
///
/// Fecha o ciclo de campo pelo app: o operador faz a sessao no editor,
/// exporta o `.jsonl` e roda o juiz — sem depender do binario de campo.
///
/// # Erros
/// String com o erro do SO se o arquivo nao puder ser criado.
#[tauri::command]
pub fn device_log_session(state: State<'_, AppState>, path: String) -> Result<bool, String> {
    state.actor.log_session(&path)
}

/// `device_log_stop` — desliga o log de fio (a sessao segue; so o log para).
///
/// # Erros
/// String de erro se o actor morreu.
#[tauri::command]
pub fn device_log_stop(state: State<'_, AppState>) -> Result<bool, String> {
    state.actor.log_stop()
}

/// `device_log_path` — **em que arquivo o log de fio está gravando agora**.
///
/// Existe para a TELA poder dizer o arquivo sem depender do `stderr` do
/// processo: desde a #130 o build de campo liga o log **sozinho** na abertura, e
/// o caminho é escolhido pelo `run()` (diretório de dados + carimbo), não pelo
/// front. `None` = nenhum log ativo (o mock, ou o log parado).
///
/// # Erros
/// String de erro se o actor morreu.
#[tauri::command]
pub fn device_log_path(state: State<'_, AppState>) -> Result<Option<String>, String> {
    state.actor.log_path()
}

/// Sistema onde o app está rodando — separado de [`comando_revelar`] para que
/// o teste cubra os TRÊS alvos em qualquer host (sem abrir janela em nenhum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Alvo {
    /// `explorer /select,<arquivo>` — abre a pasta com o arquivo MARCADO.
    Windows,
    /// `open -R <arquivo>` — o Finder "mostrar no Finder".
    Macos,
    /// `xdg-open <pasta>` — sem "selecionar arquivo" no Linux: abre a pasta.
    Linux,
}

/// O SO de hoje, por compilação (mesma escolha do resto do crate: sem env).
fn alvo_atual() -> Alvo {
    if cfg!(target_os = "windows") {
        Alvo::Windows
    } else if cfg!(target_os = "macos") {
        Alvo::Macos
    } else {
        Alvo::Linux
    }
}

/// Monta (programa, argumentos) que revela o arquivo no gerenciador do SO.
///
/// **Puro de propósito:** o `spawn` fica no command, e assim o teste confere os
/// três alvos sem abrir janela nenhuma em CI — o que o SO recebe é a parte que
/// pode divergir por plataforma.
fn comando_revelar(alvo: Alvo, caminho: &str) -> (String, Vec<String>) {
    match alvo {
        Alvo::Windows => ("explorer".into(), vec![format!("/select,{caminho}")]),
        Alvo::Macos => ("open".into(), vec!["-R".into(), caminho.into()]),
        Alvo::Linux => {
            // Sem "revelar" no Linux: o que existe é abrir a PASTA que contém
            // o arquivo. `parent` vazio = o arquivo está na raiz relativa.
            let pasta = std::path::Path::new(caminho)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .filter(|p| !p.is_empty())
                .unwrap_or_else(|| ".".into());
            ("xdg-open".into(), vec![pasta])
        }
    }
}

/// Decisão do `device_log_reveal` sem o `State` (testável) — devolve o comando
/// ou a RECUSA quando não há log ativo.
///
/// Recusar aqui é o que impede o painel de mandar o explorer para qualquer
/// lugar: o caminho não vem do front, vem do log que a sessão abriu.
fn plano_revelar(log_ativo: Option<String>) -> Result<(String, Vec<String>), String> {
    let caminho = log_ativo.ok_or_else(|| "nenhum log de fio ativo".to_string())?;
    Ok(comando_revelar(alvo_atual(), &caminho))
}

/// `device_log_reveal` — **abre o gerenciador de arquivos com o log selecionado**.
///
/// É a última etapa de entregar uma sessão de campo (#135): o painel já diz EM
/// QUE arquivo a gravação está; aqui o operador leva o arquivo até o suporte
/// sem caçar a pasta (`%APPDATA%`/`AppData/Roaming` não é óbvio para quem
/// recebe o `.jsonl`).
///
/// O caminho é o do log ATIVO (o actor), nunca um do front — ver
/// [`plano_revelar`]. O `spawn` não bloqueia: o gerenciador é outro processo.
///
/// # Erros
/// `String` quando não há log ativo ou o SO recusou abrir (explorer ausente,
/// xdg-open sem handler). O painel mostra a mensagem — ele não finge que abriu.
#[tauri::command]
pub fn device_log_reveal(state: State<'_, AppState>) -> Result<(), String> {
    let (programa, args) = plano_revelar(state.actor.log_path()?)?;
    std::process::Command::new(programa)
        .args(args)
        .spawn()
        .map(|_processo| ())
        .map_err(|e| format!("não abri a pasta do log: {e}"))
}

/// Um frame que o aparelho **receberia**, sem receber.
///
/// O hex e o mesmo que sairia pelo `send_raw` — inclusive o CRC recalculado
/// do save §13.12 e o payload nibble-expandido do set-param §13.11.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewFrame {
    /// `func` + addr do frame (ex.: `12/10030002`), para o operador saber o
    /// que esta olhando sem contar bytes.
    pub label: String,
    /// O SysEx completo em hex.
    pub hex: String,
}

/// `device_preview` — **o que o aparelho receberia, sem receber**.
///
/// Substitui o `--dry-run` do `gp100-cli`. Vive no BACKEND porque o front nao
/// pode montar SysEx (a regra da porta unica: `ui/src/ipc` nunca importa o
/// core) — e porque a trava de valor da #110 precisa valer no preview tanto
/// quanto no envio: um dry-run que aceita o `99.5` e mostra "OK" seria o
/// pior dos dois mundos.
///
/// Cobre o set-param (1 frame, §13.11) e o save (9 frames, §13.12). O
/// upload de IR e de SnapTone NAO tem preview: sao 295 chunks e 143 blocos
/// de payload opaco — "pre-visualizar" isso nao informa nada.
///
/// # Erros
/// String de erro do core (valor fora da faixa, nome nao-ASCII, slot
/// invalido). **Nenhum byte sai**: a funcao nao tem como enviar.
#[tauri::command]
pub fn device_preview(op: PreviewOp) -> Result<Vec<PreviewFrame>, String> {
    match op {
        PreviewOp::SetParam {
            slot,
            code,
            ctrl,
            value,
        } => {
            let f =
                gp100_core::codec::set_param(slot, code, ctrl, value).map_err(|e| e.to_string())?;
            Ok(vec![PreviewFrame {
                label: format!("12/10{slot:02x}0002"),
                hex: f.iter().map(|b| format!("{b:02x}")).collect(),
            }])
        }
        PreviewOp::Save { pp, pp_type, name } => {
            // `save_frames` e a MESMA funcao que o `save_preset` consome: o
            // preview e o envio nao podem divergir, porque nao ha duas
            // implementacoes da montagem do §13.12.
            let frames =
                gp100_core::session::save_frames(pp, pp_type, &name).map_err(|e| e.to_string())?;
            Ok(frames
                .into_iter()
                .map(|f| PreviewFrame {
                    label: format!(
                        "12/{}",
                        f.addr
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<String>()
                    ),
                    hex: f.sysex.iter().map(|b| format!("{b:02x}")).collect(),
                })
                .collect())
        }
    }
}

/// A operacao que o `device_preview` sabe descrever.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum PreviewOp {
    /// Um knob da cadeia (1 frame, §13.11).
    SetParam {
        /// Posição na cadeia, 1..=9.
        slot: u8,
        /// `effectCode` u32 do dicionário.
        code: u32,
        /// Índice do controle.
        ctrl: u8,
        /// Valor físico desejado.
        value: f32,
    },
    /// A gravação do preset (9 frames, §13.12).
    Save {
        /// pp de destino.
        pp: u16,
        /// ppType.
        pp_type: u16,
        /// Nome ASCII do preset.
        name: String,
    },
}

/// `pending_pushes` — backlog D7 drenado do DEVICE (inbox do mock, FIFO
/// global) como hex cru (F0…F7) para o log da UI. Consumidor alternativo
/// ao evento `device://push` (poll explícito do front).
///
/// # Erros
/// String de erro se o actor morreu.
#[tauri::command]
pub fn pending_pushes(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state.actor.drain_pushes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gp100_core::transport::mock::MockDevice;
    use gp100_core::transport::DeviceTransport;

    /// O DTO do command é derivado do snapshot REAL do actor (nunca de
    /// valores inventados): mesmo estado que o CLI `info` imprime — R1 nos DTOs.
    #[test]
    fn device_info_deriva_do_mock_real() {
        let mut mock = MockDevice::new().expect("mock montado (R4 travado no build)");
        mock.open().expect("abre");
        let actor = crate::actor::DeviceActor::spawn(mock, crate::actor::Backend::Mock);
        let info = DeviceInfo::from_snapshot(&actor.info().expect("snapshot"));
        actor.shutdown();
        assert_eq!(info.backend, "mock");
        assert_eq!(info.preset_count, 99);
        assert_eq!(info.current_pp, 0x0000);
        assert_eq!(info.current_name, "It's GP100");
        assert_eq!(info.ir_slots_with_crc, 20);
        // ppType do 1º preset — vem do .prst; o teste fixa o valor do mock.
        assert_eq!(info.current_pp_type, 4);
    }

    /// **O contrato do campo real.** Com o aparelho ligado, `backend` muda
    /// para "real" e os campos que não têm fonte no fio ficam neutros — o
    /// oposto de encher de zero, que a UI mostraria como dado. Este teste
    /// fixa a HONESTIDADE do DTO, não um valor do mock.
    #[test]
    fn device_info_do_aparelho_real_nao_inventa_campo() {
        let info = DeviceInfo::from_snapshot(&crate::actor::DeviceSnapshot {
            backend: "real",
            detail: String::new(),
            preset_count: 199,
            current_pp: 0x0000,
            // nome vazio: o layout da meta6 (13010001) ainda nao foi decifrado
            current_name: String::new(),
            current_pp_type: 0,
            ir_slots_with_crc: 0,
            ir_slots: vec![(2, "meu_ir".to_string())],
            write_verified: false,
        });
        assert_eq!(info.backend, "real");
        assert_eq!(info.current_name, "", "sem fonte no fio = vazio, nao nome");
        assert_eq!(info.current_pp_type, 0);
        assert_eq!(info.ir_slots_with_crc, 0);
        assert!(!info.write_verified, "build de leitura: escrita travada");
        // A tabela de IRs, essa sim, veio do device.
        assert_eq!(info.ir_slots.len(), 1);
        assert_eq!(info.ir_slots[0].slot, 2);
        assert!(
            info.detail.is_empty(),
            "sessão viva não inventa aviso de conexão"
        );
    }

    /// **O estado DESLIGADO (#150) é honesto:** `backend: "none"`, motivo
    /// legível, zero presets — e o `device_conectar` falha com o motivo (não
    /// há aparelho para abrir).
    #[test]
    fn device_info_desligado_nao_inventa_aparelho() {
        let actor = crate::actor::DeviceActor::desligado("Aparelho não conectado via USB");
        let info = DeviceInfo::from_snapshot(&actor.info().expect("snapshot do desligado"));
        assert_eq!(info.backend, "none");
        assert_eq!(info.detail, "Aparelho não conectado via USB");
        assert_eq!(info.preset_count, 0);
        assert!(!info.write_verified, "estado desligado não promete escrita");
        assert!(actor.conectar().is_err(), "sem aparelho não há o que abrir");
        actor.shutdown();
    }

    /// O serde em camelCase é o CONTRATO do fio IPC (ui/src/ipc/types.ts):
    /// se alguém renomear campo, o JSON diverge do TS — este teste quebra.
    #[test]
    fn device_info_serializa_camelcase() {
        let mut mock = MockDevice::new().expect("mock montado");
        mock.open().expect("abre");
        let actor = crate::actor::DeviceActor::spawn(mock, crate::actor::Backend::Mock);
        let info = DeviceInfo::from_snapshot(&actor.info().expect("snapshot"));
        actor.shutdown();
        let json = serde_json::to_value(&info).expect("serializável");
        assert!(json.get("presetCount").is_some());
        assert!(json.get("currentPp").is_some());
        assert!(json.get("currentName").is_some());
        assert!(json.get("currentPpType").is_some());
        assert!(json.get("irSlotsWithCrc").is_some());
        assert!(json.get("backend").is_some());
        assert!(json.get("detail").is_some(), "motivo do estado desligado");
        assert!(json.get("irSlots").is_some(), "contrato novo com o front");
        assert!(json.get("writeVerified").is_some(), "sinal de escrita");
        assert_eq!(json["currentPp"], 0);
        assert_eq!(json["currentName"], "It's GP100");
    }

    /// O beat de progresso serializa stage como literal ESTÁVEL (o front
    /// roteia por ele) e camelCase nos contadores.
    #[test]
    fn boot_progress_dto_serializa_camelcase() {
        let p = BootProgressDto {
            stage: "scan",
            done: 45,
            total: 2299,
            current_pp: 0x0100,
        };
        let json = serde_json::to_value(&p).expect("serializável");
        assert_eq!(json["stage"], "scan");
        assert_eq!(json["done"], 45);
        assert_eq!(json["total"], 2299);
        assert_eq!(json["currentPp"], 0x0100);
    }

    /// **Os três SOs, num host só.** O que diverge por plataforma é a linha de
    /// comando; se o teste rodasse só o SO atual, um Linux quebrado passaria
    /// na CI do Windows. Nenhum teste aqui abre janela — só monta o par.
    #[test]
    fn comando_revelar_monta_a_linha_de_cada_so() {
        let win = comando_revelar(Alvo::Windows, "C:\\dados\\wire-20261006.jsonl");
        assert_eq!(win.0, "explorer");
        assert_eq!(win.1, vec!["/select,C:\\dados\\wire-20261006.jsonl"]);

        let mac = comando_revelar(Alvo::Macos, "/Users/o/wire-20261006.jsonl");
        assert_eq!(mac.0, "open");
        assert_eq!(mac.1, vec!["-R", "/Users/o/wire-20261006.jsonl"]);

        // Linux não tem "selecionar": abre a PASTA que contém o arquivo.
        let lin = comando_revelar(Alvo::Linux, "/home/o/logs/wire-20261006.jsonl");
        assert_eq!(lin.0, "xdg-open");
        assert_eq!(lin.1, vec!["/home/o/logs"]);
    }

    /// Caminho na raiz (sem pasta) não vira argumento vazio — vira `.`, que é
    /// a única resposta que o `xdg-open` entende.
    #[test]
    fn comando_revelar_linux_sem_pasta_abre_o_diretorio_atual() {
        let lin = comando_revelar(Alvo::Linux, "wire.jsonl");
        assert_eq!(lin.1, vec!["."]);
    }

    /// **Sem log ativo não há o que revelar, e a recusa é explícita.** É o que
    /// impede o painel de abrir um gerenciador num lugar qualquer: o caminho
    /// vem do log da sessão, não de um campo do front.
    #[test]
    fn plano_revelar_sem_log_recusa_antes_de_montar_o_comando() {
        let erro = plano_revelar(None).expect_err("sem log tem de recusar");
        assert!(erro.contains("nenhum log de fio ativo"), "{erro}");
    }

    /// Com log ativo, o comando sai com o CAMINHO do log (o backend é quem
    /// sabe o arquivo — o front só clica no botão).
    #[test]
    fn plano_revelar_com_log_devolve_o_caminho_da_sessao() {
        let (programa, args) = plano_revelar(Some("/dados/wire-1.jsonl".into())).expect("plano");
        assert!(!programa.is_empty());
        assert!(
            args.iter()
                .any(|a| a.contains("wire-1.jsonl") || a.contains("/dados")),
            "o caminho do log tem de aparecer no comando: {args:?}"
        );
    }
}
