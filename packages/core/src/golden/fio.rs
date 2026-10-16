//! o FIO: hex ASCII em bytes e o SysEx completo separado em `(func, addr, data)`.

use super::shape;
use crate::{ProtocolError, SYSEX_EOX, SYSEX_HEADER};

/// Decodifica hex ASCII em bytes (pares; caixa alta/baixa).
///
/// # Erros
/// [`ProtocolError::InvalidShape`] com offset do par inválido.
pub fn hex_decode(s: &str) -> Result<Vec<u8>, ProtocolError> {
    let b = s.as_bytes();
    if !b.len().is_multiple_of(2) {
        return Err(shape(
            "hex com número par de dígitos",
            format!("len={}", b.len()),
        ));
    }
    let mut out = Vec::with_capacity(b.len() / 2);
    for (i, pair) in b.as_chunks::<2>().0.iter().enumerate() {
        let hi = (pair[0] as char).to_digit(16);
        let lo = (pair[1] as char).to_digit(16);
        match (hi, lo) {
            (Some(h), Some(l)) => out.push((h * 16 + l) as u8),
            _ => {
                return Err(shape(
                    "dígito hexadecimal",
                    format!("byte {i}: '{}{}'", pair[0] as char, pair[1] as char),
                ))
            }
        }
    }
    Ok(out)
}

/// Separa um SysEx completo em `(func, addr[4], data)` — trim no 1º `F7`
/// (regra de captura do knowledge.md; em mensagens construídas é o próprio
/// fim). Valida o cabeçalho contra [`SYSEX_HEADER`].
///
/// # Erros
/// [`ProtocolError::InvalidShape`] se header/F7/comprimento não baterem
/// (inclui truncamento SEM-HDR do ring buffer do proxy).
pub fn decode_envelope(data: &[u8]) -> Result<(u8, [u8; 4], &[u8]), ProtocolError> {
    if data.len() < SYSEX_HEADER.len() + 5 + 1 {
        return Err(shape(
            "SysEx completo (header 8 + func 1 + addr 4 + data + F7)",
            format!("recebidos {} bytes", data.len()),
        ));
    }
    if data[..SYSEX_HEADER.len()] != SYSEX_HEADER {
        return Err(shape(
            "cabeçalho F0 21 25 7F 47 50 2D 64",
            format!(
                "começo: {}",
                String::from_utf8_lossy(&data[..8.min(data.len())])
            ),
        ));
    }
    let end = data[SYSEX_HEADER.len()..]
        .iter()
        .position(|&b| b == SYSEX_EOX)
        .map(|i| SYSEX_HEADER.len() + i)
        .ok_or_else(|| shape("F7 terminador", "ausente (mensagem truncada?)"))?;
    let body = &data[SYSEX_HEADER.len()..end];
    if body.len() < 5 {
        return Err(shape(
            "func + addr(4) no corpo",
            format!("corpo de {} bytes", body.len()),
        ));
    }
    let mut addr = [0u8; 4];
    addr.copy_from_slice(&body[1..5]);
    Ok((body[0], addr, &body[5..]))
}
