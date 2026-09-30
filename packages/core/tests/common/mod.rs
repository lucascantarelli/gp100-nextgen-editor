//! Helpers compartilhados pelos testes de integração de `gp100-core/tests/`.
//!
//! Organização (modelo híbrido, análogo ao pytest):
//! - `tests/<assunto>.rs`  ≈ `tests/test_<assunto>.py` — um binário de teste
//!   por arquivo, consumindo a lib como caixa-preta (só API `pub`);
//! - `tests/common/mod.rs` ≈ `conftest.py` — helpers compartilhados. O nome
//!   `mod.rs` é o que impede esta pasta de virar um binário de teste próprio.
//!
//! Disponível aos testes: [dependencies] + [dev-dependencies] do crate
//! (`serde_json` já é dependência normal do gp100-core).

#![allow(dead_code)] // helpers podem ser usados por apenas alguns arquivos

use gp100_core::model::DICTIONARY_JSON;

use std::path::PathBuf;

/// Lê uma fixture JSONL de `analysis/fixtures/` como linhas JSON cruas
/// (`serde_json::Value`). FONTE ÚNICA do caminho de leitura (regime de
/// bytes: arquivo do repo, checkout limpo — skill `rust-practices`); antes
/// estava duplicado em `codec_wire.rs` e `replay_fixtures.rs` (achado do
/// core review 29/09). O acesso aos campos fica no consumidor (`v["dir"]`).
pub fn fixture_rows(name: &str) -> Vec<serde_json::Value> {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // packages/ (crate vive em packages/core)
    p.pop(); // raiz do repo
    p.push("analysis");
    p.push("fixtures");
    p.push(name);
    std::fs::read_to_string(&p)
        .expect("fixture existe (make_fixtures.py)")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("JSONL válido"))
        .collect()
}

/// Converte uma linha de fixture em `(s, dir, func, addr, data)` hex — a
/// forma consumida pelos testes de replay.
pub fn fixture_tuple(v: serde_json::Value) -> (String, String, String, String, String) {
    (
        v["s"].as_str().unwrap_or("").to_string(),
        v["dir"].as_str().unwrap_or("").to_string(),
        v["func"].as_str().unwrap_or("").to_string(),
        v["addr"].as_str().unwrap_or("").to_string(),
        v["data"].as_str().unwrap_or("").to_string(),
    )
}

/// Corrompe o JSON embutido via `serde_json::Value` (o caminho REAL de
/// parse — mesmas regras que o serde aplica) e devolve o JSON corrompido.
/// Usado pelos testes de rejeição de `Dictionary::from_json`.
pub fn corrupt(f: impl FnOnce(&mut serde_json::Value)) -> String {
    let mut v: serde_json::Value = serde_json::from_str(DICTIONARY_JSON).expect("json válido");
    f(&mut v);
    v.to_string()
}

/// Executa `Dictionary::from_json` esperando `ProtocolError::InvalidShape`;
/// devolve `(expected, got)` para asserts de mensagem legíveis. Panica se o
/// JSON for aceito (falta de validação = falha de teste) ou se o erro for de
/// outro tipo.
pub fn expect_invalid_shape(json: &str) -> (String, String) {
    match gp100_core::model::Dictionary::from_json(json) {
        Err(gp100_core::ProtocolError::InvalidShape { expected, got }) => (expected, got),
        Ok(_) => panic!("dicionário corrompido foi ACEITO — falta validação"),
        Err(e) => panic!("erro inesperado (não é InvalidShape): {e}"),
    }
}
