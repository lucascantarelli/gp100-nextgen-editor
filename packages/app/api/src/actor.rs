//! DeviceActor — dono ÚNICO da `Session` (D8 do ADR-6).
//!
//! A FSM é `&mut self` (D1/D8: consumidor único do stream IN); compartilhar
//! `Mutex<Session>` com a UI permitiria interleave de transações entre
//! commands. O actor resolve: UMA thread possui a `Session` e processa uma
//! fila `mpsc` de requisições na ordem de chegada; cada command envia a
//! requisição + um canal de resposta e fica bloqueado no `recv` (o command
//! síncrono do Tauri roda fora da main thread — ADR-3 preservado).
//!
//! Posse: `Session::new(transport)` TOMA o transporte por valor (ADR-4/6) —
//! o actor guarda `Option<Session<MockDevice>>` e, para o snapshot de
//! `device_info`, devolve o transporte com `into_transport()` e remonta a
//! Session (o backlog D7 de observação é descartado — não há transação em
//! curso; operações novas chegam vazias pela fila serializada).
//!
//! Progresso do boot: o command passa um `mpsc::Sender<BootProgress>`
//! (Send) na requisição; o actor fecha um callback em volta do sender e o
//! `device_boot` encaminha cada beat como evento Tauri `device://progress`
//! (a UI mostra barra — nunca trava). Callback por REFERÊNCIA não atravessa
//! a fila (não é Send); canal é o padrão seguro.
//!
//! Inventário: o actor usa o DEFAULT da `Session` (0..198). O inventário
//! da captura (pp corrente primeiro + duplicação do 0x0100) é artefato de
//! REPLAY — o app não reproduz quirks de captura, apenas o script.

use std::collections::BTreeMap;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use gp100_core::model::Dictionary;
use gp100_core::pedalboard::{board_view_for, embedded_document, preset_list, BoardView};
use gp100_core::session::{
    inventario_do_aparelho, BootProgress, BootReport, GlobalBruto, IrUploadReport, Session,
    SnapToneUploadReport,
};
use gp100_core::transport::mock::MockDevice;
use gp100_core::transport::{DeviceTransport, TransportError, WireKind};
use gp100_core::wire_log::LoggingTransport;

/// Estado snapshot do mock (alias curto; o tipo vive no core).
type MockState = gp100_core::transport::mock::MockState;

/// Estado de sincronização app ↔ device (#164) — os estados TIPADOS da
/// convergência: `Inativo` (sem conversa ainda — nada a afirmar),
/// `Sincronizado` (a última leitura confirmou veio do aparelho), `Pendente`
/// (escrita aceita; o read-back ainda não rodou), `Falhou` (a última
/// operação deu erro; a sessão segue viva) e `Offline` (o transporte caiu —
/// o app NÃO continua escrevendo em vazio).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    /// Sem boot/refresh ainda — o chip mostra "—", sem número falso.
    Inativo,
    /// A última releitura veio do aparelho (o cache bate com o fio).
    Sincronizado,
    /// Escrita aceita no fio; o read-back (debounce) ainda vai confirmar.
    Pendente,
    /// Última operação falhou (motivo em `sync_detail`); sessão viva.
    Falhou,
    /// Sessão sem aparelho (transporte caiu ou build sem transporte).
    Offline,
}

impl SyncStatus {
    /// Literal estável do fio IPC (o front não switcha em Debug).
    pub fn as_str(&self) -> &'static str {
        match self {
            SyncStatus::Inativo => "idle",
            SyncStatus::Sincronizado => "sync",
            SyncStatus::Pendente => "pending",
            SyncStatus::Falhou => "failed",
            SyncStatus::Offline => "offline",
        }
    }
}

/// Evento `device://sync` — a transição do estado de sincronização (#164).
///
/// `changed = true` SÓ no read-back que trouxe páginas diferentes do cache:
/// é o sinal de que o APARELHO mudou (ou não aplicou o que o app escreveu) —
/// o front reler o board e mostra o valor do aparelho, nunca o desejado.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncEvent {
    /// `idle` | `sync` | `pending` | `failed` | `offline`.
    pub status: &'static str,
    /// `true` quando o read-back divergiu do cache (o aparelho vence).
    pub changed: bool,
    /// Detalhe humano de `failed`/`offline` (vazio nos outros estados).
    pub detail: String,
}

/// Falha classificada do read-back (#164): `Caiu` = o transporte morreu
/// (DeviceGone — vira `Offline` e o guarda para reconexão); `Outro` = erro
/// de protocolo/decode (vira `Falhou` sem derrubar a sessão).
enum FalhaLeitura {
    Caiu(String),
    Outro(String),
}

/// Debounce do read-back pós-escrita (#164): uma RAJADA de knob (N giros
/// seguidos) vira UMA releitura das 9 páginas. 800 ms — medido contra o
/// custo real no fio (ADR-15: RTT mediano 3,05 ms no H1 → 9 páginas ≈ 27 ms;
/// o debounce custa ~3% do fio numa rajada de 2 s).
pub const READBACK_DEBOUNCE: Duration = Duration::from_millis(800);

/// Cadência do refresh periódico do preset corrente (#164): 10 s ≈ 0,9 tx/s
/// (≈3% do fio pelo RTT mediano do H1). É a ÚNICA janela em que o app vê
/// mudança feita no PEDAL: o knob físico não emite SysEx (ADR-15 — poll,
/// não notificação espontânea).
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(10);

/// Bytes crus em hex maiúsculo, sem separador — o mesmo formato que o
/// `gp100-cli` imprime e que os juízes (`h1_compare.py`/`h2_compare.py`)
/// leem dos logs.
fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02X}")).collect()
}

/// **O tipo de transporte que o build de campo carrega** (feature
/// `real-device`): `Box<dyn DeviceBackend>` — a trait deste crate, que ja
/// estende `DeviceTransport`, entao o `RealDevice` entra pela mesma fila do
/// actor sem que nenhum tipo concrete precise conhecer o outro.
///
/// **(#150)** O app sem aparelho não monta transporte de mentira: o estado é
/// [`Backend::Desligado`] com um transporte honesto — o [`DesconectadoDevice`]
/// no build sem a feature, e o `ProcuraAparelho` (que re-enumera as portas a
/// cada `device_conectar`) no build de campo.
#[cfg(feature = "real-device")]
pub type AppDevice = Box<dyn DeviceBackend + Send>;

/// `WRITE_VERIFIED` **espelhado** do core (ADR-5).
///
/// Vive como `cfg!` local e nao como import do `core::transport::real`
/// porque aquele modulo so existe com a feature `real-device` — e este
/// crate precisa da resposta tambem no build padrao (mock), para o front
/// saber que ali a escrita esta liberada por construcao.
///
/// Espelhar a feature em vez de importa-la e proposital: e o que faz o
/// binario de campo dizer "estou travado" na tela em vez de so descobrir
/// quando o operador clica.
const WRITE_VERIFIED: bool = cfg!(feature = "write-verified");

/// Qual backend esta thread possui. Vem **declarado por quem monta o
/// transporte**, nunca deduzido: a trait [`DeviceTransport`] e a fronteira
/// de BYTES CRUOS (ADR-4) e nao tem como responder "sou mock?" — e essa
/// resposta e o que decide se o app pode prometer nome de preset e CRC de
/// fabrica.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// `MockDevice` — estado local completo, sem tráfego de fio.
    ///
    /// **(#150) o runtime do app não monta mais mock** — quem constrói esta
    /// variante são os TESTES (o actor genérico precisa de um backend real
    /// para a fila) e nenhum caminho de produção. O `cfg_attr` é o mesmo da
    /// variante `Real`: o build padrão acusaria "nunca construído" onde o
    /// desenho tem construtor.
    #[cfg_attr(not(all(test, not(feature = "real-device"))), allow(dead_code))]
    Mock,
    /// `RealDevice` (feature `real-device`) — o aparelho **não tem estado
    /// local**: o que ele mostra na tela, a app precisa ler do fio.
    ///
    /// Mesmo caso do [`AppDevice`]: só a `abrir_backend` de `real-device`
    /// constrói esta variante, e o `match` do `as_str` não conta como
    /// construção — então o build padrão acusaria "nunca construído" para
    /// algo que tem construtor.
    #[cfg_attr(not(feature = "real-device"), allow(dead_code))]
    Real,
    /// App **sem aparelho** (#150): nenhum transporte conectou. NÃO é mock —
    /// não inventa preset, nome ou resposta; `device_info` responde
    /// `backend: "none"` com um motivo legível e a UI mostra o aviso de
    /// conexão. O `device_conectar` refaz o open (no build de campo o
    /// `ProcuraAparelho` re-enumera as portas; ao conectar, o estado sobe
    /// para `Real`).
    Desligado,
}

impl Backend {
    /// Etiqueta estável no fio IPC (`DeviceInfo.backend` do front).
    pub fn as_str(self) -> &'static str {
        match self {
            Backend::Mock => "mock",
            Backend::Real => "real",
            Backend::Desligado => "none",
        }
    }
}

/// Fonte do estado local do backend — só o mock tem.
///
/// Existe **neste crate**, e nao como metodo do `DeviceTransport`, para nao
/// alargar a fronteira de bytes do core (ADR-4: a trait trafega SysEx cru e
/// nada mais). O `RealDevice` devolve `None`, e o app degrada com honestidade
/// em vez de mostrar zero como se fosse um dado.
pub trait DeviceBackend: DeviceTransport + Send {
    /// Estado local do mock, quando existir.
    fn local_state(&self) -> Option<MockState>;

    /// Mensagens que chegaram ao device e **ninguém pediu** (a inbox do
    /// `MockDevice`, D7 na perspectiva do device).
    ///
    /// **Limite conhecido do aparelho real:** o `RealDevice` guarda o excedente
    /// numa fila interna de RX e nao expoe peek, entao devolve vazio. O log de
    /// pushes da UI (`pending_pushes`) e por isso **mock-only por enquanto** —
    /// o espelho fiel no aparelho precisa de um `peek` no `real.rs`, e isso
    /// esta anotado em `docs/REAL_DEVICE_GAP.md` §6.
    fn drain_inbox(&mut self) -> Vec<Vec<u8>>;
}

impl DeviceBackend for MockDevice {
    fn local_state(&self) -> Option<MockState> {
        Some(self.state().clone())
    }

    fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        MockDevice::drain_inbox(self)
    }
}

/// O `LoggingTransport` (do core) continua sendo um backend: o log e um
/// **decorador transparente**, entao o `DeviceBackend` atravessa por ele
/// sem mudar nada. E o que permite ligar o log no MEIO da sessao, sem
/// reiniciar o device nem reconstruir a fila.
impl<T: DeviceBackend> DeviceBackend for LoggingTransport<T> {
    fn local_state(&self) -> Option<MockState> {
        self.inner.local_state()
    }

    fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        self.inner.drain_inbox()
    }
}

/// `Box<dyn DeviceBackend>` tambem e um backend — e o que deixa o build de
/// campo passar o `RealDevice` pela MESMA fila do mock, sem duplicar nada.
impl<T: DeviceBackend + ?Sized> DeviceBackend for Box<T> {
    fn local_state(&self) -> Option<MockState> {
        (**self).local_state()
    }

    fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        (**self).drain_inbox()
    }
}

/// Transporte do app **sem aparelho** (issue #150) para builds sem a feature
/// `real-device`.
///
/// **Não é mock.** Ele não inventa preset, nome, cadeia ou resposta: toda
/// operação de tráfego é recusada, porque não existe fio. Ele existe para o
/// app subir com a janela viva e um estado HONESTO (`backend: "none"` +
/// motivo legível) em vez de derrubar o binário — ou pior, montar o mock e
/// fingir que é o aparelho. No build de campo quem assume este papel é o
/// `ProcuraAparelho`, que tem ação real no reconectar.
///
/// `cfg_attr` de par com o `desligado`: construído só no build SEM a feature.
#[cfg_attr(feature = "real-device", allow(dead_code))]
#[derive(Debug)]
pub struct DesconectadoDevice;

impl DeviceTransport for DesconectadoDevice {
    fn open(&mut self) -> Result<(), TransportError> {
        // OpenFailed, e não DeviceGone: o aparelho está AUSENTE (nunca
        // esteve na sessão) — é a semântica do erro no core.
        Err(TransportError::OpenFailed {
            why: "aparelho não conectado".into(),
        })
    }

    fn close(&mut self) -> Result<(), TransportError> {
        Ok(())
    }

    fn send_raw(&mut self, _data: &[u8], _kind: WireKind) -> Result<(), TransportError> {
        Err(TransportError::Closed)
    }

