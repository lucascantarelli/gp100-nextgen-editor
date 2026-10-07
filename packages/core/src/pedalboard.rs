//! pedalboard — projeção de preset → view de board para a UI (dados puros,
//! sem I/O): a UI recebe o board pronto e só renderiza/interage (R1: a
//! interpretação do preset/dicionário vive no core, nunca no front).
//!
//! **Archetypes:** a arte do pedal é escolhida pela FAMÍLIA (module do
//! `.prst`: PRE/DST/AMP/NR/CAB/EQ/MOD/DLY/RVB) — o desenho não depende do
//! algoritmo específico. Os knobs vêm do dicionário
//! (`effectCode = (nibble<<24)|index`, lookup O(1) + fallback first-wins).

use serde::Serialize;

use crate::model::{ControlKind, Dictionary};
use crate::transport::mock;
// Contrato de fio IPC: os DTOs da UI são camelCase (mesma convenção dos
// commands da api — testes serde travam os dois lados).

/// O `.prst` embedado do mock (all.prst) — R1: os dados de board vivem no
/// core; a api/UI parseiam POR AQUI, nunca de caminho de disco.
pub fn embedded_document() -> Result<Document, ProtocolError> {
    Document::parse(mock::embedded_preset().as_bytes())
}
use crate::preset::Document;
use crate::ProtocolError;

/// Família visual do slot (module do `.prst`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Family {
    /// Buffer/instrument switcher (x=0).
    Pre,
    /// Distortion/overdrive (x=1).
    Dst,
    /// Amplificador (x=2).
    Amp,
    /// Noise reduction (x=3).
    Nr,
    /// Cabinet/speaker sim (x=4).
    Cab,
    /// Equalizador (x=5).
    Eq,
    /// Modulação (x=6).
    Mod,
    /// Delay (x=7).
    Dly,
    /// Reverb (x=8).
    Rvb,
}

/// Arquétipo visual do pedal (a arte SVG por família).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Archetype {
    /// Buffer/instrument switcher.
    Buffer,
    /// Caixa de distortion/overdrive.
    Distortion,
    /// Head/amplificador.
    Amplifier,
    /// Pedal de noise gate.
    NoiseGate,
    /// Sim de cabinet/caixa.
    Cabinet,
    /// EQ gráfico.
    Eq,
    /// Pedal de modulação (chorus/flanger/tremolo…).
    Modulation,
    /// Pedal de delay.
    Delay,
    /// Pedal de reverb.
    Reverb,
}

/// Um controle do algoritmo com o valor ATUAL do preset — a spec do knob
/// para a UI renderizar (tipo, faixa, opções, default).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KnobSpec {
    /// Nome do controle (ex.: "Sustain").
    pub name: String,
    /// Posição no envelope (índice nos params_N do `.prst`).
    pub pos: u8,
    /// `knob` | `switch` | `combox`.
    pub kind: String,
    /// Faixa `[min, max]` (só knob; knobs bidirecionais podem ter min>max).
    pub range: Option<(f64, f64)>,
    /// Rótulos discretos (switch/combox).
    pub options: Vec<String>,
    /// Valor atual cru (params_N do preset).
    pub value: Option<String>,
    /// Default do dicionário (representação original; reset do knob).
    pub default: Option<String>,
}

/// Um slot do board (posição da cadeia x=0..8).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlotSpec {
    /// Slot da cadeia (x do arquivo, 0..8).
    pub slot: u8,
    /// Família visual.
    pub family: Family,
    /// Arquétipo visual (a arte).
    pub archetype: Archetype,
    /// Nome do algoritmo (ex.: "Bog RedM"; padding do arquivo removido).
    pub name: String,
    /// Variante visual (slug do algoritmo, ex.: "green-od") — a UI usa para
    /// escolher o MODELO do pedal (arte por efeito real, não genérica).
    pub variant: String,
    /// ON/OFF (effectState do preset; toggle do usuário sobrepõe no mock).
    pub state: bool,
    /// effectCode `(nibble<<24)|index` (identidade do algoritmo).
    pub code: u32,
    /// Controles do algoritmo (dicionário) com valores atuais.
    pub knobs: Vec<KnobSpec>,
}

/// View de board do preset — o que a UI renderiza no index.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BoardView {
    /// pp do preset (u16 BE no fio; cru no arquivo em hex).
    pub pp: u16,
    /// Nome do preset.
    pub name: String,
    /// Tipo/ícone (u16 cru).
    pub pp_type: u16,
    /// Rótulo do tipo (ex.: "Pop").
    pub pp_type_name: String,
    /// 9 slots da cadeia, na ORDEM DO SINAL (x=0..8).
    pub slots: Vec<SlotSpec>,
}

/// Entrada da biblioteca de presets (o "flight case" da UI).
#[derive(Debug, Clone, Serialize)]
pub struct PresetEntry {
    /// pp do preset.
    pub pp: u16,
    /// Nome do preset.
    pub name: String,
    /// Rótulo do tipo (ex.: "Pop").
    pub pp_type_name: String,
}

