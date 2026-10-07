//! Espaço único de `pp` no DOCUMENTO (#132/ADR-12) — decimal, uma base só.
//!
//! O `ppID` do `all.prst` é a string `"0".."98"`, e quem lê esse atributo
//! é a base **decimal** em três lugares que precisam CONCORDAR:
//!
//! - o core (`preset_list`, `board_view_for`, `apenas_preset` — aqui);
//! - a biblioteca (`gp100_library::seed` — teste cruzado em
//!   `packages/library/tests/seed.rs`);
//! - o artefato do front (`analysis/dump_preset_list.py` →
//!   `FACTORY_PRESETS`, que ASSERTA `0..98`).
//!
//! Antes desta issue o core lia HEX: `board_view_for(Some(24))` casava o
//! bloco `ppID="18"` (24 = 0x18), e a lista do app — decimal — clicava em
//! presets que o board resolvia DIFERENTES a partir de 10. Os testes daqui
//! são os que fazem essa divergência voltar a falhar se alguém trocar a
//! base num dos lados.

use gp100_core::model::{Dictionary, DICTIONARY_JSON};
use gp100_core::pedalboard::{board_view_for, preset_list};
use gp100_core::preset::{indice_do_documento, pp_id_decimal, Document};
use gp100_core::transport::DeviceTransport;

fn documento() -> Document {
    let mut root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    root.push("files");
    root.push("patches");
    let bytes = std::fs::read(root.join("all.prst")).expect("all.prst");
    Document::parse(&bytes).expect("all.prst válido")
}

fn dicionario() -> Dictionary {
    Dictionary::from_json(DICTIONARY_JSON).expect("dicionário")
}

/// **Critério de aceite 4 (#132): a lista do documento é 0..98 em decimal**,
/// o mesmo array que a biblioteca semeia e que o artefato do front publica.
#[test]
fn a_lista_do_documento_e_0_a_98_em_decimal() {
    let doc = documento();
    let lista = preset_list(&doc);
    assert_eq!(lista.len(), 99, "99 presets");
    for (i, e) in lista.iter().enumerate() {
        assert_eq!(e.pp, i as u16, "pp contíguo e decimal (o hex daria saltos)");
        assert!(!e.name.is_empty(), "todo preset tem nome");
    }
}

/// O teste do bug: sob base hex, `pp 24` resolvia o bloco `ppID="18"` e
/// `pp 18` nem casava o mesmo preset da lista. Decimal casa o bloco certo —
/// e casa para TODOS os 99, não só para os convenientes.
#[test]
fn o_board_resolve_o_bloco_certo_do_documento() {
    let doc = documento();
    let dict = dicionario();

    let bloco_18 = doc
        .presets()
        .find(|p| p.pp_id() == Some("18"))
        .expect("ppID 18 existe");
    let b24 = board_view_for(&doc, &dict, Some(24)).expect("pp 24 existe");
    assert_ne!(
        b24.name,
        bloco_18.pp_name().unwrap_or(""),
        "pp 24 NÃO pode casar o bloco ppID 18 (bug da base hex)"
    );

    for p in doc.presets() {
        let pp = pp_id_decimal(p.pp_id().expect("todo preset tem ppID")).expect("decimal");
        let b = board_view_for(&doc, &dict, Some(pp))
            .unwrap_or_else(|e| panic!("pp {pp} não resolveu: {e}"));
        assert_eq!(b.pp, pp, "a view devolve o pp pedido (lista 0..98)");
        assert_eq!(
            b.name,
            p.pp_name().unwrap_or(""),
            "pp {pp}: o board é o bloco de MESMO ppID"
        );
    }
}

/// O fio carrega o banco no byte alto (`0x0100..=0x0162` na captura S1);
/// o documento não tem banco — o índice é o byte baixo, e a view devolve o
/// pp do DOCUMENTO para a UI acender na lista. Sem isto, a abertura pelo
/// `current_pp` do aparelho (`0x0100`) não encontrava bloco nenhum.
#[test]
fn pp_de_banco_do_fio_vira_indice_do_documento() {
    let doc = documento();
    let dict = dicionario();

    assert_eq!(indice_do_documento(0x0100), 0);
    assert_eq!(indice_do_documento(0x0162), 98);
    assert_eq!(indice_do_documento(0x0042), 0x42, "banco 0 é identidade");
    // Byte alto fora de 0x00/0x01 passa INTACTO: `0x0200 & 0xFF` abriria o
    // preset 0 por engano — o lookup tem de falhar, não adivinhar.
    assert_eq!(indice_do_documento(0x0200), 0x0200);

    let banco1 = board_view_for(&doc, &dict, Some(0x0100)).expect("banco 1, índice 0");
    let banco0 = board_view_for(&doc, &dict, Some(0x0000)).expect("banco 0, índice 0");
    assert_eq!(
        banco1, banco0,
        "os dois bancos têm o mesmo conteúdo no índice"
    );
    assert_eq!(banco1.pp, 0, "a view devolve o pp DO DOCUMENTO (0..98)");

    let erro = board_view_for(&doc, &dict, Some(0x0200))
        .expect_err("byte alto desconhecido não resolve bloco");
    assert!(
        erro.to_string().contains("0200"),
        "a mensagem nomeia o pp pedido: {erro}"
    );
}

/// A exportação (`apenas_preset`) usa a MESMA expressão do board — o
/// "preset 25" da tela e o "preset 25" do arquivo nunca divergem, e o pp
/// do fio com banco também exporta o bloco certo.
#[test]
fn apenas_preset_usa_a_mesma_base_e_aceita_pp_de_banco() {
    let doc = documento();
    let direto = doc.apenas_preset(Some(0)).expect("recorta o 0");
    let pelo_banco = doc.apenas_preset(Some(0x0100)).expect("recorta o 0x0100");
    assert_eq!(
        direto.to_bytes(),
        pelo_banco.to_bytes(),
        "0x0100 e 0 são o MESMO bloco"
    );

    let erro = doc
        .apenas_preset(Some(0x0200))
        .expect_err("byte alto desconhecido não recorta");
    assert!(
        erro.to_string().contains("0200"),
        "a mensagem nomeia o pp pedido: {erro}"
    );
}

/// O mock deriva o `current_pp` do primeiro preset do documento — na mesma
/// base que o resto (decimal): `ppID "0"` → 0, que é o que a UI abre.
#[test]
fn o_mock_nasce_no_pp_zero_do_documento() {
    let mut mock = gp100_core::transport::mock::MockDevice::new().expect("mock");
    mock.open().expect("open");
    assert_eq!(mock.state().current_pp, 0, "ppID \"0\" lido como decimal");
}
