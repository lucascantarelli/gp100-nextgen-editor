//! o que MANDA byte ao aparelho: select, knob (D4), save (D3), IR (§13.7) e SnapTone (§5).
//!
//! Regra que vale para todas: a escrita é PRIVILÉGIO — a trava mora no
//! TRANSPORTE (ADR-5, feature `write-verified`), e nenhuma função daqui recria
//! política. `save_frames` existe separado do `save_preset` porque o dry-run do
//! CLI e a prévia do app precisam dos MESMOS bytes, de uma fonte só.

use std::time::Duration;

use crate::golden::GoldenFile;
use crate::transport::{DeviceTransport, TransportError, WireKind};
use crate::ProtocolError;

use super::inventario::faixa_legivel;
use super::tipos::{
    IrUploadReport, SnapToneUploadReport, IR_CHUNK_BYTES, IR_SLOTS, SNAP_TONE_SETTLE_MS,
};
use super::transacao::{hex_short, tx_err, TX_TIMEOUT_MS};
use super::Session;

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

impl<T: DeviceTransport> Session<T> {
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
}
