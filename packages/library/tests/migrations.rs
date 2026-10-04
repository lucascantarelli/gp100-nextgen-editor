//! Contrato das migrações versionadas (issue #26).
//!
//! Estes testes não verificam "o banco tem a tabela X". Verificam o que
//! acontece com um arquivo que JÁ EXISTE — que é onde migração quebra de
//! verdade: no banco do usuário, no segundo boot, na máquina que atualizou de
//! uma versão antiga.

use gp100_library::Library;

mod comum;
use comum::arquivo_temporario;

#[test]
fn banco_novo_vai_para_a_versao_mais_recente() {
    let lib = Library::open_in_memory().expect("abre");
    assert_eq!(
        lib.schema_version().unwrap(),
        gp100_library::migrations::latest()
    );
}

#[test]
fn abrir_e_reabrir_nao_reaplica_nada() {
    let caminho = arquivo_temporario("migra-reabre");
    {
        let lib = Library::open(caminho.path()).expect("primeira abertura");
        let v1 = lib.schema_version().unwrap();
        assert_eq!(v1, gp100_library::migrations::latest());
    }
    // A segunda abertura é o segundo boot do usuário. Se `apply` reexecutasse a
    // migration 1, o `CREATE TABLE` estouraria "table already exists".
    let lib = Library::open(caminho.path()).expect("segunda abertura");
    assert_eq!(
        lib.schema_version().unwrap(),
        gp100_library::migrations::latest()
    );
}

#[test]
fn um_banco_numa_versao_intermedia_sobe_para_a_mais_recente() {
    let caminho = arquivo_temporario("migra-intermedia");

    // Reproduz o estado de um usuário que atualizou de uma versão antiga:
    // cria o arquivo SÓ com a migration 1, manualmente.
    {
        let conn = rusqlite::Connection::open(caminho.path()).unwrap();
        conn.execute_batch(
            "CREATE TABLE preset (
                 id TEXT PRIMARY KEY, bank TEXT NOT NULL, pp INTEGER, name TEXT NOT NULL,
                 pp_type INTEGER NOT NULL, pp_type_name TEXT NOT NULL, saved_at TEXT NOT NULL);
             CREATE UNIQUE INDEX preset_factory_pp ON preset(pp) WHERE bank = 'factory';
             CREATE INDEX preset_name ON preset(name);
             PRAGMA user_version = 1;",
        )
        .unwrap();
        // Um registro do "usuário antigo" — a migration 2 NÃO pode perdê-lo.
        conn.execute(
            "INSERT INTO preset (id,bank,pp,name,pp_type,pp_type_name,saved_at)
             VALUES ('f0','factory',0,'Its GP100',4,'Rock','2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    }

    let lib = Library::open(caminho.path()).expect("abre banco antigo");
    assert_eq!(
        lib.schema_version().unwrap(),
        gp100_library::migrations::latest()
    );
    // A coluna nova veio…
    let tem_payload: i64 = lib
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('preset') WHERE name='payload'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(tem_payload, 1, "migration 2 adicionou a coluna payload");
    // …e o dado do usuário antigo continua lá.
    assert_eq!(lib.count().unwrap(), 1);
    assert_eq!(lib.get("f0").unwrap().unwrap().name, "Its GP100");
}

#[test]
fn banco_de_versao_futura_e_recusado_em_vez_de_adivinhado() {
    let caminho = arquivo_temporario("migra-futura");
    {
        let conn = rusqlite::Connection::open(caminho.path()).unwrap();
        // Simula o app de amanhã: gravou 999, este build conhece N.
        conn.pragma_update(None, "user_version", 999u32).unwrap();
    }
    let err = Library::open(caminho.path()).expect_err("deve recusar");
    let msg = err.to_string();
    assert!(
        msg.contains("999"),
        "a mensagem diz qual versão encontrou: {msg}"
    );
    assert!(
        msg.contains(&gp100_library::migrations::latest().to_string()),
        "a mensagem diz até onde este build vai: {msg}"
    );
}

#[test]
fn cada_migration_e_uma_etapa_e_uma_transacao() {
    // A lista é ordenada e sem buracos: índice+1 = versão. Um buraco aqui
    // significaria um arquivo que salta de 1 para 3 sem 2 existir.
    let sql_de = |v: u32| gp100_library::migrations::sql_da(v).unwrap();
    for v in 1..=gp100_library::migrations::latest() {
        assert!(sql_de(v).contains("preset"), "migration {v} mexe em preset");
    }
    assert!(
        gp100_library::migrations::sql_da(99).is_none(),
        "não há versão 99"
    );
}
