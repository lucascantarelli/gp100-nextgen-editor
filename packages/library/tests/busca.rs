//! Contrato da busca: nome, nº, estilo/tipo e banco (issue #26).

use gp100_library::{Bank, Library, SearchQuery};

mod comum;
use comum::{arquivo_temporario, patch_de_usuario, preset_de_fabrica};

/// Biblioteca de teste: 3 de fábrica (2 Rock, 1 Pop) + 2 de usuário.
fn bib() -> Library {
    let lib = Library::open_in_memory().expect("abre");
    lib.upsert(&preset_de_fabrica(0, "It's GP100", 4, "Rock"))
        .unwrap();
    lib.upsert(&preset_de_fabrica(1, "Blink OD", 6, "Pop"))
        .unwrap();
    lib.upsert(&preset_de_fabrica(2, "Star Clean", 6, "Pop"))
        .unwrap();
    lib.upsert(&patch_de_usuario("u1", "MEU LEAD", r#"{"slots":[]}"#))
        .unwrap();
    lib.upsert(&patch_de_usuario("u2", "Baixo", r#"{"slots":[]}"#))
        .unwrap();
    lib
}

#[test]
fn nome_e_substring_sem_distincao_de_caixa() {
    let lib = bib();
    let r = lib.search(&SearchQuery::nome("blink")).unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].name, "Blink OD");
}

#[test]
fn texto_casa_o_numero_que_a_tela_mostra() {
    let lib = bib();
    // A coluna mostra P01..P99 (1-based, como o app oficial): quem digita "02"
    // está procurando o preset exibido como P02, que se chama "Blink OD".
    let r = lib.search(&SearchQuery::nome("02")).unwrap();
    assert_eq!(r.len(), 1, "numero de display: {:?}", r);
    assert_eq!(r[0].name, "Blink OD");
    assert_eq!(r[0].pp, Some(1));

    // o número cru (ppID do all.prst, 0-based) também acha: quem leu o
    // PROTOCOL.md digita 1, não 02
    let r = lib.search(&SearchQuery::nome("1")).unwrap();
    assert!(r.iter().any(|x| x.name == "Blink OD"), "pp cru: {:?}", r);

    // patch de usuário não tem número de fábrica: casar "02" com ele seria
    // abrir o patch errado sem erro visível
    let so_user = lib
        .search(&SearchQuery {
            bank: Some(Bank::User),
            text: Some("02".into()),
            ..SearchQuery::default()
        })
        .unwrap();
    assert!(
        so_user.is_empty(),
        "user nao casa por numero: {:?}",
        so_user
    );
}

#[test]
fn curinga_do_usuario_e_literal_e_nao_seletor() {
    let lib = bib();
    // `%` em LIKE é "qualquer coisa". Se não fosse escapado, isto traria os 5.
    let r = lib.search(&SearchQuery::nome("%")).unwrap();
    assert!(r.is_empty(), "%% não casa nada: {:?}", r.len());

    // `_` casa UM caractere em LIKE: sem escape, "B_ink OD" acharia "Blink OD".
    let r = lib.search(&SearchQuery::nome("B_ink")).unwrap();
    assert!(r.is_empty(), "B_ink não casa Blink: {:?}", r.len());

    // O escape em si também é dado: um `\` no texto não vira "qualquer coisa".
    let r = lib.search(&SearchQuery::nome("\\")).unwrap();
    assert!(r.is_empty());
}

#[test]
fn numero_filtra_exatamente() {
    let lib = bib();
    let r = lib
        .search(&SearchQuery {
            pp: Some(1),
            ..SearchQuery::default()
        })
        .unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].pp, Some(1));
    assert_eq!(r[0].name, "Blink OD");
}

#[test]
fn estilo_filtra_por_tipo() {
    let lib = bib();
    // 6 = Pop: dois de fábrica + o patch de usuário (pp_type 4 = Rock).
    let pop = lib
        .search(&SearchQuery {
            pp_type: Some(6),
            bank: Some(Bank::Factory),
            ..SearchQuery::default()
        })
        .unwrap();
    assert_eq!(pop.len(), 2, "Pop tem Blink OD e Star Clean");

    let rock = lib
        .search(&SearchQuery {
            pp_type: Some(4),
            bank: Some(Bank::Factory),
            ..SearchQuery::default()
        })
        .unwrap();
    assert_eq!(rock.len(), 1);
    assert_eq!(rock[0].name, "It's GP100");
}

#[test]
fn filtros_aninhados_se_intersectam() {
    let lib = bib();
    // "OD" + Pop: só o Blink. "OD" + Rock: nada.
    let r = lib
        .search(&SearchQuery {
            text: Some("OD".into()),
            pp_type: Some(6),
            ..SearchQuery::default()
        })
        .unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].name, "Blink OD");

    let vazio = lib
        .search(&SearchQuery {
            text: Some("OD".into()),
            pp_type: Some(4),
            ..SearchQuery::default()
        })
        .unwrap();
    assert!(vazio.is_empty());
}

#[test]
fn banco_separa_e_a_ordem_da_biblioteca_e_por_numero() {
    let lib = bib();
    let fab = lib.factory_presets().unwrap();
    assert_eq!(fab.len(), 3);
    assert_eq!(
        fab.iter().map(|r| r.pp.unwrap()).collect::<Vec<_>>(),
        vec![0, 1, 2],
        "fábrica sai em ordem de número, não de inserção"
    );

    let usr = lib.user_presets().unwrap();
    assert_eq!(usr.len(), 2);
    assert!(
        usr.iter().all(|r| r.bank == "user"),
        "patch de usuário tem pp = None"
    );
    assert!(usr.iter().all(|r| r.has_payload));
}

#[test]
fn limite_corta_e_nao_quebra() {
    let lib = bib();
    let r = lib
        .search(&SearchQuery {
            limit: Some(2),
            ..SearchQuery::default()
        })
        .unwrap();
    assert_eq!(r.len(), 2);
}

#[test]
fn texto_so_espaco_nao_filtra_nada() {
    // Busca vazia na UI não deve devolver lista vazia: "campo limpo" = "todos".
    let lib = bib();
    let r = lib.search(&SearchQuery::nome("   ")).unwrap();
    assert_eq!(r.len(), 5);
}

#[test]
fn apagar_e_reler_o_mesmo_id() {
    let caminho = arquivo_temporario("busca-delete");
    let lib = Library::open(caminho.path()).unwrap();
    lib.upsert(&patch_de_usuario("u9", "Some", "{}")).unwrap();
    assert_eq!(lib.get("u9").unwrap().unwrap().name, "Some");

    assert!(lib.delete("u9").unwrap(), "primeiro delete remove");
    assert!(!lib.delete("u9").unwrap(), "segundo delete é no-op");
    assert!(lib.get("u9").unwrap().is_none());
}

#[test]
fn regrava_o_mesmo_id_substitui() {
    let lib = bib();
    let mut p = patch_de_usuario("u1", "nome antigo", "{}");
    lib.upsert(&p).unwrap();
    p.name = "nome novo".into();
    lib.upsert(&p).unwrap();
    assert_eq!(lib.count().unwrap(), 5, "não duplicou");
    assert_eq!(lib.get("u1").unwrap().unwrap().name, "nome novo");
}
