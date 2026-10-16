//! os DTOs e constantes que ATRAVESSAM a fronteira da sessão: o que o chamador vê do fio.
//!
//! Nada aqui conhece transação: são dados (relatórios, páginas cruas) e os
//! números que o resto do crate e o app comparam. Fonte: ADR-6 (rev.3) e
//! `docs/tecnico/PROTOCOL.md` §13.

// sem imports: só dados

/// Ordem REAL do script de boot (prova C do replay):
/// Tables → Scan → Probe → State5 → Names → Keepalive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootStage {
    /// T1 — tabelas de User IRs `11/12001002` (20 páginas ×2 leituras).
    Tables,
    /// T5 — scan dos presets (select+open+9 páginas por pp; o pp corrente
    /// com select/open DUPLICADOS — quirk §13.4).
    Scan,
    /// T6 — sonda do banco 02 (select const + open + 9 páginas).
    Probe,
    /// T2 — 5 leituras de `11/12001012` (44B cada). O repo a chama(va) de
    /// "setlist", mas a GP-100 **não tem função setlist** no manual
    /// (`docs/referencias/GP100_DEVICE.md` §5#1). Renomeado para `State5` (5 entradas de
    /// estado) até a semântica fina ser decifrada em campo — não expor como
    /// produto.
    State5,
    /// T3 — nomes `11/11000008` (fire-and-forget, D4; 61 leituras).
    Names,
    /// T4 — keepalives `12/00020001` ×2 (D4).
    Keepalive,
}

/// Instantâneo de progresso do boot (emitido pelo callback de
/// [`Session::boot_with_progress`] a cada transação completada).
#[derive(Debug, Clone, Copy)]
pub struct BootProgress {
    /// Etapa corrente do script (§13.10).
    pub stage: BootStage,
    /// Transações completas ATÉ agora (inclui as etapas anteriores).
    pub done: usize,
    /// Total esperado de transações do script completo (função do
    /// inventário; captura S1 = 2299 com o pp corrente duplicado).
    pub total: usize,
    /// pp corrente APÓS a transação (o scan avança o seleção — §13.10),
    /// para a UI mostrar qual preset está sendo levantado.
    pub current_pp: u16,
}

/// Callback de progresso do boot: observacional — NÃO altera a
/// sequência de fio (replay byte-a-byte continua válido; ver
/// `tests/replay_fixtures.rs::boot_progress_nao_degrada_o_replay`).
pub type BootProgressFn<'a> = dyn FnMut(BootProgress) + 'a;

/// Relatório do boot+scan (§13.10): nasce MÍNIMO (deriva das vars que os
/// templates extraem) e cresce só quando a UI pedir (ADR-6, YAGNI) — a
/// #161 (gate de boot) pediu o inventário, para o front montar a casca
/// só com o aparelho LIDO.
#[derive(Debug, Clone)]
pub struct BootReport {
    /// Nº de transações de boot+scan executadas com sucesso (replay da
    /// sequência capturada: 2299 OUT na S1).
    pub transactions: usize,
    /// Pps com cache de páginas após o scan — o inventário lido (198 no
    /// catálogo atual: 99 de fábrica + 99 de usuário).
    pub presets: usize,
    /// Pps do cache cuja pg0 decodifica com nome válido (#155) — o "198/198"
    /// que o gate da #161 valida: nomes menos que presets = leitura pela
    /// metade.
    pub names: usize,
}

/// Página de estado (família 13xx): o payload cru como veio no fio, mais
/// as vars dos templates do golden.
///
/// O corpo **nibble-exp** é decifrado por [`crate::preset_pages::decode`]
/// (2026-10-08): 4B de header `[pp u16BE][00][PG]` + 192B de nibbles →
/// 96B por página. Aqui o `raw` segue cru de propósito — é o contrato do
/// fio (R1: o formato da 13xx não vaza pra UI) e o golden valida esse
/// shape.
#[derive(Debug, Clone)]
pub struct StatePage {
    /// Payload cru da página (nibble-exp, como veio no fio).
    pub raw: Vec<u8>,
}

