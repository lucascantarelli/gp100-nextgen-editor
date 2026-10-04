//! Import/export da biblioteca em JSON versionado (issue #26).
//!
//! **Por que um envelope e não uma lista crua.** Um `.json` de biblioteca sem
//! identificação é indistinguível de qualquer outro JSON que o usuário tenha
//! arrastado para a janela — e o app não tem como dizer "isto aqui não é uma
//! biblioteca" antes de tentar gravar metade dos campos. O envelope carrega o
//! formato, a VERSÃO do formato e o que foi gravado, e [`ExportBundle::parse`]
//! recusa o que não for nosso ou for de uma versão que este build não lê.
//!
//! **A versão do envelope é a do ARQUIVO, não a do banco.** São coisas
//! diferentes de propósito: `schema` (a do SQLite, via `user_version`) muda
//! quando uma migration muda; `version` (a do envelope) muda quando o QUE
//! exportamos muda. Um import antigo ainda tem de ser legível depois que o banco
//! migrou três vezes.
//!
//! **DuasPoliticas, porque "importar" é ambíguo.** `Insert` não pisa em nada
//! que já existe (importar duas vezes a mesma biblioteca é no-op, não
//! duplicata). `Replace` é a restauração de um backup, e aí pisa. A escolha é
//! do chamador e o relatório diz exatamente o que aconteceu em cada linha — um
//! import que engole conflito em silêncio é o jeito mais rápido de perder um
//! patch.

use serde::{Deserialize, Serialize};

use crate::{Library, LibraryError, Preset};

/// Identificador do formato — se mudar, é outro tipo de arquivo.
pub const FORMATO: &str = "gp100.library";

/// Versão deste envelope. Incrementar quando a forma do JSON mudar.
pub const VERSAO_ENVELOPE: u32 = 1;

/// O que um `export` produz.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBundle {
    /// Sempre [`FORMATO`].
    pub format: String,
    /// Versão do envelope (não a do banco).
    pub version: u32,
    /// ISO-8601 do instante do export.
    pub exported_at: String,
    /// Versão do SQLite embutido que gravou (bug reproduzível).
    pub sqlite: String,
    /// `user_version` do banco no momento do export.
    pub schema: u32,
    /// Os registros.
    pub presets: Vec<Preset>,
}

/// Só o cabeçalho do envelope — o que IDENTIFICA o arquivo.
///
/// Existe por uma razão que o teste mostrou: desserializar o bundle inteiro primeiro
/// dava `missing field exportedAt` para QUALQUER JSON que o usuário arrastasse
/// para a janela. A mensagem estava errada ("faltou um campo") quando o
/// arquivo é outro ("isto não é uma biblioteca"). Parsear o cabeçalho primeiro
/// dá a recusa certa, e só depois se olha o corpo.
#[derive(Debug, Deserialize)]
struct Cabecalho {
    format: String,
    version: u32,
}

impl ExportBundle {
    /// Lê e valida um envelope. Rejeita formato desconhecido e versão futura.
    pub fn parse(json: &str) -> Result<Self, LibraryError> {
        // O corpo vem primeiro: JSON que nem é objeto não tem cabeçalho.
        let valor: serde_json::Value = serde_json::from_str(json)?;
        let cab: Cabecalho = serde_json::from_value(valor.clone()).map_err(|_| {
            LibraryError::Envelope("sem cabecalho de biblioteca (format/version)".into())
        })?;

        if cab.format != FORMATO {
            return Err(LibraryError::Envelope(format!(
                "formato '{}' (esperado '{FORMATO}')",
                cab.format
            )));
        }
        if cab.version > VERSAO_ENVELOPE {
            return Err(LibraryError::Envelope(format!(
                "versao {} do envelope (este build le ate {VERSAO_ENVELOPE})",
                cab.version
            )));
        }
        serde_json::from_value(valor).map_err(LibraryError::from)
    }
}

/// O que fazer com um id que já existe no banco.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportMode {
    /// Ignora o que já existe (importar a mesma biblioteca duas vezes não
    /// duplica nada).
    Insert,
    /// Sobrescreve (restauração de backup).
    Replace,
}

/// Contabilidade do import — o que entrou, o que ficou de fora e por quê.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    /// Registros gravados.
    pub inserted: usize,
    /// Registros que já existiam e foram sobrescritos (`Replace`).
    pub replaced: usize,
    /// Registros ignorados por já existirem (`Insert`).
    pub skipped: usize,
}

impl Library {
    /// Serializa a biblioteca inteira num envelope versionado.
    pub fn export_json(&self, agora: &str) -> Result<String, LibraryError> {
        let presets = self
            .search(&crate::SearchQuery::default())?
            .into_iter()
            .filter_map(|row| self.get(&row.id).ok().flatten())
            .collect();
        let bundle = ExportBundle {
            format: FORMATO.to_string(),
            version: VERSAO_ENVELOPE,
            exported_at: agora.to_string(),
            sqlite: crate::Library::sqlite_version().to_string(),
            schema: self.schema_version()?,
            presets,
        };
        Ok(serde_json::to_string_pretty(&bundle)?)
    }

    /// Importa um envelope.
    ///
    /// Um registro que viola o schema (banco desconhecido, pp de fábrica
    /// repetido) derruba o import INTEIRO, e não é pulado em silêncio: metade
    /// de uma biblioteca importada é pior do que nenhuma, porque o usuário não
    /// sabe qual metade.
    pub fn import_json(
        &mut self,
        json: &str,
        mode: ImportMode,
    ) -> Result<ImportReport, LibraryError> {
        let bundle = ExportBundle::parse(json)?;
        let mut report = ImportReport::default();

        let tx = self.conn_mut().unchecked_transaction()?;
        for p in &bundle.presets {
            let existe: i64 =
                tx.query_row("SELECT COUNT(*) FROM preset WHERE id = ?1", [&p.id], |r| {
                    r.get(0)
                })?;
            if existe > 0 && mode == ImportMode::Insert {
                report.skipped += 1;
                continue;
            }
            tx.execute(
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
            if existe > 0 {
                report.replaced += 1;
            } else {
                report.inserted += 1;
            }
        }
        tx.commit()?;
        Ok(report)
    }
}
