//! Migrações versionadas do esquema da biblioteca (issue #26).
//!
//! **O contrato.** A versão do esquema é `PRAGMA user_version` do próprio
//! arquivo. `MIGRATIONS` é a lista-ordered do que falta aplicar; o índice da
//! lista é o número da versão. Acrescentar uma migration é acrescentar uma
//! entrada ao FINAL — nunca editar uma que já saiu, porque ela já rodou na
//! máquina de alguém.
//!
//! **Por que `user_version` e não uma tabela `schema_migrations`.** O pragma já
//! existe, é gravado pelo SQLite dentro da transação e não pode divergir do
//! arquivo por engano. A tabela alternativa precisa de duas linhas de bookkeeping
//! (o INSERT e o UPDATE do número) e nada impede que fiquem inconsistentes entre
//! si depois de uma falha no meio.
//!
//! **Transação por migration.** Cada uma roda em transação própria. Uma que
//! falha deixa o banco na versão ANTERIOR, inteira — não num meio esquema que a
//! próxima execução tenta reparar adivinhando.

use rusqlite::Connection;

use crate::LibraryError;

/// Uma etapa versionada do esquema.
struct Migration {
    /// Versão que esta etapa produz (= `user_version` depois de aplicada).
    version: u32,
    /// SQL da etapa. Uma migration pode ter várias instruções.
    sql: &'static str,
}

/// O que falta aplicar, em ordem. Índice + 1 = número da versão.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: r#"
            CREATE TABLE preset (
                id           TEXT    PRIMARY KEY,
                bank         TEXT    NOT NULL CHECK (bank IN ('factory','user')),
                pp           INTEGER,
                name         TEXT    NOT NULL,
                pp_type      INTEGER NOT NULL,
                pp_type_name TEXT    NOT NULL,
                saved_at     TEXT    NOT NULL
            );
            CREATE UNIQUE INDEX preset_factory_pp ON preset(pp) WHERE bank = 'factory';
            CREATE INDEX preset_name ON preset(name);
        "#,
    },
    Migration {
        version: 2,
        sql: r#"
            -- A cadeia do patch de usuário (o snapshot que volta ao palco).
            -- Entra como migration própria e não no CREATE da v1: a coluna é
            -- BLOB de uso do M2 (#24/#25 também guardam conteúdo aqui), e
            -- Changing o tipo depois exigiria a recriação da tabela.
            ALTER TABLE preset ADD COLUMN payload BLOB;
            CREATE INDEX preset_type ON preset(pp_type);
        "#,
    },
];

/// Versão mais recente que este build conhece.
pub fn latest() -> u32 {
    MIGRATIONS.last().map_or(0, |m| m.version)
}

/// SQL de uma versão específica, ou `None` se este build não a tem.
///
/// Existe para o diagnóstico (`--dry-run` que mostra o que seria aplicado) e
/// para o teste de contrato, que precisa confirmar que as etapas são
/// CONTÍNUAS e que a versão 2 fala mesmo da tabela que a versão 1 criou.
pub fn sql_da(version: u32) -> Option<&'static str> {
    MIGRATIONS
        .iter()
        .find(|m| m.version == version)
        .map(|m| m.sql)
}

/// Versões que este build sabe aplicar, em ordem.
pub fn versoes() -> Vec<u32> {
    MIGRATIONS.iter().map(|m| m.version).collect()
}

/// Leva o banco à versão mais recente que ele conhece.
///
/// Um banco em versão MAIOR que a deste build é recusado: um binário velho não
/// sabe o que fazer com colunas que não conhece, e o comportamento "silenciosamente
/// segue em diante" é como se apaga dado de usuário.
pub fn apply(conn: &Connection) -> Result<u32, LibraryError> {
    let atual: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let suportada = latest();

    if atual > suportada {
        return Err(LibraryError::SchemaTooNew {
            encontrada: atual,
            suportada,
        });
    }

    for m in MIGRATIONS.iter().filter(|m| m.version > atual) {
        // A transação inclui o UPDATE do user_version: se a gravação do número
        // falhasse junto com o DDL, o banco não poderia ficar com esquema novo e
        // número velho — que é o estado que faz a etapa rodar duas vezes.
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(m.sql)?;
        tx.pragma_update(None, "user_version", m.version)?;
        tx.commit()?;
    }

    // Relê do arquivo: o valor devolvido é o que o PRAGMA diz agora, não o que
    // o código acha que fez.
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}
