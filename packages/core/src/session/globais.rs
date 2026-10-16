//! os REGISTRADORES globais (§13.13 — sessão G3–G6 de 11/10): a família que NÃO
//! está no golden.
//!
//! A forma da resposta é o COMPRIMENTO medido ([`REGS_GLOBAIS`]) — nenhum byte
//! é adivinhado e o payload volta CRU (ADR-13: o decode medido é do chamador).

use crate::transport::{DeviceTransport, WireKind};
use crate::ProtocolError;

use super::tipos::GlobalBruto;
use super::transacao::{tx_err, Forma};
use super::Session;

/// Tabela MEDIDA de registradores globais (PROTOCOL §13.13.1 + nomes da RE
/// do Suite §13.13.7): `(endereço, comprimentos de payload aceitos)`.
///
/// FORA do golden — aquele arquivo é o mundo do preset; a forma destas
/// respostas é a medida na sessão G3–G6 (4B para os contadores nibble-par,
/// 8B para o enable do Global EQ; o §13.13.1 aceita "4B ou 8B" na faixa
/// 03..0b, então os registros SYSTEM aceitam os dois).
pub const REGS_GLOBAIS: &[([u8; 4], &[usize])] = &[
    ([0x00, 0x02, 0x00, 0x03], &[4, 8]), // gVolume — volume master
    ([0x00, 0x02, 0x00, 0x04], &[4, 8]), // gInput — Input Level (medido no diff)
    ([0x00, 0x02, 0x00, 0x05], &[4, 8]), // gUsbGain
    ([0x00, 0x02, 0x00, 0x06], &[4, 8]), // gUsbMonitorVol
    ([0x00, 0x02, 0x00, 0x07], &[4, 8]), // gUsbAudioLeft
    ([0x00, 0x02, 0x00, 0x08], &[4, 8]), // gUsbAudioRight
    ([0x00, 0x02, 0x00, 0x09], &[4, 8]), // gTapTempoSetting
    ([0x00, 0x02, 0x00, 0x0a], &[4, 8]), // gNoCabLeft
    ([0x00, 0x02, 0x00, 0x0b], &[4, 8]), // gNoCabRight
    ([0x00, 0x02, 0x00, 0x11], &[8]),    // Global EQ — enable (8B medido)
    ([0x00, 0x01, 0x02, 0x01], &[4]),    // drumPlayStyle
    ([0x00, 0x01, 0x02, 0x03], &[4]),    // drumBpm (medido no diff)
    ([0x00, 0x01, 0x02, 0x04], &[4]),    // drumVolume
    ([0x00, 0x01, 0x02, 0x05], &[4]),    // drumState
];

impl<T: DeviceTransport> Session<T> {
    /// Lê UM registrador global (§13.13.1): READ `11/<addr>` SEM vars →
    /// IN `12/<addr>` payload. A forma validada é o COMPRIMENTO medido
    /// ([`REGS_GLOBAIS`]); o payload volta CRU — nenhum decode aqui
    /// (ADR-13: semântica não inventada; o decode medido é do chamador,
    /// [`crate::codec::nibble_pair_value`]).
    ///
    /// # Erros
    /// [`ProtocolError::Timeout`] se o registrador não responde na janela
    /// ADR-3, [`ProtocolError::InvalidShape`] para comprimento fora do
    /// medido.
    pub fn read_register(
        &mut self,
        addr: [u8; 4],
        formas: &'static [usize],
    ) -> Result<Vec<u8>, ProtocolError> {
        self.transport
            .send_raw(&crate::codec::read_frame(&addr), WireKind::Read)
            .map_err(tx_err)?;
        self.wait_for_com(0x12, &addr, Forma::Medido(formas))
    }

    /// Escreve num registrador global (§13.13.1: WRITE `12/<addr>`) pelo
    /// envelope canônico [`crate::codec::write_frame`]. A trava ADR-5 é do
    /// TRANSPORTE: `WireKind::Write` sem write-verified é recusado antes do
    /// driver — o byte nem sai da máquina (mesma trava do knob, #126).
    pub fn write_register(&mut self, addr: [u8; 4], payload: &[u8]) -> Result<(), ProtocolError> {
        self.transport
            .send_raw(&crate::codec::write_frame(&addr, payload), WireKind::Write)
            .map_err(tx_err)
    }

    /// Lê o bloco medido de registradores globais (§13.13) na ordem da
    /// tabela [`REGS_GLOBAIS`], devolvendo `(addr, payload cru)`. Tudo-ou-
    /// nada: a primeira falha aborta (o front vira banner com retry, #20).
    pub fn read_globals(&mut self) -> Result<Vec<GlobalBruto>, ProtocolError> {
        let mut out = Vec::with_capacity(REGS_GLOBAIS.len());
        for &(addr, formas) in REGS_GLOBAIS {
            out.push((addr, self.read_register(addr, formas)?));
        }
        Ok(out)
    }
}
