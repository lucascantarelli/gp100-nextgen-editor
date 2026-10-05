//! gp100-library — a biblioteca persistente do editor (issue #26).
//!
//! **O que este crate é.** O armazenamento de conteúdo do app: os 99 presets
//! de fábrica e os patches de usuário, em SQLite, com esquema versionado,
//! busca e import/export. É o substrate que o laboratório de IRs (#24) e o
//! gestor SnapTone/NAM (#25) vão reusar.
//!
//! **O que este crate NÃO é.** Ele não sabe que existe uma janela. Não depende
//! de Tauri, nem de React, nem do transporte do device — é `rusqlite` +
//! `serde` e nada mais. Quem fala com a UI é o shell (`gp100-ui`), por commands
//! `library_*` finos. A direção é sempre `ui → shell → library` (ADR-9).
//!
//! **Versão do esquema.** Vive em `PRAGMA user_version`, dentro do próprio
//! arquivo — nunca numa constante do código. Um banco aberto num `user_version`
//! que este build não conhece é ERRO, não migrations a esboçar: um esquema do
//! futuro lido por um binário velho é a forma de corromper dado silenciosamente.

use std::path::Path;

use serde::{Deserialize, Serialize};

pub mod migrations;
pub mod search;
pub mod seed;
pub mod snap_tone;
pub mod transfer;

pub use search::{PresetRow, SearchQuery};
pub use snap_tone::{Tone, ToneBoard, ToneRow, SLOTS};
pub use transfer::{ExportBundle, ImportMode, ImportReport, VERSAO_ENVELOPE};

/// Erro do armazenamento. Mensagem para o usuário final é responsabilidade de
/// quem chama (o shell/front); aqui o texto técnico basta e o `kind` deixa a
/// decisão explícita.
#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    /// Falha do SQLite (I/O, disco cheio, SQL inválido, banco corrompido).
    #[error("falha do sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// JSON malformado no import/export.
    #[error("json invalido: {0}")]
    Json(#[from] serde_json::Error),

    /// I/O do arquivo do banco (criar diretório, abrir arquivo).
    #[error("io do banco: {0}")]
    Io(#[from] std::io::Error),

    /// `.prst` inválido no seed de fábrica.
    #[error("prst invalido: {0}")]
    Prst(String),

    /// O arquivo foi gravado por uma versão mais nova deste app.
    #[error("banco em versao {encontrada}, este build conhece ate {suportada}")]
    SchemaTooNew {
        /// `PRAGMA user_version` encontrado no arquivo.
        encontrada: u32,
        /// Última versão que o `migrations::MIGRATIONS` deste build define.
        suportada: u32,
    },

    /// O envelope de export não é nosso, ou a versão dele não é legível.
    #[error("envelope de export invalido: {0}")]
    Envelope(String),

    /// Slot de SnapTone fora de 1..=5 (`SnapTone1..5`, §5). Tipado porque a
    /// UI escreve o número que veio de um clique — e um `0` ou um `6` ali é
    /// bug de interface, não arquivo ruim: as duas coisas precisam de
    /// mensagens diferentes.
    #[error("slot de SnapTone invalido: {0} (esperado 1..={1})")]
    SlotInvalido(u32, u8),

    /// O slot já é de outro tom. O erro carrega o dono do slot porque é o
    /// que a UI precisa mostrar: "o slot 3 e do 'Marshall 4x12'".
    #[error("o slot {slot} ja e do tom '{dono}'")]
    SlotOcupado {
        /// Slot disputado (1..=5).
        slot: u8,
        /// Nome do tom que já ocupa o slot.
        dono: String,
    },

    /// Conteúdo de tom recusado (nome vazio, modelo de 0 bytes, id
    /// inexistente): é o que o app importou ou pediu, não o banco.
    #[error("tom invalido: {0}")]
    Tone(String),
}

/// Banco da biblioteca. Dono único da conexão.
#[derive(Debug)]
pub struct Library {
    conn: rusqlite::Connection,
}

/// Registro de biblioteca, independente do banco de origem.
///
/// **camelCase no fio, como todo DTO deste projeto** (`DeviceInfo`,
/// `IrSlotDto`, `PresetRow`): o TS não deve precisar de mapping entre
/// `pp_type` e `ppType`. Um DTO em snake_case aqui passaria o typecheck do
/// crate e quebraria em silêncio no `invoke` — foi o que o teste de contrato
/// pegou.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    /// Identificador estável: `f25` (fábrica, pp 25) ou `u<n>` (usuário).
    pub id: String,
    /// De onde veio.
    pub bank: Bank,
    /// Número do preset de fábrica (0..=98); `None` para patch de usuário.
    pub pp: Option<u16>,
    /// Nome visível.
    pub name: String,
    /// Estilo/tipo numérico do `.prst` (4 = Rock, 6 = Pop…).
    pub pp_type: u16,
    /// Rótulo do estilo/tipo ("Rock", "Pop").
    pub pp_type_name: String,
    /// ISO-8601 do instante em que o registro foi gravado.
    pub saved_at: String,
    /// Cadeia serializada (JSON do snapshot) — presente em patch de usuário.
    pub payload: Option<String>,
}

