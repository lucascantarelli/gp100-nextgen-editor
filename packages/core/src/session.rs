//! session — FSM de sessão sobre o transporte (**ADR-6 rev.3** de
//! `docs/DECISIONS.md` — FONTE ÚNICA das assinaturas).
//!
//! O replay byte-a-byte das fixtures P4 (diff hex na divergência) prova a FSM.
//!
//! Regras de dispatch — resumo (FONTE ÚNICA = ADR-6, não reabrir aqui):
//! - **D1** a transação é dona do endpoint: quem pediu interpreta a PRÓXIMA
//!   msg do `(func, addr)` via `GoldenFile::match_response` (by-len resolve
//!   `12001002`: ACK 4B × tabela 75B);
//! - **D2** push é classificado pelo CONTEXTO da operação, não pela msg
//!   (`13010001` push×req têm os MESMOS 6 bytes — ordem do arquivo só
//!   desempata FORMA);
//! - **D3** save é FIRE-AND-FORGET (re-derivado 29/09): `save_preset`
//!   envia meta_block + ciclo de ops da S4 (op0 ×2 [0; +578ms] → op1 ×2
//!   [+593ms]) e NÃO espera nada; bursts `11000008`/`12000001` são
//!   sincronização de fim de sessão, NUNCA confirmação de save;
//! - **D4** fire-and-forget não drena (`set_param` §13.11 sem read-back);
//! - **D5** sem match = `InvalidShape` tipado (nunca engolir);
//! - **D6** sem retry — timeout (ADR-3: 3s/transação) sobe como
//!   `Timeout`;
//! - **D7** espera de resposta é LOOP FILTRANTE: msg de endpoint ≠ esperado
//!   vai para o backlog interno (push não solicitado não quebra D1);
//! - **D8** consumidor ÚNICO do stream IN (`&mut self`; observação = poll
//!   do backlog via [`Session::pending_pushes`], nunca listener concorrente).

use crate::golden::GoldenFile;
use crate::transport::{DeviceTransport, TransportError, WireKind};
use crate::ProtocolError;
use std::collections::BTreeMap;
use std::time::Duration;

/// Janela da transação (ADR-3): 3000 ms por request→resposta.
const TX_TIMEOUT_MS: u64 = 3000;

/// Ordem REAL do script de boot (prova C do replay):
/// Tables → Scan → Probe → Setlist → Names → Keepalive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootStage {
    /// T1 — tabelas de User IRs `11/12001002` (20 páginas ×2 leituras).
    Tables,
    /// T5 — scan dos presets (select+open+9 páginas por pp; o pp corrente
    /// com select/open DUPLICADOS — quirk §13.4).
    Scan,
    /// T6 — sonda do banco 02 (select const + open + 9 páginas).
    Probe,
    /// T2 — setlist `11/12001012` (5 leituras).
    Setlist,
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
/// templates extraem) e cresce só quando a UI pedir (ADR-6, YAGNI).
#[derive(Debug, Clone)]
pub struct BootReport {
    /// Nº de transações de boot+scan executadas com sucesso (replay da
    /// sequência capturada: 2299 OUT na S1).
    pub transactions: usize,
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
/// distinguida do ACK 4B pelo by-len de `match_response` (D1).
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

/// FSM de sessão — genérica sobre o transporte (ADR-4/ADR-6).
pub struct Session<T: DeviceTransport> {
    transport: T,
    /// pp corrente (select do boot/scan; default 0x0100 = "01 00", §13.4).
    current_pp: u16,
    /// Inventário de presets na ordem de scan. Default = o inventário do
    /// transporte: [`inventario_do_aparelho`] no aparelho (MEDIDO, captura
    /// S1 — #132) e `0..198` no mock (comportamento histórico). O replay
    /// da S1 prova a ordem da captura.
    pps: Option<Vec<u16>>,
    /// Backlog de IN não solicitado (D7), na ordem de chegada (hex cru).
    backlog: Vec<Vec<u8>>,
    /// As 9 páginas de cada pp escaneado, tal como saíram do fio
    /// (`13010003` nas pg 0..7 e `13010005` na pg 8). O scan (T5) já lia
    /// todas — guardá-las só faz o retorno deixar de ser descartado: o fio
    /// é idêntico (D1–D8 intocados, prova C intacta). Vazio antes do boot.
    pages: BTreeMap<u16, [StatePage; 9]>,
}

/// O espaço de presets no fio é BANCO/SLOT (§13.4): 198 pps em dois bancos
/// de 99 — NÃO é um intervalo linear. As capturas (S1–S4, 796 selects)
/// contêm exatamente `0000..=0062` e `0100..=0162`; os pps `0x0063..0x00C5`
/// NÃO existem, e o firmware V2.1 morre no assert `PresetNum < TOTAL_PA`
/// (`audio.c:912`) ao recebê-los — o scan do boot os mandava por suposição
/// linear nunca conferida em campo (issue #148, campo 07/10).
pub fn pp_e_valido(pp: u16) -> bool {
    let banco = pp >> 8;
    let slot = pp & 0x00FF;
    slot <= 0x0062 && (banco == 0x00 || banco == 0x01)
}

/// Os 198 `pp` que o **aparelho** tem, na ordem em que o Suite varre na
/// captura S1 (`analysis/fixtures/boot.jsonl` — os mesmos 2299 OUTs que
/// `tests/replay_fixtures.rs::replay_boot_byte_a_byte` reproduz).
///
/// **De onde o número vem (issue #132 — nada aqui é inventado).** A captura
/// do próprio Suite escaneando o pedal manda, em `11/13010000`:
///
/// ```text
/// 0100 0101 … 0162  0000 0001 … 0062
/// ```
///
/// → **99 pps no banco `0x01xx` + 99 no banco `0x00xx`**, os 99 slots de
/// `all.prst` (ppID `"0".."98"`) nos DOIS bancos, com o byte alto do banco
/// no byte mais significativo. A sonda de campo de 06/10 corroborou o
/// espaço (`dump-preset 0x0100` respondeu) e o `H1` já tinha lido
/// `0x0000`/`0x0031`/`0x0062` no aparelho.
///
/// **Por que este é o default do APARELHO e não o `0..198` de sempre.**
/// `0..197` manda 99 selects que o aparelho NÃO tem (`0x0063..0x00c5` — o
/// `PresetNum < TOTAL_PA` do assert de `audio.c:912` é literalmente sobre
/// isto) e NUNCA alcança o banco `0x01xx`, onde o pedal liga. O mock fica
/// com o `0..198` de sempre (o espaço dele é o do documento, e os testes
/// varrem pps arbitrários).
///
/// A ordem é a da captura (banco 1 primeiro) porque a ordem TAMBÉM é
/// evidência: com este inventário o `boot()` de um aparelho recém-ligado
/// reproduz a sequência do Suite.
///
/// **É a ÚNICA fonte do espaço do aparelho.** O CONJUNTO (99 no banco
/// `0x01xx` + 99 no `0x00xx`) e a ORDEM vivem aqui — não existe uma segunda
/// lista do mesmo espaço, com outra ordem, para divergir desta. A ordem
/// canônica do banco/slot (`0x00xx` primeiro) não tem consumidor: quem
/// varre é a captura. `pp_e_valido` declara o MESMO espaço como predicado
/// estático, e `pp_gate.rs` prende as duas no fixture da S1.
pub fn inventario_do_aparelho() -> Vec<u16> {
    let mut pps: Vec<u16> = (0x0100u16..=0x0162).collect();
    pps.extend(0x0000u16..=0x0062);
    pps
}

/// O inventário escrito como faixas legíveis para o runbook de campo
/// (`0x0000..0x0062, 0x0100..0x0162`) — a mensagem da recusa precisa dizer
/// não só O QUE foi recusado, mas até onde o aparelho vai.
fn faixa_legivel(pps: &[u16]) -> String {
    let mut ordenados = pps.to_vec();
    ordenados.sort_unstable();
    ordenados.dedup();
    let mut faixas: Vec<String> = Vec::new();
    let mut inicio = match ordenados.first() {
        Some(&p) => p,
        None => return "(inventário vazio)".into(),
    };
    let mut anterior = inicio;
    for &p in ordenados.iter().skip(1) {
        if p == anterior + 1 {
            anterior = p;
            continue;
        }
        faixas.push(descreve_faixa(inicio, anterior));
        inicio = p;
        anterior = p;
    }
    faixas.push(descreve_faixa(inicio, anterior));
    faixas.join(", ")
}

fn descreve_faixa(inicio: u16, fim: u16) -> String {
    if inicio == fim {
        format!("{inicio:#06x}")
    } else {
        format!("{inicio:#06x}..{fim:#06x}")
    }
}

/// Um frame de escrita do `save` (§13.12) com o seu endereco — a forma que
/// o `--dry-run` do CLI e o `device_preview` do app mostram ao operador.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveFrame {
    /// Endereço 4B do frame (o mesmo que vai no `ADDR` do envelope).
    pub addr: [u8; 4],
    /// O SysEx completo, como sairia pelo `send_raw`.
    pub sysex: Vec<u8>,
}

