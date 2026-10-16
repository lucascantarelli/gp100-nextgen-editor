//! a FAMILIA GP-50 (SnapTone/NAM, §5): o segundo framing do mock.

use super::MockDevice;
use crate::transport::TransportError;

impl MockDevice {
    /// O modelo de SnapTone remontado e quantas transferências chegaram.
    ///
    /// O par vem junto porque os dois contam a MESMA coisa vista de dois
    /// lados: uma transferência com N bytes pode ser 1 upload ou vários
    /// índices 0 seguidos, e só os dois juntos distinguem.
    pub fn snap_tone(&self) -> (&[u8], usize) {
        (&self.state.snap_tone_model, self.state.snap_tone_transfers)
    }

    /// Despacha um frame do framing da FAMÍLIA (`F0` + nibbles + `F7`): o
    /// SnapTone/NAM (§5), que NÃO usa o envelope do GP-100.
    ///
    /// O que o mock valida (e recusa com D5, incrementando `rejected`):
    /// o CRC-8 do BUF (§3), o `length` contra o payload realmente recebido, e
    /// o `command`. O que ele **remonta** é o modelo, para o upload ser
    /// verificável de ponta a ponta sem hardware — o teste pede 2 700 bytes,
    /// o mock devolve os mesmos 2 700.
    ///
    /// O conteúdo do ACK é **zeros**: o ACK tem 16 bytes (§5), mas o que
    /// esses 16 bytes dizem não está evidenciado, e o mock não inventa
    /// semântica (D5). A FSM valida o tamanho — que é o que a evidência
    /// sustenta.
    pub(super) fn ingest_familia(&mut self, msg: &[u8]) -> Result<(), TransportError> {
        let b = crate::codec::nibble_collapse(&msg[1..msg.len() - 1])
            .map_err(|e| TransportError::SendFailed { why: e.to_string() })?;
        // BUF mínimo = [crc, command, index, length] (§2).
        if b.len() < 4 {
            self.state.rejected += 1;
            return Err(TransportError::SendFailed {
                why: format!("BUF de familia com {} bytes (minimo 4, §2)", b.len()),
            });
        }
        // CRC (§3): o byte do CRC entra ZERADO no cálculo — comparar o valor
        // gravado contra o calculado sobre o BUF com o byte zerado.
        let mut com_crc_zero = b.clone();
        com_crc_zero[0] = 0;
        if crate::snap_tone::crc8(&com_crc_zero) != b[0] {
            self.state.rejected += 1;
            return Err(TransportError::SendFailed {
                why: format!(
                    "CRC do bloco: esperado {:02X}, recebeu {:02X}",
                    crate::snap_tone::crc8(&com_crc_zero),
                    b[0]
                ),
            });
        }
        let (cmd, index, len) = (b[1], b[2], usize::from(b[3]));
        let payload = &b[4..];
        if cmd != crate::snap_tone::CMD_SNAP_TONE {
            self.state.rejected += 1;
            return Err(TransportError::SendFailed {
                why: format!("command da familia {cmd:02X} != SnapTone 92"),
            });
        }
        if payload.len() != len {
            self.state.rejected += 1;
            return Err(TransportError::SendFailed {
                why: format!("length {len} != payload de {} bytes", payload.len()),
            });
        }
        // Índice 0 = transferência nova. O fio NÃO carrega o slot (R1: só o
        // layout do bloco está evidenciado, e nele não há slot), então o
        // mock reabre o stream no 0 — que é o único sinal de começo que a
        // evidência sustenta.
        if index == 0 {
            self.state.snap_tone_model.clear();
            self.state.snap_tone_transfers += 1;
        }
        self.state.snap_tone_model.extend_from_slice(payload);
        self.state.snap_tone_acks += 1;
        let ack = {
            let mut m = vec![0xF0];
            m.extend(crate::codec::nibble_expand(
                &[0u8; crate::snap_tone::ACK_LEN],
            ));
            m.push(crate::SYSEX_EOX);
            m
        };
        self.queue_raw(ack);
        Ok(())
    }
}