    fn recv_raw(&mut self, _timeout: std::time::Duration) -> Result<Vec<u8>, TransportError> {
        Err(TransportError::Closed)
    }
}

impl DeviceBackend for DesconectadoDevice {
    fn local_state(&self) -> Option<MockState> {
        // Sem aparelho, sem estado local: o mesmo neutro do `RealDevice`.
        None
    }

    fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        Vec::new()
    }
}

/// Transporte que **procura o aparelho a cada `open`** (issue #150, build de
/// campo): o app que subiu sem USB mantém o botão "Reconectar" com ação real
/// — o open re-enumera as portas MIDI e monta o `RealDevice` na hora, em vez
/// de desistir para sempre.
#[cfg(feature = "real-device")]
#[derive(Debug, Default)]
pub struct ProcuraAparelho {
    /// O aparelho, quando um open já achou as portas.
    device: Option<gp100_core::transport::real::RealDevice>,
}

#[cfg(feature = "real-device")]
impl DeviceTransport for ProcuraAparelho {
    fn open(&mut self) -> Result<(), TransportError> {
        // Re-enumeração REAL a cada tentativa: ligar o aparelho DEPOIS de
        // abrir o app tem que funcionar sem reiniciar o editor.
        match gp100_core::transport::real::RealDevice::new() {
            Ok(d) => {
                self.device = Some(d);
                Ok(())
            }
            Err(e) => {
                self.device = None;
                Err(e)
            }
        }
    }

    fn close(&mut self) -> Result<(), TransportError> {
        self.device = None;
        Ok(())
    }

    fn send_raw(&mut self, data: &[u8], kind: WireKind) -> Result<(), TransportError> {
        match self.device.as_mut() {
            Some(d) => d.send_raw(data, kind),
            None => Err(TransportError::Closed),
        }
    }

    fn recv_raw(&mut self, timeout: std::time::Duration) -> Result<Vec<u8>, TransportError> {
        match self.device.as_mut() {
            Some(d) => d.recv_raw(timeout),
            None => Err(TransportError::Closed),
        }
    }

    fn e_aparelho(&self) -> bool {
        // Quando um open achou o aparelho, os frames saem para o REAL —
        // e a trava de faixa do pp (#132) precisa valer.
        self.device.as_ref().is_some()
    }
}

#[cfg(feature = "real-device")]
impl DeviceBackend for ProcuraAparelho {
    fn local_state(&self) -> Option<MockState> {
        None
    }

    fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        match self.device.as_mut() {
            Some(d) => d.drain_inbox(),
            None => Vec::new(),
        }
    }
}

#[cfg(feature = "real-device")]
impl DeviceBackend for gp100_core::transport::real::RealDevice {
    fn local_state(&self) -> Option<MockState> {
        // O aparelho nao tem estado local: o que ele mostra na tela, a app
        // precisa ler do fio. Encher isto seria a app inventando dado.
        None
    }

    fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        // Limite conhecido, nao um "nao ha nada": ver a doc da trait.
        Vec::new()
    }
}

/// O que o `dump` traz do aparelho: meta6 + páginas de estado, em hex.
///
/// **Hex cru e não decodificado, de propósito.** O layout byte-a-byte da
/// família `13xx` ainda não foi decifrado (o
/// [`gp100_core::session::StatePage`] é opaco por decisão — R1: nunca
/// adivinhar protocolo). Devolver o bruto é o que permite decifrar depois;
/// devolver algo pretty mas errado seria inventar.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DumpReport {
    /// pp que foi lido.
    pub pp: u16,
    /// Página meta6 do pp (6B, §13.9).
    pub meta6: String,
    /// Páginas 0..=7 (196B cada) + a final de 4B, em hex, na ordem do fio.
    pub pages: Vec<String>,
}

/// O que a app pode afirmar sobre o device, **por backend**.
///
/// A distincao que importa: com o mock, `current_name` e `ir_slots_with_crc`
/// sao leituras locais. Com o aparelho real, o nome do pp vem da pagina
/// meta6 (`13010001`) cujo layout **ainda nao foi decifrado** (o
/// [`gp100_core::session::StatePage`] e opaco de proposito), e o CRC de
/// fabrica nao existe em lugar nenhum do fio. Encher esses campos com zero
/// seria a app mentindo sobre o aparelho — por isso eles vao marcados.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSnapshot {
    /// Backend ativo ("mock" | "real" | "none") — nunca mais um literal.
    pub backend: &'static str,
    /// Motivo humano quando `backend` é `"none"` (issue #150): "aparelho não
    /// conectado…" ou "build sem o transporte…". Vazio nos outros backends —
    /// preencher com sessão viva seria inventar aviso.
    pub detail: String,
    /// Nº de presets. No mock, o do `all.prst`; no real, o **inventário que
    /// o boot percorreu** (ver `docs/REAL_DEVICE_GAP.md` §4.3 — hoje é o
    /// default 0..198, não uma contagem descoberta no aparelho).
    pub preset_count: usize,
    /// pp corrente (lido da Session nos dois backends).
    pub current_pp: u16,
    /// Nome do pp corrente. **Só no mock** — ver a nota de `DeviceSnapshot`.
    pub current_name: String,
    /// ppType do pp corrente. **Só no mock.**
    pub current_pp_type: u16,
    /// Slots de IR com CRC de fabrica. **Só no mock** — o fio nao tem esse campo.
    pub ir_slots_with_crc: usize,
    /// Tabela de 20 User IRs **lida do device** (§13.12) — existe nos dois.
    pub ir_slots: Vec<(u8, String)>,
    /// `true` quando o binario foi compilado com `write-verified` (ADR-5).
    /// O front usa isto para desabilitar os botoes de escrita com explicacao,
    /// em vez de descobrir a recusa depois de clicar.
    pub write_verified: bool,
    /// Estado de sincronização (#164): "idle" | "sync" | "pending" |
    /// "failed" | "offline" — preenchido pelo loop do actor (o snapshot é
    /// montado na thread que atende o `Info`, dona do estado).
    pub sync: &'static str,
    /// Detalhe humano de `failed`/`offline` (vazio nos outros estados).
    pub sync_detail: String,
}

impl DeviceSnapshot {
    /// Estado de partida quando nao houve boot (nem mock nem real).
    fn empty(backend: Backend) -> Self {
        Self {
            backend: backend.as_str(),
            detail: String::new(),
            preset_count: 0,
            current_pp: 0,
            current_name: String::new(),
            current_pp_type: 0,
            ir_slots_with_crc: 0,
            ir_slots: Vec::new(),
            write_verified: WRITE_VERIFIED,
            sync: SyncStatus::Inativo.as_str(),
            sync_detail: String::new(),
        }
    }
}

/// Biblioteca de presets (o flight case da UI) + corrente.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetLibrary {
    /// Entradas (pp, nome, tipo) de TODOS os presets do arquivo.
    pub entries: Vec<PresetEntry>,
    /// pp corrente do mock.
    pub current_pp: u16,
}

/// Uma entrada da biblioteca (camelCase no fio IPC).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetEntry {
    /// pp do preset.
    pub pp: u16,
    /// Nome do preset.
    pub name: String,
    /// Rótulo do tipo (ex.: "Pop").
    pub pp_type_name: String,
}

/// Requisições atendidas pelo actor (uma por vez — D8).
enum Request {
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

/// Snapshot do estado via take/remount da Session (mesma semântica do
/// `Info`).
///
/// **O que muda com o aparelho real:** o `MockState` so existe no mock, e
/// `local_state()` devolve `None` no `RealDevice`. Os campos que vinham
/// dele (nome do pp, ppType, CRC de fabrica) ficam no estado neutro em vez
/// de receber um zero que a UI mostraria como dado — ver
/// [`DeviceSnapshot`].
fn device_snapshot<T: DeviceBackend>(
    session: &mut Option<Session<T>>,
    backend: Backend,
    nomes: &BTreeMap<u16, String>,
) -> DeviceSnapshot {
    let Some(s) = session.take() else {
        return DeviceSnapshot::empty(backend);
    };
    let transport = s.into_transport();
    let local = transport.local_state();
    let mut s = Session::new(transport);
    let current_pp = s.current_pp();
    let ir_slots = s.list_user_irs().map(|t| t.slots).unwrap_or_default();
    *session = Some(s);
    let pp_corrente = local.as_ref().map_or(current_pp, |m| m.current_pp);
    DeviceSnapshot {
        backend: backend.as_str(),
        // O motivo do Desligado é preenchido no braço do `Info` (quem o
        // conhece é o spawn, não esta função).
        detail: String::new(),
        preset_count: local.as_ref().map_or(0, |m| m.preset_count),
        current_pp: pp_corrente,
        // #155 §5.1: `current_name` vem do CACHE do scan (pg0 decodificada)
        // — é o nome que o aparelho ESTÁ mostrando. O `local` (mock) fica
        // de reserva para quando o boot não rodou; sem as duas fontes,
        // vazio (nunca um nome inventado).
        current_name: nomes
            .get(&pp_corrente)
            .cloned()
            .or_else(|| local.as_ref().map(|m| m.current_name.clone()))
            .unwrap_or_default(),
        current_pp_type: local.as_ref().map_or(0, |m| m.current_pp_type),
        ir_slots_with_crc: local
            .as_ref()
            .map_or(0, |m| m.ir_crcs.iter().filter(|c| **c != 0).count()),
        ir_slots,
        write_verified: WRITE_VERIFIED,
        sync: SyncStatus::Inativo.as_str(),
        sync_detail: String::new(),
    }
}

/// Envia um evento `device://sync` pelo sink registrado (#164). Sem sink
/// (testes que não observam eventos, ou antes do `run()` registrar) o evento
/// é descartado — o estado NÃO depende do sink, ele fica no `info`.
fn emite(sink: &std::sync::Arc<std::sync::Mutex<Option<mpsc::Sender<SyncEvent>>>>, ev: SyncEvent) {
    if let Ok(g) = sink.lock() {
        if let Some(tx) = g.as_ref() {
            let _ = tx.send(ev);
        }
    }
}

/// Handle do actor: clonável para múltiplos commands (a fila serializa).
#[derive(Clone)]
pub struct DeviceActor {
    tx: mpsc::Sender<Request>,
    handle: std::sync::Arc<std::sync::Mutex<Option<JoinHandle<()>>>>,
    /// Destinatário dos eventos `device://sync` (#164) — registrado pelo
    /// `run()` depois do build do Tauri. Sem sink os eventos são descartados
    /// (o estado continua consultável pelo `info`).
    sync_sink: std::sync::Arc<std::sync::Mutex<Option<mpsc::Sender<SyncEvent>>>>,
}

impl DeviceActor {
    /// Spawn do actor com o device.
    ///
    /// **O parametro e `T: DeviceBackend`, nao `MockDevice`.** Era essa
    /// assinatura concreta que impedia o app de falar com a pedaleira: a
    /// feature `real-device` do crate existia e nao mudava nada, porque
    /// nao havia como *passar* um `RealDevice` para ca. Com o generico, o
    /// build de campo passa `Box<dyn DeviceTransport>` e o build de
    /// desenvolvimento passa o `MockDevice` — o mesmo caminho do CLI
    /// (`--real`), sem duplicar a fila nem a FSM.
    ///
    /// `backend` e **declarado por quem monta** (o `run()` decide pelo
    /// build), porque a trait de bytes nao responde "sou mock?".
    ///
    /// Falha de pânico na thread (bug do core) vira erro de canal — nunca
    /// derruba o app.
    ///
    /// **(#150) no build padrão só os TESTES montam actor com device** (o
    /// runtime sobe `desligado`); no build de campo é o `RealDevice` que
    /// entra por aqui. Mesmo `cfg_attr` da variante `Mock`.
    #[cfg_attr(not(all(test, not(feature = "real-device"))), allow(dead_code))]
    pub fn spawn<T: DeviceBackend + 'static>(device: T, backend: Backend) -> Self {
        Self::spawn_com_motivo(device, backend, String::new())
    }

    /// Spawn do app **sem aparelho** (issue #150): estado [`Backend::Desligado`]
    /// com o motivo que a UI mostra no aviso de conexão. Nenhum dado é servido
    /// — `info` responde honesto e todo tráfego recusa.
    ///
    /// O build de campo usa o [`desligado_procurando`] (transporte que re-
    /// enumera portas); este aqui existe para o build SEM a feature.
    #[cfg_attr(feature = "real-device", allow(dead_code))]
    pub fn desligado(motivo: impl Into<String>) -> Self {
        Self::spawn_com_motivo(DesconectadoDevice, Backend::Desligado, motivo.into())
    }