/// Banco de origem do registro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Bank {
    /// Os 99 presets de fábrica do `all.prst`.
    Factory,
    /// Patch salvo pelo dono.
    User,
}

impl Bank {
    /// Texto do banco como vai para o SQL e para o JSON.
    pub fn as_str(self) -> &'static str {
        match self {
            Bank::Factory => "factory",
            Bank::User => "user",
        }
    }
}

impl Library {
    /// Abre (ou cria) o banco no caminho dado e leva o esquema à versão atual.
    ///
    /// O pai é criado se faltar: o caminho padrão do app fica dentro do
    /// diretório de dados da plataforma, que nem sempre existe num primeiro
    /// boot.
    pub fn open(path: &Path) -> Result<Self, LibraryError> {
        if let Some(pai) = path.parent() {
            if !pai.as_os_str().is_empty() {
                std::fs::create_dir_all(pai)?;
            }
        }
        let conn = rusqlite::Connection::open(path)?;
        Self::from_conn(conn)
    }

    /// Banco efêmero em memória — o alvo dos testes e do `--dry-run`.
    pub fn open_in_memory() -> Result<Self, LibraryError> {
        Self::from_conn(rusqlite::Connection::open_in_memory()?)
    }

    fn from_conn(conn: rusqlite::Connection) -> Result<Self, LibraryError> {
        // WAL + foreign_keys: o primeiro evita `database is locked` quando a UI
        // lê enquanto o seed escreve; o segundo é o default do SQLite desligado
        // e ligado por default em todo outro banco do mundo — divergir do
        // padrão é o tipo de coisa que morre em produção.
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let lib = Self { conn };
        lib.migrate()?;
        Ok(lib)
    }

    /// Versão do esquema gravada no arquivo (0 = banco virgem).
    pub fn schema_version(&self) -> Result<u32, LibraryError> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    /// Aplica as migrations pendentes e grava o `user_version` resultante.
    ///
    /// Cada migration é uma transação própria: uma que falha deixa o banco na
    /// versão anterior, não num meio estado que a próxima execução repara.
    pub fn migrate(&self) -> Result<u32, LibraryError> {
        migrations::apply(&self.conn)
    }

    /// Versão do SQLite embutido — entra no export para um bug de biblioteca
    /// ser reproduzível. `bundled` (ADR-9) garante que seja a mesma em toda
    /// máquina.
    pub fn sqlite_version() -> &'static str {
        rusqlite::version()
    }

    /// Acesso cru — só para o seed e para testes. Quem chama é responsável por
    /// não fazer SQL fora das migrações.
    pub fn conn(&self) -> &rusqlite::Connection {
        &self.conn
    }

    /// Acesso cru mutável. O `rusqlite::Connection` é `&self` em tudo (o banco
    /// tem sincronização interna), então a distinção aqui é de API, não de
    /// concorrência: quem escreve em transação precisa da conexão viva.
    pub fn conn_mut(&mut self) -> &mut rusqlite::Connection {
        &mut self.conn
    }

    /// Grava (ou regrava) um registro. O `id` é a chave: dois presets de
    /// fábrica nunca colidem porque o id carrega o pp, e o id de usuário é
    /// gerado no momento do save.
    pub fn upsert(&self, p: &Preset) -> Result<(), LibraryError> {
        self.conn.execute(
            "INSERT INTO preset (id, bank, pp, name, pp_type, pp_type_name, saved_at, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                bank = excluded.bank,
                pp = excluded.pp,
                name = excluded.name,
                pp_type = excluded.pp_type,
                pp_type_name = excluded.pp_type_name,
                saved_at = excluded.saved_at,
                payload = excluded.payload",
            rusqlite::params![
                p.id,
                p.bank.as_str(),
                p.pp,
                p.name,
                p.pp_type,
                p.pp_type_name,
                p.saved_at,
                p.payload,
            ],
        )?;
        Ok(())
    }

    /// Apaga pelo id. `false` = não existia (apagar duas vezes é o mesmo que
    /// apagar uma — a UI não precisa distinguir).
    pub fn delete(&self, id: &str) -> Result<bool, LibraryError> {
        let n = self
            .conn
            .execute("DELETE FROM preset WHERE id = ?1", [id])?;
        Ok(n > 0)
    }

    /// Lê um registro pelo id.
    pub fn get(&self, id: &str) -> Result<Option<Preset>, LibraryError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, bank, pp, name, pp_type, pp_type_name, saved_at, payload
             FROM preset WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        let bank: String = row.get(1)?;
        Ok(Some(Preset {
            id: row.get(0)?,
            bank: match bank.as_str() {
                "factory" => Bank::Factory,
                _ => Bank::User,
            },
            pp: row.get(2)?,
            name: row.get(3)?,
            pp_type: row.get(4)?,
            pp_type_name: row.get(5)?,
            saved_at: row.get(6)?,
            payload: row.get(7)?,
        }))
    }

    /// Nº de registros, para o relatório de import e os testes.
    pub fn count(&self) -> Result<i64, LibraryError> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM preset", [], |r| r.get(0))?)
    }
}
