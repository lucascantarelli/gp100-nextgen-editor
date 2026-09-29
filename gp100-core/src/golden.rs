//! golden — consumidor de `docs/protocol_golden.json` (ROADMAP M0.3).
//!
//! **Insumo:** o golden é a especificação executável do protocolo (Baseline
//! v1.0, hash no §13 do PROTOCOL.md, regime de bytes `-text`). Este módulo
//! materializa o JSON em tipos — [`GoldenFile`] → [`Template`] → [`Pattern`] —
//! com as duas operações do DoD:
//!
//! - [`Template::build_request`] — monta o SysEx COMPLETO
//!   (`F0 21 25 7F 47 50 2D 64 | FUNC | ADDR(4B BE) | DATA | F7`) substituindo
//!   os segmentos `var` (posicionais, na ordem) pelos bytes fornecidos;
//! - [`Template::matches_response`] / [`GoldenFile::match_response`] — casa um
//!   payload recebido contra o padrão (len + segmentos `const`) e EXTRAÍ os
//!   trechos variáveis (≈ "Vars" do DoD).
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
//! no codec (M0.4, ADR-1). O consumidor aqui só sabe: estes N bytes variam.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use serde::Deserialize;

use crate::{ProtocolError, SYSEX_EOX, SYSEX_HEADER};

/// O golden embutido no binário (`include_str!` — spec é insumo, R1;
/// bytes congelados pelo `-text` do .gitattributes).
pub const GOLDEN_JSON: &str = include_str!("../../docs/protocol_golden.json");

/// Constrói `ProtocolError::InvalidShape` (helper local).
fn shape(expected: impl Into<String>, got: impl Into<String>) -> ProtocolError {
    ProtocolError::InvalidShape {
        expected: expected.into(),
        got: got.into(),
    }
}

/// Decodifica hex ASCII em bytes (pares; caixa alta/baixa).
///
/// # Erros
/// [`ProtocolError::InvalidShape`] com offset do par inválido.
pub fn hex_decode(s: &str) -> Result<Vec<u8>, ProtocolError> {
    let b = s.as_bytes();
    if !b.len().is_multiple_of(2) {
        return Err(shape(
            "hex com número par de dígitos",
            format!("len={}", b.len()),
        ));
    }
    let mut out = Vec::with_capacity(b.len() / 2);
    for (i, pair) in b.as_chunks::<2>().0.iter().enumerate() {
        let hi = (pair[0] as char).to_digit(16);
        let lo = (pair[1] as char).to_digit(16);
        match (hi, lo) {
            (Some(h), Some(l)) => out.push((h * 16 + l) as u8),
            _ => {
                return Err(shape(
                    "dígito hexadecimal",
                    format!("byte {i}: '{}{}'", pair[0] as char, pair[1] as char),
                ))
            }
        }
    }
    Ok(out)
}

/// Separa um SysEx completo em `(func, addr[4], data)` — trim no 1º `F7`
/// (regra de captura do knowledge.md; em mensagens construídas é o próprio
/// fim). Valida o cabeçalho contra [`SYSEX_HEADER`].
///
/// # Erros
/// [`ProtocolError::InvalidShape`] se header/F7/comprimento não baterem
/// (inclui truncamento SEM-HDR do ring buffer do proxy).
pub fn decode_envelope(data: &[u8]) -> Result<(u8, [u8; 4], &[u8]), ProtocolError> {
    if data.len() < SYSEX_HEADER.len() + 5 + 1 {
        return Err(shape(
            "SysEx completo (header 8 + func 1 + addr 4 + data + F7)",
            format!("recebidos {} bytes", data.len()),
        ));
    }
    if data[..SYSEX_HEADER.len()] != SYSEX_HEADER {
        return Err(shape(
            "cabeçalho F0 21 25 7F 47 50 2D 64",
            format!(
                "começo: {}",
                String::from_utf8_lossy(&data[..8.min(data.len())])
            ),
        ));
    }
    let end = data[SYSEX_HEADER.len()..]
        .iter()
        .position(|&b| b == SYSEX_EOX)
        .map(|i| SYSEX_HEADER.len() + i)
        .ok_or_else(|| shape("F7 terminador", "ausente (mensagem truncada?)"))?;
    let body = &data[SYSEX_HEADER.len()..end];
    if body.len() < 5 {
        return Err(shape(
            "func + addr(4) no corpo",
            format!("corpo de {} bytes", body.len()),
        ));
    }
    let mut addr = [0u8; 4];
    addr.copy_from_slice(&body[1..5]);
    Ok((body[0], addr, &body[5..]))
}

