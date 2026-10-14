//! Tipos do contrato do actor: estado de sincronização, backend, snapshot e os
//! DTOs que atravessam o IPC (`camelCase` via serde).
//!
//! Fonte: `docs/dev/DECISIONS.md` (ADR-6/D8 — dono único da sessão; ADR-15 —
//! poll/read-back) e `docs/referencias/REAL_DEVICE_GAP.md` §1. A marcação campo
//! a campo do [`DeviceSnapshot`] é deliberada: é o que separa "lei do mock" de
//! "veio do fio", e o que a UI usa para não mostrar zero como se fosse dado.

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

/// Bytes crus em hex maiúsculo, sem separador — o mesmo formato que o
/// `gp100-cli` imprime e que os juízes (`h1_compare.py`/`h2_compare.py`)
/// leem dos logs.
pub(super) fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02X}")).collect()
}

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
pub(super) const WRITE_VERIFIED: bool = cfg!(feature = "write-verified");

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
    /// o boot percorreu** (ver `docs/referencias/REAL_DEVICE_GAP.md` §4.3 — hoje é o
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
    pub(super) fn empty(backend: Backend) -> Self {
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
