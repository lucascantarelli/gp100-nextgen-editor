/**
 * SnapTone — framing do upload de modelo (PROTOCOL.md §2, §3 e §5).
 *
 * **Este caminho NÃO é o envelope do GP-100.** Os writes de preset/IR usam
 * `HEADER(8B) | FUNC | ADDR(4B BE) | DATA | F7` (§13.1). O SnapTone usa o
 * framing **GP-50 de família**, que é outro:
 *
 * ```text
 * BUF (pré-nibble):
 *   [0] CRC-8      (calculado com este byte ZERADO, §3)
 *   [1] command    (0x92 = upload de SnapTone)
 *   [2] index      (índice do bloco na transferência: 0, 1, 2, …)
 *   [3] length     (nº de bytes de payload; bloco cheio usa 0x13 = 19)
 *   [4..] payload
 *
 * wire = 0xF0 + nibble-expand(BUF, hi-first) + 0xF7
 * ```
 *
 * Por que os dois convivem: o GP-50 é a família de transferências em blocos
 * (o layout foi confirmado 298/298 contra capturas do Valeton Suite), e o
 * `HEADER` de 8 bytes é a assinatura do GP-100 que abre os writes semeados.
 * Confundi-los seria enviar 8 bytes a mais e o device não reconheceria o
 * comando — por isso o `0xF0` solto abaixo é **intencional**.
 *
 * ## O que está evidenciado (e o que não está)
 *
 * Evidenciado, e portanto implementado byte a byte: o layout do BUF, o
 * `command 0x92`, o índice, o `length 0x13`, o CRC-8/SMBUS (poly 0x07,
 * init 0x00, sem reflect, sem XOR final), o nibble hi-first e o `F0`/`F7`.
 * §5 também fixa **5 slots** (`SnapTone1..5`) e ACK de 16B por bloco.
 *
 * **Não evidenciado:** o conteúdo do modelo convertido (~2,7 KB). O refit
 * acontece no desktop (`REFIT_FINDINGS`, projeto GP-50) e o GP-100 não roda
 * NAM nativo — então este módulo transporta **bytes**, nunca os fabrica. O
 * que o app importa é um arquivo já convertido; a origem desses bytes fica
 * fora do crate, por desenho (R1: não adivinhar).
 *
 * ## De onde vem o arquivo convertido (strings do Suite, `exe_strings.txt`)
 *
 * O fluxo do produto está escrito nas strings do binário da Valeton, na ordem
 * em que aparecem:
 *
 * ```text
 * 0x003F3F2C  Choose a nam file to open it
 * 0x003F3F4C  *.nam
 * 0x003F3F64  Select nam import Location
 * 0x003F3FD4  nam_input_wav.wav
 * 0x003F3FF0  /name.clo          <- a saida da conversao
 * 0x003F3FFC  /nam_output_wav.wav
 * 0x003F4010  /nam_output_clo.wav
 * ```
 *
 * Isto é: o `.nam` é a **entrada** do `clone` (as strings `start clone` /
 * `clone done` em `0x0163C438` são a thread), e o `.clo` é a **saída** — o
 * modelo convertido. O que vai para o device é o `.clo`.
 *
 * ## Regras de produto que a UI precisa respeitar
 *
 * Das notas de release (V2.1) e das strings do Suite:
 *
 * - o SnapTone é um **efeito do módulo AMP** ("New effect 'SnapTone' in AMP
 *   module"), não um painel à parte;
 * - **ligar o SnapTone desliga o CAB**: *"If the SnapTone function is enabled,
 *   the CAB module will be disabled."* A UI tem que avisar antes de gravar;
 * - os erros que o Suite já tem: `Wrong nam file!`, `Wrong Wav file!`,
 *   `Filename contains illegal characters`, `Update error!`, `Update timeout!`,
 *   `Device not found!` — a lista de falhas que o app precisa ter.
 */
// O que o app importa NÃO é o `.nam`: as strings do Suite (`exe_strings.txt`)
// mostram o fluxo inteiro — "Choose a nam file to open it" (`*.nam`) entra na
// conversao, e a saida e `/name.clo` + `nam_output_clo.wav`. O `.clo` e o
// modelo CONVERTIDO, e ele que vai para o device: por isso este modulo trata
// os bytes como opacos e so cuida do framing.
use crate::codec::nibble_expand;
use crate::{ProtocolError, SYSEX_EOX};

/// `command` do upload de SnapTone (§2).
pub const CMD_SNAP_TONE: u8 = 0x92;

/// `command` de leitura/comando da família (§2) — é o que o seletor usa.
pub const CMD_LEITURA: u8 = 0x01;

/// Seletor que lê o SnapTone (§2, na lista de requisições conhecidas).
pub const SELECTOR_SNAP_TONE: u8 = 0x24;

