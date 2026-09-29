//! Contrato do README (M0.8): os exemplos de [gp100-core/README.md](../README.md)
//! RODAM como estão escritos — DoD da M0.8 ("contribuidor novo compila, testa e
//! entende o core em <15min") e a régua da casa de doc que mente (rust-practices:
//! "documentação que mente quebra o `cargo test`"). Se este arquivo compila e
//! passa, o README não mente. Ao alterar um exemplo do README, atualizar AQUI
//! também (1:1, propósito deliberado).

use gp100_core::golden::{decode_envelope, GoldenFile};
use gp100_core::model::Dictionary;
use gp100_core::preset::Document;
use gp100_core::session::Session;
use gp100_core::transport::mock::MockDevice;
use gp100_core::transport::DeviceTransport;

/// Exemplo "Dicionário" do README: 185 algs, lookup por nome.
#[test]
fn readme_dicionario() {
    let dict = Dictionary::from_json(gp100_core::model::DICTIONARY_JSON).expect("carga");
    assert_eq!(dict.len(), 185);
    let bog = dict.by_name("Bog RedM");
    assert!(
        bog.is_some(),
        "Bog RedM existe no dicionário (vetor da fixture knobs)"
    );
}

/// Exemplo "Presets .prst" do README: parse + views + round-trip byte-idêntico.
#[test]
fn readme_presets() {
    // O README lê do disco; o caminho canônico do repo a partir de tests/ é
    // <raiz>/files/patches/all.prst (regime de bytes: arquivo do repo).
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.push("files");
    p.push("patches");
    p.push("all.prst");
    let xml = std::fs::read_to_string(&p).expect("all.prst no repo");
    let doc = Document::parse(xml.as_bytes()).expect("parse");
    let first = doc.presets().next().expect("all.prst tem 99 presets");
    println!(
        "pp={} nome={}",
        first.pp_id().unwrap_or("?"),
        first.pp_name().unwrap_or("?")
    );
    let bytes = doc.to_bytes();
    assert_eq!(bytes, xml.as_bytes(), "round-trip byte-idêntico (R4)");
}

/// Exemplo "Golden-file" do README: build_request do select + decode_envelope.
#[test]
fn readme_golden() {
    let golden = GoldenFile::embedded().expect("golden embutido");
    let tpl = golden
        .request_template(0x11, &[0x13, 0x01, 0x00, 0x00], 2)
        .expect("t9 (2 vars)");
    let select = tpl.build_request(&[0x01, 0x00]).expect("select");
    let (func, addr, _payload) = decode_envelope(&select).expect("envelope");
    assert_eq!(func, 0x11);
    assert_eq!(addr, [0x13, 0x01, 0x00, 0x00]);
}

/// Exemplo "Sessão sobre o MockDevice" do README: a sequência EXATA da doc —
/// list_user_irs → select_preset → state_page → set_param → save_preset.
#[test]
fn readme_sessao_sobre_o_mock() {
    let mut mock = MockDevice::new().expect("mock");
    mock.open().expect("open do chamador");
    let mut session = Session::new(&mut mock);

    let table = session.list_user_irs().expect("tabela dos 20 User IRs");
    assert_eq!(table.slots.len(), 20);
    session.select_preset(0x0007).expect("select → meta6");
    let page = session.state_page(0).expect("página 0");
    assert_eq!(page.raw.len(), 196, "shape 13xx (196B)");
    session
        .set_param(3, 0x0700_006e, 0, 15.0)
        .expect("knob §13.11");
    session
        .save_preset(0x0007, 6, "Blink OD")
        .expect("save §13.12");
    drop(session); // devolve o transporte ao dono
}
