//! a TRANSACAO do golden: o par OUT/IN e a materializacao dele no fio (request e resposta).

use serde::Deserialize;

use super::fio::hex_decode;
use super::padrao::{Pattern, PatternKind, SegmentKind};
use super::shape;
use crate::{ProtocolError, SYSEX_EOX, SYSEX_HEADER};

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