fn family_of(module: &str) -> Family {
    match module {
        "DST" => Family::Dst,
        "AMP" => Family::Amp,
        "NR" => Family::Nr,
        "CAB" => Family::Cab,
        "EQ" => Family::Eq,
        "MOD" => Family::Mod,
        "DLY" => Family::Dly,
        "RVB" => Family::Rvb,
        _ => Family::Pre,
    }
}

fn archetype_of(f: Family) -> Archetype {
    match f {
        Family::Pre => Archetype::Buffer,
        Family::Dst => Archetype::Distortion,
        Family::Amp => Archetype::Amplifier,
        Family::Nr => Archetype::NoiseGate,
        Family::Cab => Archetype::Cabinet,
        Family::Eq => Archetype::Eq,
        Family::Mod => Archetype::Modulation,
        Family::Dly => Archetype::Delay,
        Family::Rvb => Archetype::Reverb,
    }
}

/// Slug estável p/ `variant` (mesma regra do `dump_fx_map.py`).
fn slug(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "fx".into()
    } else {
        s
    }
}

fn shape_err(expected: &str, got: &str) -> ProtocolError {
    ProtocolError::InvalidShape {
        expected: expected.to_string(),
        got: got.to_string(),
    }
}

/// Biblioteca de presets do arquivo (o flight case da UI).
///
/// O `pp` é o `ppID` **decimal** — o mesmo número que a semente da
/// biblioteca e o artefato do front usam, e o índice que vai ao fio como
/// `u16 BE` no banco `0x00xx` (#132/ADR-12; ver
/// [`crate::preset::pp_id_decimal`]).
pub fn preset_list(doc: &Document) -> Vec<PresetEntry> {
    doc.presets()
        .filter_map(|p| {
            let pp = p.pp_id().and_then(crate::preset::pp_id_decimal)?;
            Some(PresetEntry {
                pp,
                name: p.pp_name().unwrap_or("").to_string(),
                pp_type_name: p.pp_type_name().unwrap_or("").to_string(),
            })
        })
        .collect()
}

/// Constrói a view de board do pp indicado (`None` = 1º preset do arquivo).
///
/// O `pp` pode vir do fio com byte de banco (`0x0100` = índice 0 — a
/// captura S1 varre os dois bancos); o documento só tem o índice
/// ([`crate::preset::indice_do_documento`]), e a view devolve o `pp` do
/// DOCUMENTO, que é o que a UI acende na lista (0..98).
///
/// # Erros
/// [`ProtocolError::InvalidShape`] se o arquivo não tem preset ou o pp
/// indicado não existe.
pub fn board_view_for(
    doc: &Document,
    dict: &Dictionary,
    pp: Option<u16>,
) -> Result<BoardView, ProtocolError> {
    let alvo = pp.map(crate::preset::indice_do_documento);
    let pv = match alvo {
        None => doc
            .presets()
            .next()
            .ok_or_else(|| shape_err("preset no arquivo", "nenhum"))?,
        Some(target) => doc
            .presets()
            .find(|p| p.pp_id().and_then(crate::preset::pp_id_decimal) == Some(target))
            .ok_or_else(|| shape_err(&format!("preset {target:#06x}"), "não encontrado"))?,
    };

    // Cada slot: família/arquétipo pelo module; knobs pelo dicionário
    // (nibble+index do effectCode); valor atual = params_N do preset.
    let mut slots: Vec<SlotSpec> = pv
        .effects()
        .map(|e| {
            let family = family_of(e.module());
            let code = e.code().unwrap_or(0);
            let nibble = ((code >> 24) & 0xFF) as u8;
            let index = code & 0x00FF_FFFF;
            let knobs = match dict.algorithm(nibble, index) {
                Some(a) => a
                    .controls
                    .iter()
                    .map(|c| KnobSpec {
                        name: c.name.clone(),
                        pos: c.pos,
                        kind: match c.kind {
                            ControlKind::Knob => "knob",
                            ControlKind::Switch => "switch",
                            ControlKind::Combox => "combox",
                        }
                        .to_string(),
                        range: c.range(),
                        options: c.options.clone(),
                        value: e.param(c.pos).map(|s| s.to_string()),
                        default: c.default.clone(),
                    })
                    .collect(),
                // Algoritmo fora do dicionário: pedal renderiza sem knobs
                // (R1: nunca adivinhar controle).
                None => Vec::new(),
            };
            SlotSpec {
                slot: e
                    .element()
                    .attr("x")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                family,
                archetype: archetype_of(family),
                name: e.name().unwrap_or("").trim().to_string(),
                variant: slug(e.name().unwrap_or("")),
                state: e.state() == Some("1"),
                code,
                knobs,
            }
        })
        .collect();
    slots.sort_by_key(|s| s.slot);

    Ok(BoardView {
        pp: alvo.unwrap_or_else(|| {
            pv.pp_id()
                .and_then(crate::preset::pp_id_decimal)
                .unwrap_or(0)
        }),
        name: pv.pp_name().unwrap_or("").to_string(),
        pp_type: pv.pp_type().and_then(|s| s.parse().ok()).unwrap_or(4),
        pp_type_name: pv.pp_type_name().unwrap_or("").to_string(),
        slots,
    })
}
