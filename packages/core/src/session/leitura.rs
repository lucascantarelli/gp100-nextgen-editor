//! o que só PERGUNTA: a tabela dos 20 User IRs (§13.12), as 5 entradas de 12001012
//! (T2, semântica aberta) e o backlog de pushes (D7/D8).

use crate::golden::GoldenFile;
use crate::transport::DeviceTransport;
use crate::ProtocolError;

use super::tipos::UserIrTable;
use super::Session;

impl<T: DeviceTransport> Session<T> {
    /// Tabela dos 20 User IRs: req em `12001002`; o by-len de
    /// `match_shape` garante que a resposta lida é a TABELA (75B
    /// nibble-exp), não um ACK (4B) — D1. Os nomes vêm do hardware
    /// (nibble-collapse do payload), sem comparação com golden.
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

    /// As 5 leituras de `11/12001012` do boot (T2), devolvidas CRUAS.
    ///
    /// É a etapa que o repo chamava de "setlist" e agora se chama
    /// [`BootStage::State5`]: a GP-100 **não tem função setlist** no manual
    /// (`docs/referencias/GP100_DEVICE.md` §5#1). O payload é 44B nibble-exp com cara de
    /// `12 10 2c 00 [i] 00 01 …` (PROTOCOL §13.3) — a semântica fina ainda NÃO
    /// está decifrada; este método existe justamente para medir em campo
    /// (CLI `read-state5 --log`) sem inventar decode.
    ///
    /// Leitura pura: não seleciona preset, não muda estado (o mesmo req T2 do
    /// boot, provado pelo replay).
    pub fn read_state5(&mut self) -> Result<Vec<Vec<u8>>, ProtocolError> {
        let golden = GoldenFile::embedded()?;
        let mut entradas = Vec::with_capacity(5);
        for i in 0u8..5 {
            entradas.push(self.tx_req(golden, 0x11, &[0x12, 0x00, 0x10, 0x12], &[i])?);
        }
        Ok(entradas)
    }

    // ─── Registradores globais (§13.13 — sessão G3–G6/G7 de 11/10) ────

    /// Pushes de IN não solicitado acumulados no backlog (D7), para
    /// observação (D8: poll, nunca listener concorrente). A drenagem do
    /// backlog é da operação seguinte compatível.
    pub fn pending_pushes(&mut self) -> Result<Vec<Vec<u8>>, ProtocolError> {
        Ok(std::mem::take(&mut self.backlog))
    }
}