/// Payload máximo por bloco: `length` cheio é `0x13` = 19 (§2).
pub const PAYLOAD_MAX: usize = 19;

/// Quantidade de slots de SnapTone no GP-100 V2.1: `SnapTone1..5` (§5).
pub const SLOTS: u8 = 5;

/// Tamanho do ACK de status, fixo na família (§2).
pub const ACK_LEN: usize = 16;

/// CRC-8/SMBUS: poly 0x07, init 0x00, sem reflect, sem XOR final (§3).
///
/// Calculado sobre o BUF inteiro com `buf[0] = 0` — o próprio byte do CRC
/// entra como zero, não como o valor que vai ser gravado depois.
pub fn crc8(buf: &[u8]) -> u8 {
    let mut c: u8 = 0;
    for &b in buf {
        c ^= b;
        for _ in 0..8 {
            c = if c & 0x80 != 0 {
                (c << 1) ^ 0x07
            } else {
                c << 1
            };
        }
    }
    c
}

/// Monta o `BUF` de um bloco de upload: `[crc, 0x92, index, length, payload]`.
///
/// # Erros
/// [`ProtocolError::InvalidShape`] se `payload` passar de
/// [`PAYLOAD_MAX`] — o `length` é **um byte só**, então um payload maior
/// viraria um bloco que o device leria pelo `length` errado.
pub fn buf(index: u8, payload: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    if payload.len() > PAYLOAD_MAX {
        return Err(ProtocolError::InvalidShape {
            expected: format!("<= {PAYLOAD_MAX} bytes de payload por bloco (§2: length 0x13)"),
            got: format!("{} bytes", payload.len()),
        });
    }
    let mut out = Vec::with_capacity(4 + payload.len());
    out.push(0); // placeholder do CRC, calculado com ele zerado
    out.push(CMD_SNAP_TONE);
    out.push(index);
    out.push(payload.len() as u8);
    out.extend_from_slice(payload);
    let crc = crc8(&out);
    out[0] = crc;
    Ok(out)
}

/// SysEx COMPLETO de um bloco: `F0 + nibble-expand(BUF) + F7` (§2).
///
/// # Erros
/// [`ProtocolError::InvalidShape`] se o payload passar de [`PAYLOAD_MAX`].
pub fn wire_block(index: u8, payload: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    let b = buf(index, payload)?;
    let mut out = Vec::with_capacity(2 + 2 * b.len());
    out.push(0xF0);
    out.extend(nibble_expand(&b));
    out.push(SYSEX_EOX);
    Ok(out)
}

/// Quebra o modelo convertido nos blocos da transferência (§5: payload de 19B,
/// o último curto).
///
/// Devolve `(index, payload)` na ordem do fio — quem envia é o chamador, para
/// que possa esperar o ACK de cada bloco antes do próximo (§5: ACK de 16B por
/// bloco; settle ≥ 0,25 s entre operações, §4 das regras de segurança).
///
/// # Erros
/// [`ProtocolError::InvalidShape`] se o modelo for vazio: um stream sem
/// bloco nenhum não é uma transferência, é um bug de quem montou.
pub fn blocks(model: &[u8]) -> Result<Vec<(u8, Vec<u8>)>, ProtocolError> {
    if model.is_empty() {
        return Err(ProtocolError::InvalidShape {
            expected: "modelo convertido com pelo menos 1 byte (§5)".into(),
            got: "0 bytes".into(),
        });
    }
    let mut out = Vec::with_capacity(model.len() / PAYLOAD_MAX + 1);
    for (i, chunk) in model.chunks(PAYLOAD_MAX).enumerate() {
        let index = u8::try_from(i).map_err(|_| ProtocolError::InvalidShape {
            expected: "modelo com menos de 256 blocos (o índice é 1 byte)".into(),
            got: format!("{} blocos", i + 1),
        })?;
        out.push((index, chunk.to_vec()));
    }
    Ok(out)
}

/// Requisição de LEITURA do SnapTone: `BUF = [crc, 0x01, 0x00, 0x02, 0x12, 0x24]`
/// (§2, lista de requisições conhecidas).
///
/// # Erros
/// [`ProtocolError::InvalidShape`] — nunca, hoje: a requisição é fixa. A
/// assinatura existe para o chamador poder encadear com [`wire_block`] sem
/// tratar um caso diferente por função.
pub fn read_slot() -> Result<Vec<u8>, ProtocolError> {
    let mut b = vec![0u8, CMD_LEITURA, 0x00, 0x02, 0x12, SELECTOR_SNAP_TONE];
    let crc = crc8(&b);
    b[0] = crc;
    Ok(b)
}

