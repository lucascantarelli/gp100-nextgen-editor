//! golden — consumidor de `docs/referencias/protocol_golden.json`.
//!
//! **Insumo:** o golden é a especificação executável do protocolo (Baseline
//! v1.0, hash no §13 do PROTOCOL.md, regime de bytes `-text`). Este módulo
//! materializa o JSON em tipos — [`GoldenFile`] → [`Template`] → [`Pattern`] —
//! com as duas operações fundamentais:
//!
//! - [`Template::build_request`] — monta o SysEx COMPLETO
//!   (`F0 21 25 7F 47 50 2D 64 | FUNC | ADDR(4B BE) | DATA | F7`) substituindo
//!   os segmentos `var` (posicionais, na ordem) pelos bytes fornecidos;
//! - [`Template::matches_response`] / [`GoldenFile::match_response`] — casa um
//!   payload recebido contra o padrão (len + segmentos `const`) e EXTRAÍ os
//!   trechos variáveis ("Vars" do golden).
//!
//! **Propriedade fundamental** (provada em `tests/golden_consumer.rs` para os
//! 40 templates): extrair as vars do exemplo e reconstruir reproduz o exemplo
//! byte a byte — gerador e matcher são inversos exatos.
//!
//! **Padrões de payload** (gerados por `build_golden.py`):
//! - `const`/`var`/`mixed`: lista de segmentos `const` (hex fixo) e `var`
//!   (N bytes que variam);
//! - `empty`: payload de 0 bytes (requests de leitura);
//! - `by-len`: o MESMO endereço responde com payloads de tamanhos diferentes
//!   (ex.: `12|12001002` = ACK de 4B do upload de IR **e** tabela de 75B —
//!   §13.7/§13.12): o sub-padrão é escolhido pelo comprimento;
//! - `variable-len`: não existe no golden atual (defensivo: nunca casa).
//!
//! **Escopo de largura de campo:** o golden descreve O QUE varia, não a
//! semântica dos bytes — pp/PG/CRC BE, effectCode/float LE e nibble ficam
//! no codec (ADR-1). O consumidor aqui só sabe: estes N bytes variam.

mod despacho;
mod fio;
mod padrao;
mod template;

pub use fio::{decode_envelope, hex_decode};
pub use padrao::{Pattern, PatternKind, Segment, SegmentKind};
pub use template::{Endpoint, Example, Template};

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use serde::Deserialize;

use crate::ProtocolError;

/// O golden embutido no binário (`include_str!` — spec é insumo, R1;
/// bytes congelados pelo `-text` do .gitattributes).
pub const GOLDEN_JSON: &str = include_str!("../../../docs/referencias/protocol_golden.json");

/// Constrói `ProtocolError::InvalidShape` (helper local).
fn shape(expected: impl Into<String>, got: impl Into<String>) -> ProtocolError {
    ProtocolError::InvalidShape {
        expected: expected.into(),
        got: got.into(),
    }
}

/// Forma CRUA do JSON (serde materializa isto; [`GoldenFile`] é o validado).
#[derive(Debug, Deserialize)]
struct RawGolden {
    /// Metadados do arquivo (título, convenções, fontes) — preservados.
    #[serde(rename = "_meta", default)]
    meta: serde_json::Value,
    /// Os templates de transação.
    transactions: Vec<Template>,
}

/// O golden-file validado, com índices O(1) por endpoint TIPADO.
#[derive(Debug, Clone)]
pub struct GoldenFile {
    /// Metadados de proveniência.
    meta: serde_json::Value,
    /// Templates na ordem do arquivo.
    templates: Vec<Template>,
    /// (func, addr) OUT -> posições dos templates `req`/`write` com esse
    /// endpoint. PODE ter 2 entradas (caso congelado + geral do boot) —
    /// desambiguar por `var_count` (ver [`GoldenFile::request_template`]).
    by_out: HashMap<(u8, [u8; 4]), Vec<usize>>,
    /// (func, addr) IN -> posições dos templates `req`/`push` (multi por
    /// by-len; ordem do arquivo preservada).
    by_in: HashMap<(u8, [u8; 4]), Vec<usize>>,
}

