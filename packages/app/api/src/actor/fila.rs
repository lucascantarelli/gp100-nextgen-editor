//! A fila do actor: `Request` é o contrato entre os commands (thread do Tauri)
//! e a thread que possui a `Session` (ADR-6/D8). Cada variante carrega o canal
//! de resposta — o command síncrono bloqueia no `recv` (ADR-3 preservado).

use std::sync::mpsc;

use gp100_core::pedalboard::BoardView;
use gp100_core::session::{
    BootProgress, BootReport, GlobalBruto, IrUploadReport, SnapToneUploadReport,
};

use super::tipos::{DeviceSnapshot, DumpReport, PresetLibrary};

/// Requisições atendidas pelo actor (uma por vez — D8).
pub(super) enum Request {
    /// Snapshot do estado do device (command `device_info`).
    Info {
        reply: mpsc::Sender<Result<DeviceSnapshot, String>>,
    },
    /// Reconecta o transporte (issue #150 — command `device_conectar`):
    /// `open` de novo no MESMO objeto (ADR-4 — o `RealDevice::open` após
    /// falha/DeviceGone reabre as portas). Com sessão viva é re-open por cima;
    /// sem sessão (app que subiu sem aparelho) é o primeiro open.
    Conectar {
        reply: mpsc::Sender<Result<(), String>>,
    },
    /// Relê as 9 páginas do preset corrente AGORA (#164 — command
    /// `device_sync_now`): adianta o prazo do read-back sem esperar o
    /// refresh de cadência. É a ação "Reler agora" do chip de
    /// sincronização — este braço NÃO lê nada por conta própria, só marca o
    /// relógio; a leitura roda no mesmo caminho (e com as mesmas falhas) do
    /// debounce, e o desfecho chega pelo evento `device://sync`.
    SyncNow {
        reply: mpsc::Sender<Result<(), String>>,
    },
    /// Board do preset: slots/arquétipos/knobs (projeção pura no core).
    Board {
        pp: Option<u16>,
        reply: mpsc::Sender<Result<BoardView, String>>,
    },
    /// Biblioteca de presets (flight case da UI).
    Library {
        reply: mpsc::Sender<Result<PresetLibrary, String>>,
    },
    /// Select de preset (write `13010000` §13.10; o mock troca o corrente).
    Select {
        pp: u16,
        reply: mpsc::Sender<Result<(), String>>,
    },
    /// Set-param (knob §13.11, fire-and-forget D4; o mock valida e guarda).
    SetParam {
        slot: u8,
        code: u32,
        ctrl: u8,
        value: f32,
        reply: mpsc::Sender<Result<(), String>>,
    },
    /// Boot completo (§13.10) com canal de progresso opcional
    /// (command `device_boot`): 1 mensagem por transação do script.
    Boot {
        progress: Option<mpsc::Sender<BootProgress>>,
        reply: mpsc::Sender<Result<BootReport, String>>,
    },
    /// Tabela dos 20 User IRs (§13.12; command `list_user_irs`) —
    /// transação própria da FSM (by-len do golden distingue tabela 75B
    /// do ACK 4B, D1).
    ListIrs {
        reply: mpsc::Sender<Result<Vec<(u8, String)>, String>>,
    },
    /// Registradores globais (§13.13 — command `device_globals`): o bloco
    /// medido SYSTEM `000200xx` + DRUM `000102xx`, payload crU por
    /// registrador (o decode medido fica no chamador — ADR-13).
    Globals {
        reply: mpsc::Sender<Result<Vec<GlobalBruto>, String>>,
    },
    /// Write num registrador global (§13.13.1 — command `device_set_global`):
    /// envelope canônico; a trava ADR-5 é do TRANSPORTE (write-verified).
    SetGlobal {
        addr: [u8; 4],
        payload: Vec<u8>,
        reply: mpsc::Sender<Result<(), String>>,
    },
    /// Drena pushes não consumidos do DEVICE (inbox do mock, FIFO global —
    /// D7 na perspectiva do device) como hex cru (command `pending_pushes`;
    /// o `device_boot` usa o mesmo caminho para reemitir `device://push`).
    DrainPushes {
        reply: mpsc::Sender<Result<Vec<String>, String>>,
    },
    /// Envia um modelo de SnapTone ao device (§5): stream de blocos com ACK
    /// de 16B por bloco e o settle de 250 ms do §4. É a transação mais LONGA
    /// do app depois do boot — 143 blocos para um `.clo` de ~2,7 KB — e é por
    /// isso que ela fica na fila do actor e não em `spawn`: a fila é o que
    /// impede um upload de bytes se misturar com um knob do dono.
    UploadSnapTone {
        slot: u8,
        model: Vec<u8>,
        reply: mpsc::Sender<Result<SnapToneUploadReport, String>>,
    },
    /// Envia um IR de usuário ao device (§13.7): `ir_begin` + chunks de 15B
    /// com ACK por chunk + o último chunk DUPLICADO (o marcador de fim).
    ///
    /// Fica na fila como todo tráfego de fio (D8): um IR de 300 KB são
    /// 20.000 chunks, e misturar isso com um knob do dono seria a mesma
    /// corrida que a fila existe para impedir.
    UploadIr {
        slot: u8,
        blob: Vec<u8>,
        reply: mpsc::Sender<Result<IrUploadReport, String>>,
    },
    /// Salva o preset no aparelho (§13.12) — 9 frames de escrita: 5 do meta
    /// e 4 ops (D3). Este era o ÚNICO jeito de persistir uma mudança: o
    /// `set-param` é fire-and-forget e morre com a sessão. O `gp100-cli`
    /// tinha `save` e o app não tinha command nenhum — a primeira
    /// funcionalidade do CLI trazida para dentro do app.
    Save {
        pp: u16,
        pp_type: u16,
        name: String,
        reply: mpsc::Sender<Result<(), String>>,
    },
    /// Lê o preset do aparelho (§13.9: meta6 + páginas de estado) e devolve
    /// o hex cru. Também era exclusivo do CLI (`dump-preset`).
    Dump {
        pp: u16,
        reply: mpsc::Sender<Result<DumpReport, String>>,
    },
    /// Liga (ou religa) o log de fio da sessao no schema P4 -- o MESMO que
    /// o `--log` do CLI grava e que `h1_compare.py`/`h2_compare.py` leem.
    ///
    /// E o que fecha o ciclo de campo pelo app: o operador abre a sessao no
    /// editor, faz o que precisa, exporta o `.jsonl` e roda o juiz. Sem
    /// isto, a unica forma de capturar o fio era pelo binario de campo.
    LogSession {
        path: String,
        reply: mpsc::Sender<Result<bool, String>>,
    },
    /// Desliga o log de fio (a sessao segue; so o log para).
    LogStop {
        reply: mpsc::Sender<Result<bool, String>>,
    },
    /// Em que arquivo o log de fio esta gravando agora (`None` = sem log).
    ///
    /// Existe porque o caminho **nao e do front**: desde a #130 o build de campo
    /// liga o log sozinho na abertura, com o nome que o `run()` escolhe
    /// (diretorio de dados + carimbo). Sem isto a tela mostraria "nenhum log" e
    /// ofereceria "gravar" por cima de uma sessao que **ja esta em disco**.
    LogPath { reply: mpsc::Sender<Option<String>> },
    /// Encerra a thread do actor (drop do `DeviceActor`).
    Shutdown,
}