/// Resultado de `list_user_irs`: a TABELA dos 20 User IRs (§13.12), já
/// distinguida do ACK 4B pelo by-len de `match_shape` (D1).
#[derive(Debug, Clone)]
pub struct UserIrTable {
    /// Slots decodificáveis: (slot u8, nome ASCII até 32B). CRC de slot
    /// ocupado é pendência menor (§13.12) — não entra aqui.
    pub slots: Vec<(u8, String)>,
}

/// Bytes de payload por chunk no upload de IR (§13.7).
///
/// 30 nibbles na SysEx = **15 bytes reais** por chunk. O valor é público e
/// único porque a biblioteca recusa na importação o `.ir` que não é múltiplo
/// dele (strict, rev.2 do ADR-6) — um arquivo guardado que não pode ser
/// enviado só adia o erro para o momento do upload, com o dono já no
/// aparelho. Duas constantes numéricas em crates diferentes são um `mod 15`
/// que aceita um e outro.
pub const IR_CHUNK_BYTES: usize = 15;

/// Quantidade de slots de User IR (`<ppIRInfo0..19>`, §13.12).
///
/// Quem define o slot é a POSIÇÃO da tag no `.prst` — e o slot **0 é
/// válido**, ao contrário do SnapTone que começa em 1. A biblioteca valida
/// contra este mesmo número.
pub const IR_SLOTS: u8 = 20;

/// Relatório do upload de IR (§13.7).
///
/// `Serialize` pelo mesmo motivo do [`SnapToneUploadReport`]: este relatório é
/// o que volta pela ponte IPC e o crate do Tauri não compila no host — o
/// contrato verificado aqui é verificado nas três plataformas da matriz.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrUploadReport {
    /// Slot de destino (0..=19).
    pub slot: u8,
    /// Chunks de dados enviados (exclui o frame final duplicado idx 0x226).
    pub chunks: usize,
    /// ACKs validados por chunk (D1; marcador de fim = duplicação do
    /// último chunk idx 0x226 — payload é a cauda REAL do blob, §13.7).
    pub acks: usize,
    /// Bytes do `.ir` que foram para o fio — o que a tela mostra ao lado do
    /// número de chunks, porque "296 chunks" sozinho não diz se entrou um
    /// arquivo de 4 KB ou de 400 KB.
    pub bytes: usize,
}

/// Settle mínimo entre duas operações de SnapTone, em ms (PROTOCOL §4,
/// regra 1 adotada da comunidade: *"settle ≥ 0,25 s entre operações — 0,15 s
/// corrompe; loop tight **trava a pedaleira**"*).
///
/// O valor não é afinação: é o piso que a comunidade mediu. Subir é seguro
/// (o upload só fica mais lento); descer é o que corrompe o stream.
pub const SNAP_TONE_SETTLE_MS: u64 = 250;

/// Um registrador global lido do aparelho: `(endereço, payload CRU)`.
/// Alias público porque [`Session::read_globals`] devolve a lista desses
/// pares — o decode (nibble-par) é do chamador (ADR-13).
pub type GlobalBruto = ([u8; 4], Vec<u8>);

/// Relatório do upload de SnapTone (§5).
///
/// `Serialize` porque este relatório é o que volta pela ponte IPC: o command
/// do Tauri devolve o valor como está e o TS lê `blocks`/`acks`. Derivando o
/// serde aqui, no crate que compila nas três plataformas da matriz, o contrato
/// do fio é testado no CI comum em vez de depender de uma compilação Windows.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SnapToneUploadReport {
    /// Slot de destino (1..=5 — `SnapTone1..5`).
    pub slot: u8,
    /// Blocos enviados (= `ceil(len / 19)`).
    pub blocks: usize,
    /// ACKs de 16B validados — um por bloco (§5).
    pub acks: usize,
    /// Bytes do modelo convertido que foram para o fio.
    pub bytes: usize,
}
