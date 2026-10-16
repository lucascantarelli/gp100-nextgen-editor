//! o ESTADO do mock: o que ele POSSUI — derivado de `all.prst` + dicionario.

use std::collections::HashMap;

use super::{shape_err, ALL_PRST};
use crate::preset::Document;
use crate::ProtocolError;

/// Estado do device simulado, derivado de `all.prst` + dicionário.
#[derive(Debug, Clone)]
pub struct MockState {
    /// Nº de presets carregados do `.prst` (99 em `all.prst`).
    pub preset_count: usize,
    /// pp corrente (seleção via write `13010000`; default = 1º do `.prst`).
    pub current_pp: u16,
    /// Nome do pp corrente (do `.prst`; trocado por write `11000000`).
    pub current_name: String,
    /// ppType do pp corrente (do `.prst`; trocado por write `11000005`).
    pub current_pp_type: u16,
    /// CRC32 IEEE de fábrica dos 20 slots de IR (ppIRCRC do `.prst`).
    pub ir_crcs: [u32; 20],
    /// Parâmetros setados: `((nibble, ctrl), (code, value))` — last-wins.
    pub set_params: HashMap<(u8, u8), (u32, f32)>,
    /// Modelo de SnapTone **remontado** dos blocos recebidos (§5). É o que o
    /// mock pode observar do upload: o fio carrega o modelo em blocos com
    /// índice, e não há seletor de slot no stream (R1 — só a contagem e o
    /// payload estão evidenciados), então o que fica é a transferência
    /// corrente, reaberta quando um índice 0 chega.
    pub snap_tone_model: Vec<u8>,
    /// Quantas transferências de SnapTone chegaram (um índice 0 = uma nova).
    pub snap_tone_transfers: usize,
    /// ACKs de SnapTone emitidos — um por bloco aceito (§5).
    pub snap_tone_acks: usize,
    /// Registradores globais (§13.13): `addr → payload` nos ENDEREÇOS da
    /// tabela `session::REGS_GLOBAIS` (mesma família do aparelho: 4B
    /// contador nibble-par, 8B enable do EQ). Read (`11/<addr>`) devolve o
    /// valor guardado; write (`12/<addr>`) atualiza — o mock pode ser lido
    /// E escrito como o aparelho medido (roundtrip do write-through).
    pub globals: HashMap<[u8; 4], Vec<u8>>,
    /// nº de frames recusados por D5 (diagnóstico de divergência).
    pub rejected: usize,
}

impl MockState {
    /// Carrega o estado de `all.prst` (embedado). O pp corrente inicial é o
    /// do 1º preset (`ppID` decimal = índice 0-based, o mesmo espaço do
    /// fio); os ppIRCRC de fábrica povoam os 20 slots.
    ///
    /// # Erros
    /// [`ProtocolError::InvalidShape`] se o `.prst` embedado não parseia ou
    /// nenhum preset tem ppID (impossível no build normal: R4 é travado).
    pub fn load() -> Result<Self, ProtocolError> {
        let doc = Document::parse(ALL_PRST.as_bytes())?;
        let mut ir_crcs = [0u32; 20]; // Os 20 slots de IR são as tags <ppIRInfo0..19> no container
                                      // <ppIRInfo> — irmão de <preset_info> NO NÍVEL DA RAIZ <GP>
                                      // (achado estrutural; o ppIRInfo NÃO está dentro de preset_info nem de
                                      // um preset). CRC: i32 decimal → u32 (ppIRCRC="-1871114785").
        if let Some(info) = doc.root().child("ppIRInfo") {
            for (i, tag) in info.children().iter().enumerate().take(20) {
                if let Some(crc) = tag.attr("ppIRCRC") {
                    if let Ok(c) = crc.parse::<i32>() {
                        ir_crcs[i] = c as u32;
                    }
                }
            }
        }
        let first = doc
            .presets()
            .next()
            .ok_or_else(|| shape_err("preset no all.prst", "nenhum"))?;
        let current_pp = first
            .pp_id()
            .and_then(crate::preset::pp_id_decimal)
            .ok_or_else(|| shape_err("ppID decimal no 1º preset", "ausente"))?;
        Ok(Self {
            preset_count: doc.presets().count(),
            current_pp,
            current_name: first.pp_name().unwrap_or("").to_string(),
            current_pp_type: first.pp_type().and_then(|s| s.parse().ok()).unwrap_or(4),
            ir_crcs,
            set_params: HashMap::new(),
            snap_tone_model: Vec::new(),
            snap_tone_transfers: 0,
            snap_tone_acks: 0,
            globals: defaults_globais(),
            rejected: 0,
        })
    }
}

/// Valores iniciais dos registradores globais (§13.13) — os MESMOS defaults
/// da UI hoje (master 99 · drum 120/80), codificados no contador nibble-par
/// medido ([d0, d1, 00, 00] com valor = d0*16 + d1; §13.13.4). Assim o
/// mock "é o aparelho" com o estado que a tela já desenha: o seed do boot
/// não muda nada visível, e o write-through tem onde roundtrippar.
fn defaults_globais() -> HashMap<[u8; 4], Vec<u8>> {
    let nib = |v: u16| vec![(v / 16) as u8, (v % 16) as u8, 0, 0];
    let mut m = HashMap::new();
    // família SYSTEM 000200xx (nomes §13.13.7)
    m.insert([0x00, 0x02, 0x00, 0x03], nib(99)); // gVolume (master)
    m.insert([0x00, 0x02, 0x00, 0x04], nib(50)); // gInput
    m.insert([0x00, 0x02, 0x00, 0x05], nib(100)); // gUsbGain
    m.insert([0x00, 0x02, 0x00, 0x06], nib(80)); // gUsbMonitorVol
    m.insert([0x00, 0x02, 0x00, 0x07], nib(1)); // gUsbAudioLeft
    m.insert([0x00, 0x02, 0x00, 0x08], nib(1)); // gUsbAudioRight
    m.insert([0x00, 0x02, 0x00, 0x09], vec![0, 0, 0, 0]); // gTapTempoSetting
    m.insert([0x00, 0x02, 0x00, 0x0a], vec![0, 0, 0, 0]); // gNoCabLeft
    m.insert([0x00, 0x02, 0x00, 0x0b], vec![0, 0, 0, 0]); // gNoCabRight
    m.insert([0x00, 0x02, 0x00, 0x11], vec![0u8; 8]); // Global EQ enable (8B)

    // família DRUM 000102xx
    m.insert([0x00, 0x01, 0x02, 0x01], vec![0, 0, 0, 0]); // drumPlayStyle
    m.insert([0x00, 0x01, 0x02, 0x03], nib(120)); // drumBpm
    m.insert([0x00, 0x01, 0x02, 0x04], nib(80)); // drumVolume
    m.insert([0x00, 0x01, 0x02, 0x05], vec![0, 0, 0, 0]); // drumState
    m
}