    /// Spawn do build de campo **sem USB** (issue #150): o transporte procura
    /// o aparelho a cada `device_conectar` — o "Reconectar" da UI tem ação
    /// real (re-enumera as portas MIDI) em vez de desistir para sempre. Ao
    /// conectar, o estado do actor sobe para [`Backend::Real`].
    #[cfg(feature = "real-device")]
    pub fn desligado_procurando(motivo: impl Into<String>) -> Self {
        Self::spawn_com_motivo(
            ProcuraAparelho::default(),
            Backend::Desligado,
            motivo.into(),
        )
    }

    /// Corpo comum do spawn; `motivo` só é lido quando o backend é
    /// [`Backend::Desligado`].
    fn spawn_com_motivo<T: DeviceBackend + 'static>(
        device: T,
        backend: Backend,
        motivo: String,
    ) -> Self {
        Self::spawn_interno(device, backend, motivo, READBACK_DEBOUNCE, REFRESH_INTERVAL)
    }

    /// Spawn com cadências ENCURTADAS (#164) — para os testes do ciclo de
    /// convergência dormirem milissegundos, não segundos. Produção usa as
    /// constantes [`READBACK_DEBOUNCE`]/[`REFRESH_INTERVAL`].
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn spawn_com_cadencia<T: DeviceBackend + 'static>(
        device: T,
        backend: Backend,
        debounce: Duration,
        intervalo: Duration,
    ) -> Self {
        Self::spawn_interno(device, backend, String::new(), debounce, intervalo)
    }

    /// Corpo real do spawn (cadência parametrizada pelos wrappers acima).
    fn spawn_interno<T: DeviceBackend + 'static>(
        device: T,
        backend: Backend,
        motivo: String,
        debounce: Duration,
        intervalo: Duration,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<Request>();
        let sync_sink: std::sync::Arc<std::sync::Mutex<Option<mpsc::Sender<SyncEvent>>>> =
            std::sync::Arc::new(std::sync::Mutex::new(None));
        let sink = std::sync::Arc::clone(&sync_sink);
        let handle = std::thread::spawn(move || {
            // O transporte entra embrulhado no LoggingTransport desde o
            // inicio: e um decorador transparente (byte entra, byte sai,
            // `kind` intacto), entao o log pode ser LIGADO no meio da sessao
            // sem que o app precise reiniciar o device nem reconstruir a
            // fila. O `drain_inbox` do backend mora por tras do wrapper.
            let mut logged = LoggingTransport::new(device);
            // **O open que falha NÃO encerra a thread (#150).** O app segue no
            // estado `Desligado` (aviso + Reconectar na tela) em vez de virar
            // um actor morto cuja única resposta é "não está mais rodando".
            // O `device_conectar` refaz o open no MESMO transporte (ADR-4).
            let mut backend = backend;
            let mut transporte: Option<LoggingTransport<T>> = None;
            let mut session: Option<Session<LoggingTransport<T>>> = None;
            match logged.open() {
                Ok(()) => session = Some(Session::new(logged)),
                Err(e) => {
                    eprintln!(
                        "[device] open do backend {backend:?} falhou: {e} — o app segue sem aparelho (reconectar refaz o open)"
                    );
                    transporte = Some(logged);
                }
            }
            // O caminho ATIVO do log — estado do actor, e nao do transporte: o
            // `WireLogger` guarda o arquivo, nao o nome dele, e quem pergunta e
            // a tela (`device_log_path`).
            let mut log_path: Option<String> = None;
            // #155 §5.1/§5.2: (pp → nome) e (pp → BoardView) decodificados
            // das páginas do scan. Moram AQUI, fora da `Session`, de
            // propósito: `device_snapshot` reconstrói a Session por
            // `into_transport` a cada `Info`/`Library` — e era aqui que os
            // dados tinham de sobreviver (a spec manda "o actor grava em
            // cache", não a Session).
            let mut nomes: BTreeMap<u16, String> = BTreeMap::new();
            let mut boards: BTreeMap<u16, BoardView> = BTreeMap::new();
            // Convergência (#164): estado tipado + prazos de read-back.
            // Sem sessão (open falhou / build sem transporte) o estado já
            // nasce Offline com o motivo — o chip nunca mente "conectado".
            let mut sync = if session.is_some() {
                SyncStatus::Inativo
            } else {
                SyncStatus::Offline
            };
            let mut sync_detail = if session.is_some() {
                String::new()
            } else if motivo.is_empty() {
                "aparelho não conectou (abrir falhou)".to_string()
            } else {
                motivo.clone()
            };
            let mut debounced: Option<Instant> = None;
            let mut proximo_refresh: Option<Instant> = None;
            // Classificação do desfecho de uma ESCRITA no fio (#164): ok →
            // `Pendente` (o read-back do debounce confirma ou reconcilia);
            // `DeviceGone` → `Offline` + transporte guardado para o
            // `device_conectar`; outro erro → `Falhou` (sessão segue viva).
            macro_rules! marca {
                ($r:expr) => {
                    match &$r {
                        Ok(_) => {
                            sync = SyncStatus::Pendente;
                            sync_detail.clear();
                            debounced = Some(Instant::now() + debounce);
                            emite(
                                &sink,
                                SyncEvent {
                                    status: sync.as_str(),
                                    changed: false,
                                    detail: String::new(),
                                },
                            );
                        }
                        Err(e) => {
                            if matches!(e, gp100_core::ProtocolError::DeviceGone { .. }) {
                                sync = SyncStatus::Offline;
                                sync_detail = format!("aparelho desconectado: {e}");
                                // Sessão viva com transporte morto (ADR-4):
                                // retry tipado rápido e reconexão por cima.
                                debounced = None;
                                proximo_refresh = None;
                            } else {
                                sync = SyncStatus::Falhou;
                                sync_detail = e.to_string();
                            }
                            emite(
                                &sink,
                                SyncEvent {
                                    status: sync.as_str(),
                                    changed: false,
                                    detail: sync_detail.clone(),
                                },
                            );
                        }
                    }
                };
            }
            // Fila com RELOGIO (#164): os prazos do read-back (debounce da
            // rajada) e do refresh periódico dividem o `recv` com a fila;
            // sem prazo o `recv` segue bloqueante como antes (nenhum poll
            // ocioso, nenhum sleep — a thread só acorda para fila ou prazo).
            loop {
                let agora = Instant::now();
                let debounce_venceu = debounced.is_some_and(|t| t <= agora);
                let mut refresh_venceu = proximo_refresh.is_some_and(|t| t <= agora);
                // Rajada em curso: o refresh espera a rajada fechar — um
                // read-back cobre os dois (9 páginas lidas uma vez só).
                if refresh_venceu && !debounce_venceu && debounced.is_some() {
                    proximo_refresh = Some(agora + intervalo);
                    refresh_venceu = false;
                }
                if debounce_venceu || refresh_venceu {
                    debounced = None;
                    if refresh_venceu {
                        proximo_refresh = Some(agora + intervalo);
                    }
                    // READ-BACK (#164): relê as 9 páginas do preset
                    // corrente pelo caminho do BOOT (`reload_paginas_correntes`:
                    // open → pág0 + PG 0..7 → págs 1..8, §13.10 — tudo
                    // `WireKind::Read`, NENHUMA escrita nova) e reconcilia o
                    // cache do actor: o aparelho vence, sempre.
                    let r: Result<bool, FalhaLeitura> = (|| {
                        let s = session
                            .as_mut()
                            .ok_or_else(|| FalhaLeitura::Outro("sem sessão com aparelho".into()))?;
                        let pp = s.current_pp();
                        let pags = s.reload_paginas_correntes().map_err(|e| match e {
                            gp100_core::ProtocolError::DeviceGone { why } => {
                                FalhaLeitura::Caiu(why)
                            }
                            outro => FalhaLeitura::Outro(outro.to_string()),
                        })?;
                        let dec = gp100_core::preset_pages::decode(&pags)
                            .map_err(|e| FalhaLeitura::Outro(e.to_string()))?;
                        let dict = Dictionary::from_json(gp100_core::model::DICTIONARY_JSON)
                            .map_err(|e| FalhaLeitura::Outro(e.to_string()))?;
                        let off = gp100_core::preset_pages::Offsets::carregado()
                            .map_err(|e| FalhaLeitura::Outro(e.to_string()))?;
                        let novo = dec
                            .slots(pp, &dict, &off)
                            .map_err(|e| FalhaLeitura::Outro(e.to_string()))?;
                        if let Ok(nome) = dec.nome() {
                            nomes.insert(pp, nome.to_string());
                        }
                        let changed = boards.get(&pp).is_none_or(|b| b != &novo);
                        boards.insert(pp, novo);
                        Ok(changed)
                    })();
                    let ev = match r {
                        Ok(changed) => {
                            sync = SyncStatus::Sincronizado;
                            sync_detail.clear();
                            SyncEvent {
                                status: sync.as_str(),
                                changed,
                                detail: String::new(),
                            }
                        }
                        Err(FalhaLeitura::Caiu(why)) => {
                            sync = SyncStatus::Offline;
                            sync_detail = format!("aparelho desconectado: {why}");
                            // A sessão FICA viva com o transporte morto (ADR-4:
                            // retry devolve DeviceGone tipado rápido, e o
                            // `device_conectar` reabre por cima — contrato do
                            // `device_morrendo_no_meio_da_sessao`).
                            debounced = None;
                            proximo_refresh = None;
                            SyncEvent {
                                status: sync.as_str(),
                                changed: false,
                                detail: sync_detail.clone(),
                            }
                        }
                        Err(FalhaLeitura::Outro(msg)) => {
                            sync = SyncStatus::Falhou;
                            sync_detail = msg.clone();
                            SyncEvent {
                                status: sync.as_str(),
                                changed: false,
                                detail: msg,
                            }
                        }
                    };
                    emite(&sink, ev);
                    continue;
                }
                // Prazo mais próximo (ou fila, quando não há prazo).
                let espera = [debounced, proximo_refresh]
                    .into_iter()
                    .flatten()
                    .map(|t| t.saturating_duration_since(agora))
                    .min();
                let req = match espera {
                    None => match rx.recv() {
                        Ok(r) => r,
                        Err(_) => break,
                    },
                    Some(d) => match rx.recv_timeout(d) {
                        Ok(r) => r,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    },
                };
                match req {
                    Request::SyncNow { reply } => {
                        // Só adianta o prazo no RELÓGIO da fila: o braço do
                        // read-back roda no próximo passo do loop, com os
                        // MESMOS caminhos de falha (DeviceGone → Offline,
                        // decode → Failed) — sem caminho paralelo de leitura.
                        debounced = Some(Instant::now());
                        let _ = reply.send(Ok(()));
                    }
                    Request::Info { reply } => {
                        let mut snap = device_snapshot(&mut session, backend, &nomes);
                        if backend == Backend::Desligado {
                            snap.detail = motivo.clone();
                        }
                        snap.sync = sync.as_str();
                        snap.sync_detail = sync_detail.clone();
                        let _ = reply.send(Ok(snap));
                    }
                    Request::Conectar { reply } => {
                        // Reconexão no MESMO transporte (ADR-4): com sessão
                        // viva o open é refeito por cima (device sumiu e
                        // voltou — o backlog D7 é descartado, como no remount
                        // do Info); sem sessão é o PRIMEIRO open (app que
                        // subiu sem aparelho). No build de campo o
                        // `ProcuraAparelho` re-enumera as portas — e, achando
                        // o aparelho, o estado sobe de `Desligado` para `Real`
                        // (o open do `RealDevice` SÓ sucede com o aparelho
                        // presente, então a promoção é evidência, não chute).
                        let mut t = match session.take() {
                            Some(s) => s.into_transport(),
                            None => match transporte.take() {
                                Some(t) => t,
                                None => {
                                    let _ =
                                        reply.send(Err("sem transporte para reconectar".into()));
                                    continue;
                                }
                            },
                        };
                        match t.open() {
                            Ok(()) => {
                                if backend == Backend::Desligado {
                                    backend = Backend::Real;
                                    eprintln!("[device] aparelho conectado pelo device_conectar");
                                }
                                session = Some(Session::new(t));
                                // #164: reconectou mas ainda não leu nada —
                                // `Inativo` (nunca "sincronizado" sem prova);
                                // o read-back do debounce confirma em seguida.
                                sync = SyncStatus::Inativo;
                                sync_detail.clear();
                                debounced = Some(Instant::now() + debounce);
                                emite(
                                    &sink,
                                    SyncEvent {
                                        status: sync.as_str(),
                                        changed: false,
                                        detail: String::new(),
                                    },
                                );
                                let _ = reply.send(Ok(()));
                            }
                            Err(e) => {
                                transporte = Some(t);
                                sync = SyncStatus::Offline;
                                sync_detail = format!("aparelho não conectou: {e}");
                                debounced = None;
                                proximo_refresh = None;
                                emite(
                                    &sink,
                                    SyncEvent {
                                        status: sync.as_str(),
                                        changed: false,
                                        detail: sync_detail.clone(),
                                    },
                                );
                                let _ = reply.send(Err(format!("aparelho não conectou: {e}")));
                            }
                        }
                    }
                    Request::Board { pp, reply } => {
                        // #155 §5.2: o palco vem do CACHE do scan, nunca de
                        // um select novo — ler OUTRO pp não pode mudar o
                        // preset que o pedal está mostrando.
                        //
                        // Sem cache (boot não rodou, ou pp fora do
                        // inventário) cai na projeção pura do `all.prst`, o
                        // caminho de sempre. Com cache mas sem decode, ERRO:
                        // spec §6 — um `BoardView` com chute é pior que erro,
                        // porque o palco desenha e o usuário acredita.
                        let alvo = pp.or_else(|| session.as_ref().map(|s| s.current_pp()));
                        let r = (|| -> Result<BoardView, String> {
                            // **O palco vem do CACHE do scan (#155 §5.2)** — as
                            // páginas que o boot já leu, decodificadas. Nunca
                            // de um `select` novo: ler outro pp não pode mudar
                            // o preset que o pedal está mostrando.
                            if let Some(a) = alvo {
                                if let Some(b) = boards.get(&a) {
                                    return Ok(b.clone());
                                }
                                if let Some(pags) = session.as_ref().and_then(|s| s.preset_state(a))
                                {
                                    let dict =
                                        Dictionary::from_json(gp100_core::model::DICTIONARY_JSON)
                                            .map_err(|e| e.to_string())?;
                                    let off = gp100_core::preset_pages::Offsets::carregado()
                                        .map_err(|e| e.to_string())?;
                                    let dec = gp100_core::preset_pages::decode(pags)
                                        .map_err(|e| e.to_string())?;
                                    let b = dec.slots(a, &dict, &off).map_err(|e| e.to_string())?;
                                    boards.insert(a, b.clone());
                                    return Ok(b);
                                }
                            }
                            // Sem cache no APARELHO: erro honesto (#150 + spec
                            // §6) — projetar o `all.prst` no palco do aparelho
                            // desenharia uma cadeia que não está gravada nele.
                            if backend == Backend::Real {
                                return Err(
                                    "páginas deste pp não foram lidas no boot — refaça o boot \
                                     (o palco do aparelho vem do cache, nunca do artefato)"
                                        .into(),
                                );
                            }
                            // Mock: projeção PURA do artefato (doc embedado +
                            // dicionário) — não toca a Session nem o fio.
                            let doc = embedded_document().map_err(|e| e.to_string())?;
                            let dict = Dictionary::from_json(gp100_core::model::DICTIONARY_JSON)
                                .map_err(|e| e.to_string())?;
                            board_view_for(&doc, &dict, pp).map_err(|e| e.to_string())
                        })();
                        let _ = reply.send(r);
                    }
                    Request::Library { reply } => {
                        let r = (|| -> Result<PresetLibrary, String> {
                            // App desligado (#150): a biblioteca NÃO é servida
                            // do artefato — sem aparelho, o front mostra o aviso
                            // de conexão, não uma lista falsa.
                            if session.is_none() {
                                return Err("session do actor ausente".into());
                            }
                            // **No APARELHO a biblioteca é o que ele tem (#150).**
                            // O inventário é o MEDIDO (captura S1 — ADR-12) e o
                            // NOME vem do CACHE do scan (#155 §5.1 — pg0
                            // decodificada, custo zero no fio). Sem cache, vazio:
                            // honestidade — nunca o dicionário de fábrica
                            // emprestado do all.prst, que não diz o que está
                            // gravado no slot do dono.
                            if backend == Backend::Real {
                                let state = device_snapshot(&mut session, backend, &nomes);
                                return Ok(PresetLibrary {
                                    entries: inventario_do_aparelho()
                                        .into_iter()
                                        .map(|pp| PresetEntry {
                                            pp,
                                            name: nomes.get(&pp).cloned().unwrap_or_default(),
                                            pp_type_name: String::new(),
                                        })
                                        .collect(),
                                    current_pp: state.current_pp,
                                });
                            }
                            let doc = embedded_document().map_err(|e| e.to_string())?;
                            let state = device_snapshot(&mut session, backend, &nomes); // corrente
                            Ok(PresetLibrary {
                                entries: preset_list(&doc)
                                    .into_iter()
                                    .map(|e| PresetEntry {
                                        pp: e.pp,
                                        // #155 §5.1: o nome real do fio
                                        // (pg0 decodificada) no lugar do do
                                        // arquivo — o que muda é o user
                                        // patch renomeado no aparelho.
                                        name: nomes.get(&e.pp).cloned().unwrap_or(e.name),
                                        pp_type_name: e.pp_type_name,
                                    })
                                    .collect(),
                                current_pp: state.current_pp,
                            })
                        })();
                        let _ = reply.send(r);
                    }
                    Request::Select { pp, reply } => match session.as_mut() {
                        Some(s) => {
                            let r = s.select_preset(pp);
                            marca!(&r);
                            let _ = reply.send(r.map_err(|e| e.to_string()));
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::SetParam {
                        slot,
                        code,
                        ctrl,
                        value,
                        reply,
                    } => match session.as_mut() {
                        Some(s) => {
                            let r = s.set_param(slot, code, ctrl, value);
                            // Escrita aceita sem resposta do fio (D4): até o
                            // read-back, o valor é SÓ do app — o estado fica
                            // `Pendente` e a UI pode reconciliar (#164).
                            marca!(&r);
                            let _ = reply.send(r.map_err(|e| e.to_string()));
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::Globals { reply } => match session.as_mut() {
                        Some(s) => {
                            let r = s.read_globals().map_err(|e| e.to_string());
                            let _ = reply.send(r);
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::SetGlobal {
                        addr,
                        payload,
                        reply,
                    } => match session.as_mut() {
                        Some(s) => {
                            let r = s.write_register(addr, &payload).map_err(|e| e.to_string());
                            let _ = reply.send(r);
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::Boot { progress, reply } => match session.as_mut() {
                        Some(s) => {
                            // Callback roda NA THREAD do actor: fecha em
                            // volta do sender e repassa por coerção para
                            // `&mut dyn FnMut` (canal é o padrão seguro —
                            // callback emprestado não atravessa a fila).
                            let r = match progress {
                                Some(tx) => {
                                    let mut cb = move |p: BootProgress| {
                                        let _ = tx.send(p);
                                    };
                                    s.boot_with_progress(Some(&mut cb))
                                }
                                None => s.boot_with_progress(None),
                            };
                            // #155 §5.1: com o scan completo, decodifica a
                            // pg0 de cada pp e grava (pp → nome). É custo
                            // zero no fio — as páginas já vieram no scan.
                            // O que não decodificar simplesmente não entra
                            // (o `all.prst` continua de reserva).
                            //
                            // **§5.2 na MESMA passada:** o BoardView de cada
                            // pp também é gravado AQUI, no actor. O
                            // `device_snapshot` remonta a Session por
                            // `into_transport` a cada `Info`/`Library` — e
                            // descarta o cache de páginas DELA. Com o palco
                            // dependendo só da Session, o PRIMEIRO desses
                            // calls de montagem destruía as páginas do scan e
                            // o `device_board` seguinte respondia "páginas
                            // deste pp não foram lidas no boot": o palco
                            // morria segundos depois de um boot perfeito
                            // (2297/198/198 — wire log de campo 12/10).
                            if r.is_ok() {
                                let dict =
                                    Dictionary::from_json(gp100_core::model::DICTIONARY_JSON).ok();
                                let off = gp100_core::preset_pages::Offsets::carregado().ok();
                                for pp in s.cached_pps() {
                                    if let Some(pags) = s.preset_state(pp) {
                                        if let Ok(pg) = gp100_core::preset_pages::decode(pags) {
                                            if let Ok(nome) = pg.nome() {
                                                nomes.insert(pp, nome.to_string());
                                            }
                                        }
                                        if let (Some(dict), Some(off)) = (&dict, &off) {
                                            if let Ok(dec) = gp100_core::preset_pages::decode(pags)
                                            {
                                                if let Ok(b) = dec.slots(pp, dict, off) {
                                                    boards.insert(pp, b);
                                                }
                                            }
                                        }
                                    }
                                }
                                // #164: o boot LEU tudo — sincronizado de
                                // direito, e o refresh periódico passa a
                                // armar (10 s; poll é a única janela em que
                                // a mudança do PEDAL aparece).
                                sync = SyncStatus::Sincronizado;
                                sync_detail.clear();
                                proximo_refresh = Some(Instant::now() + intervalo);
                                emite(
                                    &sink,
                                    SyncEvent {
                                        status: sync.as_str(),
                                        changed: false,
                                        detail: String::new(),
                                    },
                                );
                            } else {
                                marca!(&r);
                            }
                            let _ = reply.send(r.map_err(|e| e.to_string()));
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::ListIrs { reply } => match session.as_mut() {
                        Some(s) => {
                            let r = s
                                .list_user_irs()
                                .map(|t| t.slots)
                                .map_err(|e| e.to_string());
                            let _ = reply.send(r);
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::DrainPushes { reply } => match session.take() {
                        Some(s) => {
                            let mut transport = s.into_transport();
                            let hexes: Vec<String> = transport
                                .drain_inbox()
                                .iter()
                                .map(|m| m.iter().map(|b| format!("{b:02X}")).collect())
                                .collect();
                            session = Some(Session::new(transport));
                            let _ = reply.send(Ok(hexes));
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::UploadSnapTone { slot, model, reply } => match session.as_mut() {
                        Some(s) => {
                            let r = s.upload_snap_tone(slot, &model).map_err(|e| e.to_string());
                            let _ = reply.send(r);
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::UploadIr { slot, blob, reply } => match session.as_mut() {
                        Some(s) => {
                            let r = s.upload_ir(slot, &blob).map_err(|e| e.to_string());
                            let _ = reply.send(r);
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::Save {
                        pp,
                        pp_type,
                        name,
                        reply,
                    } => match session.as_mut() {
                        Some(s) => {
                            let r = s.save_preset(pp, pp_type, &name);
                            marca!(&r);
                            let _ = reply.send(r.map_err(|e| e.to_string()));
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::Dump { pp, reply } => match session.as_mut() {
                        Some(s) => {
                            let r = (|| -> Result<DumpReport, String> {
                                let meta6 = s.state_page(0).map_err(|e| e.to_string())?;
                                let mut pages = Vec::with_capacity(9);
                                for p in 1..=8u8 {
                                    let pg = s.state_page(p).map_err(|e| e.to_string())?;
                                    pages.push(hex(&pg.raw));
                                }
                                Ok(DumpReport {
                                    pp,
                                    meta6: hex(&meta6.raw),
                                    pages,
                                })
                            })();
                            let _ = reply.send(r);
                        }
                        None => {
                            let _ = reply.send(Err("session do actor ausente".into()));
                        }
                    },
                    Request::LogSession { path, reply } => {
                        let Some(s) = session.take() else {
                            let _ = reply.send(Err("session do actor ausente".into()));
                            continue;
                        };
                        let mut t = s.into_transport();
                        let r = t.enable_log(std::path::Path::new(&path));
                        session = Some(Session::new(t));
                        // So um log que ABRIU passa a ser o ativo: um caminho
                        // que nao pode ser criado nao pode aparecer na tela como
                        // "gravando".
                        if r.is_ok() {
                            log_path = Some(path);
                        }
                        let _ = reply.send(r.map(|()| true));
                    }
                    Request::LogStop { reply } => {
                        let Some(s) = session.take() else {
                            let _ = reply.send(Err("session do actor ausente".into()));
                            continue;
                        };
                        let mut t = s.into_transport();
                        t.logger = None;
                        session = Some(Session::new(t));
                        log_path = None;
                        let _ = reply.send(Ok(true));
                    }
                    Request::LogPath { reply } => {
                        // Clonado: o estado continua no actor, a resposta e do
                        // chamador (a tela precisa do nome, nao da posse dele).
                        let _ = reply.send(log_path.clone());
                    }
                    Request::Shutdown => break,
                }
            }
        });
        Self {
            tx,
            handle: std::sync::Arc::new(std::sync::Mutex::new(Some(handle))),
            sync_sink,
        }
    }

    /// Registra o destinatário dos eventos `device://sync` (#164). Quem
    /// registra é o `run()` (reemite para o front); sem sink os eventos são
    /// descartados e o estado continua consultável pelo `info`.
    pub fn set_sync_sink(&self, sink: mpsc::Sender<SyncEvent>) {
        if let Ok(mut g) = self.sync_sink.lock() {
            *g = Some(sink);
        }
    }

    /// Snapshot do estado (o que o app **pode afirmar** sobre o device).
    ///
    /// No mock: nome do pp, ppType e CRC de fabrica sao leituras locais.
    /// No aparelho real: `current_pp` e a tabela de IRs sao **lidos do fio**,
    /// e os campos locais ficam no neutro — a UI mostra "desconhecido", nao
    /// um zero que parece dado.
    ///
    /// # Erros
    /// String de erro se a thread do actor morreu (pânico do core).
    pub fn info(&self) -> Result<DeviceSnapshot, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Info { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Info".to_string())?
    }

    /// Boot completo (§13.10) com progresso opcional pelo canal `progress`
    /// (1 [`BootProgress`] por transação). Bloqueia até o fim do script
    /// (2297 transações no inventário default do MOCK) — o command que chama é
    /// síncrono e roda fora da main thread (ADR-3).
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string (Timeout D6,
    /// InvalidShape D5) ou morte da thread do actor.
    pub fn boot(&self, progress: Option<mpsc::Sender<BootProgress>>) -> Result<BootReport, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Boot {
                progress,
                reply: tx,
            })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Boot".to_string())?
    }

    /// Board do preset (slots/arquétipos/knobs) — projeção pura no core.
    /// `pp = None` = corrente.
    ///
    /// # Erros
    /// String de erro de parse/projeção ou morte da thread do actor.
    pub fn board(&self, pp: Option<u16>) -> Result<BoardView, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Board { pp, reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Board")?
    }

    /// Biblioteca de presets (flight case) + pp corrente.
    ///
    /// # Erros
    /// String de erro de parse ou morte da thread do actor.
    pub fn library(&self) -> Result<PresetLibrary, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Library { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Library")?
    }

    /// Reconecta o transporte do device (issue #150 — command
    /// `device_conectar`): refaz o `open` no MESMO objeto; no build de campo
    /// o `ProcuraAparelho` re-enumera as portas MIDI.
    ///
    /// # Erros
    /// String com o motivo (aparelho ausente) ou morte da thread do actor.
    pub fn conectar(&self) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Conectar { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Conectar")?
    }

    /// Select de preset (§13.10 — write + meta6). Bloqueia até a FSM
    /// completar a transação (mock: 1 ida e volta).
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string.
    pub fn select_preset(&self, pp: u16) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Select { pp, reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Select")?
    }

    /// Set-param do knob (§13.11, fire-and-forget D4). O mock valida o
    /// shape contra o golden e guarda o valor no estado.
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string.
    pub fn set_param(&self, slot: u8, code: u32, ctrl: u8, value: f32) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::SetParam {
                slot,
                code,
                ctrl,
                value,
                reply: tx,
            })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no SetParam")?
    }

    /// Agenda um read-back IMEDIATO do preset corrente (#164 — command
    /// `device_sync_now`, a ação "Reler agora" do chip de sincronização).
    /// Resolve quando o prazo foi agendado; o RESULTADO da leitura (sync/
    /// failed/offline + `changed`) chega pelo evento `device://sync`.
    pub fn sync_now(&self) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::SyncNow { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no SyncNow")?
    }

    /// Registradores globais (§13.13): o bloco medido lido DO device, com o
    /// payload crU por registrador — o decode medido (nibble-par) é do
    /// chamador (`commands::device_globals`).
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string (Timeout D6
    /// se um registrador não responde; InvalidShape para forma fora do
    /// medido) ou morte da thread do actor.
    pub fn globals(&self) -> Result<Vec<GlobalBruto>, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Globals { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Globals")?
    }

    /// Write num registrador global (§13.13.1) — payload pronto (o encode
    /// nibble-par é do chamador). Sem write-verified o TRANSPORTE recusa
    /// antes do driver (ADR-5) e o erro sobe aqui.
    pub fn set_global(&self, addr: [u8; 4], payload: Vec<u8>) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::SetGlobal {
                addr,
                payload,
                reply: tx,
            })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no SetGlobal")?
    }

    /// Tabela dos 20 User IRs (§13.12). Bloqueia até o actor completar as
    /// 20 leituras de tabela (mock: responde por página com eco — D1).
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string.
    pub fn list_user_irs(&self) -> Result<Vec<(u8, String)>, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::ListIrs { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no ListIrs")?
    }

    /// Drena pushes não consumidos do device como hex cru (F0…F7). O
    /// command `pending_pushes` devolve a lista; o `device_boot` reemite
    /// como evento `device://push` para o log da UI.
    ///
    /// # Erros
    /// String de erro se a thread do actor morreu.
    pub fn drain_pushes(&self) -> Result<Vec<String>, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::DrainPushes { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no DrainPushes")?
    }

    /// Envia um modelo de SnapTone ao device (§5).
    ///
    /// Bloqueia até a FSM fechar TODOS os blocos esperando o ACK de 16B de
    /// cada um — dezenas de segundos para um `.clo` de ~2,7 KB, porque o
    /// settle do §4 é 250 ms por operação. É o comportamento correto (§4:
    /// 0,15 s corrompe o stream e loop tight trava a pedaleira) e por isso
    /// quem chama precisa mostrar "enviando…" e esperar o `invoke`.
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string — `Timeout`
    /// (D6) quando um ACK não chega na janela, `InvalidShape` (D5) quando o
    /// ACK vem com outro tamanho ou o slot é inválido.
    pub fn upload_snap_tone(&self, slot: u8, model: &[u8]) -> Result<SnapToneUploadReport, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::UploadSnapTone {
                slot,
                model: model.to_vec(),
                reply: tx,
            })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv()
            .map_err(|_| "actor morreu no upload de SnapTone".to_string())?
    }

    /// Envia um IR de usuário ao aparelho (§13.7): `ir_begin` + chunks de 15B
    /// com ACK por chunk, o último duplicado como marcador de fim.
    ///
    /// **BLOQUEIA por muito tempo.** Um IR de 300 KB são ~20.000 chunks, e cada
    /// um espera o próprio ACK (timeout ADR-3). É a transação mais longa do
    /// app — o comando do Tauri roda fora da main thread (ADR-3), então a
    /// janela continua desenhando enquanto isso.
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string — `Timeout`
    /// (D6) quando um ACK não chega na janela, `InvalidShape` (D5) quando o
    /// blob não é múltiplo de 15B ou o slot está fora de 0..=19, e
    /// `UnexpectedAck` quando o ACK não é o do chunk que foi enviado.
    pub fn upload_ir(&self, slot: u8, blob: &[u8]) -> Result<IrUploadReport, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::UploadIr {
                slot,
                blob: blob.to_vec(),
                reply: tx,
            })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv()
            .map_err(|_| "actor morreu no upload de IR".to_string())?
    }

    /// **Salva o preset no aparelho** (§13.12): 5 writes de meta + 4 ops
    /// (D3). O `set-param` é fire-and-forget, então **sem isto a mudança
    /// morre com a sessão** — é a operação que persiste o que se mexeu.
    ///
    /// Substitui o `save` do `gp100-cli`, que era o único caminho para
    /// isso antes.
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string — nome não
    /// ASCII ou `TransportError::WriteBlocked` no build de leitura (ADR-5).
    pub fn save_preset(&self, pp: u16, pp_type: u16, name: &str) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Save {
                pp,
                pp_type,
                name: name.to_string(),
                reply: tx,
            })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Save".to_string())?
    }

    /// **Lê o preset do aparelho** (§13.9): meta6 + as páginas de estado,
    /// em hex cru. Substitui o `dump-preset` do `gp100-cli`.
    ///
    /// # Erros
    /// [`ProtocolError`](gp100_core::ProtocolError) como string (timeout,
    /// shape inesperado) ou morte da thread do actor.
    pub fn dump_preset(&self, pp: u16) -> Result<DumpReport, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::Dump { pp, reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no Dump".to_string())?
    }

    /// **Liga o log de fio da sessao** (schema P4) — o MESMO formato que o
    /// `--log` do CLI e que os juiz (`h1_compare.py`/`h2_compare.py`) leem.
    ///
    /// E o que fecha o ciclo de campo pelo app: sessao no editor, exporta o
    /// `.jsonl`, roda o juiz. Liga e desliga **sem reiniciar o device**.
    ///
    /// # Erros
    /// String com o erro do SO se o arquivo nao puder ser criado.
    pub fn log_session(&self, path: &str) -> Result<bool, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::LogSession {
                path: path.to_string(),
                reply: tx,
            })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv()
            .map_err(|_| "actor morreu no LogSession".to_string())?
    }

    /// Desliga o log de fio (a sessao segue; so o log para).
    ///
    /// # Erros
    /// String de erro se a thread do actor morreu.
    pub fn log_stop(&self) -> Result<bool, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::LogStop { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv()
            .map_err(|_| "actor morreu no LogStop".to_string())?
    }

    /// Em que arquivo o log de fio esta gravando agora (`None` = nenhum).
    ///
    /// E o que deixa a TELA dizer o arquivo sem depender do `stderr` do
    /// processo: desde a #130 o build de campo liga o log sozinho na abertura, e
    /// o caminho e escolhido pelo `run()` (diretorio de dados + carimbo), nao
    /// pelo front.
    ///
    /// # Erros
    /// String de erro se a thread do actor morreu.
    pub fn log_path(&self) -> Result<Option<String>, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Request::LogPath { reply: tx })
            .map_err(|_| "actor de device não está mais rodando".to_string())?;
        rx.recv().map_err(|_| "actor morreu no LogPath".to_string())
    }

    /// Envia Shutdown e agrega a thread (idempotente).
    pub fn shutdown(&self) {
        let _ = self.tx.send(Request::Shutdown);
        if let Ok(mut slot) = self.handle.lock() {
            if let Some(h) = slot.take() {
                let _ = h.join();
            }
        }
    }
}

// SEM impl Drop: o handle é Clone e qualquer clone caindo fora de escopo
// NÃO pode derrubar o actor compartilhado (bug clássico de shutdown por
// Drop em handle clonável — pegaria a thread de teste no meio do uso). O
// shutdown é EXPLÍCITO: o app o tem pelo ciclo da AppState (processo);
// os testes chamam no fim.

#[cfg(test)]
mod tests {
    use super::*;
    use gp100_core::transport::mock::MockFault;
    use gp100_core::transport::{TransportError, WireKind};

    /// Boot completo via actor: 2297 transações — o inventário default do
    /// MOCK é `0..198` (#132/ADR-12), que NÃO contém o 0x0100 e por isso não
    /// paga o quirk §13.4. O APARELHO é 2299 e quem o prova é o
    /// `tests/pp_gate.rs` do core.
    ///
    /// O resultado chega ao chamador pelo canal de resposta.
    #[test]
    fn boot_via_actor_completa_e_responde() {
        let mock = MockDevice::new().expect("mock montado");
        let actor = DeviceActor::spawn(mock, Backend::Mock);
        let report = actor.boot(None).expect("boot contra o mock via actor");
        // 2297 = o script do MOCK: o inventário default dele é `0..198`
        // (#132/ADR-12), que NÃO contém o 0x0100 e não paga o quirk §13.4.
        // O APARELHO é 2299 (a prova C do `validate_golden.py`, 2299/2299)
        // e quem o prova é o `tests/pp_gate.rs` do core. Este crate fica
        // FORA do workspace (`exclude` no Cargo.toml raiz), então
        // `cargo test --workspace` não o alcança — rode-o aqui.
        assert_eq!(report.transactions, 2297);
        actor.shutdown();
    }

    /// O progresso atravessa a fronteira do actor PELO CANAL: cada
    /// transação produz 1 beat no `mpsc` (o command vai reemitir como
    /// evento `device://progress`). Beats monótonos até o total.
    #[test]
    fn progresso_atravessa_o_actor_pelo_canal() {
        let mock = MockDevice::new().expect("mock montado");
        let actor = DeviceActor::spawn(mock, Backend::Mock);
        let (ptx, prx) = mpsc::channel::<BootProgress>();
        let report = actor.boot(Some(ptx)).expect("boot com progresso");
        let beats: Vec<BootProgress> = prx.try_iter().collect();
        assert_eq!(beats.len(), report.transactions);
        assert_eq!(beats.last().map(|p| p.done), Some(report.transactions));
        assert!(beats.windows(2).all(|w| w[0].done + 1 == w[1].done));
        actor.shutdown();
    }

    /// Info sobe pelo actor (mesmo estado que o CLI imprime; R1 nos DTOs) —
    /// e DEPOIS do info a Session continua utilizável (boot funciona).
    #[test]
    fn info_via_actor_deriva_do_mock() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        let st = actor.info().expect("snapshot");
        assert_eq!(st.preset_count, 99);
        assert_eq!(st.current_name, "It's GP100");
        let report = actor.boot(None).expect("boot após info");
        assert_eq!(report.transactions, 2297);
        actor.shutdown();
    }

    /// **#161 — o relatório HONESTO atravessa o actor com inventário
    /// VARIANTE.** O caminho scan → `BootReport` → DTO é o que a UI consome
    /// (o `commands.rs` copia os 3 campos para o `BootReportDto`): aqui se
    /// prova o boundary do APP com um device saudável que tem um slot sem
    /// nome gravado. O relatório que sai daqui — e que a UI recebe — tem de
    /// ser `198/197`: inventário lido por inteiro, nome só onde o fio
    /// entregou. Se o relatório mentisse 198/198 (contando slot por
    /// inventário em vez de por nome lido), o gate da #161 não teria o que
    /// barrar e a casca montaria sobre um aparelho pela metade.
    #[test]
    fn boot_via_actor_entrega_relatorio_honesto_com_slot_sem_nome() {
        let mock = MockDevice::new()
            .expect("mock montado")
            .with_slot_sem_nome(0x0042);
        let actor = DeviceActor::spawn(mock, Backend::Mock);
        let report = actor.boot(None).expect("boot com slot sem nome");
        assert_eq!(
            report.presets, 198,
            "inventário lido por inteiro — o slot sem nome existe no fio"
        );
        assert_eq!(
            report.names, 197,
            "197/198 honesto pelo actor: o DTO que a UI recebe carrega a medição do scan"
        );
        actor.shutdown();
    }

    /// **O log de fio do app grava o fio de verdade.** É o que faz um incidente
    /// de campo ser ATRIBUÍVEL em vez de dedutivo: o arquivo tem o frame que
    /// saiu, com `func`/`addr`/`data`. No incidente de 06/10 não havia log
    /// nenhum, e a causa teve de ser lida do código.
    ///
    /// O `select` é o comando daquele incidente (`11/13010000` com o `pp`): ele
    /// atravessa o actor com o logger ligado e tem de aparecer no arquivo — não
    /// basta o `WireLogger` ter teste próprio no core, porque o que se prova
    /// aqui é que o caminho do APP (actor → `LoggingTransport`) o aciona.
    #[test]
    fn log_de_fio_do_actor_registra_o_select_que_atravessa() {
        let dir = std::env::temp_dir().join("gp100-actor-wire-log");
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("sessao.jsonl");
        let _ = std::fs::remove_file(&path);

        let actor = DeviceActor::spawn(MockDevice::new().expect("mock montado"), Backend::Mock);
        actor
            .log_session(path.to_str().expect("caminho utf8"))
            .expect("log ligado");
        actor.select_preset(0).expect("select no mock");
        actor.shutdown();

        let texto = std::fs::read_to_string(&path).expect("o log tem de existir");
        let linhas: Vec<&str> = texto.lines().filter(|l| !l.is_empty()).collect();
        assert!(!linhas.is_empty(), "o log nao pode sair vazio");

        // Schema P4 — as chaves que `h1_compare.py`/`h2_compare.py` leem.
        for l in &linhas {
            for chave in [
                "\"s\":",
                "\"t\":",
                "\"dir\":",
                "\"func\":",
                "\"addr\":",
                "\"data\":",
            ] {
                assert!(l.contains(chave), "falta {chave} em: {l}");
            }
        }

        // O frame do incidente: o select do preset 0 (`11/13010000`).
        let select = linhas
            .iter()
            .find(|l| l.contains("\"dir\":\"out\"") && l.contains("\"addr\":\"13010000\""))
            .unwrap_or_else(|| panic!("o select nao foi registrado: {linhas:?}"));
        assert!(select.contains("\"func\":\"11\""), "{select}");
    }

    /// **O caminho do log é do BACKEND, e a tela pergunta por ele.** Sem isto o
    /// painel mostraria "nenhum log" enquanto a sessão de campo já está em disco
    /// (#130 liga sozinho) — e o botão ofereceria "gravar" por cima do arquivo
    /// que o próprio app abriu.
    #[test]
    fn log_path_reflete_o_log_ligado_e_parado() {
        let dir = std::env::temp_dir().join("gp100-actor-log-path");
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("sessao.jsonl");
        let _ = std::fs::remove_file(&path);

        let actor = DeviceActor::spawn(MockDevice::new().expect("mock montado"), Backend::Mock);
        assert_eq!(actor.log_path().expect("consulta"), None, "nasce sem log");

        actor
            .log_session(path.to_str().expect("caminho utf8"))
            .expect("log ligado");
        assert_eq!(
            actor.log_path().expect("consulta"),
            Some(path.to_string_lossy().to_string()),
            "ligado: o caminho e o que foi pedido"
        );

        actor.log_stop().expect("log parado");
        assert_eq!(
            actor.log_path().expect("consulta"),
            None,
            "parado volta a nada"
        );
        actor.shutdown();
    }

    /// **App sem aparelho responde honesto (#150).** `backend: "none"` com o
    /// motivo; a biblioteca NÃO é inventada e o reconectar falha com o motivo
    /// — o oposto do mock fingindo ser aparelho.
    #[test]
    fn app_sem_aparelho_e_estado_honesto() {
        let actor = DeviceActor::desligado("aparelho não conectado via USB");
        let snap = actor.info().expect("info responde mesmo desligado");
        assert_eq!(snap.backend, "none");
        assert_eq!(snap.preset_count, 0);
        assert_eq!(snap.detail, "aparelho não conectado via USB");
        assert!(
            actor.library().is_err(),
            "sem aparelho, a biblioteca não é inventada"
        );
        let e = actor.conectar().expect_err("reconectar sem aparelho falha");
        assert!(e.contains("aparelho não conectou"), "motivo legível: {e}");
        actor.shutdown();
    }

    /// Reconectar com a sessão viva é no-op: o open é refeito por cima do
    /// transporte que já responde, e a sessão continua funcionando.
    #[test]
    fn conectar_com_sessao_viva_e_no_op() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock montado"), Backend::Mock);
        actor.conectar().expect("sessão viva: reconectar é no-op");
        assert_eq!(
            actor.info().expect("info após reconectar").backend,
            "mock",
            "a sessão sobreviveu ao re-open"
        );
        actor.shutdown();
    }

    /// **A prova de que o generico nao e vazio.** Um backend que **nao** e
    /// o mock — sem `MockState`, sem `drain_inbox` de mock — entra na MESMA
    /// fila e responde. E o que garante que o `RealDevice` do build de campo
    /// tem caminho ate o fio, e nao so uma assinatura que compila.
    ///
    /// O `AparelhoFake` responde `Ok(vec![])` no `recv_raw`; o que importa
    /// aqui e que `info` **degrada honestamente**: `backend` = "real", e os
    /// campos que so o mock tem ficam no neutro em vez de virarem zero
    /// que a UI mostraria como dado.
    struct AparelhoFake {
        sent: Vec<Vec<u8>>,
    }

    impl std::fmt::Debug for AparelhoFake {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("AparelhoFake")
                .field("sent", &self.sent.len())
                .finish()
        }
    }

    impl DeviceTransport for AparelhoFake {
        fn open(&mut self) -> Result<(), TransportError> {
            Ok(())
        }
        fn close(&mut self) -> Result<(), TransportError> {
            Ok(())
        }
        fn send_raw(&mut self, data: &[u8], _kind: WireKind) -> Result<(), TransportError> {
            self.sent.push(data.to_vec());
            Ok(())
        }
        fn recv_raw(&mut self, _t: std::time::Duration) -> Result<Vec<u8>, TransportError> {
            Err(TransportError::RecvTimeout { timeout_ms: 0 })
        }
        /// O fake REPRESENTA o aparelho (é declarado `Backend::Real`), então
        /// ele é aparelho para a trava de faixa do `pp` (#132) — sem isto o
        /// caminho real do app não teria como ser testado sem hardware.
        fn e_aparelho(&self) -> bool {
            true
        }
    }

    impl DeviceBackend for AparelhoFake {
        fn local_state(&self) -> Option<MockState> {
            None
        }
        fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
            Vec::new()
        }
    }

    #[test]
    fn um_backend_que_nao_e_mock_entra_na_mesma_fila() {
        let actor = DeviceActor::spawn(AparelhoFake { sent: Vec::new() }, Backend::Real);
        let st = actor.info().expect("info de um backend sem estado local");
        assert_eq!(st.backend, "real", "o backend vem DECLARADO, nao deduzido");
        assert_eq!(st.current_name, "", "sem estado local = vazio, nao nome");
        assert_eq!(st.current_pp_type, 0);
        assert_eq!(st.ir_slots_with_crc, 0);
        assert_eq!(st.preset_count, 0);
        // O knob dentro da faixa (15.0) sai; fora (99.5) e recusado ANTES do
        // `send_raw` — a trava da #110 e de conteudo, e vale no caminho
        // real tambem.
        actor
            .set_param(3, 0x0700_006e, 0, 15.0)
            .expect("dentro da faixa");
        assert!(
            actor.set_param(3, 0x0700_006e, 0, 99.5).is_err(),
            "acima do teto"
        );
        actor.shutdown();
    }

    /// **#155 §5.2/§5.3: o palco vem do CACHE do scan, nunca de um select.**
    ///
    /// `board` para OUTRO pp tem de devolver sem tocar no fio — um `select`
    /// mudaria o preset que o pedal está mostrando, inaceitável num editor
    /// (spec §5.2). E com o boot feito o palco NÃO é mais recusado.
    #[test]
    fn board_vem_do_cache_sem_select() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        actor.boot(None).expect("boot");
        // 0x0000 é o PRIMEIRO pp do inventário — longe do corrente (0x0162,
        // fim do scan): ler este só pode vir do cache.
        let b = actor.board(Some(0x0000)).expect("board do cache");
        assert_eq!(b.pp, 0x0000, "o palco é do pp pedido");
        assert!(!b.slots.is_empty(), "a cadeia tem slots");
        actor.shutdown();
    }

    /// **Campo 12/10: o palco sobrevive aos snapshots de MONTAGEM.**
    ///
    /// No shell real o `device_info` e o `device_preset_library` rodam no
    /// MESMO commit do pós-boot, e cada um remonta a Session
    /// (`device_snapshot` → `into_transport`) — descartando o cache de
    /// páginas DELA. Com o `BoardView` dependendo só da Session, o primeiro
    /// desses calls destruía as páginas do scan e o `device_board`
    /// seguinte respondia "páginas deste pp não foram lidas no boot": o
    /// palco morria segundos depois de um boot perfeito (wire log
    /// `wire-20261012201602`, 2297/198/198 e a UI em erro).
    ///
    /// O backend é `Real` de propósito: é o único caminho que NÃO projeta o
    /// `all.prst` — sem o cache do actor, este board erra mesmo.
    #[test]
    fn board_do_cache_sobrevive_aos_snapshots_de_montagem() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Real);
        let rel = actor.boot(None).expect("boot");
        assert_eq!(rel.presets, 198, "o scan leu o inventário inteiro");

        // Info e Library remontam a Session — é o que o shell faz ao montar.
        let snap = actor.info().expect("info");
        assert!(
            !snap.current_name.is_empty(),
            "o nome do corrente vem do cache do scan"
        );
        let lib = actor.library().expect("library");
        assert!(
            !lib.entries.is_empty(),
            "a biblioteca é o inventário do aparelho"
        );

        // SEM o cache do actor este `board` ERRARIA: o backend Real não
        // projeta o artefato, e as páginas da Session já foram descartadas.
        let b = actor
            .board(Some(0x0000))
            .expect("board sobrevive aos snapshots");
        assert_eq!(b.pp, 0x0000, "o palco é do pp pedido");
        assert!(!b.slots.is_empty(), "a cadeia vem do decode do scan");
        actor.shutdown();
    }

    /// **#155 §5.1: `current_name` vem da pg0 decodificada no scan.**
    ///
    /// Depois do boot o nome tem de estar lá (a pg0 de cada pp foi lida no
    /// scan e decodificada); antes, a única fonte é o estado local do mock.
    ///
    /// **Limitação do harness declarada:** o `local_state` do mock e o
    /// exemplo congelado do golden são o MESMO preset ("It's GP100"), então
    /// aqui as duas fontes produzem o mesmo nome e não são distinguíveis
    /// por valor. A prova de que o CACHE alimenta a UI é o teste ao lado,
    /// onde elas são distinguíveis (99 nomes distintos do `all.prst` vs 1
    /// do cache do mock).
    #[test]
    fn current_name_vem_do_cache_apos_o_boot() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        let antes = actor.info().expect("info sem boot");
        assert!(!antes.current_name.is_empty(), "estado local tem nome");

        actor.boot(None).expect("boot");

        let depois = actor.info().expect("info com boot");
        assert!(
            !depois.current_name.is_empty(),
            "a pg0 decodificada no scan traz o nome do pp corrente"
        );
        assert_eq!(
            depois.current_pp, 0x00C5,
            "o boot termina o scan no último pp do inventário do MOCK (197); \
             no APARELHO seria 0x0162 — provado no `pp_gate.rs`"
        );
        actor.shutdown();
    }

    /// **#155 §5.1: a biblioteca não devolve inventário sem nome.**
    ///
    /// A prova é DIFERENÇA, não presença: o `all.prst` tem 99 nomes
    /// distintos, então entradas que saem com outro nome só podem ter vindo
    /// do cache (pg0 decodificada no scan). Com o mock a distinção é nítida
    /// — ele serve o MESMO preset para todos os pps do fio.
    ///
    /// O bug de endereçamento que limitava esta prova (#156) foi corrigido:
    /// o `ppID` é DECIMAL e todo ponto do core passa por
    /// `preset::pp_id_decimal` — os 99 da UI casam com o fio. Aqui se trava
    /// o contrato do cache: nenhuma entrada sem nome, e o cache respondendo
    /// por (quase) toda a lista.
    #[test]
    fn biblioteca_servida_dos_nomes_do_cache() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        actor.boot(None).expect("boot");
        let lib = actor.library().expect("biblioteca");
        assert!(!lib.entries.is_empty(), "tem entradas");
        assert!(
            lib.entries.iter().all(|e| !e.name.is_empty()),
            "nenhuma entrada sem nome — o cache alimenta a lista"
        );

        let doc = embedded_document().expect("doc");
        let do_arquivo: BTreeMap<u16, String> = preset_list(&doc)
            .into_iter()
            .map(|e| (e.pp, e.name))
            .collect();
        let do_fio = lib
            .entries
            .iter()
            .filter(|e| do_arquivo.get(&e.pp).map(String::as_str) != Some(e.name.as_str()))
            .count();
        // Com o cache ligado, o cache cobre os 98 pps restantes — o único
        // que não "muda" é o índice 0, que o mock e o arquivo chamam igual
        // ("It's GP100"). Sem o cache seria 0.
        assert!(
            do_fio >= 90,
            "o cache alimenta quase toda a lista (medido: {do_fio}/99; sem \
             cache seria 0)"
        );
        actor.shutdown();
    }

    /// **#156: os 99 presets são selecionáveis.**
    ///
    /// O bug de endereçamento fazia o app endereçar em HEX e o fio em
    /// DECIMAL banco/slot: 36 dos 99 presets tinham um pp que o fio nem tem
    /// (`select_preset` devolvia "esperado pp no espaço banco/slot:
    /// 0000..=0062 ou 0100..=0162"). O teste falhava em 36 deles.
    #[test]
    fn todos_os_99_presets_sao_selecionaveis() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        let doc = embedded_document().expect("doc");
        let lista = preset_list(&doc);
        assert_eq!(lista.len(), 99);
        let mut ok = 0;
        for e in &lista {
            if let Err(err) = actor.select_preset(e.pp) {
                panic!("pp {:#06x} ({}) recusado pelo fio: {err}", e.pp, e.name);
            }
            ok += 1;
        }
        assert_eq!(ok, 99, "os 99 selecionáveis");
        actor.shutdown();
    }

    /// **A trava de faixa do `pp` (#132) atravessa o actor no caminho REAL.**
    /// O app não pode mandar um `select` que o aparelho não provou ter — o
    /// frame do assert `PresetNum < TOTAL_PA` (`audio.c:912`) é recusado com
    /// a faixa na mensagem, antes do fio. O lado "dentro da faixa sai" está
    /// provado no core (`tests/pp_gate.rs`); aqui o que importa é a borda do
    /// APP: o erro SOBE como string, com o endereço e a faixa legíveis.
    #[test]
    fn select_fora_do_inventario_e_recusado_no_caminho_real() {
        let actor = DeviceActor::spawn(AparelhoFake { sent: Vec::new() }, Backend::Real);

        let e = actor
            .select_preset(0x00c5)
            .expect_err("0x00c5 (197) não existe no aparelho");
        assert!(e.contains("11/13010000"), "nomeia o frame: {e}");
        assert!(e.contains("pp"), "nomeia o campo: {e}");
        assert!(
            e.contains("0x0000..0x0062") && e.contains("0x0100..0x0162"),
            "diz até onde o aparelho vai: {e}"
        );

        // O MESMO número no mock segue passando — a trava é do aparelho
        // ("no mock, comportamento atual").
        let mock = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        mock.select_preset(0x00c5).expect("mock sem trava");
        mock.shutdown();
        actor.shutdown();
    }

    /// **`save` e `dump` no app — as duas capacidades que eram só do CLI.**
    ///
    /// O `save` é a operação que PERSISTE: o `set-param` é fire-and-forget
    /// (§13.11, D4), então sem isto a mudança morre com a sessão. O `dump`
    /// é a leitura de campo que o `h2_field.sh` usa para conferir o antes e
    /// o depois na página 0 (é o que provou o F1 do H2).
    ///
    /// As contagens nao sao decorativas: sao as do §13.12 (5 writes de meta +
    /// 4 ops, D3) e as 9 transações do §13.9 (meta6 + 8 páginas). Um `save`
    /// que imprimisse "enviado" sem mandar os 9 frames seria o pior defeito
    /// possível — o operador leria, acreditaria, e o preset nunca foi
    /// gravado.
    #[test]
    fn save_e_dump_via_actor_sao_as_duas_capacidades_do_cli() {
        let mut mock = MockDevice::new().expect("mock montado");
        mock.open().expect("abre");
        let antes = mock.state().current_pp_type;
        let actor = DeviceActor::spawn(mock, Backend::Mock);
        actor.boot(None).expect("boot antes do dump");

        let dump = actor.dump_preset(0).expect("dump do pp corrente");
        assert_eq!(dump.pp, 0);
        assert!(!dump.meta6.is_empty(), "a meta6 veio com conteudo");
        assert_eq!(dump.pages.len(), 8, "§13.9: paginas 0..7 + a final");

        actor
            .save_preset(0, antes, "H2 APP")
            .expect("save §13.12: 9 frames");

        // O nome ASCII foi para o estado do mock (o `11000000` do §13.12).
        assert_eq!(actor.info().expect("info").current_pp_type, antes);
        actor.shutdown();
    }

    /// Nome com byte não-ASCII é recusado pelo codec ANTES do fio — e o
    /// mock fica com o nome antigo. Nao e um detalhe: o §13.12 grava ASCII
    /// cru e um byte alto viraria lixo no preset.
    #[test]
    fn save_recusa_nome_nao_ascii_sem_tocar_o_aparelho() {
        let mut mock = MockDevice::new().expect("mock montado");
        mock.open().expect("abre");
        let actor = DeviceActor::spawn(mock, Backend::Mock);
        let e = actor
            .save_preset(0, 4, "H2 Ã")
            .expect_err("byte nao-ASCII no nome");
        assert!(e.to_string().contains("ASCII"), "{e}");
        actor.shutdown();
    }

    /// Shutdown: depois dele, requisições novas falham com erro limpo
    /// (string) — nunca pânico no command.
    #[test]
    fn shutdown_depois_falha_limpo() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        actor.shutdown();
        assert!(actor.info().is_err(), "info pós-shutdown = erro, não panic");
    }

    /// Tabela via actor: 20 slots, nomes ASCII (vazio = 0xFF no fio).
    #[test]
    fn list_user_irs_via_actor() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        let slots = actor.list_user_irs().expect("tabela dos 20 IRs");
        assert_eq!(slots.len(), 20);
        assert_eq!(slots[0].0, 0, "slot 0");
        assert_eq!(slots[19].0, 19, "slot 19");
        actor.shutdown();
    }

    /// O boot deixa o backlog D7 na inbox do DEVICE: os nomes são
    /// fire-and-forget (D4 — a Session não espera) e o mock RESPONDE a
    /// eles (61 pushes @11000008) — a drenagem os devolve como hex
    /// (F0…F7) e a 2ª drenagem vem vazia (dreno esvazia).
    #[test]
    fn boot_deixa_backlog_d7_drenavel() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        actor.boot(None).expect("boot");
        let pushes = actor.drain_pushes().expect("drenagem");
        assert!(
            !pushes.is_empty(),
            "respostas tardias dos nomes = backlog D7 do device"
        );
        for h in &pushes {
            assert!(h.starts_with("F0") && h.ends_with("F7"), "SysEx: {h}");
        }
        assert!(actor.drain_pushes().expect("2ª").is_empty());
        actor.shutdown();
    }

    /// Upload de SnapTone pelo actor: um modelo de 19 bytes é UM bloco, e um
    /// bloco não paga settle (§4 — não há operação seguinte). Por isso este
    /// teste é rápido mesmo com o piso de 250 ms em produção: o caminho real
    /// do actor é exercitado sem transformar o teste em 36 s.
    #[test]
    fn upload_de_snap_tone_via_actor_fecha_o_stream() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        let rel = actor
            .upload_snap_tone(2, &[0xABu8; 19])
            .expect("upload de 1 bloco");
        assert_eq!(rel.slot, 2);
        assert_eq!(rel.blocks, 1);
        assert_eq!(rel.acks, 1, "ACK de 16B por bloco (§5)");
        assert_eq!(rel.bytes, 19);
        actor.shutdown();
    }

    /// Slot fora de 1..=5 é erro TIPADO até a borda do actor — e não um
    /// "enviado com sucesso" para um slot que não existe no aparelho
    /// (`SnapTone1..5`).
    #[test]
    fn upload_com_slot_invalido_e_erro() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        let e = actor
            .upload_snap_tone(6, &[0xABu8; 19])
            .expect_err("slot 6 nao existe");
        assert!(e.contains("1..=5"), "a mensagem diz o intervalo: {e}");
        actor.shutdown();
    }

    /// Fila serializa (D8): info + boot misturados de "duas threads" chegam
    /// na ordem de envio — sem interleave de transações.
    #[test]
    fn fila_serializa_requisicoes() {
        let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
        let a = actor.clone();
        let t1 = std::thread::spawn(move || a.info().expect("info 1"));
        let st = actor.info().expect("info 2");
        let _ = t1.join().expect("thread 1");
        let report = actor.boot(None).expect("boot após infos");
        assert_eq!(st.preset_count, 99);
        assert_eq!(report.transactions, 2297);
        actor.shutdown();
    }

    /// #48 — ponta-a-ponta no shell: o transporte MORRE no meio da sessão
    /// (`MockFault::DieAfter`) e o erro sobe TIPADO (`ProtocolError::DeviceGone`)
    /// até a borda do actor — nunca achatado em `InvalidShape` ("transporte
    /// saudável"). O actor segue vivo (info responde), a morte aborta na hora
    /// (jamais espera a janela de 3 s) e o retry (⟳ da navbar) falha de novo,
    /// estável: o mock não ressuscita.
    #[test]
    fn device_morrendo_no_meio_da_sessao_nao_mata_o_actor() {
        let mock = MockDevice::new()
            .expect("mock montado")
            .with_fault(MockFault::DieAfter(300));
        let actor = DeviceActor::spawn(mock, Backend::Mock);

        let t0 = std::time::Instant::now();
        let err = actor.boot(None).expect_err("boot com o device morrendo");
        assert!(
            err.contains("device sumiu no meio da sessão"),
            "erro tipado do DeviceGone até o actor: {err}"
        );
        assert!(
            !err.contains("transporte saudável"),
            "DeviceGone não pode virar InvalidShape: {err}"
        );
        assert!(
            t0.elapsed() < std::time::Duration::from_secs(2),
            "a morte aborta a transação na hora (sem janela de 3 s): {:?}",
            t0.elapsed()
        );

        // O actor continua operante: o snapshot do mock não toca o fio.
        let st = actor.info().expect("info após a morte do device");
        assert_eq!(st.preset_count, 99);

        // Retry estável: a falha se repete rápida e tipada (device não volta).
        let t1 = std::time::Instant::now();
        let err2 = actor
            .boot(None)
            .expect_err("retry com o device ainda morto");
        assert!(err2.contains("device sumiu no meio da sessão"), "{err2}");
        assert!(
            t1.elapsed() < std::time::Duration::from_secs(2),
            "retry aborta na hora: {:?}",
            t1.elapsed()
        );
        actor.shutdown();
    }

    /// #164 — ciclo de convergência COMPLETO, com a prova anti-brick da
    /// fase nova: boot → `sync`; escrita → `pending`; read-back → `sync` de
    /// novo E `changed = true` quando o fio veio DIFERENTE do cache — o
    /// board passa a mostrar o valor LIDO do aparelho, não o que o app
    /// escreveu. Toda a fase de leitura é registrada: NENHUM
    /// `WireKind::Write` pode aparecer nela. No boot a ÚNICA escrita
    /// tolerada é o keepalive T4 `12/00020001` (D4 — decisão do owner
    /// 06/10: bate-coração que não toca dados; sem ele um build de
    /// leitura cairia no último passo). Qualquer outro frame de escrita
    /// no caminho é violação anti-brick.
    #[test]
    fn convergencia_reconcilia_o_valor_lido_e_nunca_escreve() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Mutex as StdMutex;

        /// Transporte que MUTA as páginas depois de armado (#164): simula o
        /// aparelho mudando por baixo do app (knob físico / pedal),
        /// trocando a sequência nibble do valor-alvo no payload — mesmo
        /// encoding do core, sem tocar em shape nem endereço.
        ///
        /// Uma "fase" registrada: o que saiu do fio (kind + frame crU) — o
        /// teste decodifica cada escrita para provar o anti-brick.
        type FaseRegistrada = (WireKind, Vec<u8>);

        struct PaginasVivas {
            inner: MockDevice,
            mutar: std::sync::Arc<AtomicBool>,
            /// `(needle, novo)` — bytes nibble do f32 LE a trocar.
            alvo: std::sync::Arc<StdMutex<(Vec<u8>, Vec<u8>)>>,
            registrando: std::sync::Arc<AtomicBool>,
            /// O que saiu do fio enquanto registrado (ver [`FaseRegistrada`]).
            fases: std::sync::Arc<StdMutex<Vec<FaseRegistrada>>>,
        }

        impl std::fmt::Debug for PaginasVivas {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("PaginasVivas").finish_non_exhaustive()
            }
        }

        impl DeviceTransport for PaginasVivas {
            fn open(&mut self) -> Result<(), TransportError> {
                self.inner.open()
            }
            fn close(&mut self) -> Result<(), TransportError> {
                self.inner.close()
            }
            fn send_raw(&mut self, data: &[u8], kind: WireKind) -> Result<(), TransportError> {
                if self.registrando.load(Ordering::SeqCst) {
                    self.fases
                        .lock()
                        .expect("fases")
                        .push((kind, data.to_vec()));
                }
                self.inner.send_raw(data, kind)
            }
            fn recv_raw(&mut self, t: Duration) -> Result<Vec<u8>, TransportError> {
                let mut msg = self.inner.recv_raw(t)?;
                if self.mutar.load(Ordering::SeqCst) {
                    let (needle, novo) = {
                        let g = self.alvo.lock().expect("alvo");
                        (g.0.clone(), g.1.clone())
                    };
                    if needle.len() == novo.len() && !needle.is_empty() {
                        if let Some(pos) = msg.windows(needle.len()).position(|w| w == needle) {
                            msg[pos..pos + needle.len()].copy_from_slice(&novo);
                        }
                    }
                }
                Ok(msg)
            }
        }

        impl DeviceBackend for PaginasVivas {
            fn local_state(&self) -> Option<MockState> {
                self.inner.local_state()
            }
            fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
                self.inner.drain_inbox()
            }
        }

        /// Codificação nibble do f32 LE (o mesmo `b >> 4 | b & 0xF` do
        /// fio) — o que o `nibble_collapse` do core desfaz no `decode`.
        fn nibble_de(v: f32) -> Vec<u8> {
            v.to_le_bytes()
                .iter()
                .flat_map(|b| [b >> 4, b & 0x0F])
                .collect()
        }

        let mutar = std::sync::Arc::new(AtomicBool::new(false));
        let registrando = std::sync::Arc::new(AtomicBool::new(false));
        let fases = std::sync::Arc::new(StdMutex::new(Vec::new()));
        let alvo_bytes = std::sync::Arc::new(StdMutex::new((Vec::new(), Vec::new())));
        let actor = DeviceActor::spawn_com_cadencia(
            PaginasVivas {
                inner: MockDevice::new().expect("mock"),
                mutar: mutar.clone(),
                alvo: alvo_bytes.clone(),
                registrando: registrando.clone(),
                fases: fases.clone(),
            },
            Backend::Mock,
            Duration::from_millis(300),
            Duration::from_secs(30),
        );
        let (ev_tx, ev_rx) = mpsc::channel::<SyncEvent>();
        actor.set_sync_sink(ev_tx);

        // SEM conversa ainda: `idle` — o chip nunca afirma "sincronizado"
        // sem ter lido nada (sem número falso).
        assert_eq!(actor.info().expect("info").sync, "idle");

        // Boot = leitura pura + keepalive T4 (D4). Escrita que NÃO é o
        // bate-coração `12/00020001` é violação anti-brick: o boot não
        // pode mandar SetParam, save, meta nem IR — nada que toque dados.
        registrando.store(true, Ordering::SeqCst);
        actor.boot(None).expect("boot");
        {
            let mut f = fases.lock().expect("fases");
            assert!(!f.is_empty(), "o boot manda transações");
            for (k, frame) in f.iter() {
                if !matches!(k, WireKind::Write) {
                    continue;
                }
                let (func, addr, _) = gp100_core::golden::decode_envelope(frame)
                    .expect("envelope bem formado no fio do boot");
                assert!(
                    func == 0x12 && addr == [0x00, 0x02, 0x00, 0x01],
                    "escrita no boot ALÉM do keepalive T4 (D4): func {func:02x} \
                     addr {addr:02x?} — o boot só pode ler e bater o keepalive"
                );
            }
            f.clear();
            // Desarma: o boot acabou; a escrita do app (set_param) roda
            // DESARMADA — só a fase de leitura volta a ser registrada.
            registrando.store(false, Ordering::SeqCst);
        }
        assert_eq!(actor.info().expect("info").sync, "sync");

        // O palco ANTES: daqui sai o valor-alvo e o slot/code/ctrl do knob.
        let antes = actor.board(None).expect("board antes");
        let (slot_n, code, pos, valor) = antes
            .slots
            .iter()
            .flat_map(|s| s.knobs.iter().map(move |k| (s, k)))
            .find(|(_, k)| {
                k.kind == "knob"
                    && k.value
                        .as_deref()
                        .and_then(|v| v.parse::<f32>().ok())
                        .is_some_and(|v| (1.0..=127.0).contains(&v))
            })
            .map(|(s, k)| {
                (
                    s.slot + 1,
                    s.code,
                    k.pos,
                    k.value.as_deref().unwrap().parse::<f32>().unwrap(),
                )
            })
            .expect("um knob dentro de faixa");

        // O aparelho "gira sozinho" (+7) por baixo do app: armamos a mutação
        // das páginas; a PRÓXIMA leitura vai devolver o valor do device.
        {
            let mut g = alvo_bytes.lock().expect("alvo");
            g.0 = nibble_de(valor);
            g.1 = nibble_de(valor + 7.0);
        }
        mutar.store(true, Ordering::SeqCst);

        // O app escreve (o MESMO valor atual — sempre dentro da faixa ADR-10):
        // D4 sem resposta → estado `pending` até o read-back.
        actor
            .set_param(slot_n, code, pos, valor)
            .expect("escrita aceita");
        assert_eq!(
            actor.info().expect("info").sync,
            "pending",
            "escrita aceita = pendente até o read-back confirmar"
        );
        // Arma SÓ a fase de leitura: o boot desarmou ao terminar e o
        // `set_param` (escrita do app, registrada ou não) já foi processado
        // — daqui até o read-back, só frames de leitura podem aparecer.
        registrando.store(true, Ordering::SeqCst);

        // Espera o read-back (debounce 300 ms) — evento `changed = true`.
        // Cada evento NÃO-changed fica registrado: a falha mostra o que
        // chegou de verdade (status/detail), não só "não veio".
        let mut eventos: Vec<SyncEvent> = Vec::new();
        let mut viu_changed = false;
        let prazo = std::time::Instant::now() + Duration::from_secs(8);
        while std::time::Instant::now() < prazo {
            match ev_rx.recv_timeout(Duration::from_millis(400)) {
                Ok(ev) if ev.changed => {
                    assert_eq!(ev.status, "sync", "read-back OK = sincronizado");
                    viu_changed = true;
                    break;
                }
                Ok(ev) => eventos.push(ev),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        assert!(
            viu_changed,
            "o read-back reconciliou (changed = true); eventos: {eventos:?}"
        );

        // Anti-brick da fase nova: TUDO que saiu depois da escrita foi leitura.
        {
            let f = fases.lock().expect("fases");
            assert!(!f.is_empty(), "o read-back manda as 9 páginas");
            assert!(
                f.iter().all(|(k, _)| matches!(k, WireKind::Read)),
                "o read-back NÃO pode escrever: {:?}",
                f.iter().find(|(k, _)| !matches!(k, WireKind::Read))
            );
            registrando.store(false, Ordering::SeqCst);
        }

        // Reconciliação: o board MOSTRA o valor do aparelho (+7), não o que
        // o app quis escrever (valor).
        let depois = actor.board(None).expect("board depois");
        let lido = depois
            .slots
            .iter()
            .flat_map(|s| s.knobs.iter())
            .find(|k| k.kind == "knob" && k.pos == pos)
            .and_then(|k| k.value.as_deref())
            .and_then(|v| v.parse::<f32>().ok())
            .expect("o mesmo knob depois");
        assert_eq!(
            lido,
            valor + 7.0,
            "o aparelho vence: o valor LIDO é o do device, não o escrito"
        );
        assert_eq!(actor.info().expect("info").sync, "sync");
        actor.shutdown();
    }

    /// #164 — queda do transporte: o read-back morre com `DeviceGone`, o
    /// estado vira `offline` com motivo legível, e a escrita seguinte FALHA
    /// sem ir ao fio (o app não continua escrevendo em vazio). A sessão
    /// fica viva (ADR-4): o retry devolve o erro tipado rápido.
    #[test]
    fn queda_no_meio_do_readback_vira_offline_e_bloqueia_escrita() {
        let mock = MockDevice::new()
            .expect("mock")
            .with_fault(MockFault::DieAfter(4));
        let actor = DeviceActor::spawn_com_cadencia(
            mock,
            Backend::Mock,
            Duration::from_millis(150),
            Duration::from_secs(60),
        );

        // tx1 = set_param aceito → `pending` (a prova de que a escrita foi
        // ANTES da morte, e não um estado herdado).
        actor
            .set_param(3, 0x0700_006e, 0, 15.0)
            .expect("escrita antes da queda");
        assert_eq!(actor.info().expect("info").sync, "pending");

        // O read-back dispara (debounce 150 ms) e morre no meio das 9
        // páginas (transmissões 3, 4 e a 5 estoura o DieAfter).
        std::thread::sleep(Duration::from_millis(500));

        let st = actor.info().expect("info continua respondendo");
        assert_eq!(st.sync, "offline", "transporte morto = offline");
        assert!(
            st.sync_detail.contains("desconectado"),
            "o motivo é legível: {}",
            st.sync_detail
        );

        // Escrita seguinte: FALHA (DeviceGone tipado) — nenhum frame sai.
        let err = actor
            .set_param(3, 0x0700_006e, 0, 20.0)
            .expect_err("sem aparelho não existe escrita");
        assert!(
            err.contains("device sumiu") || err.contains("session do actor ausente"),
            "erro tipado rápido, não janela de timeout: {err}"
        );
        assert_eq!(actor.info().expect("info").sync, "offline");
        actor.shutdown();
    }

    /// #164 — `device_sync_now` dispara o read-back AGORA: o spawn tem
    /// refresh de 1 HORA, então o `sync` que vier em seguida SÓ pode ser o
    /// agendamento do comando — a leitura não espera a cadência.
    #[test]
    fn sync_now_dispara_readback_sem_esperar_o_refresh() {
        let actor = DeviceActor::spawn_com_cadencia(
            MockDevice::new().expect("mock"),
            Backend::Mock,
            Duration::from_millis(100),
            Duration::from_secs(3600),
        );
        let (ev_tx, ev_rx) = mpsc::channel::<SyncEvent>();
        actor.set_sync_sink(ev_tx);

        actor.boot(None).expect("boot");
        // Drena os eventos do boot (o `sync` final já está na fila) — daqui
        // em diante qualquer evento nasce do sync_now.
        while ev_rx.recv_timeout(Duration::from_millis(200)).is_ok() {}

        actor.sync_now().expect("agenda o read-back");
        let ev = ev_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("o read-back do sync_now emite sem esperar a cadência");
        assert_eq!(ev.status, "sync", "leitura OK = sincronizado");
        actor.shutdown();
    }
}
