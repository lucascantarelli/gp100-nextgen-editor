//! model — dicionário de algoritmos/controles do GP-100 (ROADMAP M0.1).
//!
//! **Insumo:** `analysis/parameters.json` (185 algs / 639 controles, validado
//! em 3 vias — ver `meta` no próprio arquivo). Embedado via `include_str!`
//! (skill rust-practices: spec é insumo, não produto — R1).
//!
//! **Regras de ouro deste módulo** (todas com evidência nos dados):
//! - `effectCode = (nibble << 24) | index` — vale nos 185 registros
//!   (§13.11/knob_map; no FIO o effectCode viaja u32 LE — ADR-1 — mas a
//!   identidade matemática é a mesma do `.prst`);
//! - identidade do algoritmo é a TRIPLA `(module, nibble, index)` (achado
//!   M0.1: Boost/14 Boost existem em PRE e DST); `(nibble, index)` sozinho
//!   é chave de FALLBACK O(1) (first-wins);
//! - knobs bidirecionais vêm com min>max (Pitch.L-Pitch 0..-24): a faixa
//!   validada é finita/não-degenerada — normalizar com [`Control::range`];
//! - `switch`/`combox` têm `options` e `option_ids` de MESMO comprimento
//!   (58 switch + 6 combox nas amostras).

use std::collections::HashMap;

use serde::{Deserialize, Deserializer};

use crate::ProtocolError;

/// O dicionário embutido no binário (`include_str!` — spec é insumo, R1).
pub const DICTIONARY_JSON: &str = include_str!("../../analysis/parameters.json");

/// Tipo de controle físico/UI mapeado do JSON (`knob`/`switch`/`combox`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ControlKind {
    /// Knob contínuo com faixa física `[min, max]` (float).
    Knob,
    /// Chave discreta: `options` com `option_ids` (id é o valor de fio).
    Switch,
    /// ComboBox discreta: mesma forma do switch, N opções.
    Combox,
}

/// Desserializa u32 aceitando ausente OU null como 0 (68 algs nunca
/// observados nos patches omitem/ anulam `observed_in_patches`).
fn de_u32_or_zero<'de, D>(d: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<u32> = Option::deserialize(d)?;
    Ok(v.unwrap_or(0))
}

/// Desserializa Vec<String> aceitando ausente OU null como vazio.
fn de_vec_or_empty<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<Vec<String>> = Option::deserialize(d)?;
    Ok(v.unwrap_or_default())
}
///
/// O JSON traz `default` como **string** ("20.0", "1") — representação
/// escolhida pelo `build_parameters.py`; preservamos como `String` e a
/// interpretação numérica fica a cargo de quem usa.
#[derive(Debug, Clone, Deserialize)]
pub struct Control {
    /// Tipo do controle (knob/switch/combox). No JSON o campo se chama
    /// `type` (palavra reservada em Rust) — daí o rename.
    #[serde(rename = "type")]
    pub kind: ControlKind,
    /// Nome no dicionário (ex.: "Sustain", "Mode").
    pub name: String,
    /// Posição do controle = índice nos `params_N` do `.prst` (§13.9) e no
    /// payload de set-param (§13.11). Validado sequencial 0..len-1.
    pub pos: u8,
    /// Default como string (preserva a representação original).
    pub default: Option<String>,
    /// Mínimo físico (só `knob`; `None` em switch/combox). ⚠️ Pode ser MAIOR
    /// que `max` em knobs bidirecionais (0 = centro) — usar [`Control::range`].
    pub min: Option<f64>,
    /// Máximo físico (só `knob`; ver aviso em `min`).
    pub max: Option<f64>,
    /// Rótulos discretos (switch/combox).
    #[serde(default)]
    pub options: Vec<String>,
    /// Ids de fio dos rótulos, MESMO comprimento de `options` (validado).
    #[serde(default)]
    pub option_ids: Vec<u8>,
}

