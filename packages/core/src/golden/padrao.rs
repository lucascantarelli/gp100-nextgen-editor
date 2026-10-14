//! o PADRAO de payload: o que varia e o que e fixo — casar, extrair vars e construir.

use std::collections::HashMap;

use serde::Deserialize;

use super::fio::hex_decode;
use super::shape;
use crate::ProtocolError;

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