/// Tipo de padrão de payload (espelha `payload_pattern` do build_golden.py).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PatternKind {
    /// 0 bytes (requests de leitura).
    Empty,
    /// Todos os bytes fixos entre as instâncias.
    Const,
    /// Todos os bytes variam.
    Var,
    /// Blocos fixos e variáveis alternados.
    Mixed,
    /// Comprimento não previsível (não existe no golden atual).
    VariableLen,
    /// Vários formatos por comprimento (mesmo endereço, respostas distintas).
    ByLen,
}

/// Um trecho do padrão: `const` (hex fixo) ou `var` (N bytes variáveis).
#[derive(Debug, Clone, Deserialize)]
pub struct Segment {
    /// `const` ou `var`.
    #[serde(rename = "kind")]
    pub segment_kind: SegmentKind,
    /// Bytes fixos em hex (só `const`).
    #[serde(default)]
    pub hex: String,
    /// Quantos bytes variam (só `var`).
    #[serde(default)]
    pub count: usize,
}

/// Tipo de segmento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SegmentKind {
    /// Bytes fixos (`hex`).
    Const,
    /// N bytes variáveis (`count`).
    Var,
}

/// Padrão de payload de um template (request ou resposta).
#[derive(Debug, Clone, Deserialize)]
pub struct Pattern {
    /// Tipo do padrão.
    #[serde(rename = "kind")]
    pub pattern_kind: PatternKind,
    /// Comprimento total esperado (`None` em `by-len`/`variable-len`).
    #[serde(default)]
    pub len: Option<usize>,
    /// Segmentos const/var (ausente em `empty`/`by-len`).
    #[serde(default)]
    pub segments: Vec<Segment>,
    /// Comprimentos disponíveis (só `by-len`).
    #[serde(default)]
    pub lens: Vec<usize>,
    /// Sub-padrões por comprimento, chaveados em decimal como string (só `by-len`).
    #[serde(default)]
    pub by_len: HashMap<String, Pattern>,
}

impl Pattern {
    /// Comprimento esperado (`None` = variável por definição).
    pub fn expected_len(&self) -> Option<usize> {
        match self.pattern_kind {
            PatternKind::ByLen | PatternKind::VariableLen => None,
            _ => self.len,
        }
    }

    /// `true` se `data` satisfaz o padrão (comprimento + segmentos const).
    pub fn matches_data(&self, data: &[u8]) -> bool {
        self.extract_vars(data).is_some()
    }

    /// Casaria o padrão e devolve os trechos variáveis (posicionais, na
    /// ordem dos segmentos `var`). `None` = não casa.
    pub fn extract_vars<'a>(&self, data: &'a [u8]) -> Option<Vec<&'a [u8]>> {
        match self.pattern_kind {
            PatternKind::VariableLen => None,
            PatternKind::ByLen => {
                let sub = self.by_len.get(&data.len().to_string())?;
                sub.extract_vars(data)
            }
            _ => {
                if Some(data.len()) != self.len {
                    return None;
                }
                let mut vars = Vec::new();
                let mut pos = 0usize;
                for s in &self.segments {
                    match s.segment_kind {
                        SegmentKind::Const => {
                            let fixed = hex_decode(&s.hex).ok()?;
                            if data[pos..pos + fixed.len()] != fixed[..] {
                                return None;
                            }
                            pos += fixed.len();
                        }
                        SegmentKind::Var => {
                            vars.push(&data[pos..pos + s.count]);
                            pos += s.count;
                        }
                    }
                }
                debug_assert_eq!(pos, data.len());
                Some(vars)
            }
        }
    }

    /// Constrói o payload no `out`, consumindo `cursor` (as vars, em ordem).
    ///
    /// # Erros
    /// [`ProtocolError::InvalidShape`] se o padrão não é buildável
    /// (`by-len`/`variable-len` são só de resposta) ou faltam vars.
    pub fn build_into(&self, out: &mut Vec<u8>, cursor: &mut &[u8]) -> Result<(), ProtocolError> {
        match self.pattern_kind {
            PatternKind::Empty => Ok(()),
            PatternKind::ByLen => Err(shape(
                "padrão de request buildável",
                "by-len é só de resposta (despacho por comprimento)",
            )),
            PatternKind::VariableLen => Err(shape(
                "padrão de request buildável",
                "variable-len não é suportado",
            )),
            PatternKind::Const | PatternKind::Var | PatternKind::Mixed => {
                for s in &self.segments {
                    match s.segment_kind {
                        SegmentKind::Const => out.extend_from_slice(&hex_decode(&s.hex)?),
                        SegmentKind::Var => {
                            if cursor.len() < s.count {
                                return Err(shape(
                                    format!("{} bytes de var (segmento {})", s.count, s.hex),
                                    format!("restam {} no cursor", cursor.len()),
                                ));
                            }
                            out.extend_from_slice(&cursor[..s.count]);
                            *cursor = &cursor[s.count..];
                        }
                    }
                }
                Ok(())
            }
        }
    }
}

