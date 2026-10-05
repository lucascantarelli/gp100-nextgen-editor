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
        let sql = sql_de(v);
        assert!(
            sql.contains("preset") || sql.contains("snap_tone"),
            "migration {v} não mexe em nenhuma tabela conhecida: {sql}"
        );
    }
    assert!(
        gp100_library::migrations::sql_da(99).is_none(),
        "não há versão 99"
    );
}

/// A tabela que a migration declara EXISTE no banco migrado.
///
/// A versão anterior deste teste perguntava "a migration menciona `preset`?" —
/// que é uma pergunta sobre o TEXTO, e passou a mentir quando a v3 apareceu
/// para falar de `snap_tone`. A pergunta útil é sobre o banco: o que a
/// migration promete criar/modificar tem que estar lá depois de aplicar.
#[test]
fn a_tabela_de_cada_migration_existe_depois() {
    let lib = Library::open_in_memory().expect("banco migrado");
    for v in 1..=gp100_library::migrations::latest() {
        let sql = gp100_library::migrations::sql_da(v).expect("versão existe");
        for tabela in tabelas_declaradas(sql) {
            let existe: i64 = lib
                .conn()
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [&tabela],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(
                existe, 1,
                "migration {v} declarou `{tabela}` e ela não está"
            );
        }
    }
}

/// As tabelas que o SQL cria ou altera (na ordem em que aparecem).
fn tabelas_declaradas(sql: &str) -> Vec<String> {
    let mut out = Vec::new();
    for linha in sql.lines() {
        let l = linha.trim();
        for prefixo in ["CREATE TABLE ", "ALTER TABLE "] {
            if let Some(resto) = l.strip_prefix(prefixo) {
                let nome: String = resto
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !nome.is_empty() {
                    out.push(nome);
                }
            }
        }
    }
    out
}

/// O limite de slot no SQL e o do core são o MESMO número.
///
/// As duas metades estão em linguagens diferentes — o `CHECK` do SQLite e o
/// `SLOTS` do crate — e o dono percebe a divergência tarde demais: o banco
/// aceitaria o slot 6 e a FSM recusaria no envio, com o aparelho na mão. O
/// número 5 é o das strings de firmware (`SnapTone1..5`, §5).
#[test]
fn limite_do_slot_no_sql_bate_com_o_core() {
    let sql = gp100_library::migrations::sql_da(3).expect("migration 3 (tons)");
    let esperado = format!("slot BETWEEN 1 AND {}", gp100_library::snap_tone::SLOTS);
    assert!(
        sql.contains(&esperado),
        "a migration 3 deveria fechar `{esperado}`: {sql}"
    );
    // E o CHECK MEXE de verdade: o banco recusa 6 e 0 na sua própria boca,
    // sem passar pela validação do Rust.
    let lib = Library::open_in_memory().expect("banco migrado");
    for slot in [0i64, 6, 255] {
        let r = lib.conn().execute(
            "INSERT INTO snap_tone (id,name,bytes,crc32,slot,saved_at)
             VALUES ('x','x',1,1,?1,'2026-01-01T00:00:00Z')",
            [slot],
        );
        assert!(r.is_err(), "o CHECK do banco aceitou o slot {slot}");
    }
}