impl Control {
    /// Faixa normalizada `(lo, hi)` do knob, com lo <= hi.
    ///
    /// Existe porque o dicionário lista knobs bidirecionais com min>max
    /// (evidência: `Pitch.L-Pitch` = min 0, max -24; 0 é o centro). Só tem
    /// sentido para `ControlKind::Knob` — `None` caso contrário/sem dados.
    pub fn range(&self) -> Option<(f64, f64)> {
        let (min, max) = (self.min?, self.max?);
        Some(if min <= max { (min, max) } else { (max, min) })
    }
}

/// Um algoritmo do dicionário — identidade = tripla `(module, nibble, index)`.
///
/// ⚠️ **Achado estrutural do M0.1** (evidência: 185 linhas do dicionário):
/// o MESMO effectCode pode estar listado em DOIS módulos — `Boost` e
/// `14 Boost` (nibble 0) existem em PRE **e** DST, com defaults divergentes
/// (Bright: "1" no PRE vs "0" no DST). Por isso a chave primária inclui o
/// módulo (que o editor sempre conhece: é o slot da cadeia); para display
/// rápido existe o fallback por `(nibble, index)` (primeira ocorrência,
/// mesma semântica first-wins do `validate_knob_map.py` em campo).
#[derive(Debug, Clone, Deserialize)]
pub struct Algorithm {
    /// Nome curto (ex.: "COMP", "Bog RedM").
    pub name: String,
    /// Módulo da cadeia onde o alg é listado: PRE/DST/AMP/NR/CAB/EQ/MOD/
    /// DLY/RVB. O mesmo effectCode pode existir em 2 módulos (ver topo).
    pub module: String,
    /// u32 do dicionário: `(nibble << 24) | index`.
    pub code: u32,
    /// Nibble do slot na cadeia (§13.11; ex.: 0x07 = AMP).
    pub nibble: u8,
    /// Índice do algoritmo dentro do nibble (24 bits).
    pub index: u32,
    /// Controles do algoritmo (`pos` = índice no Vec; máx. 9 nas amostras).
    pub controls: Vec<Control>,
    /// Em quantos dos 909 slots de patch o alg foi observado (metadado;
    /// 68 dos 185 algs nunca aparecem nos patches: campo ausente OU null).
    #[serde(default, deserialize_with = "de_u32_or_zero")]
    pub observed_in_patches: u32,
    /// Slots (string) onde o alg aparece nos patches (metadado).
    #[serde(default, deserialize_with = "de_vec_or_empty")]
    pub observed_slots: Vec<String>,
}

/// Forma CRUA do JSON (o que o serde materializa direto do arquivo).
#[derive(Debug, Deserialize)]
struct RawDictionary {
    /// Metadados de proveniência (product, validation, ...) — preservados.
    meta: serde_json::Value,
    /// Os algoritmos.
    algorithms: Vec<Algorithm>,
}

/// Dicionário validado, com lookup O(1) por (module,nibble,index) e
/// fallback por (nibble,index).
#[derive(Debug, Clone)]
pub struct Dictionary {
    /// Metadados de proveniência do arquivo.
    meta: serde_json::Value,
    /// Algoritmos na ordem do arquivo.
    algorithms: Vec<Algorithm>,
    /// (module, nibble, index) -> posição em `algorithms` (chave primária).
    by_full: HashMap<(String, u8, u32), usize>,
    /// (nibble, index) -> posição da PRIMEIRA ocorrência no arquivo
    /// (fallback de display; 183 chaves — Boost/14 Boost têm 2 módulos).
    by_key: HashMap<(u8, u32), usize>,
}

