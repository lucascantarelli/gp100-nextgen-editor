//! Busca da biblioteca: nome, nº, estilo/tipo e banco (issue #26).
//!
//! **"estilo" e "tipo" são o mesmo eixo.** No `.prst` (§13.9) o preset tem
//! `ppID` (número) e `ppType` (Rock, Pop, …) — a taxonomia do aparelho tem uma
//! dimensão só. A issue escreve "nome/nº/estilo/tipo" com quatro palavras; o
//! filtro `pp_type` cobre as duas últimas e o tipo de retorno expõe o número e
//! o rótulo, então a UI mostra os dois sem precisar de dois campos.
//!
//! **Escapamento do texto.** `LIKE` trata `%` e `_` como curinga. Uma caixa de
//! busca com `_` no texto (que aparece em nome de preset e em caminho de
//! arquivo) casaria com qualquer caractere e a lista viraria lixo. O input do
//! usuário é dado, não sintaxe: [`escape_like`] neutraliza os dois e o `ESCAPE`
//! diz ao SQLite que aquilo é literal.

use serde::{Deserialize, Serialize};

use crate::{Bank, LibraryError};

/// Uma linha da biblioteca, como a UI consome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetRow {
    /// Identificador estável (`f25`, `u7`).
    pub id: String,
    /// Banco de origem.
    pub bank: String,
    /// Nº do preset de fábrica (`None` no patch de usuário).
    pub pp: Option<u16>,
    /// Nome.
    pub name: String,
    /// Estilo/tipo numérico.
    pub pp_type: u16,
    /// Rótulo do estilo/tipo.
    pub pp_type_name: String,
    /// ISO-8601 da gravação.
    pub saved_at: String,
    /// A cadeia está guardada? (a UI usa para habilitar "abrir").
    pub has_payload: bool,
}

/// Filtro da busca. `None` em um campo = não filtra por ele.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchQuery {
    /// Substring do nome. case-insensitive para ASCII.
    pub text: Option<String>,
    /// Nº exato do preset de fábrica.
    pub pp: Option<u16>,
    /// Estilo/tipo exato (`4` = Rock, `6` = Pop).
    pub pp_type: Option<u16>,
    /// Restringe a um banco.
    pub bank: Option<Bank>,
    /// Teto de linhas. `None` = sem limite.
    pub limit: Option<usize>,
}

impl SearchQuery {
    /// Busca só por nome (o que a caixa de texto da biblioteca monta).
    pub fn nome(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Self::default()
        }
    }

    /// Teto padrão quando a chamada não diz: a biblioteca de fábrica tem 99
    /// linhas e a UI rola; um teto baixo esconderia preset que existe.
    pub const LIMITE_PADRAO: usize = 500;
}

/// Neutraliza os curingas do `LIKE`. O `\` entra PRIMEIRO para que escapar o
/// próprio escape funcione.
fn escape_like(bruto: &str) -> String {
    let mut out = String::with_capacity(bruto.len());
    for ch in bruto.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Colunas da listagem, na ordem do DTO. Uma constante para o SELECT e para o
/// `row` — dois lugares para lembrar de manter em ordem é como um campo
/// trocasse de nome no caminho.
const COLUNAS: &str = "id, bank, pp, name, pp_type, pp_type_name, saved_at, payload IS NOT NULL";

impl crate::Library {
    /// Executa a busca, na ordem de banco e número (a ordem da biblioteca real:
    /// P1, P2, … — o dono procura por "o preset 42", não por ordem de inserção).
    pub fn search(&self, q: &SearchQuery) -> Result<Vec<PresetRow>, LibraryError> {
        let mut sql = format!("SELECT {COLUNAS} FROM preset WHERE 1=1");
        let mut filtros: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(bank) = q.bank {
            sql.push_str(" AND bank = ?");
            filtros.push(Box::new(bank.as_str()));
        }
        if let Some(pp) = q.pp {
            sql.push_str(" AND pp = ?");
            filtros.push(Box::new(pp));
        }
        if let Some(pp_type) = q.pp_type {
            sql.push_str(" AND pp_type = ?");
            filtros.push(Box::new(pp_type));
        }
        if let Some(texto) = q.text.as_deref().filter(|t| !t.trim().is_empty()) {
            // `LIKE` do SQLite é case-insensitive só para ASCII (mesmo `lower`).
            // Acento NÃO é dobrado — e isso está documentado de propósito: sem
            // coluna normalizada, "acucar" achando "açúcar" seria uma promessa
            // que o SQLite não cumpre. Fold de acento aqui seria um
            // `unorm` em Rust para cada linha, mudando o plano de consulta.
            sql.push_str(" AND name LIKE ? ESCAPE '\\'");
            filtros.push(Box::new(format!("%{}%", escape_like(texto.trim()))));
        }

        sql.push_str(" ORDER BY bank DESC, pp IS NULL, pp ASC, name ASC");
        if let Some(limite) = q.limit {
            // O limite vai pela MESMA lista de parâmetros: interpolar inteiro
            // seria SQL injection por tipo. (Isto aqui é `usize`, mas o
            // hábito de interpolar é o que mata quando o tipo muda.)
            sql.push_str(" LIMIT ?");
            filtros.push(Box::new(limite as i64));
        }

        let refs: Vec<&dyn rusqlite::ToSql> = filtros.iter().map(|b| b.as_ref()).collect();
        let mut stmt = self.conn().prepare(&sql)?;
        let linhas = stmt.query_map(refs.as_slice(), |r| {
            Ok(PresetRow {
                id: r.get(0)?,
                bank: r.get(1)?,
                pp: r.get(2)?,
                name: r.get(3)?,
                pp_type: r.get(4)?,
                pp_type_name: r.get(5)?,
                saved_at: r.get(6)?,
                has_payload: r.get(7)?,
            })
        })?;

        let mut saida = Vec::new();
        for l in linhas {
            saida.push(l?);
        }
        Ok(saida)
    }

    /// Todos os presets de fábrica, na ordem do número. É o seed de fábrica e o
    /// que a biblioteca mostra antes de existir busca.
    pub fn factory_presets(&self) -> Result<Vec<PresetRow>, LibraryError> {
        self.search(&SearchQuery {
            bank: Some(Bank::Factory),
            ..SearchQuery::default()
        })
    }

    /// Registros guardados, ordenados do mais novo para o mais antigo.
    pub fn user_presets(&self) -> Result<Vec<PresetRow>, LibraryError> {
        let mut linhas = self.search(&SearchQuery {
            bank: Some(Bank::User),
            ..SearchQuery::default()
        })?;
        linhas.sort_by(|a, b| b.saved_at.cmp(&a.saved_at).then_with(|| a.id.cmp(&b.id)));
        Ok(linhas)
    }
}