/// Os 9 frames do `save` (§13.12), **sem enviar nada**.
///
/// **POR QUE ISTO EXISTE SEPARADO DE [`Session::save_preset`].** O dry-run do
/// CLI e o `device_preview` do app precisam exatamente dos mesmos bytes que
/// seriam enviados — e a tentacao é reimplementar a montagem no consumidor,
/// que é como um dry-run passa a mentir (mostra um frame e o aparelho recebe
/// outro). Aqui a montagem tem **uma fonte só** e o `save_preset` consome
/// esta mesma função, então preview e envio não podem divergir por construção.
///
/// Ordem (D3, ciclo de ops da S4): 5 writes do meta (§13.12) e então
/// op0, op0, op1, op1 — o fim dos writes é o commit, e **ZERO IN** é
/// esperado.
///
/// # Erros
/// [`ProtocolError::InvalidShape`] se o nome não for ASCII imprimível.
pub fn save_frames(pp: u16, pp_type: u16, name: &str) -> Result<Vec<SaveFrame>, ProtocolError> {
    // write_frame = ponto ÚNICO de montagem de envelope de write (§13.1);
    // 5 writes do meta (§13.12) + ciclo de ops da S4 (D3): op0 com o
    // meta, op0 de novo, op1 ×2 — fim dos writes = commit, ZERO IN esperado
    let mut out: Vec<SaveFrame> = Vec::with_capacity(9);
    for (addr, payload) in crate::codec::meta_block(pp, pp_type, name)? {
        out.push(SaveFrame {
            addr,
            sysex: crate::codec::write_frame(&addr, &payload),
        });
    }
    for op in [0u8, 0, 1, 1] {
        let addr = [0x00, 0x02, 0x00, 0x00];
        out.push(SaveFrame {
            addr,
            sysex: crate::codec::write_frame(&addr, &crate::codec::op_payload(op)),
        });
    }
    Ok(out)
}

/// Alimenta o array de páginas de um pp com a resposta de uma transação.
///
/// O índice da página vem do **próprio payload** (`raw[3]`, 0..=8) — não da
/// posição do pedido: o `open` `13010002` entrega a página 0, os reqs
/// `13010004` PG 0..7 entregam as páginas 1..8, e o req PG 8 devolve um
/// ACK de 4B em `13010005` que NÃO é página. Só o comprimento certifica:
/// 196B (4B header + 192B) ou 32B (4B + 28B) é corpo de página.
///
/// Sem efeito no fio — só o VALOR, que já era esperado por `wait_for`.
fn guarda_pagina(paginas: &mut [StatePage; 9], raw: Vec<u8>) {
    if !matches!(raw.len(), 196 | 32) {
        return; // ACK de 4B, push ou meta6 — não é corpo de página
    }
    if let Some(idx) = raw.get(3).copied().filter(|&i| i < 9) {
        paginas[idx as usize] = StatePage { raw };
    }
}