impl Dictionary {
    /// Carrega e valida o dicionário de um JSON em memória.
    ///
    /// # Erros
    /// [`ProtocolError::InvalidShape`] com mensagem descritiva se qualquer
    /// invariante violar (schema, identidade `code`, duplicata, `pos`
    /// sequencial, faixa de knob, opções de switch/combox).
    pub fn from_json(json: &str) -> Result<Self, ProtocolError> {
        // Passo 1 — parse do schema cru.
        let raw: RawDictionary =
            serde_json::from_str(json).map_err(|e| ProtocolError::InvalidShape {
                expected: "schema do parameters.json (meta+algorithms)".into(),
                got: e.to_string(),
            })?;

        // Passo 2 — identidade e unicidade: code = (nibble<<24)|index e
        // tripla (module,nibble,index) única (é o índice O(1); duplicata =
        // bug grave). NOTA: (nibble,index) PODE repetir entre módulos —
        // Boost/14 Boost existem em PRE e DST (achado do M0.1).
        let mut by_full: HashMap<(String, u8, u32), usize> =
            HashMap::with_capacity(raw.algorithms.len());
        let mut by_key: HashMap<(u8, u32), usize> = HashMap::with_capacity(raw.algorithms.len());
        for (i, a) in raw.algorithms.iter().enumerate() {
            if a.code != ((a.nibble as u32) << 24) | a.index {
                return Err(ProtocolError::InvalidShape {
                    expected: format!("code == (nibble<<24)|index para '{}'", a.name),
                    got: format!("code={} nibble={} index={}", a.code, a.nibble, a.index),
                });
            }
            if by_full
                .insert((a.module.clone(), a.nibble, a.index), i)
                .is_some()
            {
                return Err(ProtocolError::InvalidShape {
                    expected: "triplas (module,nibble,index) únicas".into(),
                    got: format!(
                        "duplicata em ({}, nibble={:#04x}, index={})",
                        a.module, a.nibble, a.index
                    ),
                });
            }
            // fallback de display: primeira ocorrência no arquivo vence
            // (mesma regra first-wins validada em campo pelo knob_map).
            by_key.entry((a.nibble, a.index)).or_insert(i);
        }

        // Passo 3 — invariantes de controles: pos sequencial 0..len-1;
        // knob exige min<max; switch/combox exigem options==option_ids.
        for a in &raw.algorithms {
            for (i, c) in a.controls.iter().enumerate() {
                if c.pos as usize != i {
                    return Err(ProtocolError::InvalidShape {
                        expected: format!("pos sequencial em '{}'", a.name),
                        got: format!("pos={} no índice {}", c.pos, i),
                    });
                }
                match c.kind {
                    ControlKind::Knob => {
                        let (lo, hi) = match (c.min, c.max) {
                            (Some(lo), Some(hi)) => (lo, hi),
                            _ => {
                                return Err(ProtocolError::InvalidShape {
                                    expected: format!(
                                        "min/max presentes no knob '{}.{}'",
                                        a.name, c.name
                                    ),
                                    got: "min ou max ausente".into(),
                                })
                            }
                        };
                        // Faixa DEVE ser finita e não-degenerada. A ORDEM não
                        // é validada de propósito: knobs bidirecionais (ex.:
                        // Pitch.L-Pitch 0..-24) vêm com min>max no dicionário
                        // — normalização fica em Control::range().
                        if !lo.is_finite() || !hi.is_finite() || lo == hi {
                            return Err(ProtocolError::InvalidShape {
                                expected: format!(
                                    "min != max (finitos) no knob '{}.{}'",
                                    a.name, c.name
                                ),
                                got: format!("min={lo}, max={hi}"),
                            });
                        }
                    }
                    // switch/combox: precisa de >=1 opção e pares rótulo/id.
                    ControlKind::Switch | ControlKind::Combox => {
                        if c.options.is_empty() || c.options.len() != c.option_ids.len() {
                            return Err(ProtocolError::InvalidShape {
                                expected: format!(
                                    "options.len() == option_ids.len() em '{}.{}'",
                                    a.name, c.name
                                ),
                                got: format!(
                                    "options={}, option_ids={}",
                                    c.options.len(),
                                    c.option_ids.len()
                                ),
                            });
                        }
                    }
                }
            }
        }

        Ok(Dictionary {
            meta: raw.meta,
            algorithms: raw.algorithms,
            by_full,
            by_key,
        })
    }
    /// Lookup O(1) pela identidade COMPLETA `(module, nibble, index)` — o
    /// editor sempre conhece o módulo (vem do slot da cadeia do preset).
    /// Resolve inclusive os efeitos dual-módulo (Boost em PRE vs DST, com
    /// defaults divergentes).
    pub fn algorithm_in_module(&self, module: &str, nibble: u8, index: u32) -> Option<&Algorithm> {
        self.by_full
            .get(&(module.to_string(), nibble, index))
            .map(|&i| &self.algorithms[i])
    }

