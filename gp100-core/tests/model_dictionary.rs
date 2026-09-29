//! Testes de CONTRATO do dicionário — ROADMAP M0.1 (modelo híbrido).
//!
//! Ângulo deliberado: `gp100-core` é consumido como **caixa-preta** (só API
//! `pub`), exatamente como o app/CLI fará. Invariantes que exigem acesso a
//! internals ficariam como unitários em `src/` (hoje nenhum é necessário —
//! os testes de consts/framing do lib.rs permanecem lá).

mod common;

use common::{corrupt, expect_invalid_shape};
use gp100_core::model::{Dictionary, DICTIONARY_JSON};

/// O dicionário embutido carrega 100% válido (DoD M0.1: 185/639).
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
    let (expected, _got) = expect_invalid_shape(&json);
    assert!(expected.contains("únicas"), "mensagem: {expected}");
}

/// Achado estrutural: Boost (nibble 0, index 26) existe em PRE e DST com
/// defaults divergentes; o fallback por (nibble,index) casa com a semântica
/// first-wins do validate_knob_map (PRE vence, ordem do arquivo).
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
    let bright = |a: &gp100_core::model::Algorithm| {
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
    let (expected, got) = expect_invalid_shape(&json);
    assert!(expected.contains("min != max"), "mensagem: {expected}");
    assert!(got.contains("min=50"), "detalhe: {got}");
}

/// Achado estrutural: knobs BIDIRECIONAIS vêm com min>max no dicionário
/// (Pitch.L-Pitch: 0..-24, 0 = centro). `range()` normaliza para (lo, hi);
/// o dicionário REAL carrega 100%.
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
    let (expected, _got) = expect_invalid_shape(&json);
    assert!(
        expected.contains("(nibble<<24)|index"),
        "mensagem: {expected}"
    );
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
    let (expected, _got) = expect_invalid_shape(&json);
    assert!(expected.contains("option_ids"), "mensagem: {expected}");
}
