//! session — FSM de sessão sobre o transporte (**ADR-6 rev.3** de
//! `docs/dev/DECISIONS.md` — FONTE ÚNICA das assinaturas).
//!
//! O replay byte-a-byte das fixtures P4 (diff hex na divergência) prova a FSM.
//!
//! Regras de dispatch — resumo (FONTE ÚNICA = ADR-6, não reabrir aqui):
//! - **D1** a transação é dona do endpoint: quem pediu interpreta a PRÓXIMA
//!   msg do `(func, addr)` via `GoldenFile::match_shape` (by-len resolve
//!   `12001002`: ACK 4B × tabela 75B). **FORMA só** (#177): o conteúdo da
//!   resposta é do hardware e passa pelo decode dos objetos — nunca é
//!   comparado com os bytes `const` do golden (snapshot de análise);
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
//!
//! **Layout do módulo (#257):** este arquivo é o HANDLE — a `Session` (o que a
//! FSM POSSUI) e o ciclo de vida do transporte (`new`/`into_transport`). O resto
//! vive nos submódulos, um por assunto: [`tipos`] (DTOs), [`transacao`] (o
//! pedido→resposta), [`boot`] (o script §13.10), [`paginas`] (13xx + cache),
//! [`escrita`] (o que manda byte), [`globais`] (§13.13), [`leitura`] (o que só
//! pergunta) e [`inventario`] (o espaço de pps). Todo caminho público continua
//! `crate::session::<Item>` — os `pub use` abaixo são o contrato.

mod boot;
mod escrita;
mod globais;
mod inventario;
mod leitura;
mod paginas;
mod tipos;
mod transacao;

pub use escrita::{save_frames, SaveFrame};
pub use globais::REGS_GLOBAIS;
pub use inventario::{inventario_do_aparelho, pp_e_valido};
pub use tipos::{
    BootProgress, BootProgressFn, BootReport, BootStage, GlobalBruto, IrUploadReport,
    SnapToneUploadReport, StatePage, UserIrTable, IR_CHUNK_BYTES, IR_SLOTS, SNAP_TONE_SETTLE_MS,
};

use std::collections::BTreeMap;

use crate::transport::DeviceTransport;

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
    pub(super) fn inventory(&self) -> Vec<u16> {
        self.pps.clone().unwrap_or_else(|| {
            if self.transport.e_aparelho() {
                inventario_do_aparelho()
            } else {
                (0u16..198).collect()
            }
        })
    }

    /// O pp corrente (atualizado por [`Session::select_preset`] e pelo
    /// scan do [`Session::boot`]).
    pub fn current_pp(&self) -> u16 {
        self.current_pp
    }

    /// Devolve o transporte (close()/reuso é do chamador) — `Session` não
    /// possui o ciclo de vida (ADR-4/ADR-6).
    pub fn into_transport(self) -> T {
        self.transport
    }
}
