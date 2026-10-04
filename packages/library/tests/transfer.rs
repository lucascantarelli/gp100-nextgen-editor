//! Contrato de import/export (issue #26).

// `serde_json` é dev-dependency: o teste o usa direto para fixar o formato do
// fio (camelCase) sem passar pelo banco.
use gp100_library::{Bank, ExportBundle, ImportMode, Library, SearchQuery, VERSAO_ENVELOPE};

mod comum;
use comum::{patch_de_usuario, preset_de_fabrica};

fn bib() -> Library {
    let lib = Library::open_in_memory().expect("abre");
    lib.upsert(&preset_de_fabrica(0, "It's GP100", 4, "Rock"))
        .unwrap();
    lib.upsert(&preset_de_fabrica(1, "Blink OD", 6, "Pop"))
        .unwrap();
    lib.upsert(&patch_de_usuario("u1", "MEU LEAD", r#"{"slots":[1,2]}"#))
        .unwrap();
    lib
}

#[test]
fn export_carrega_o_envelope_com_a_versao_e_o_schema() {
    let lib = bib();
    let json = lib.export_json("2026-03-03T12:00:00Z").unwrap();
    let b = ExportBundle::parse(&json).expect("envelope valido");
    assert_eq!(b.format, gp100_library::transfer::FORMATO);
    assert_eq!(b.version, VERSAO_ENVELOPE);
    assert_eq!(b.exported_at, "2026-03-03T12:00:00Z");
    assert_eq!(b.schema, lib.schema_version().unwrap());
    assert!(!b.sqlite.is_empty(), "a versão do SQLite viaja junto");
    assert_eq!(b.presets.len(), 3);
}

#[test]
fn export_import_em_banco_vazio_devolve_a_mesma_biblioteca() {
    let origem = bib();
    let json = origem.export_json("2026-03-03T12:00:00Z").unwrap();

    let mut destino = Library::open_in_memory().unwrap();
    let rel = destino.import_json(&json, ImportMode::Insert).unwrap();
    assert_eq!(rel.inserted, 3);
    assert_eq!(rel.skipped, 0);
    assert_eq!(rel.replaced, 0);

    // O que importa: os DADOS, não a ordem dos ids.
    let antes = origem.search(&SearchQuery::default()).unwrap();
    let depois = destino.search(&SearchQuery::default()).unwrap();
    assert_eq!(antes, depois);
}

#[test]
fn payload_do_patch_sobrevive_ao_ida_e_volta() {
    let origem = bib();
    let json = origem.export_json("2026-03-03T12:00:00Z").unwrap();
    let mut destino = Library::open_in_memory().unwrap();
    destino.import_json(&json, ImportMode::Insert).unwrap();

    let p = destino.get("u1").unwrap().expect("patch existe");
    assert_eq!(p.payload.as_deref(), Some(r#"{"slots":[1,2]}"#));
    assert_eq!(p.bank, Bank::User);
    assert_eq!(p.pp, None);
}

#[test]
fn insert_nao_pisa_em_id_que_ja_existe() {
    let mut destino = Library::open_in_memory().unwrap();
    destino
        .upsert(&patch_de_usuario("u1", "MEU LEAD", "{}"))
        .unwrap();

    let json = bib().export_json("2026-03-03T12:00:00Z").unwrap();
    let rel = destino.import_json(&json, ImportMode::Insert).unwrap();
    assert_eq!(rel.skipped, 1, "o u1 já existia");
    assert_eq!(rel.inserted, 2);
    assert_eq!(destino.count().unwrap(), 3, "importar 2 vezes não duplica");

    // Reimportar o MESMO arquivo é no-op total.
    let rel2 = destino.import_json(&json, ImportMode::Insert).unwrap();
    assert_eq!(rel2.inserted, 0);
    assert_eq!(rel2.skipped, 3);
    assert_eq!(destino.count().unwrap(), 3);
}

#[test]
fn replace_sobrescreve_o_que_existe() {
    let mut destino = Library::open_in_memory().unwrap();
    destino
        .upsert(&patch_de_usuario("u1", "NOME ANTIGO", "{}"))
        .unwrap();

    let json = bib().export_json("2026-03-03T12:00:00Z").unwrap();
    let rel = destino.import_json(&json, ImportMode::Replace).unwrap();
    assert_eq!(rel.replaced, 1);
    assert_eq!(rel.inserted, 2);
    assert_eq!(destino.get("u1").unwrap().unwrap().name, "MEU LEAD");
}

#[test]
fn o_registro_viaja_em_camelcase_como_todo_o_resto_do_projeto() {
    // Este teste nasceu de um bug real: `Preset` estava em snake_case, o
    // typecheck do crate passava, e o `library_save` do shell ia RECEBER um
    // objeto que o front nem consegue produzir. O front fala camelCase
    // (`ppType`, como `DeviceInfo` e `PresetRow`); um DTO em snake_case só
    // quebra em silencio no invoke.
    let json = r#"{
        "id":"u1","bank":"user","pp":null,"name":"MEU LEAD",
        "ppType":4,"ppTypeName":"Rock","savedAt":"2026-02-02T00:00:00Z",
        "payload":"{\"slots\":[]}"
    }"#;
    let p: gp100_library::Preset = serde_json::from_str(json).expect("o front produz este JSON");
    assert_eq!(p.pp_type, 4);
    assert_eq!(p.pp_type_name, "Rock");
    assert_eq!(p.saved_at, "2026-02-02T00:00:00Z");
    assert_eq!(p.bank, Bank::User);

    // E o caminho de volta: o que o crate grava é lido de volta por ele.
    let volta = serde_json::to_string(&p).unwrap();
    assert!(volta.contains("\"ppType\":4"), "{volta}");
    assert!(volta.contains("\"ppTypeName\":\"Rock\""), "{volta}");
    assert!(
        !volta.contains("pp_type"),
        "snake_case vazou para o fio: {volta}"
    );
}

#[test]
fn arquivo_que_nao_e_biblioteca_e_recusado() {
    let mut destino = Library::open_in_memory().unwrap();
    let err = destino
        .import_json(
            r#"{"format":"outro.coisa","version":1}"#,
            ImportMode::Insert,
        )
        .expect_err("formato estranho tem de recusar");
    assert!(err.to_string().contains("outro.coisa"), "{err}");
    assert_eq!(destino.count().unwrap(), 0, "recusa não grava nada");
}

#[test]
fn envelope_de_versao_futura_e_recusado() {
    let mut destino = Library::open_in_memory().unwrap();
    let json = format!(
        r#"{{"format":"gp100.library","version":{},"exportedAt":"x","sqlite":"y","schema":1,"presets":[]}}"#,
        VERSAO_ENVELOPE + 1
    );
    let err = destino
        .import_json(&json, ImportMode::Replace)
        .expect_err("versão futura tem de recusar");
    assert!(err.to_string().contains("versao"), "{err}");
}

#[test]
fn json_truncado_nao_apaga_a_biblioteca_que_ja_existia() {
    let mut destino = Library::open_in_memory().unwrap();
    destino
        .upsert(&patch_de_usuario("u1", "MEU LEAD", "{}"))
        .unwrap();

    let bom = bib().export_json("2026-03-03T12:00:00Z").unwrap();
    let truncado = &bom[..bom.len() / 2];
    let err = destino.import_json(truncado, ImportMode::Replace);
    assert!(err.is_err(), "JSON pela metade não pode importar");
    assert_eq!(destino.count().unwrap(), 1, "o banco original está intacto");
}
