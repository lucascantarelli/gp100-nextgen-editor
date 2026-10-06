//! Testes de CONTRATO da exportação em JSON (#114) — ângulo caixa-preta.
//!
//! O que estes testes trancam, e por que cada um:
//!
//! 1. **`prst → JSON → prst` byte-idêntico.** É o DoD literal da issue, e o
//!    único jeito de provar que o JSON não perdeu nada. Se a árvore esquecer um
//!    atributo, uma quebra de linha ou o indent, este teste falha no arquivo de
//!    4 KB — não é preciso comparar semântica para descobrir a perda.
//! 2. **`prst → JSON → prst → JSON` idêntico.** O JSON é canônico: exportar o
//!    que foi importado devolve o mesmo texto. Sem isto, dois exports do mesmo
//!    preset divergiriam em formatação e o diff da ferramenta viraria ruído.
//! 3. **Versão futura recusa.** Um envelope com `version` maior tem de dar erro
//!    ANTES de tocar a árvore, e a mensagem tem de dizer as duas versões.
//!
//! O material é o mesmo dos outros testes de `.prst`: os 3 arquivos reais de
//! `files/patches/`, incluindo o `all.prst` de 99 presets (413 KB), que é o
//! caso grande — o de 4 KB sozinho não provaria que a reconstrução escala.

use std::path::PathBuf;

use gp100_core::preset::Document;
use gp100_core::preset_json::{from_json, to_json, FORMATO, VERSAO};

/// Os 3 arquivos reais (all.prst = 99 presets; os outros, 1 cada).
fn patch_files() -> Vec<PathBuf> {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop(); // packages/ (crate vive em packages/core)
    root.pop(); // raiz do repo
    root.push("files");
    root.push("patches");
    ["all.prst", "Blink OD.prst", "its gp100.prst"]
        .into_iter()
        .map(|n| root.join(n))
        .collect()
}

/// DoD #114: `.prst` → JSON → `.prst` reproduz o arquivo BYTE a byte.
#[test]
fn prst_para_json_para_prst_e_byte_identico() {
    for path in patch_files() {
        let original = std::fs::read(&path).expect("arquivo existe");
        let doc = Document::parse(&original).expect("dialecto válido");

        let json = to_json(&doc).expect("exporta");
        let voltou = from_json(&json).expect("importa");

        assert_eq!(
            voltou.to_bytes(),
            original,
            "round-trip pelo JSON divergiu em {}",
            path.display()
        );
    }
}

/// O JSON é canônico: o export do import é o mesmo texto do primeiro export.
#[test]
fn json_e_canonico() {
    for path in patch_files() {
        let original = std::fs::read(&path).expect("arquivo existe");
        let doc = Document::parse(&original).expect("válido");

        let primeira = to_json(&doc).expect("exporta");
        let voltou = from_json(&primeira).expect("importa");
        let segunda = to_json(&voltou).expect("re-exporta");

        assert_eq!(
            segunda,
            primeira,
            "o JSON não é canônico em {}",
            path.display()
        );
    }
}

/// O round-trip é idempotente também no EIXO do documento.
#[test]
fn documento_reconstruido_reparseia_igual() {
    let original = std::fs::read(&patch_files()[1]).expect("Blink OD.prst");
    let doc = Document::parse(&original).expect("válido");
    let json = to_json(&doc).expect("exporta");
    let a = from_json(&json).expect("importa");
    let b = from_json(&json).expect("importa de novo");
    assert_eq!(a.to_bytes(), b.to_bytes());
}

/// O envelope se identifica: `format` e `version` estão no topo do JSON.
#[test]
fn envelope_carrega_formato_e_versao() {
    let original = std::fs::read(&patch_files()[1]).expect("Blink OD.prst");
    let doc = Document::parse(&original).expect("válido");
    let json = to_json(&doc).expect("exporta");
    let v: serde_json::Value = serde_json::from_str(&json).expect("JSON");
    assert_eq!(v["format"], FORMATO);
    assert_eq!(v["version"], VERSAO);
}

/// `version` futura recusa, com as DUAS versões na mensagem.
#[test]
fn versao_futura_recusa_com_erro_claro() {
    let original = std::fs::read(&patch_files()[1]).expect("Blink OD.prst");
    let doc = Document::parse(&original).expect("válido");
    let json = to_json(&doc).expect("exporta");

    // muda só a versão, mexendo no texto (o corpo continua válido)
    let mais_novo = json.replacen(
        &format!("\"version\": {VERSAO}"),
        &format!("\"version\": {}", VERSAO + 7),
        1,
    );
    assert_ne!(mais_novo, json, "o teste precisa ter mudado a versão");

    let erro = from_json(&mais_novo).expect_err("versão futura tem de recusar");
    let msg = erro.to_string();
    assert!(
        msg.contains(&format!("version = {}", VERSAO + 7)),
        "a mensagem tem de dizer a versão que veio: {msg}"
    );
    assert!(
        msg.contains(&format!("version = {VERSAO}")),
        "a mensagem tem de dizer a versão que este build lê: {msg}"
    );
}

/// Formato alheio recusa dizendo QUAL formato veio (não "faltou campo").
#[test]
fn formato_alheio_recusa() {
    let json = r#"{"format":"outra.coisa","version":1,"declaracao":"","raiz":{},"depois":""}"#;
    let erro = from_json(json).expect_err("formato alheio tem de recusar");
    assert!(
        erro.to_string().contains("outra.coisa"),
        "a mensagem tem de nomear o formato que veio: {erro}"
    );
}

/// JSON que nem é nosso (sem cabeçalho) recusa como "não é um preset".
#[test]
fn json_sem_cabecalho_recusa_como_nao_sendo_preset() {
    let erro = from_json(r#"{"presets":[]}"#).expect_err("tem de recusar");
    let msg = erro.to_string();
    assert!(
        msg.contains("format"),
        "a recusa tem de apontar a ausência do cabeçalho: {msg}"
    );
    assert!(
        !msg.contains("missing field"),
        "não pode acusar campo faltando — o arquivo é outro: {msg}"
    );
}

/// JSON malformado recusa sem panic.
#[test]
fn json_invalido_recusa() {
    assert!(from_json("{nao e json}").is_err());
    assert!(from_json("").is_err());
    assert!(from_json("[]").is_err());
}
