//! Testes de CONTRATO da folha de timbre (#114) — ângulo caixa-preta.
//!
//! **O teste que importa é o do xref.** Um PDF pode ter o cabeçalho certo, o
//! `%%EOF` e todo o texto esperado, e ainda assim não abrir — porque a tabela
//! `xref` guarda o OFFSET EM BYTES de cada objeto, e um offset errado por um
//! byte faz o leitor desistir. Por isso [`xref_aponta_para_os_objetos`] percorre
//! a tabela e confere que cada offset cai exatamente no começo do objeto
//! declarado. É a checagem que substitui "abri no leitor e funcionou" por algo
//! que roda na CI dos 3 SOs.
//!
//! **Duas armadilhas que estes testes atravessam, e que custaram uma rodada:**
//!
//! 1. o texto do PDF é um literal ENTRE PARÊNTESES, e parênteses DENTRO dele são
//!    escapados com barra. Procurar `"CADEIA (ordem do sinal)"` cru no arquivo
//!    não acha nada, porque o arquivo tem `CADEIA \(ordem do sinal\)`. Daí
//!    [`tem_literal`]/[`tem_trecho`], que escapam a agulha;
//! 2. o cabeçalho do PDF tem um comentário com bytes > 127 (a marca de arquivo
//!    binário). `String::from_utf8_lossy` troca cada byte inválido por um
//!    caractere de 3 bytes, então **índice de string não é índice de byte** — e
//!    conferir offset de xref numa `String` dá panic em fronteira de caractere.
//!    A conferência de offset é feita nos BYTES crus.
//!
//! A verificação final é fora do Rust: `pdftotext` (poppler) lê a folha como um
//! humano leria, e foi o que se rodou para aceitar o layout.

use std::path::PathBuf;

use gp100_core::model::{Dictionary, DICTIONARY_JSON};
use gp100_core::preset::Document;
use gp100_core::preset_json::from_json;
use gp100_core::tone_sheet::tone_sheet_pdf;

fn patch_files() -> Vec<PathBuf> {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    root.push("files");
    root.push("patches");
    ["all.prst", "Blink OD.prst", "its gp100.prst"]
        .into_iter()
        .map(|n| root.join(n))
        .collect()
}

fn dicionario() -> Dictionary {
    Dictionary::from_json(DICTIONARY_JSON).expect("dicionário embutido válido")
}

fn folha(nome: &str) -> Vec<u8> {
    let bytes = std::fs::read(
        patch_files()
            .into_iter()
            .find(|p| p.file_name().unwrap().to_string_lossy() == nome)
            .expect("arquivo existe"),
    )
    .expect("ler");
    let doc = Document::parse(&bytes).expect("válido");
    tone_sheet_pdf(&doc, &dicionario()).expect("gera")
}