impl<T: DeviceTransport> Session<T> {
    /// Constrói a FSM sobre o transporte. NÃO abre o transporte (ciclo de
    /// vida é do chamador — ADR-4/ADR-6). O golden vem de
    /// `GoldenFile::embedded()` (1 parse por processo) nos métodos — não é
    /// campo (rev.2: valor único `&'static` em campo é ruído de API).
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            current_pp: 0x0100,
            pps: None,
            backlog: Vec::new(),
            pages: BTreeMap::new(),
        }
    }

    /// Define o inventário de pps do scan (ordem = ordem de seleção do
    /// boot; default = [`inventario_do_aparelho`] no aparelho e `0..198` no
    /// mock). A ordem da S1 é provada pelo replay.
    pub fn set_inventory(&mut self, pps: Vec<u16>) {
        self.pps = Some(pps);
    }

    /// O inventário em uso: o que [`Session::set_inventory`](Self::set_inventory)
    /// fixou, ou o default do TRANSPORTE — [`inventario_do_aparelho`]
    /// quando ele é o aparelho (é este conjunto que o `boot` varre E que a
    /// trava de `select_preset` confere, #132), `0..198` no mock.
    fn inventory(&self) -> Vec<u16> {
        self.pps.clone().unwrap_or_else(|| {
            if self.transport.e_aparelho() {
                inventario_do_aparelho()
            } else {
                (0u16..198).collect()
            }
        })
    }

    /// Boot + scan (§13.10) SEM progresso — a forma canônica do ADR-6
    /// (assinatura/sequência intocadas; os contratos de replay a consomem).
    /// Para barra de progresso na UI, use [`Session::boot_with_progress`].
    ///
    /// # Erros
    /// [`ProtocolError::Timeout`] (D6), [`ProtocolError::InvalidShape`]
    /// (D5) — e [`ProtocolError::UnexpectedAck`] (diff hex da resposta).
    pub fn boot(&mut self) -> Result<BootReport, ProtocolError> {
        self.boot_inner(None)
    }

    /// Boot + scan (§13.10) com hook de progresso: chama `on_progress`
    /// a cada transação completada com [`BootProgress`] { stage, done, total }.
    /// O total é função do inventário (pp corrente duplicado soma +2 — quirk
    /// §13.4); `done` cresce monótono até `total` na última transação.
    ///
    /// # Erros
    /// Os mesmos de [`Session::boot`] (o hook é observacional — nada muda no
    /// fio: D1–D8 intocados).
    pub fn boot_with_progress(
        &mut self,
        on_progress: Option<&mut BootProgressFn<'_>>,
    ) -> Result<BootReport, ProtocolError> {
        self.boot_inner(on_progress)
    }

    /// Corpo do boot + scan (§13.10): o script completo do Suite ao ligar,
    /// gerado pelas MESMAS regras da prova C do `validate_golden.py`
    /// (2299/2299): T1) tabela de IRs `11/12001002`: 20 páginas ×2 leituras;
    /// T2) `11/12001012`: índices 0..4; T3) nomes `11/11000008`: bancos
    /// 0x00–0x02 completos (16) + 0x03 com 13; T4) keepalive `12/00020001`
    /// ×2; T5) scan: para cada pp (ordem do inventário; o ATUAL 0x0100 com
    /// select+open DUPLICADOS — quirk de boot): select `11/13010000` [pp],
    /// open `12/13010002` [pp]01, 9 páginas `12/13010004` [pp][PG]01;
    /// T6) sonda do banco 02: select 0000 + open 000001 + 9 páginas.
    ///
    /// INTERLEAVE (D2): cada transação espera a PRÓPRIA resposta (D1) e
    /// pushes de boot não solicitados (dump 13000000, meta6 13010001,
    /// páginas 13010003, setlist 12001012, nomes 11000008) vão para o
    /// BACKLOG (D7), não confundem as transações. `progress` é
    /// observacional: um `beat!` por transação, nada no fio.
    fn boot_inner(
        &mut self,
        mut progress: Option<&mut BootProgressFn<'_>>,
    ) -> Result<BootReport, ProtocolError> {
        let golden = GoldenFile::embedded()?;
        let mut tx = 0usize;
        // Total esperado: T1(40) + scan(11/pp +2 se pp corrente duplicado)
        // + sonda(11) + setlist(5) + nomes(61) + keepalive(2). O replay da
        // captura (198 pps, 0x0100 duplicado) fecha em 2299 = prova C.
        let pps = self.inventory();
        let doubled = pps.iter().filter(|&&p| p == 0x0100).count();
        let total = 40 + pps.len() * 11 + doubled * 2 + 11 + 5 + 61 + 2;
        let mut stage = BootStage::Tables;
        macro_rules! beat {
            () => {
                if let Some(cb) = progress.as_mut() {
                    cb(BootProgress {
                        stage,
                        done: tx,
                        total,
                        current_pp: self.current_pp,
                    });
                }
            };
        }

        // ORDEM REAL do boot S1 (prova C / replay): T1 → scan (T5) →
        // sonda 1302 (T6) → setlist (T2) → nomes (T3) → keepalives ×2.
        // T1: 20 páginas ×2 (regra da prova C)
        for p in 0u8..0x14 {
            for _ in 0..2 {
                self.tx_req(golden, 0x11, &[0x12, 0x00, 0x10, 0x02], &[p])?;
                tx += 1;
                beat!();
            }
        }
        stage = BootStage::Scan;
        // T5: scan — pareamento REAL (replay S1): select `13010000` →
        // meta6 `13010001`; open `13010002` [pp]01 → página 0 em `13010003`;
        // req `13010004` [pp][PG]01 (PG 0..7) → página PG+1 (196B/32B);
        // PG 8 → `13010005` (4B). O preset ATUAL 0x0100 tem select e open
        // DUPLICADOS (quirk §13.4: 2 selects + 2 opens; página 0 extra).
        for pp in self.inventory() {
            let pp_be = pp.to_be_bytes();
            let doubled = pp == 0x0100;
            if doubled {
                self.tx_req_in(
                    golden,
                    0x11,
                    &[0x13, 0x01, 0x00, 0x00],
                    &pp_be,
                    &[0x13, 0x01, 0x00, 0x01],
                    WireKind::Read,
                )?;
                tx += 1;
                beat!();
            }
            self.tx_req_in(
                golden,
                0x11,
                &[0x13, 0x01, 0x00, 0x00],
                &pp_be,
                &[0x13, 0x01, 0x00, 0x01],
                WireKind::Read,
            )?;
            tx += 1;
            beat!();
            // O `open` entrega a página 0 (`raw[3] == 0`) — as 9 páginas
            // do pp, capturadas sem mudar o fio: o scan já esperava cada
            // resposta em `wait_for`; só o VALOR era descartado com `?`.
            let mut paginas: [StatePage; 9] =
                std::array::from_fn(|_| StatePage { raw: Vec::new() });
            let abertura = self.tx_req_in(
                golden,
                0x12,
                &[0x13, 0x01, 0x00, 0x02],
                &pp_be,
                &[0x13, 0x01, 0x00, 0x03],
                WireKind::Read,
            )?;
            guarda_pagina(&mut paginas, abertura);
            tx += 1;
            beat!();
            if doubled {
                // ...e open duplicado: a página 0 chega DE NOVO em
                // `13010003` (captura S1 rows 89–93: open open → pág0 pág0;
                // NÃO é meta6 — era este o desalinhamento do replay)
                let abertura2 = self.tx_req_in(
                    golden,
                    0x12,
                    &[0x13, 0x01, 0x00, 0x02],
                    &pp_be,
                    &[0x13, 0x01, 0x00, 0x03],
                    WireKind::Read,
                )?;
                guarda_pagina(&mut paginas, abertura2);
                tx += 1;
                beat!();
            }
            for pg in 0u16..9u16 {
                // t8: var2 (pp) + const 00 + var1 (PG baixo) + const 01
                let vars = [pp_be[0], pp_be[1], pg as u8];
                let in_addr: &[u8; 4] = if pg < 8 {
                    &[0x13, 0x01, 0x00, 0x03]
                } else {
                    &[0x13, 0x01, 0x00, 0x05]
                };
                // PG 0..7 respondem em `13010003` com a página SEGUINTE
                // (`raw[3] == pg+1`); PG 8 responde em `13010005` com um
                // ACK de 4B — `guarda_pagina` distingue pelo comprimento.
                let resposta = self.tx_req_in(
                    golden,
                    0x12,
                    &[0x13, 0x01, 0x00, 0x04],
                    &vars,
                    in_addr,
                    WireKind::Read,
                )?;
                guarda_pagina(&mut paginas, resposta);
                tx += 1;
                beat!();
            }
            self.pages.insert(pp, paginas);
            self.current_pp = pp;
        }
        stage = BootStage::Probe;
        // T6: sonda do banco 02 (mesmo pareamento; 13020001/13020003/13020005).
        // O select é CONST "0000" no golden (sem var de pp — é a sonda do
        // banco 02, não um select de preset).
        self.tx_req_in(
            golden,
            0x11,
            &[0x13, 0x02, 0x00, 0x00],
            &[],
            &[0x13, 0x02, 0x00, 0x01],
            WireKind::Read,
        )?;
        tx += 1;
        beat!();
        self.tx_req_in(
            golden,
            0x12,
            &[0x13, 0x02, 0x00, 0x02],
            &[],
            &[0x13, 0x02, 0x00, 0x03],
            WireKind::Read,
        )?;
        tx += 1;
        beat!();
        for pg in 0u16..9u16 {
            // t15: const 000000 + var1 (PG baixo) + const 01
            let vars = [pg as u8];
            if pg < 8 {
                self.tx_req_in(
                    golden,
                    0x12,
                    &[0x13, 0x02, 0x00, 0x04],
                    &vars,
                    &[0x13, 0x02, 0x00, 0x03],
                    WireKind::Read,
                )?;
            } else {
                self.tx_req_in(
                    golden,
                    0x12,
                    &[0x13, 0x02, 0x00, 0x04],
                    &vars,
                    &[0x13, 0x02, 0x00, 0x05],
                    WireKind::Read,
                )?;
            }
            tx += 1;
            beat!();
        }
        stage = BootStage::Setlist;
        // T2: setlist 5 entradas
        for i in 0u8..5 {
            self.tx_req(golden, 0x11, &[0x12, 0x00, 0x10, 0x12], &[i])?;
            tx += 1;
            beat!();
        }
        stage = BootStage::Names;
        // T3: nomes — FIRE-AND-FORGET (D4; captura S1: 61 leituras, só 57
        // respostas — o device omitiu 4 do banco 00 [idx 01,03,06,0a] e o
        // Suite seguiu): respostas = pushes de contexto (D2) → backlog (D7);
        // esperar por-leitura estouraria Timeout (D6). Vars = [banco, idx]:
        // o template é mixed 2 vars + const 0000 (o golden completa a cauda).
        for bank in 0u8..3 {
            for idx in 0u8..16 {
                self.send_build(
                    golden,
                    0x11,
                    &[0x11, 0x00, 0x00, 0x08],
                    &[bank, idx],
                    WireKind::Read,
                )?;
                tx += 1;
                beat!();
            }
        }
        for idx in 0u8..13 {
            self.send_build(
                golden,
                0x11,
                &[0x11, 0x00, 0x00, 0x08],
                &[3, idx],
                WireKind::Read,
            )?;
            tx += 1;
            beat!();
        }
        stage = BootStage::Keepalive;
        // T4: keepalive ×2 (D4) — contam como transação para o progresso.
        //
        // Um transporte que NÃO deixa escrever (ADR-5 com a trava fechada)
        // recusaria este frame, e o boot inteiro cairia no ÚLTIMO passo —
        // um build de leitura ficaria sem leitura. Por isso ele é OMITIDO,
        // não mandado e ignorado: a contagem do relatório é do que de fato
        // saiu no fio. Decisão do owner (06/10, face (A) da #126); a
        // alternativa registrada em `tests/write_gate.rs` era exigir
        // `write-verified` no passo B5 do H1 — a leitura não podia existir
        // antes disso.
        if self.transport.permite_escrita() {
            for _ in 0..2 {
                self.send_build(
                    golden,
                    0x12,
                    &[0x00, 0x02, 0x00, 0x01],
                    &[],
                    WireKind::Write,
                )?;
                tx += 1;
                beat!();
            }
        }
        Ok(BootReport { transactions: tx })
    }

    /// O pp corrente (atualizado por [`Session::select_preset`] e pelo
    /// scan do [`Session::boot`]).
    pub fn current_pp(&self) -> u16 {
        self.current_pp
    }

    /// Seleciona um preset no device (§13.10): select `11/13010000 [pp]`
    /// esperando o meta6 push `12/13010001` (D1; o MESMO endpoint tem o
    /// push espontâneo do boot — D2 resolve pelo contexto do pedido, como
    /// no ciclo do scan provado pelo replay). Atualiza o pp corrente.
    ///
    /// **Trava de faixa no aparelho (#132/ADR-12 — a irmã da ADR-10).**
    /// Quando o transporte É o aparelho, um `pp` fora do inventário é
    /// recusado com [`ProtocolError::ValueOutOfRange`] **antes** de existirem
    /// bytes: o espaço do pedal é o que a captura prova
    /// ([`inventario_do_aparelho`]) e um `select` fora dele é o frame do
    /// assert `PresetNum < TOTAL_PA` (`audio.c:912`) — que derruba o
    /// firmware até o power-cycle. No mock nada muda: ele continua aceitando
    /// qualquer `pp` (o espaço dele é o do documento, e os testes varrem pps
    /// arbitrários).
    ///
    /// # Erros
    /// [`ProtocolError::ValueOutOfRange`] (pp fora do inventário, no
    /// aparelho) e os erros do fio de antes (timeout D6, shape D5, ack).
    pub fn select_preset(&mut self, pp: u16) -> Result<(), ProtocolError> {
        if self.transport.e_aparelho() {
            let inventario = self.inventory();
            if !inventario.contains(&pp) {
                return Err(ProtocolError::ValueOutOfRange {
                    addr: "11/13010000".into(),
                    param: "pp".into(),
                    got: format!("{pp:#06x}"),
                    allowed: faixa_legivel(&inventario),
                });
            }
        }
        let golden = GoldenFile::embedded()?;
        let pp_be = pp.to_be_bytes();
        self.tx_req_in(
            golden,
            0x11,
            &[0x13, 0x01, 0x00, 0x00],
            &pp_be,
            &[0x13, 0x01, 0x00, 0x01],
            WireKind::Read,
        )?;
        self.current_pp = pp;
        Ok(())
    }

    /// Uma página (0..=8) do preset selecionado (§13.10): req
    /// `12/13010004 [pp][PG]01` — PG 0..7 respondem em `13010003` (by-len
    /// 196/32B), PG 8 em `13010005` (4B). Layout byte-a-byte da 13xx segue
    /// Fora de escopo: a página devolve os bytes crus em [`StatePage`].
    pub fn state_page(&mut self, page: u8) -> Result<StatePage, ProtocolError> {
        if page > 8 {
            return Err(ProtocolError::InvalidShape {
                expected: "página 0..=8 (§13.10)".into(),
                got: format!("{page}"),
            });
        }
        let golden = GoldenFile::embedded()?;
        let pp_be = self.current_pp.to_be_bytes();
        let vars = [pp_be[0], pp_be[1], page];
        let in_addr: &[u8; 4] = if page < 8 {
            &[0x13, 0x01, 0x00, 0x03]
        } else {
            &[0x13, 0x01, 0x00, 0x05]
        };
        let payload = self.tx_req_in(
            golden,
            0x12,
            &[0x13, 0x01, 0x00, 0x04],
            &vars,
            in_addr,
            WireKind::Read,
        )?;
        Ok(StatePage { raw: payload })
    }

    /// Página 0 do pp corrente (equivalente a
    /// [`Session::state_page`]`(0)` — mantida para os contratos existentes).
    pub fn scan_state(&mut self) -> Result<StatePage, ProtocolError> {
        self.state_page(0)
    }

    /// As 9 páginas brutas de um pp, guardadas pelo scan (T5).
    ///
    /// `None` antes do boot (ou para pp fora do inventário escaneado) — é
    /// a matéria-prima de [`crate::preset_pages::decode`]: o MESMO payload
    /// que [`Session::state_page`] mandaria buscar agora, só que sem tocar
    /// no fio (o boot já leu tudo).
    pub fn preset_state(&self, pp: u16) -> Option<&[StatePage; 9]> {
        self.pages.get(&pp)
    }

    /// Pps com cache de páginas, em ordem crescente (o escaneado no boot).
    pub fn cached_pps(&self) -> Vec<u16> {
        self.pages.keys().copied().collect()
    }

    /// Knob da UI (§13.11): `codec::set_param` fire-and-forget, SEM
    /// read-back (captura não tem resposta — a FSM não inventa uma, D4).
    /// `chain_slot` = 1..=9, posição na CADEIA — NÃO confundir com o
    /// `ir_slot` de [`Session::upload_ir`] (0..=19; rev.2 do ADR-6).
    pub fn set_param(
        &mut self,
        chain_slot: u8,
        code: u32,
        ctrl: u8,
        value: f32,
    ) -> Result<(), ProtocolError> {
        let sysex = crate::codec::set_param(chain_slot, code, ctrl, value)?;
        self.transport
            .send_raw(&sysex, WireKind::Write)
            .map_err(tx_err)
    }

    /// Save (§13.12 RE-DERIVADO — D3): envia `codec::meta_block(pp, pp_type,
    /// name)` seguido do ciclo de ops CAPTURADO na S4: op 0 com o meta,
    /// op 0 de novo em +578ms, op 1 ×2 em +593ms; FIM dos writes = commit.
    /// ZERO IN esperado. Persistência no display é da pedaleira (S4,
    /// BLOCKERS 11); "salvou?" em H2 = display/`list_user_irs`, NUNCA
    /// interpretar burst 11xx tardio como confirmação.
    pub fn save_preset(&mut self, pp: u16, pp_type: u16, name: &str) -> Result<(), ProtocolError> {
        for s in save_frames(pp, pp_type, name)? {
            self.transport
                .send_raw(&s.sysex, WireKind::Write)
                .map_err(tx_err)?;
        }
        Ok(())
    }

    /// IR (§13.7): `ir_begin(ir_slot)` + chunks 15B/33B esperando o ACK
    /// `[slot][idx u16 BE][01]` POR chunk (timeout ADR-3 cada). DUPLICAR o
    /// último chunk é regra da FSM (capturado, idx 0x0226 slot 1) — nunca do
    /// codec. `ir_slot` = 0..=19; `blob` tem de ser múltiplo de 15B (strict,
    /// rev.2: pedaço final é REJEITADO, não padado).
    pub fn upload_ir(&mut self, ir_slot: u8, blob: &[u8]) -> Result<IrUploadReport, ProtocolError> {
        if ir_slot >= IR_SLOTS {
            return Err(ProtocolError::InvalidShape {
                expected: "ir_slot 0..=19".into(),
                got: format!("{ir_slot}"),
            });
        }
        // O blob vazio é um `.ir` de 0 bytes escolhido pelo dono. `0` é
        // múltiplo de 15 (passaria na checagem de baixo), mas deixa a lista
        // de chunks VAZIA — e o marcador de fim indexa `chunks[len - 1]`,
        // que com lista vazia dá panic por underflow, não erro. O panic
        // morre do lado do Tauri sem mensagem: a tela precisa de um erro que
        // ela saiba traduzir.
        if blob.is_empty() {
            return Err(ProtocolError::InvalidShape {
                expected: "blob com ao menos 1 chunk (15B)".into(),
                got: "0 bytes".into(),
            });
        }
        if !blob.len().is_multiple_of(IR_CHUNK_BYTES) {
            return Err(ProtocolError::InvalidShape {
                expected: "blob múltiplo de 15B (strict, rev.2)".into(),
                got: format!("{} bytes", blob.len()),
            });
        }
        self.transport
            .send_raw(&crate::codec::ir_begin(ir_slot)?, WireKind::Write)
            .map_err(tx_err)?;
        let chunks = blob.as_chunks::<IR_CHUNK_BYTES>().0;
        let mut acks = 0usize;
        for (i, chunk) in chunks.iter().enumerate() {
            // idx em PÁGINAS de 128 (§13.7): base = página×256 — F7 nunca
            // aparece no índice (razão provável dos gaps)
            let idx = ((i / 128) as u16) * 256 + (i % 128) as u16;
            let sysex = crate::codec::ir_chunk(ir_slot, idx, chunk)?;
            self.transport
                .send_raw(&sysex, WireKind::Write)
                .map_err(tx_err)?;
            // D1: ACK [slot][idx BE][01] no endpoint 12/12001002, com
            // filtro D7 (pushes de outros endpoints vão para o backlog)
            let payload = self.wait_for(0x12, &[0x12, 0x00, 0x10, 0x02])?;
            let expect = crate::codec::ir_chunk_ack_payload(ir_slot, idx);
            if payload.as_slice() != expect {
                return Err(ProtocolError::UnexpectedAck {
                    addr: "12/12001002".into(),
                    got: format!(
                        "esperado {}, chegou {}",
                        hex_short(&expect),
                        hex_short(&payload)
                    ),
                });
            }
            acks += 1;
        }
        // marcador de fim: o ÚLTIMO chunk é enviado 2× NO TOTAL (a captura
        // tem 296 sends/ACKs por slot = 295 únicos + 1 repetição do idx
        // 0x226 — §13.7 corrigido); payload real, 1 envio extra, 1 ACK
        let last: &[u8] = &chunks[chunks.len() - 1];
        let last_idx = ((chunks.len() - 1) / 128) as u16 * 256 + ((chunks.len() - 1) % 128) as u16;
        let sysex = crate::codec::ir_chunk(ir_slot, last_idx, last)?;
        self.transport
            .send_raw(&sysex, WireKind::Write)
            .map_err(tx_err)?;
        let payload = self.wait_for(0x12, &[0x12, 0x00, 0x10, 0x02])?;
        let expect = crate::codec::ir_chunk_ack_payload(ir_slot, last_idx);
        if payload.as_slice() != expect {
            return Err(ProtocolError::UnexpectedAck {
                addr: "12/12001002".into(),
                got: format!(
                    "esperado {}, chegou {}",
                    hex_short(&expect),
                    hex_short(&payload)
                ),
            });
        }
        acks += 1;
        Ok(IrUploadReport {
            slot: ir_slot,
            chunks: chunks.len(),
            acks,
            bytes: blob.len(),
        })
    }

    /// SnapTone/NAM (§5): stream de blocos `cmd=0x92` com o **ACK de 16B de
    /// cada bloco** esperado antes do próximo (§2/§5), e settle de
    /// [`SNAP_TONE_SETTLE_MS`] entre operações (§4, regra 1 — loop tight
    /// trava a pedaleira).
    ///
    /// `model` são os bytes do arquivo **já convertido** (`.clo`), nunca um
    /// `.nam`: a conversão acontece no desktop (strings do Suite em
    /// `exe_strings.txt`) e o crate transporta bytes, não fabrica modelo.
    ///
    /// `slot` é 1..=5 (`SnapTone1..5`, §5) e é validado aqui — mas ele **não
    /// vai no fio**: o layout de bloco evidenciado (`[crc, 0x92, index,
    /// length, payload]`, §2) não tem campo de slot, e inventar um comando de
    /// seleção seria inventar campo sem captura (R1). O relatório carrega o
    /// slot para a UI mostrar o que ela pediu; quando uma captura de campo
    /// mostrar como o slot é escolhido, é um comando novo ao lado deste
    /// método, e nada aqui precisa mudar.
    ///
    /// # Erros
    /// [`ProtocolError::InvalidShape`] (slot fora de 1..=5, modelo vazio),
    /// [`ProtocolError::Timeout`] (D6 — ACK que não veio na janela ADR-3) e
    /// [`ProtocolError::DeviceGone`] (transporte).
    pub fn upload_snap_tone(
        &mut self,
        slot: u8,
        model: &[u8],
    ) -> Result<SnapToneUploadReport, ProtocolError> {
        self.upload_snap_tone_settled(slot, model, Duration::from_millis(SNAP_TONE_SETTLE_MS))
    }

    /// O upload de [`Session::upload_snap_tone`] com o **settle injetado**.
    ///
    /// Existe separado porque o settle é caro de propósito: um modelo de
    /// ~2,7 KB são 143 blocos, e 143 × 250 ms dão ~36 s por upload. Um teste
    /// que pagasse esse relógio provaria que o tempo passa, não que a FSM
    /// envia a ordem certa — e o contrato que importa aqui é a ORDEM dos
    /// blocos e o ACK de cada um. O piso de §4 tem prova própria
    /// (`tests/snap_tone_upload.rs::o_settle_entre_blocos_e_real`), que mede
    /// o tempo de verdade com um settle pequeno mas não nulo.
    pub fn upload_snap_tone_settled(
        &mut self,
        slot: u8,
        model: &[u8],
        settle: Duration,
    ) -> Result<SnapToneUploadReport, ProtocolError> {
        if slot == 0 || slot > crate::snap_tone::SLOTS {
            return Err(ProtocolError::InvalidShape {
                expected: format!(
                    "slot 1..={} (§5: SnapTone1..{})",
                    crate::snap_tone::SLOTS,
                    crate::snap_tone::SLOTS
                ),
                got: format!("{slot}"),
            });
        }
        let blocos = crate::snap_tone::blocks(model)?;
        let mut acks = 0usize;
        for (i, (index, payload)) in blocos.iter().enumerate() {
            let wire = crate::snap_tone::wire_block(*index, payload)?;
            self.transport
                .send_raw(&wire, WireKind::Write)
                .map_err(tx_err)?;
            // D1: o ACK de 16B é DA OPERAÇÃO — antes do próximo bloco, senão
            // a fila do device acumula o stream inteiro e o settle vira a
            // única proteção contra o buffer do driver estourar.
            let ack = self.wait_snap_ack()?;
            crate::snap_tone::check_ack(&ack)?;
            acks += 1;
            // Settle depois do ACK, e só entre operações: depois do ÚLTIMO
            // bloco não há operação seguinte, e esperar ali seria 250 ms de
            // delay devolvidos ao dono sem nenhum ganho.
            if i + 1 < blocos.len() {
                std::thread::sleep(settle);
            }
        }
        Ok(SnapToneUploadReport {
            slot,
            blocks: blocos.len(),
            acks,
            bytes: model.len(),
        })
    }

    /// Espera o ACK de 16B de um bloco de SnapTone (§5).
    ///
    /// **O filtro D7 aqui é o framing, não o endpoint.** A família responde no
    /// framing dela (`F0` + nibbles + `F7`) e o envelope do GP-100 começa com
    /// `F0 21 …` — `decode_envelope` exige o cabeçalho de 8B, que a família
    /// não carrega, então os dois formatos se separam sem ambiguidade (ver
    /// [`crate::snap_tone::eh_familia`]). Tudo que decodifica como envelope
    /// GP-100 é push de endpoint e vai para o backlog como sempre (D7); o que
    /// sobra é o ACK da operação corrente.
    fn wait_snap_ack(&mut self) -> Result<Vec<u8>, ProtocolError> {
        loop {
            let msg = self
                .transport
                .recv_raw(Duration::from_millis(TX_TIMEOUT_MS))
                .map_err(|e| match e {
                    TransportError::RecvTimeout { timeout_ms } => ProtocolError::Timeout {
                        timeout_ms,
                        addr: "snapTone/ack".into(),
                    },
                    TransportError::DeviceGone { why } => ProtocolError::DeviceGone { why },
                    other => ProtocolError::InvalidShape {
                        expected: "transporte saudável".into(),
                        got: other.to_string(),
                    },
                })?;
            if crate::golden::decode_envelope(&msg).is_ok() {
                self.backlog.push(msg); // D7: push da família GP-100, não é o ACK
                continue;
            }
            return crate::snap_tone::ack_body(&msg);
        }
    }

    /// Tabela dos 20 User IRs: req em `12001002`; o by-len de
    /// `match_response` garante que a resposta lida é a TABELA (75B
    /// nibble-exp), não um ACK (4B) — D1.
    pub fn list_user_irs(&mut self) -> Result<UserIrTable, ProtocolError> {
        let mut slots = Vec::new();
        for page in 0u8..0x14 {
            let payload = self.tx_req(
                GoldenFile::embedded()?,
                0x11,
                &[0x12, 0x00, 0x10, 0x02],
                &[page],
            )?;
            // §13.12: [0]=pág/slot; [1..33] nome nibble-exp (0xFF = vazio)
            let name = crate::codec::nibble_collapse(&payload[1..33])?;
            let empty = name.iter().all(|&b| b == 0xFF);
            let s: String = if empty {
                String::new()
            } else {
                name.iter()
                    .take_while(|&&b| b != 0 && b != 0xFF)
                    .map(|&b| b as char)
                    .collect()
            };
            slots.push((payload[0], s));
        }
        Ok(UserIrTable { slots })
    }

    /// Pushes de IN não solicitado acumulados no backlog (D7), para
    /// observação (D8: poll, nunca listener concorrente). A drenagem do
    /// backlog é da operação seguinte compatível.
    pub fn pending_pushes(&mut self) -> Result<Vec<Vec<u8>>, ProtocolError> {
        Ok(std::mem::take(&mut self.backlog))
    }

    /// Uma transação: build_request (desambiguação por var_count) → send →
    /// espera a resposta no endpoint de RESPOSTA (D1/D7). Devolve o payload.
    fn tx_req(
        &mut self,
        golden: &GoldenFile,
        func_out: u8,
        addr: &[u8; 4],
        vars: &[u8],
    ) -> Result<Vec<u8>, ProtocolError> {
        self.tx_req_in(golden, func_out, addr, vars, addr, WireKind::Read)
    }

    /// Igual a [`Session::tx_req`], com endpoint de RESPOSTA explícito —
    /// o fio NÃO responde no mesmo endereço (replay S1): select/open
    /// `13010002` respondem meta6 em `13010001`; página `13010004` (PG 0..7)
    /// responde página em `13010003` (by-len 196/32); PG 8 responde em
    /// `13010005`; select `13010000` (família 1302) responde meta6 em
    /// `13020001`; read `13020004` responde em `13020003`; PG 8 em `13020005`.
    fn tx_req_in(
        &mut self,
        golden: &GoldenFile,
        func_out: u8,
        addr: &[u8; 4],
        vars: &[u8],
        in_addr: &[u8; 4],
        kind: WireKind,
    ) -> Result<Vec<u8>, ProtocolError> {
        let req = golden.build_request(func_out, addr, vars)?;
        self.transport.send_raw(&req, kind).map_err(tx_err)?;
        self.wait_for(0x12, in_addr)
    }

    /// Só envia (write fire-and-forget, D4).
    fn send_build(
        &mut self,
        golden: &GoldenFile,
        func: u8,
        addr: &[u8; 4],
        vars: &[u8],
        kind: WireKind,
    ) -> Result<(), ProtocolError> {
        let req = golden.build_request(func, addr, vars)?;
        self.transport.send_raw(&req, kind).map_err(tx_err)
    }

    /// Espera uma mensagem no endpoint `(func, addr)` e devolve o PAYLOAD
    /// (D1/D7): msgs de outros endpoints vão para o backlog; no endpoint
    /// certo, sem match no golden = InvalidShape (D5); nada na janela ADR-3
    /// = Timeout (D6).
    fn wait_for(&mut self, func: u8, addr: &[u8; 4]) -> Result<Vec<u8>, ProtocolError> {
        loop {
            let msg = self
                .transport
                .recv_raw(Duration::from_millis(TX_TIMEOUT_MS))
                .map_err(|e| match e {
                    TransportError::RecvTimeout { timeout_ms } => ProtocolError::Timeout {
                        timeout_ms,
                        addr: format!("{func:02x}/{}", addr_hex(addr)),
                    },
                    // DeviceGone tem TIPO próprio (o front decide diferente:
                    // LED off + retry explícito, sem retry automático).
                    TransportError::DeviceGone { why } => ProtocolError::DeviceGone { why },
                    other => ProtocolError::InvalidShape {
                        expected: "transporte saudável".into(),
                        got: other.to_string(),
                    },
                })?;
            let (f, a, payload) = crate::golden::decode_envelope(&msg)?;
            if std::env::var_os("GP100_TRACE").is_some() {
                eprintln!(
                    "wait_for {func:02x}/{} <- {f:02x}/{} ({}B)",
                    addr_hex(addr),
                    addr_hex(&a),
                    payload.len()
                );
            }
            if f == func && a == *addr {
                GoldenFile::embedded()?
                    .match_response(f, &a, payload)
                    .ok_or_else(|| ProtocolError::InvalidShape {
                        expected: format!("resposta do golden em {f:02x}/{}", addr_hex(&a)),
                        got: hex_short(payload),
                    })?;
                return Ok(payload.to_vec());
            }
            self.backlog.push(msg); // D7
        }
    }

    /// Devolve o transporte (close()/reuso é do chamador) — `Session` não
    /// possui o ciclo de vida (ADR-4/ADR-6).
    pub fn into_transport(self) -> T {
        self.transport
    }
}

/// Mapeia erro de transporte para erro de transação (D6/D5).
///
/// `DeviceGone` NÃO é achatado em `InvalidShape`: o device ausente é uma
/// falha com tipo próprio (a UI para de tentar e mostra recuperação; o H1
/// trata como divergência de campo).
fn tx_err(e: TransportError) -> ProtocolError {
    match e {
        TransportError::SendFailed { why } => ProtocolError::InvalidShape {
            expected: "frame aceito pelo device".into(),
            got: why,
        },
        TransportError::DeviceGone { why } => ProtocolError::DeviceGone { why },
        other => ProtocolError::InvalidShape {
            expected: "transporte saudável".into(),
            got: other.to_string(),
        },
    }
}

/// Hex curto (até 12 bytes) para diffs de divergência.
fn hex_short(data: &[u8]) -> String {
    let s: String = data.iter().take(12).map(|b| format!("{b:02x}")).collect();
    if data.len() > 12 {
        format!("{s}…(+{})", data.len() - 12)
    } else {
        s
    }
}

/// Addr 4B em hex (mensagens de erro).
fn addr_hex(addr: &[u8; 4]) -> String {
    addr.iter().map(|b| format!("{b:02x}")).collect()
}