/// Endereço de fio (func + addr 4B) de um lado da transação.
#[derive(Debug, Clone, Deserialize)]
pub struct Endpoint {
    /// FUNC em hex (`11` = read, `12` = dados/write — §13.1).
    pub func: String,
    /// ADDR em hex (8 dígitos, u32 BE no fio).
    pub addr: String,
}

/// Exemplo real capturado (hex do PAYLOAD — sem envelope; convenção do
/// build_golden.py: data.hex(), não o SysEx completo).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Example {
    /// Sessão de origem (S1..S4).
    #[serde(default)]
    pub session: Option<String>,
    /// Payload do lado único (write/push).
    #[serde(default)]
    pub hex: Option<String>,
    /// Payload do request (req).
    #[serde(default)]
    pub request_hex: Option<String>,
    /// Payload da resposta (req).
    #[serde(default)]
    pub response_hex: Option<String>,
}

/// Um template de transação do golden.
#[derive(Debug, Clone, Deserialize)]
pub struct Template {
    /// `req` (request→resposta), `write` (fire-and-forget) ou `push` (IN sem OUT).
    #[serde(rename = "type")]
    pub template_type: String,
    /// Ocorrências nas 4 sessões (11325 no total).
    pub count: usize,
    /// Sessões onde aparece.
    #[serde(default)]
    pub sessions: Vec<String>,
    /// Lado OUT (req/write).
    #[serde(default)]
    pub out: Option<Endpoint>,
    /// Lado IN (req/push). JSON usa `in` (keyword em Rust) — daí o rename.
    #[serde(default, rename = "in")]
    pub in_: Option<Endpoint>,
    /// Padrão do payload do request (req/write).
    #[serde(default, rename = "request_payload")]
    pub request_payload: Option<Pattern>,
    /// Padrão do payload da resposta (req/push).
    #[serde(default, rename = "response_payload")]
    pub response_payload: Option<Pattern>,
    /// Exemplo real capturado.
    #[serde(default)]
    pub example: Example,
    /// Semântica por endereço (narrativa §13).
    #[serde(default)]
    pub semantic: Option<String>,
}

impl Template {
    /// FUNC do lado OUT (req/write).
    pub fn func_out(&self) -> Option<&str> {
        self.out.as_ref().map(|e| e.func.as_str())
    }

    /// ADDR do lado OUT (req/write).
    pub fn addr_out(&self) -> Option<&str> {
        self.out.as_ref().map(|e| e.addr.as_str())
    }

    /// FUNC do lado IN (req/push).
    pub fn func_in(&self) -> Option<&str> {
        self.in_.as_ref().map(|e| e.func.as_str())
    }

    /// ADDR do lado IN (req/push).
    pub fn addr_in(&self) -> Option<&str> {
        self.in_.as_ref().map(|e| e.addr.as_str())
    }

    /// Padrão do payload do request.
    pub fn request_pattern(&self) -> Option<&Pattern> {
        self.request_payload.as_ref()
    }

    /// Padrão do payload da resposta.
    pub fn response_pattern(&self) -> Option<&Pattern> {
        self.response_payload.as_ref()
    }

