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
    Migration {
        version: 3,
        sql: r#"
            -- TONS de SnapTone/NAM (issue #25). O `.clo` importado é OPACO:
            -- a conversão do `.nam` acontece no Suite e o motor NAM mora no
            -- exe da Valeton, então aqui só há bytes e o CRC que prova que
            -- eles não mudaram no caminho.
            --
            -- O CHECK de slot é a MESMA regra de `snap_tone.rs` e a mesma
            -- constante do core (`SnapTone1..5` no firmware, §5) — o limite
            -- 5 está escrito aqui porque `MIGRATIONS` é SQL estático, e o
            -- teste `limite_do_slot_no_sql_bate_com_o_core` amarra os dois.
            -- Se divergirem, o banco aceitaria 6 e a FSM recusaria: o dono
            -- veria o erro na atribuição, tarde demais.
            CREATE TABLE snap_tone (
                id       TEXT    PRIMARY KEY,
                name     TEXT    NOT NULL,
                bytes    INTEGER NOT NULL,
                crc32    INTEGER NOT NULL,
                slot     INTEGER CHECK (slot IS NULL OR (slot BETWEEN 1 AND 5)),
                saved_at TEXT    NOT NULL
            );
            -- Um slot, um tom: o índice parcial é o que impede dois registros
            -- apontarem para o mesmo slot do device.
            CREATE UNIQUE INDEX snap_tone_slot ON snap_tone(slot) WHERE slot IS NOT NULL;
            CREATE INDEX snap_tone_name ON snap_tone(name);
            -- O modelo (~2,7 KB) mora à parte: a lista do gestor não paga
            -- 2,7 KB por linha para mostrar um nome. O CASCADE faz o blob
            -- sumir junto com o tom.
            CREATE TABLE snap_tone_model (
                id    TEXT PRIMARY KEY REFERENCES snap_tone(id) ON DELETE CASCADE,
                model BLOB NOT NULL
            );
        "#,
    },
    Migration {
        version: 4,
        sql: r#"
            -- O PREVIEW do tom: o `nam_output_wav.wav` que o Valeton Suite
            -- renderiza ao lado do `.clo` (strings em `exe_strings.txt`:
            -- "nam_output_clo.wav"). É o ÁUDIO que aquele modelo produz, e é o
            -- que o A/B do gestor toca — o app não reimplementa o motor NAM
            -- (`BLOCKERS.md`), então o audio vem do Suite.
            --
            -- Entra como ALTER e não no CREATE da v3 porque a v3 já rodou na
            -- máquina de alguém: recriar a tabela para ganhar uma coluna
            -- apaga os modelos de quem já tinha importado.
            --
            -- NULL é o caso NORMAL de um tom sem preview: o dono pode ter o
            -- `.clo` e não o WAV (o Suite so exporta o audio se pedirem), e
            -- nesse caso o botão de tocar fica desabilitado COM o motivo.
            ALTER TABLE snap_tone_model ADD COLUMN preview BLOB;
        "#,
    },
    Migration {
        version: 5,
        sql: r#"
            -- IRs do USUÁRIO (issue #24): o laboratorio. O `.ir` escolhido no
            -- disco é opaco aqui como o `.clo` é para o tom — quem conhece o
            -- formato é o aparelho (§13.7), e o banco guarda bytes + o CRC que
            -- prova que eles não mudaram no caminho.
            --
            -- O CHECK de slot é a MESMA regra de `ir.rs` e a mesma constante do
            -- core (`<ppIRInfo0..19>`, §13.12) — o limite 20 está escrito aqui
            -- porque `MIGRATIONS` é SQL estático, e o teste
            -- `limite_do_slot_do_ir_no_sql_bate_com_o_core` amarra os dois. A
            -- faixa começa em **0**: para IR o slot 0 é o primeiro slot, e um
            -- `BETWEEN 1 AND 5` copiado do SnapTone recusaria o primeiro slot
            -- do aparelho.
            CREATE TABLE ir_lib (
                id       TEXT    PRIMARY KEY,
                name     TEXT    NOT NULL,
                bytes    INTEGER NOT NULL,
                crc32    INTEGER NOT NULL,
                slot     INTEGER CHECK (slot IS NULL OR (slot BETWEEN 0 AND 19)),
                saved_at TEXT    NOT NULL
            );
            -- Um slot, um IR: sem este índice parcial dois registros apontariam
            -- para o mesmo slot do aparelho, e a tela passaria a mentir sobre
            -- o que está gravado la.
            CREATE UNIQUE INDEX ir_lib_slot ON ir_lib(slot) WHERE slot IS NOT NULL;
            CREATE INDEX ir_lib_name ON ir_lib(name);
            -- O conteúdo mora à parte: a lista do laboratorio não paga centenas
            -- de KB por linha para mostrar um nome. O CASCADE faz o blob
            -- sumir junto com o IR.
            CREATE TABLE ir_lib_blob (
                id   TEXT PRIMARY KEY REFERENCES ir_lib(id) ON DELETE CASCADE,
                blob BLOB NOT NULL
            );
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