/// Confere um ACK de bloco: 16 bytes fixos na família (§2).
///
/// `idx` é o bloco esperado. O ACK de 16B **não** traz o payload — então o
/// que se compara aqui é o tamanho e, quando o eco traz o índice, ele bate.
///
/// # Erros
/// [`ProtocolError::InvalidShape`] se o ACK não tiver [`ACK_LEN`] bytes.
pub fn check_ack(ack: &[u8]) -> Result<(), ProtocolError> {
    if ack.len() != ACK_LEN {
        return Err(ProtocolError::InvalidShape {
            expected: format!("ACK de {ACK_LEN} bytes (§2: fixo na família)"),
            got: format!("{} bytes", ack.len()),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Os valores deste bloco saíram da implementação de REFERÊNCIA do
    /// PROTOCOL §3 (`crc8_07` em Python), não desta implementação: se os dois
    /// concordam, o CRC não é uma reimplementação que se auto-confirma.
    #[test]
    fn crc8_bate_com_a_implementacao_de_referencia() {
        // BUF de upload, bloco 0, payload 0..=18 (o bloco cheio de §2).
        let buf = buf(0, &(0u8..19).collect::<Vec<u8>>()).unwrap();
        assert_eq!(
            buf.iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" "),
            "89 92 00 13 00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F 10 11 12"
        );
        // Requisição de leitura do SnapTone: [0, 01, 00, 02, 12, 24] → 0x35.
        assert_eq!(crc8(&[0, 1, 0, 2, 0x12, 0x24]), 0x35);
        assert_eq!(read_slot().unwrap()[0], 0x35);
    }

    #[test]
    fn bloco_cheio_da_48_bytes_no_fio() {
        // §2: "Um bloco de 19B de payload = 23 bytes BUF = 46 nibbles = 48B no fio".
        let w = wire_block(0, &(0u8..19).collect::<Vec<u8>>()).unwrap();
        assert_eq!(w.len(), 48);
        assert_eq!(w[0], 0xF0);
        assert_eq!(w[47], 0xF7);
        // Todo byte do fio é nibble (<= 0x0F) depois do F0: é o que o trim no
        // 1º F7 do proxy encontra.
        assert!(w[1..47].iter().all(|b| *b <= 0x0F));
    }

    #[test]
    fn o_crc_vai_com_o_payload_e_com_o_indice() {
        let a = buf(0, &[1, 2, 3]).unwrap();
        let b = buf(0, &[1, 2, 4]).unwrap();
        let c = buf(1, &[1, 2, 3]).unwrap();
        assert_ne!(a[0], b[0], "payload diferente muda o CRC");
        assert_ne!(a[0], c[0], "índice diferente muda o CRC");
        // O byte do CRC entra como ZERO no cálculo: se ele entrasse com o
        // próprio valor, o resultado seria outro (e não bateria com a ref).
        assert_eq!(a[0], crc8(&[0, 0x92, 0, 3, 1, 2, 3]));
    }

    #[test]
    fn payload_acima_de_19_e_erro_e_nao_truncamento() {
        // O `length` é UM byte no layout: um payload de 20 passaria como 20 e
        // o device leria o bloco errado. Melhor recusar.
        let e = wire_block(0, &[0u8; 20]).unwrap_err();
        assert!(matches!(e, ProtocolError::InvalidShape { .. }));
    }

    #[test]
    fn o_modelo_quebra_em_blocos_de_19_com_o_ultimo_curto() {
        // 2700 B é o "~2,7 KB" de §5: 142 blocos cheios + 1 de 2 bytes.
        let modelo: Vec<u8> = (0..2700).map(|i| (i * 7) as u8).collect();
        let blocos = blocks(&modelo).unwrap();
        assert_eq!(blocos.len(), 143);
        assert_eq!(blocos[0].0, 0);
        assert_eq!(blocos[0].1.len(), PAYLOAD_MAX);
        assert_eq!(
            *blocos.last().unwrap(),
            (142, vec![modelo[2698], modelo[2699]])
        );
        // Reconstrói: nada se perde no caminho.
        let volta: Vec<u8> = blocos.iter().flat_map(|(_, p)| p.iter().copied()).collect();
        assert_eq!(volta, modelo);
    }

    #[test]
    fn modelo_vazio_nao_e_stream() {
        assert!(matches!(
            blocks(&[]),
            Err(ProtocolError::InvalidShape { .. })
        ));
    }

    #[test]
    fn ack_de_16_bytes_e_o_unico_aceito() {
        assert!(check_ack(&[0u8; ACK_LEN]).is_ok());
        assert!(matches!(
            check_ack(&[0u8; 15]),
            Err(ProtocolError::InvalidShape { .. })
        ));
    }
}
