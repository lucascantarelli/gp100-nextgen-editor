//! Contratos do RECORTE de um preset (#114) — ângulo caixa-preta.
//!
//! O app embute o `all.prst` inteiro e exporta UM preset. Estes testes travam
//! o que o recorte promete, e o mais importante **não** é a forma do arquivo:
//! é que a **view de board seja idêntica** à do preset dentro do arquivo
//! original. Se o recorte perder um slot, um knob ou o estado de ligado, o
//! arquivo continua "válido" e o dono ainda perde o timbre — e é exatamente
//! esse o bug que um teste de forma não pega.
//!
//! O segundo é o round-trip: o recorte tem de sobreviver a
//! `.prst → JSON → .prst` byte a byte, como qualquer outro documento. Sem isso,
//! o app exportaria um JSON que não volta.

use std::path::PathBuf;

use gp100_core::model::{Dictionary, DICTIONARY_JSON};
use gp100_core::pedalboard::board_view_for;
use gp100_core::preset::Document;
use gp100_core::preset_json::{from_json, to_json};

fn all_prst() -> Vec<u8> {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    root.push("files");
    root.push("patches");
    std::fs::read(root.join("all.prst")).expect("all.prst")
}

fn documento() -> Document {
    Document::parse(&all_prst()).expect("all.prst válido")
}

fn dicionario() -> Dictionary {
    Dictionary::from_json(DICTIONARY_JSON).expect("dicionário")
}

/// O recorte fica com UM `<presets>`, sem a tabela de IRs e com `count` certo.
#[test]
fn recorte_fica_com_um_preset_e_a_forma_dos_arquivos_reais() {
    let recorte = documento().apenas_preset(Some(0x19)).expect("recorta");

    let blocos: Vec<_> = recorte
        .root()
        .children()
        .iter()
        .filter(|c| c.name() == "presets")
        .collect();
    assert_eq!(blocos.len(), 1, "um bloco de preset, e só");

    // a tabela de IRs é do arquivo de biblioteca; o `.prst` de um preset não tem
    assert!(
        recorte.root().child("ppIRInfo").is_none(),
        "sem <ppIRInfo> no recorte"
    );
    assert_eq!(
        recorte.preset_info().and_then(|i| i.attr("count")),
        Some("1"),
        "count descreve ESTE documento"
    );
    assert!(
        recorte.root().child("preset_info").is_some(),
        "os metadados do arquivo continuam"
    );
}

/// **O teste que importa:** a view de board do recorte é a do preset original.
#[test]
fn a_cadeia_sobrevive_ao_recorte() {
    let doc = documento();
    let dict = dicionario();
    for pp in [0x00u16, 0x18, 0x19, 0x30] {
        let original = board_view_for(&doc, &dict, Some(pp)).expect("pp existe no all.prst");
        let recorte = doc.apenas_preset(Some(pp)).expect("recorta");
        let do_recorte = board_view_for(&recorte, &dict, None).expect("preset do recorte");
        assert_eq!(
            do_recorte, original,
            "a cadeia do preset {pp:#06x} mudou no recorte"
        );
        assert_eq!(do_recorte.slots.len(), 9, "a cadeia tem os 9 slots");
    }
}

/// O recorte é um documento válido: reparseia (é o que o parser strict prova).
#[test]
fn recorte_reparseia() {
    let recorte = documento().apenas_preset(Some(0x05)).expect("recorta");
    let bytes = recorte.to_bytes();
    let de_novo = Document::parse(&bytes).expect("o recorte é relível");
    assert_eq!(de_novo.to_bytes(), bytes, "writer idempotente no recorte");
}

/// O recorte sobrevive ao JSON byte a byte (o caminho que o app usa).
#[test]
fn recorte_passa_pelo_json_byte_identico() {
    let recorte = documento().apenas_preset(Some(0x19)).expect("recorta");
    let bytes = recorte.to_bytes();
    let json = to_json(&recorte).expect("exporta");
    let voltou = from_json(&json).expect("importa");
    assert_eq!(voltou.to_bytes(), bytes, "o recorte volta byte a byte");
    assert_eq!(
        to_json(&voltou).expect("re-exporta"),
        json,
        "e o JSON continua canônico"
    );
}

/// `None` = o primeiro preset do arquivo (mesma regra do `board_view_for`).
#[test]
fn sem_pp_pega_o_primeiro() {
    let doc = documento();
    let recorte = doc.apenas_preset(None).expect("recorta o primeiro");
    assert_eq!(
        recorte.presets().count(),
        1,
        "o recorte tem exatamente um preset"
    );
    assert_eq!(
        recorte.presets().next().and_then(|p| p.pp_name()),
        doc.presets().next().and_then(|p| p.pp_name()),
        "e é o MESMO preset do arquivo"
    );
}

/// Um pp inexistente é erro claro, não um recorte silencioso de outro preset.
///
/// O pp é DECIMAL (o mesmo espaço do fio — #156), então a mensagem tem de
/// nomeá-lo nessa base: `32767` e não `7fff`.
#[test]
fn pp_inexistente_e_erro() {
    let doc = documento();
    let erro = doc
        .apenas_preset(Some(32767))
        .expect_err("pp inexistente tem de recusar");
    let msg = erro.to_string();
    assert!(
        msg.contains("32767"),
        "a mensagem tem de nomear o pp pedido: {msg}"
    );
}

/// Documento de um preset só recortado por pp continua funcionando (idempotente).
#[test]
fn recortar_um_recorte_e_estavel() {
    let doc = documento();
    let uma = doc.apenas_preset(Some(0x19)).expect("recorta");
    assert_eq!(
        uma.to_bytes(),
        uma.apenas_preset(None).expect("recorta de novo").to_bytes(),
        "recortar o próprio recorte dá o mesmo documento"
    );
}