    /// Lookup O(1) por `(nibble, index)` — display/quick-look (retorna a
    /// PRIMEIRA ocorrência no arquivo; Boost cai no PRE, não no DST).
    /// Para diferenças por módulo usar [`Dictionary::algorithm_in_module`].
    pub fn algorithm(&self, nibble: u8, index: u32) -> Option<&Algorithm> {
        self.by_key
            .get(&(nibble, index))
            .map(|&i| &self.algorithms[i])
    }

    /// Os algoritmos na ordem do arquivo (para varredura/UI).
    pub fn algorithms(&self) -> &[Algorithm] {
        &self.algorithms
    }

    /// Metadados de proveniência do arquivo (product, validation, ...).
    pub fn meta(&self) -> &serde_json::Value {
        &self.meta
    }

    /// Busca por nome exato (diagnóstico).
    pub fn by_name(&self, name: &str) -> Option<&Algorithm> {
        self.algorithms.iter().find(|a| a.name == name)
    }

    /// Número de algoritmos (185 no arquivo atual).
    pub fn len(&self) -> usize {
        self.algorithms.len()
    }

    /// `true` se o dicionário não tem algoritmos (arquivo vazio/corrompido).
    pub fn is_empty(&self) -> bool {
        self.algorithms.is_empty()
    }