static GOLDEN: OnceLock<Result<GoldenFile, Arc<ProtocolError>>> = OnceLock::new();

impl GoldenFile {
    /// Carrega e valida o golden de um JSON em memória.
    ///
    /// Valida, além do schema: hex de func/addr (1B/4B), coerência de lados
    /// por tipo (`req`=OUT+IN, `write`=só OUT, `push`=só IN) e presença dos
    /// padrões de payload correspondentes.
    ///
    /// # Erros
    /// [`ProtocolError::InvalidShape`] com evidência da violação.
    pub fn from_json(json: &str) -> Result<Self, ProtocolError> {
        let raw: RawGolden = serde_json::from_str(json).map_err(|e| {
            shape(
                "schema do protocol_golden.json (_meta+transactions)",
                e.to_string(),
            )
        })?;
        if raw.transactions.is_empty() {
            return Err(shape("templates de transação", "arquivo sem transações"));
        }

        // Passo 1 — coerência estrutural por tipo + parse dos endpoints
        // (hex válido é invariante da carga, não do dispatch).
        let mut by_out: HashMap<(u8, [u8; 4]), Vec<usize>> = HashMap::new();
        let mut by_in: HashMap<(u8, [u8; 4]), Vec<usize>> = HashMap::new();
        for (i, t) in raw.transactions.iter().enumerate() {
            let ty = t.template_type.as_str();
            let where_ = format!("template {i} ({ty})");
            match ty {
                "req" => {
                    let out = t
                        .out
                        .as_ref()
                        .ok_or_else(|| shape("req com lado OUT", where_.clone()))?;
                    let inc = t
                        .in_
                        .as_ref()
                        .ok_or_else(|| shape("req com lado IN", where_.clone()))?;
                    if t.request_payload.is_none() {
                        return Err(shape("req com request_payload", where_));
                    }
                    if t.response_payload.is_none() {
                        return Err(shape("req com response_payload", where_));
                    }
                    by_out.entry(out.parsed()?).or_default().push(i);
                    by_in.entry(inc.parsed()?).or_default().push(i);
                }
                "write" => {
                    let out = t
                        .out
                        .as_ref()
                        .ok_or_else(|| shape("write com lado OUT", where_.clone()))?;
                    if t.in_.is_some() {
                        return Err(shape("write SEM lado IN", where_.clone()));
                    }
                    if t.request_payload.is_none() {
                        return Err(shape("write com request_payload", where_));
                    }
                    if t.response_payload.is_some() {
                        return Err(shape("write SEM response_payload", where_));
                    }
                    by_out.entry(out.parsed()?).or_default().push(i);
                }
                "push" => {
                    let inc = t
                        .in_
                        .as_ref()
                        .ok_or_else(|| shape("push com lado IN", where_.clone()))?;
                    if t.out.is_some() {
                        return Err(shape("push SEM lado OUT", where_.clone()));
                    }
                    if t.response_payload.is_none() {
                        return Err(shape("push com response_payload", where_));
                    }
                    by_in.entry(inc.parsed()?).or_default().push(i);
                }
                other => return Err(shape("tipo req/write/push", format!("{where_}: '{other}'"))),
            }
        }

        Ok(GoldenFile {
            meta: raw.meta,
            templates: raw.transactions,
            by_out,
            by_in,
        })
    }

    /// O golden EMBUTIDO (parse único por processo via `OnceLock`).
    ///
    /// # Erros
    /// [`ProtocolError::InvalidShape`] (só se o JSON embutido for inválido —
    /// impossível no build normal, pois os testes provam a carga).
    pub fn embedded() -> Result<&'static GoldenFile, ProtocolError> {
        GOLDEN
            .get_or_init(|| GoldenFile::from_json(GOLDEN_JSON).map_err(Arc::new))
            .as_ref()
            .map_err(|e| ProtocolError::clone(e))
    }

    /// Metadados de proveniência do arquivo.
    pub fn meta(&self) -> &serde_json::Value {
        &self.meta
    }

    /// Todos os templates (ordem do arquivo).
    pub fn templates(&self) -> &[Template] {
        &self.templates
    }
}
