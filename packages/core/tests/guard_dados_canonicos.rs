//! Guard da Era Hardware (HW-3, #163): `packages/**` NUNCA referencia a
//! pasta de análises. O dado canônico versionado mora em
//! `packages/core/data/` (dicionário, offsets, fixtures do fio); a pasta
//! de análises é histórico/processo da era de RE, jamais insumo de runtime
//! ou de teste. DoD literal da issue: `git grep` pela sequência proibida
//! em `packages/` = 0 — este teste é a versão executável do guard (roda em
//! qualquer checkout, sem depender de git).

use std::path::Path;

/// Extensões de fonte/manifesto cobertas pelo guard (código, comentário,
/// include e manifesto).
const EXTENSOES: &[&str] = &["rs", "ts", "tsx", "toml", "json", "py", "md", "yml", "yaml"];

/// Diretórios pulados: artefatos de build/dependências nunca são fonte.
const PULA: &[&str] = &[
    "target",
    "node_modules",
    "dist",
    "test-results",
    "playwright-report",
    ".git",
];

/// Varre `dir` em profundidade; registra `arquivo:linha` de cada ocorrência
/// da sequência proibida.
fn varra(dir: &Path, achar: &str, achados: &mut Vec<String>) {
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entradas.flatten() {
        let p = e.path();
        let nome = e.file_name().to_string_lossy().into_owned();
        if p.is_dir() {
            if PULA.contains(&nome.as_str()) {
                continue;
            }
            varra(&p, achar, achados);
        } else if EXTENSOES
            .iter()
            .any(|x| p.extension().is_some_and(|ext| ext == *x))
        {
            if let Ok(texto) = std::fs::read_to_string(&p) {
                if let Some(i) = texto.find(achar) {
                    let linha = texto[..i].matches('\n').count() + 1;
                    achados.push(format!("{}:{linha}", p.display()));
                }
            }
        }
    }
}

#[test]
fn packages_nao_referencia_a_pasta_de_analises() {
    // A sequência proibida é montada em runtime: este arquivo não pode
    // contê-la literalmente (senão o guard se acusaria) — e é justamente
    // o que o DoD pede: zero ocorrências do path de análises no pacote.
    let achar = ["analy", "sis", "/"].concat();
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("packages/core tem pai (packages/)");
    let mut achados = Vec::new();
    varra(raiz, &achar, &mut achados);
    achados.sort();
    assert!(
        achados.is_empty(),
        "packages/** referencia a pasta de análises — o dado canônico é \
         packages/core/data (HW-3/#163). Mova o dado para cá e aponte o \
         caminho morto para a nova casa:\n{}",
        achados.join("\n")
    );
}