/// Escapa como o writer de PDF: `(`, `)` e `\` ganham uma barra.
fn escapa(texto: &str) -> String {
    let mut out = String::with_capacity(texto.len());
    for c in texto.chars() {
        if matches!(c, '(' | ')' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// O arquivo contém o trecho `texto` em algum literal de string?
///
/// Escapa só a agulha: os parênteses que DELIMITAM o literal não são escapados
/// no arquivo, e só os que aparecem DENTRO do texto é que ganham barra.
fn tem_trecho(pdf: &[u8], texto: &str) -> bool {
    String::from_utf8_lossy(pdf).contains(&escapa(texto))
}

/// O arquivo contém um literal de string EXATAMENTE igual a `texto`?
fn tem_literal(pdf: &[u8], texto: &str) -> bool {
    String::from_utf8_lossy(pdf).contains(&format!("({})", escapa(texto)))
}

#[test]
fn pdf_tem_cabecalho_e_trailer() {
    let pdf = folha("Blink OD.prst");
    let s = String::from_utf8_lossy(&pdf);
    assert!(s.starts_with("%PDF-1.4"), "cabeçalho de versão do PDF");
    assert!(s.trim_end().ends_with("%%EOF"), "marcador de fim");
    assert!(s.contains("trailer"), "trailer obrigatório");
    assert!(s.contains("/Root 1 0 R"), "o trailer aponta o catálogo");
}

/// A checagem estrutural de verdade: cada offset da `xref` cai no objeto certo.
#[test]
fn xref_aponta_para_os_objetos() {
    let pdf = folha("Blink OD.prst");

    // Nos BYTES crus: o comentário binário do cabeçalho faz a `String` do
    // `from_utf8_lossy` ter índices diferentes dos do arquivo.
    let marca = b"\nxref\n";
    let pos = pdf
        .windows(marca.len())
        .position(|w| w == marca)
        .expect("tabela xref")
        + marca.len();
    let tabela = std::str::from_utf8(&pdf[pos..]).expect("a xref é ASCII");

    let mut linhas = tabela.lines();
    let cabecalho = linhas.next().expect("cabeçalho da xref");
    let mut partes = cabecalho.split_whitespace();
    let primeiro: usize = partes.next().unwrap().parse().unwrap();
    let total: usize = partes.next().unwrap().parse().unwrap();
    assert_eq!(primeiro, 0, "a xref sempre começa na entrada 0");

    let entradas: Vec<&str> = linhas.take(total).collect();
    assert_eq!(entradas.len(), total, "uma entrada por objeto");
    assert_eq!(entradas[0], "0000000000 65535 f ", "entrada 0 é a livre");

    for (num, entrada) in entradas.iter().enumerate().skip(1) {
        let offset: usize = entrada[..10].parse().expect("offset decimal");
        let esperado = format!("{num} 0 obj").into_bytes();
        assert!(
            pdf[offset..].starts_with(&esperado),
            "a xref diz que o objeto {num} começa em {offset}, mas lá tem: {:?}",
            String::from_utf8_lossy(&pdf[offset..(offset + 20).min(pdf.len())])
        );
    }
}

/// A folha mostra o preset, os 9 módulos e os knobs com valor.
#[test]
fn folha_mostra_preset_cadeia_e_knobs() {
    let pdf = folha("Blink OD.prst");
    assert!(tem_literal(&pdf, "Blink OD"), "o nome do preset aparece");
    for modulo in ["PRE", "DST", "AMP", "NR", "CAB", "EQ", "MOD", "DLY", "RVB"] {
        assert!(tem_literal(&pdf, modulo), "falta o módulo {modulo}");
    }
    // o NOME do knob vem do dicionário; o VALOR vem do .prst (Sustain =
    // params_0 do COMP, e o arquivo traz "50")
    assert!(tem_trecho(&pdf, "Sustain"), "nome do knob pelo dicionário");
    assert!(tem_trecho(&pdf, "50"), "valor do knob pelo .prst");
    assert!(
        tem_trecho(&pdf, "folha de timbre"),
        "o rodapé identifica a folha"
    );
}

/// O título da cadeia e o estado do slot aparecem EM TEXTO (não só em cinza).
#[test]
fn folha_diz_o_estado_do_slot_em_texto() {
    let pdf = folha("Blink OD.prst");
    assert!(tem_literal(&pdf, "CADEIA (ordem do sinal)"));
    assert!(tem_trecho(&pdf, "(ligado)"), "slot ligado dito em texto");
    assert!(tem_trecho(&pdf, "(bypass)"), "slot em bypass dito em texto");
}

/// Um `.prst` de fábrica tem 1 preset; o `all.prst` tem 99 — e 99 páginas.
#[test]
fn uma_pagina_por_preset() {
    let um = folha("Blink OD.prst");
    let s1 = String::from_utf8_lossy(&um);
    assert_eq!(s1.matches("/Type /Page ").count(), 1, "uma página");
    assert!(
        tem_literal(&um, "1/1"),
        "o brasão diz que é o preset 1 de 1"
    );

    let todos = folha("all.prst");
    let s = String::from_utf8_lossy(&todos);
    assert_eq!(
        s.matches("/Type /Page ").count(),
        99,
        "99 presets = 99 páginas"
    );
    assert!(s.contains("/Count 99"), "o nó /Pages declara a contagem");
    assert!(
        tem_literal(&todos, "99/99"),
        "o último preset se identifica como 99 de 99"
    );
}

/// A folha é determinística: dois pedidos dão bytes idênticos.
#[test]
fn folha_e_deterministica() {
    assert_eq!(folha("Blink OD.prst"), folha("Blink OD.prst"));
}

/// Não há stream comprimido: o conteúdo é inspecionável no olho.
///
/// O teste é o CONTRATO, não uma checagem de linker: o PDF é montado byte a byte
/// em memória, sem I/O e sem zlib. Se alguém trocar isso por uma crate de PDF
/// com stream comprimido, este teste cai — e é ele que sustenta a afirmação de
/// "sem dependência de sistema" da issue.
#[test]
fn stream_nao_e_comprimido_e_o_texto_e_legivel() {
    let pdf = folha("its gp100.prst");
    let s = String::from_utf8_lossy(&pdf);
    assert!(
        !s.contains("/Filter"),
        "nenhum filtro: o stream é texto puro"
    );
    assert!(s.contains("stream\n"), "há stream de conteúdo");
    assert!(s.contains("Tj ET"), "os operadores de texto são visíveis");
}

/// Documento sem `<presets>` é erro CLARO, não um PDF de folha em branco.
///
/// O caso chega pelo JSON, que é o único caminho público capaz de montar um
/// `<GP-100>` sem filhos (o parser aceita a raiz sozinha; quem exige um
/// `<presets>` é a folha, que não tem o que desenhar sem ele).
#[test]
fn documento_sem_presets_e_erro() {
    let json = r#"{
      "format": "gp100.preset",
      "version": 1,
      "declaracao": "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
      "depois": "\r\n",
      "raiz": { "nome": "GP-100", "antes": "\r\n\r\n", "fechaAntes": "\r\n" }
    }"#;
    let doc = from_json(json).expect("envelope válido");
    let erro = tone_sheet_pdf(&doc, &dicionario()).expect_err("gerar folha sem preset é erro");
    assert!(
        erro.to_string().contains("presets"),
        "a mensagem tem de dizer o que falta: {erro}"
    );
}
