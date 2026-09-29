//! session — FSM de sessão sobre o transporte (ROADMAP M0.6; **ADR-6 rev.3**
//! de `docs/DECISIONS.md`, aceito 29/09 — FONTE ÚNICA das assinaturas).
//!
//! **Este arquivo é ESQUELETO:** as assinaturas estão travadas no ADR-6 e
//! compilam; os corpos são `todo!("M0.6: …")`. A implementação (replay
//! byte-a-byte das fixtures P4, diff hex na divergência) é a issue M0.6.
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
//! - **D6** sem retry no M0.6 — timeout (ADR-3: 3s/transação) sobe como
//!   `Timeout`;
//! - **D7** espera de resposta é LOOP FILTRANTE: msg de endpoint ≠ esperado
//!   vai para o backlog interno (push não solicitado não quebra D1);
//! - **D8** consumidor ÚNICO do stream IN (`&mut self`; observação = poll
//!   do backlog via [`Session::pending_pushes`], nunca listener concorrente).

use crate::golden::GoldenFile;
use crate::transport::DeviceTransport;
use crate::ProtocolError;

/// Relatório do boot+scan (§13.10): nasce MÍNIMO (deriva das vars que os
/// templates extraem) e cresce só quando a UI pedir (ADR-6, YAGNI).
#[derive(Debug, Clone)]
pub struct BootReport {
    /// Nº de transações de boot+scan executadas com sucesso (replay da
    /// sequência capturada: 2299 OUT na S1).
    pub transactions: usize,
}

/// Página de estado do pp corrente (família 13xx). **Opaca de propósito**:
/// o layout byte-a-byte da 13xx está FORA de escopo (ROADMAP); carrega os
/// bytes crus + as vars extraídas pelos templates do golden.
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

/// Relatório do upload de IR (§13.7).
#[derive(Debug, Clone)]
pub struct IrUploadReport {
    /// Slot de destino (0..=19).
    pub slot: u8,
    /// Chunks de dados enviados (exclui o frame final duplicado idx 0x226).
    pub chunks: usize,
    /// ACKs validados por chunk (D1; marcador de fim = duplicação do
    /// último chunk idx 0x226 — payload é a cauda REAL do blob, §13.7).
    pub acks: usize,
}

/// FSM de sessão — genérica sobre o transporte (ADR-4/ADR-6).
pub struct Session<T: DeviceTransport> {
    transport: T,
}

impl<T: DeviceTransport> Session<T> {
    /// Constrói a FSM sobre o transporte. NÃO abre o transporte (ciclo de
    /// vida é do chamador — ADR-4/ADR-6). O golden vem de
    /// `GoldenFile::embedded()` (1 parse por processo) nos métodos — não é
    /// campo (rev.2: valor único `&'static` em campo é ruído de API).
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    /// Boot + scan (§13.10): replay da sequência capturada via
    /// `GoldenFile::build_request` + `match_response` por transação.
    ///
    /// # Erros
    /// [`ProtocolError::Timeout`] (D6), [`ProtocolError::InvalidShape`]
    /// (D5) — e [`ProtocolError::UnexpectedAck`] se o IN não casar com o
    /// que a transação pediu (D1/D3).
    pub fn boot(&mut self) -> Result<BootReport, ProtocolError> {
        let _ = GoldenFile::embedded(); // amarra a dependência (M0.6 implmenta)
        todo!("M0.6: boot+scan §13.10 (replay boot.jsonl, caso congelado vs geral via var_count)")
    }

    /// Página de estado do pp corrente (família 13xx; dispatch por CONTEXTO,
    /// regra D2). Layout byte-a-byte da 13xx segue FORA (ROADMAP).
    pub fn scan_state(&mut self) -> Result<StatePage, ProtocolError> {
        todo!("M0.6: página de estado 13xx (D2: contexto decide semântica)")
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
        let _ = (chain_slot, code, ctrl, value);
        todo!("M0.6: set_param §13.11 via codec::set_param (D4: sem drain)")
    }

    /// Save (§13.12 RE-DERIVADO — D3): envia `codec::meta_block(pp, pp_type,
    /// name)` seguido do ciclo de ops CAPTURADO na S4: op 0 com o meta,
    /// op 0 de novo em +578ms, op 1 ×2 em +593ms; FIM dos writes = commit.
    /// ZERO IN esperado. Persistência no display é da pedaleira (S4,
    /// BLOCKERS 11); "salvou?" em H2 = display/`list_user_irs`, NUNCA
    /// interpretar burst 11xx tardio como confirmação.
    pub fn save_preset(&mut self, pp: u16, pp_type: u16, name: &str) -> Result<(), ProtocolError> {
        let _ = (pp, pp_type, name);
        todo!("M0.6: meta_block + ciclo de ops da S4 (D3: fire-and-forget, sem drain)")
    }

    /// IR (§13.7): `ir_begin(ir_slot)` + chunks 15B/33B esperando o ACK
    /// `[slot][idx u16 BE][01]` POR chunk (timeout ADR-3 cada). DUPLICAR o
    /// último chunk é regra da FSM (capturado, idx 0x0226 slot 1) — nunca do
    /// codec. `ir_slot` = 0..=19; `blob` tem de ser múltiplo de 15B (strict,
    /// rev.2: pedaço final é REJEITADO, não padado).
    pub fn upload_ir(&mut self, ir_slot: u8, blob: &[u8]) -> Result<IrUploadReport, ProtocolError> {
        let _ = (ir_slot, blob);
        todo!("M0.6: begin + chunks + ACK por chunk (D1/D6) + último chunk duplicado")
    }

    /// Tabela dos 20 User IRs: req em `12001002`; o by-len de
    /// `match_response` garante que a resposta lida é a TABELA (75B
    /// nibble-exp), não um ACK (4B) — D1.
    pub fn list_user_irs(&mut self) -> Result<UserIrTable, ProtocolError> {
        todo!("M0.6: req 12001002 + decode nome+CRC (§13.12; CRC de ocupado é pendência)")
    }

    /// Pushes de IN não solicitado acumulados no backlog (D7), para
    /// observação (D8: poll, nunca listener concorrente). A drenagem do
    /// backlog é da operação seguinte compatível.
    pub fn pending_pushes(&mut self) -> Result<Vec<Vec<u8>>, ProtocolError> {
        todo!("M0.6: accessor do backlog D7 (D8: poll)")
    }

    /// Devolve o transporte (close()/reuso é do chamador) — `Session` não
    /// possui o ciclo de vida (ADR-4/ADR-6).
    pub fn into_transport(self) -> T {
        self.transport
    }
}