    /// Semântica narrativa (§13) do endereço, se presente no golden.
    pub fn semantic(&self) -> Option<&str> {
        self.semantic.as_deref()
    }

    /// Monta o SysEx COMPLETO do request, substituindo os segmentos `var`
    /// (posicionais, na ordem) por `vars`. Strict: vars em excesso = erro.
    ///
    /// # Erros
    /// [`ProtocolError::InvalidShape`] se o template é `push` (não tem
    /// request), o padrão não é buildável, faltam/excedem vars, ou hex
    /// de func/addr é inválido (não deve ocorrer no golden congelado).
    pub fn build_request(&self, vars: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let pat = self.request_payload.as_ref().ok_or_else(|| {
            shape(
                "template com request (req/write)",
                format!("tipo '{}' (push não tem request)", self.template_type),
            )
        })?;
        let mut data = Vec::with_capacity(pat.expected_len().unwrap_or(vars.len()));
        let mut cursor = vars;
        pat.build_into(&mut data, &mut cursor)?;
        if !cursor.is_empty() {
            return Err(shape(
                "vars exatamente suficientes para o padrão",
                format!("{} bytes sobrando", cursor.len()),
            ));
        }
        let func = hex_decode(self.func_out().unwrap_or_default())?;
        let addr = hex_decode(self.addr_out().unwrap_or_default())?;
        let mut sysex =
            Vec::with_capacity(SYSEX_HEADER.len() + func.len() + addr.len() + data.len() + 1);
        sysex.extend_from_slice(&SYSEX_HEADER);
        sysex.extend_from_slice(&func);
        sysex.extend_from_slice(&addr);
        sysex.extend_from_slice(&data);
        sysex.push(SYSEX_EOX);
        Ok(sysex)
    }

    /// Casa `data` (payload BRUTO, sem envelope) contra o padrão da resposta
    /// e devolve os trechos variáveis extraídos (`None` = não casa).
    pub fn matches_response<'a>(&self, data: &'a [u8]) -> Option<Vec<&'a [u8]>> {
        self.response_payload.as_ref()?.extract_vars(data)
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

/// O golden-file validado: 40 templates consultáveis por endpoint.
#[derive(Debug, Clone)]
pub struct GoldenFile {
    /// Metadados de proveniência.
    meta: serde_json::Value,
    /// Templates na ordem do arquivo.
    templates: Vec<Template>,
}

static GOLDEN: OnceLock<Result<GoldenFile, Arc<ProtocolError>>> = OnceLock::new();

impl GoldenFile {
    /// Carrega e valida o golden de um JSON em memória.
    ///
    /// # Erros
    /// [`ProtocolError::InvalidShape`] se o schema não casar.
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
        Ok(GoldenFile {
            meta: raw.meta,
            templates: raw.transactions,
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

    /// O template cujo OUT é `(func, addr)` — requests (11xx/12xx de leitura)
    /// e writes. Vários writes compartilham endereço? No golden atual não
    /// (endereços de write distintos por semântica); devolve o primeiro.
    pub fn for_request(&self, func: &str, addr: &str) -> Option<&Template> {
        self.templates.iter().find(|t| {
            t.template_type != "push" && t.func_out() == Some(func) && t.addr_out() == Some(addr)
        })
    }

    /// TODOS os templates cujo IN é `(func, addr)` — respostas podem ter
    /// múltiplos formatos (`by-len`: ACK 4B × tabela 75B em `12001002`).
    pub fn for_response(&self, func: &str, addr: &str) -> Vec<&Template> {
        self.templates
            .iter()
            .filter(|t| t.func_in() == Some(func) && t.addr_in() == Some(addr))
            .collect()
    }

    /// Despacha uma resposta: procura entre os candidatos de `(func, addr)`
    /// aquele cujo padrão casa com `data`; devolve o template + as vars.
    /// `data` é o payload BRUTO (use [`decode_envelope`] para separar).
    pub fn match_response<'a>(
        &'a self,
        func: &str,
        addr: &str,
        data: &'a [u8],
    ) -> Option<(&'a Template, Vec<&'a [u8]>)> {
        self.for_response(func, addr)
            .into_iter()
            .find_map(|t| t.matches_response(data).map(|vars| (t, vars)))
    }
}