    /// Total de controles somados (639 no arquivo atual).
    pub fn controls_count(&self) -> usize {
        self.algorithms.iter().map(|a| a.controls.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Corrompe o JSON embutido via Value (caminho REAL de parse) e devolve
    /// o JSON corrompido — usado pelos testes de rejeição.
    fn corrupt(f: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut v: serde_json::Value = serde_json::from_str(DICTIONARY_JSON).expect("json válido");
        f(&mut v);
        v.to_string()
    }

    /// O dicionário embutido deve carregar 100% válido (DoD: 185/639).
    #[test]
    fn loads_embedded_dictionary() {
        let d = Dictionary::from_json(DICTIONARY_JSON).expect("dicionário válido");
        assert_eq!(d.len(), 185, "185 algoritmos (validação 3 vias)");
        assert_eq!(d.controls_count(), 639, "639 controles");
        assert!(d.by_name("COMP").is_some(), "alg baseline presente");
    }

    /// Lookup O(1) por (nibble,index) — caso REAL da 1ª edição da fixture
    /// knobs.jsonl: payload colapsa p/ [6e 00 00 07][00][00][f32 LE 15.0]
    /// = nibble 0x07, index 0x6e = "Bog RedM" (AMP), code 0x0700006E.
    #[test]
    fn lookup_by_key_matches_knob_fixture() {
        let d = Dictionary::from_json(DICTIONARY_JSON).expect("válido");
        let a = d.algorithm(0x07, 0x6e).expect("alg (0x07,0x6e) existe");
        assert_eq!(a.name, "Bog RedM");
        assert_eq!(a.module, "AMP");
        assert_eq!(a.code, 0x0700_006e);
        assert!(!a.controls.is_empty());
        // e o caminho reverso (por nome) acha o mesmo registro
        assert_eq!(d.by_name("Bog RedM").map(|x| x.code), Some(0x0700_006e));
    }

    /// Teste DoD: rejeita dicionário com tripla (module,nibble,index) DUPLICADA.
    #[test]
    fn rejects_duplicate_key() {
        let json = corrupt(|v| {
            let algos = v["algorithms"].as_array_mut().expect("array");
            let first = algos[0].clone();
            algos[1] = first; // duplicata sintética da tripla de alg[0]
        });
        let err = Dictionary::from_json(&json).expect_err("deve rejeitar");
        assert!(matches!(err, ProtocolError::InvalidShape { .. }));
    }

    /// Achado estrutural: Boost (nibble 0, index 26) existe em PRE e DST com
    /// defaults divergentes; o fallback por (nibble,index) casa com a
    /// semântica first-wins do validate_knob_map (PRE vence, ordem do arquivo).
    #[test]
    fn dual_module_boost_resolves_by_module() {
        let d = Dictionary::from_json(DICTIONARY_JSON).expect("válido");
        let pre = d
            .algorithm_in_module("PRE", 0, 26)
            .expect("Boost@PRE existe");
        let dst = d
            .algorithm_in_module("DST", 0, 26)
            .expect("Boost@DST existe");
        assert_eq!((pre.name.as_str(), pre.module.as_str()), ("Boost", "PRE"));
        assert_eq!((dst.name.as_str(), dst.module.as_str()), ("Boost", "DST"));
        // defaults divergentes preservados (Bright: PRE "1", DST "0")
        let bright = |a: &Algorithm| {
            a.controls
                .iter()
                .find(|c| c.name == "Bright")
                .and_then(|c| c.default.clone())
                .expect("Bright existe")
        };
        assert_eq!((bright(pre).as_str(), bright(dst).as_str()), ("1", "0"));
        // fallback first-wins: mesma linha do PRE
        assert_eq!(d.algorithm(0, 26).map(|a| a.module.as_str()), Some("PRE"));
        // 14 Boost também é dual-módulo
        assert!(d.algorithm_in_module("DST", 0, 14).is_some());
    }

    /// Teste DoD: rejeita knob DEGENERADO (min == max — faixa nula).
    #[test]
    fn rejects_bad_range() {
        let json = corrupt(|v| {
            v["algorithms"][0]["controls"][0]["min"] = serde_json::json!(50.0);
            v["algorithms"][0]["controls"][0]["max"] = serde_json::json!(50.0);
        });
        let err = Dictionary::from_json(&json).expect_err("deve rejeitar");
        assert!(matches!(err, ProtocolError::InvalidShape { .. }));
    }

    /// Achado estrutural: knobs BIDIRECIONAIS vêm com min>max no dicionário
    /// (Pitch.L-Pitch: 0..-24, 0 = centro). `range()` normaliza para
    /// (lo, hi); o dicionário REAL carrega 100%.
    #[test]
    fn bidirectional_knob_range_normalizes() {
        let d = Dictionary::from_json(DICTIONARY_JSON).expect("dicionário real carrega");
        let pitch = d.by_name("Pitch").expect("Pitch existe");
        let lp = pitch
            .controls
            .iter()
            .find(|c| c.name == "L-Pitch")
            .expect("L-Pitch existe");
        assert_eq!(lp.min, Some(0.0));
        assert_eq!(lp.max, Some(-24.0));
        assert_eq!(lp.range(), Some((-24.0, 0.0)));
    }

    /// Teste DoD: rejeita code que não bate com (nibble,index) — quebra a
    /// identidade do §13.11/`.prst` (se algum dia o gerador mudar).
    #[test]
    fn rejects_broken_code_identity() {
        let json = corrupt(|v| {
            v["algorithms"][0]["code"] = serde_json::json!(12345);
        });
        let err = Dictionary::from_json(&json).expect_err("deve rejeitar");
        assert!(matches!(err, ProtocolError::InvalidShape { .. }));
    }

    /// Teste DoD: rejeita switch com options/option_ids descasados.
    #[test]
    fn rejects_mismatched_options() {
        let json = corrupt(|v| {
            // acha o primeiro switch e remove uma option
            for a in v["algorithms"].as_array_mut().expect("array") {
                for c in a["controls"].as_array_mut().expect("array") {
                    if c["type"] == "switch" {
                        c["options"].as_array_mut().expect("array").pop();
                        return;
                    }
                }
            }
            panic!("nenhum switch no dicionário?");
        });
        let err = Dictionary::from_json(&json).expect_err("deve rejeitar");
        assert!(matches!(err, ProtocolError::InvalidShape { .. }));
    }
}
