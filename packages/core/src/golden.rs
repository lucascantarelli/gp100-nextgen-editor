//! golden — consumidor de `docs/protocol_golden.json`.
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

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use serde::Deserialize;

use crate::{ProtocolError, SYSEX_EOX, SYSEX_HEADER};

/// O golden embutido no binário (`include_str!` — spec é insumo, R1;
/// bytes congelados pelo `-text` do .gitattributes).
pub const GOLDEN_JSON: &str = include_str!("../../../docs/protocol_golden.json");

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

    /// Nº de bytes VARIÁVEIS que o padrão consome ao construir (`None` se o
    /// padrão não é buildável — `by-len`/`variable-len`, só de resposta).
    ///
    /// É a CHAVE DE DESAMBIGUAÇÃO entre templates que compartilham o mesmo
    /// endpoint OUT (ver [`GoldenFile::request_template`]).
    pub fn var_count(&self) -> Option<usize> {
        match self.pattern_kind {
            PatternKind::Empty => Some(0),
            PatternKind::Const | PatternKind::Var | PatternKind::Mixed => Some(
                self.segments
                    .iter()
                    .filter(|s| s.segment_kind == SegmentKind::Var)
                    .map(|s| s.count)
                    .sum(),
            ),
            PatternKind::ByLen | PatternKind::VariableLen => None,
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

    /// `true` se `data` tem a FORMA do padrão: o comprimento esperado (no
    /// `by-len`, um dos comprimentos da tabela). **Não compara os bytes
    /// `const`** — ver [`Template::matches_shape`].
    pub fn matches_shape(&self, data: &[u8]) -> bool {
        match self.pattern_kind {
            PatternKind::ByLen => self
                .by_len
                .get(&data.len().to_string())
                .is_some_and(|sub| sub.matches_shape(data)),
            // `variable-len` não ocorre no golden (0 ocorrências, baseline
            // v1.1); sem comprimento conhecido não há forma a validar.
            PatternKind::VariableLen => false,
            _ => Some(data.len()) == self.len,
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

impl Endpoint {
    /// Par tipado `(func u8, addr 4B BE)` — a forma que o FIO entrega.
    ///
    /// # Erros
    /// [`ProtocolError::InvalidShape`] se hex inválido (a carga do golden
    /// valida; não deve ocorrer).
    pub fn parsed(&self) -> Result<(u8, [u8; 4]), ProtocolError> {
        parse_endpoint(&self.func, &self.addr)
    }
}

/// Converte `func`/`addr` hex do golden para a forma tipada do fio.
fn parse_endpoint(func: &str, addr: &str) -> Result<(u8, [u8; 4]), ProtocolError> {
    let f = hex_decode(func)?;
    let a = hex_decode(addr)?;
    if f.len() != 1 || a.len() != 4 {
        return Err(shape(
            "func com 1 byte e addr com 4 bytes (hex)",
            format!("func={func} addr={addr}"),
        ));
    }
    Ok((f[0], [a[0], a[1], a[2], a[3]]))
}

/// Addr 4B em hex minúsculo (mensagens de erro).
fn addr_hex(addr: &[u8; 4]) -> String {
    addr.iter().map(|b| format!("{b:02x}")).collect()
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
        // Endpoint tipado (parse validado na carga; sem hex de strings por
        // chamada — a FSM constrói requests em loop).
        let out = self.out.as_ref().ok_or_else(|| {
            shape(
                "template com lado OUT",
                format!("tipo '{}'", self.template_type),
            )
        })?;
        let (func, addr) = out.parsed()?;
        let mut sysex = Vec::with_capacity(SYSEX_HEADER.len() + 1 + 4 + data.len() + 1);
        sysex.extend_from_slice(&SYSEX_HEADER);
        sysex.push(func);
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

    /// `true` se `data` tem a FORMA da resposta — comprimento (no `by-len`,
    /// um dos comprimentos da tabela) — sem comparar os bytes `const`.
    ///
    /// **Por que existe:** os segmentos `const` do golden são o payload
    /// CONGELADO das capturas S1–S4 — o estado de FÁBRICA do aparelho
    /// analisado (ADR R2/R3). Uma resposta legítima do aparelho do dono
    /// difere sempre que o dono editou o objeto: renomear um preset muda o
    /// nome em pg0 (medido em campo no boot do H4, 2026-10-09: pp 0x0000
    /// renomeado para `H2 TESTE` deixou o `0x30` stale do nome antigo).
    /// `match_response` completo rejeitaria essa resposta — comparar
    /// conteúdo vivo contra snapshot de análise é a falha que a issue #177
    /// registra. A FORMA (endereço + comprimento) é schema; o conteúdo é do
    /// hardware, e é validado pelo DECODE dos objetos (`preset_pages`,
    /// `codec`), não pelo golden.
    pub fn matches_shape(&self, data: &[u8]) -> bool {
        self.response_payload
            .as_ref()
            .is_some_and(|p| p.matches_shape(data))
    }

    /// Monta o SysEx COMPLETO da RESPOSTA (lado IN) — o simétrico de
    /// [`Template::build_request`], consumido pelo `MockDevice`:
    /// o mock responde pela SPEC, nunca por hex hardcode.
    ///
    /// `fill(idx, count)` devolve os bytes do `idx`-ésimo segmento `var`
    /// (com `count` bytes); `desired_len` escolhe o sub-padrão em respostas
    /// `by-len` (ex.: ACK 4B × tabela 75B em `12001002`) — obrigatório
    /// nesse caso.
    ///
    /// # Erros
    /// [`ProtocolError::InvalidShape`] se o template não tem lado IN, o
    /// padrão não tem segmentos (`empty`), `by-len` sem `desired_len` (ou
    /// len inexistente), `fill` devolve tamanho errado, ou hex de
    /// func/addr inválido.
    pub fn build_response(
        &self,
        fill: &mut dyn FnMut(usize, usize) -> Vec<u8>,
        desired_len: Option<usize>,
    ) -> Result<Vec<u8>, ProtocolError> {
        let pat = self.response_payload.as_ref().ok_or_else(|| {
            shape(
                "template com resposta (req/push)",
                format!("tipo '{}' sem response_payload", self.template_type),
            )
        })?;
        let chosen = match pat.pattern_kind {
            PatternKind::ByLen => {
                let len = desired_len.ok_or_else(|| {
                    shape(
                        "desired_len em resposta by-len",
                        format!("lens disponíveis: {:?}", pat.lens),
                    )
                })?;
                pat.by_len
                    .get(&len.to_string())
                    .ok_or_else(|| shape("len de sub-padrão by-len", format!("{len}")))?
            }
            _ => pat,
        };
        if matches!(chosen.pattern_kind, PatternKind::Empty) {
            return Err(shape("resposta com payload", "padrão empty (0 bytes)"));
        }
        let mut data = Vec::with_capacity(chosen.expected_len().unwrap_or(0));
        let mut var_idx = 0usize;
        for s in &chosen.segments {
            match s.segment_kind {
                SegmentKind::Const => data.extend_from_slice(&hex_decode(&s.hex)?),
                SegmentKind::Var => {
                    let bytes = fill(var_idx, s.count);
                    if bytes.len() != s.count {
                        return Err(shape(
                            format!("var {var_idx} com {count} bytes", count = s.count),
                            format!("fill devolveu {}", bytes.len()),
                        ));
                    }
                    data.extend_from_slice(&bytes);
                    var_idx += 1;
                }
            }
        }
        let inc = self.in_.as_ref().ok_or_else(|| {
            shape(
                "template com lado IN",
                format!("tipo '{}'", self.template_type),
            )
        })?;
        let (func, addr) = inc.parsed()?;
        let mut sysex = Vec::with_capacity(SYSEX_HEADER.len() + 1 + 4 + data.len() + 1);
        sysex.extend_from_slice(&SYSEX_HEADER);
        sysex.push(func);
        sysex.extend_from_slice(&addr);
        sysex.extend_from_slice(&data);
        sysex.push(SYSEX_EOX);
        Ok(sysex)
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

    /// O template de REQUEST (`req`/`write`) para o endpoint TIPADO
    /// `(func u8, addr 4B BE)` — a forma que a FSM tem em mãos.
    ///
    /// # Desambiguação
    /// Três endpoints do golden têm DOIS templates (caso CONGELADO do boot —
    /// payload const — e caso GERAL — pp variável). A chave é o nº de vars:
    /// `var_count == vars.len()` escolhe o certo; sem duplicata, 0 vars
    /// devolve o único. Erro se nenhum/nenhum único casar.
    pub fn request_template(
        &self,
        func: u8,
        addr: &[u8; 4],
        vars: usize,
    ) -> Result<&Template, ProtocolError> {
        let idx = self.by_out.get(&(func, *addr)).ok_or_else(|| {
            shape(
                "endpoint de request no golden",
                format!("func={func:02x} addr={}", addr_hex(addr)),
            )
        })?;
        let mut found = idx.iter().filter(|&&i| {
            self.templates[i]
                .request_pattern()
                .and_then(|p| p.var_count())
                == Some(vars)
        });
        let first = found.next().ok_or_else(|| {
            shape(
                format!(
                    "template com {vars} bytes de var em {func:02x}/{}",
                    addr_hex(addr)
                ),
                "nenhum candidato",
            )
        })?;
        if found.next().is_some() {
            return Err(shape(
                "desambiguação única por var_count",
                format!(
                    "2+ templates com {vars} vars em {func:02x}/{}",
                    addr_hex(addr)
                ),
            ));
        }
        Ok(&self.templates[*first])
    }

    /// Atalho ergonômico da FSM: resolve o template pelos TIPOS e já monta
    /// o SysEx completo. Equivale a `request_template(func, addr,
    /// vars.len())?.build_request(vars)`.
    pub fn build_request(
        &self,
        func: u8,
        addr: &[u8; 4],
        vars: &[u8],
    ) -> Result<Vec<u8>, ProtocolError> {
        self.request_template(func, addr, vars.len())?
            .build_request(vars)
    }

    /// TODOS os templates cujo IN é o endpoint TIPADO — respostas podem ter
    /// múltiplos formatos (by-len: ACK 4B × tabela 75B em `12001002`).
    pub fn for_response(&self, func: u8, addr: &[u8; 4]) -> Vec<&Template> {
        self.by_in
            .get(&(func, *addr))
            .map(|ids| ids.iter().map(|&i| &self.templates[i]).collect())
            .unwrap_or_default()
    }

    /// Despacha uma resposta: entre os candidatos do endpoint TIPADO, o
    /// primeiro cujo padrão casa com `data`. `data` é o payload BRUTO
    /// (use [`decode_envelope`] para separar do envelope).
    pub fn match_response<'a>(
        &'a self,
        func: u8,
        addr: &[u8; 4],
        data: &'a [u8],
    ) -> Option<(&'a Template, Vec<&'a [u8]>)> {
        self.for_response(func, addr)
            .into_iter()
            .find_map(|t| t.matches_response(data).map(|vars| (t, vars)))
    }

    /// Despacho por FORMA: entre os templates do endpoint TIPADO, o primeiro
    /// cujo comprimento casa (no `by-len`, um dos comprimentos da tabela).
    /// **Não compara bytes `const`** — ver [`Template::matches_shape`].
    ///
    /// É o que a FSM (`wait_for`) usa para validar respostas do aparelho:
    /// o conteúdo é do hardware e passa pelo decode dos objetos, nunca por
    /// comparação com snapshot de análise (#177).
    pub fn match_shape(&self, func: u8, addr: &[u8; 4], data: &[u8]) -> Option<&Template> {
        self.for_response(func, addr)
            .into_iter()
            .find(|t| t.matches_shape(data))
    }

    /// Todos os templates de REQUEST cujo OUT é o endpoint TIPADO — o
    /// despacho do `MockDevice`: entre os candidatos, vence o PRIMEIRO
    /// (ordem do arquivo, D2/rev.2) cujo `request_pattern` casa com o
    /// payload do frame recebido. Ex.: em `12001002`, o req de CHUNK
    /// (33B var) e o de TABELA (75B nibble-exp) distinguem por len+consts.
    pub fn for_request(&self, func: u8, addr: &[u8; 4]) -> Vec<&Template> {
        self.by_out
            .get(&(func, *addr))
            .map(|ids| ids.iter().map(|&i| &self.templates[i]).collect())
            .unwrap_or_default()
    }

    /// O request recebido casa com ALGUM template cujo OUT é o endpoint?
    /// Devolve o índice do template na ordem do arquivo (para log/erro).
    pub fn match_request(&self, func: u8, addr: &[u8; 4], data: &[u8]) -> Option<usize> {
        self.for_request(func, addr)
            .into_iter()
            .find(|t| {
                t.request_payload
                    .as_ref()
                    .is_some_and(|p| p.matches_data(data))
            })
            .and_then(|t| self.templates.iter().position(|x| std::ptr::eq(x, t)))
    }
}
