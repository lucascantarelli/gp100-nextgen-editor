//! Decode das páginas 13xx (família `13 01 00 03`) — o palco vivo (#155).
//!
//! **O que estava errado com toda a análise anterior.** A página vem
//! **nibble-expandida**: cada byte do fio guarda um nibble e o par vira um
//! byte real (`04 09` → `0x49` = `I`). A varredura que a #155 pedia — offsets
//! crus 0..28 × comprimentos 6/10/12 — procura o nome em bytes que nunca
//! foram o nome. Decodificado, o nome está na **pg0, offset 2, em 198/198**
//! (`00 00 | nome[12, pad NUL] | …`).
//!
//! **Módulo puro, sem fio e sem `Session`** — é assim que o dataset inteiro
//! (1.782 páginas de `analysis/fixtures/boot.jsonl`) é testado sem hardware,
//! o mesmo princípio de `param_range.rs` (#110) e do inventário (#132).
//!
//! **O decode em si não é meu:** reusa [`crate::codec::nibble_collapse`],
//! que já é a primitiva strict do repo (recusa byte > 0x0F e comprimento
//! ímpar, de propósito — mascarar esconderia corrupção). Aqui só se valida a
//! estrutura da família (tamanho e `PG`) e se traduz o erro para o
//! diagnóstico com página/offset, que é o que o relatório de campo precisa.
//!
//! # Layout da pg0 (medido em 198/198, não inferido)
//!
//! Corpo decodificado, 96 bytes:
//!
//! | offset | tamanho | conteúdo |
//!|---|---|---|
//! | 0..2 | 2B | `pp` em **u16 LE** (0x0000..0x0062 / 0x0100..0x0162) |
//! | **2..14** | **12B** | **nome ASCII, pad NUL** — [`NOME_LEN`] = 12 |
//! | 14..32 | 18B | **9 × u16 LE** = cadeia (0 = slot vazio, 1..8 = efeito) |
//!
//! **Por que 12 e não 16:** a cadeia começa no offset 14, então um nome de
//! 14 bytes invadiria os slots. Com 16 o teste estourava com `byte 0x01
//! depois do NUL no offset 16` — o `01 00` de um preset cuja cadeia começa
//! por um efeito em posição 0. Os 9 pares vêm **reordenados** por preset
//! (`05 00 07 00 06 00` — a ordem real da cadeia), não sempre `01..08`.
//!
//! **Por que `escape_value` na comparação:** o fio traz `Dub&Vibe` (literal)
//! e o `.prst` guarda `Dub&amp;Vibe` (o parser mantém o valor BRUTO, ver
//! `preset.rs`). Comparar bruto contra escapado seria uma divergência
//! falsa — então a comparação é feita no mesmo espaço com
//! [`crate::preset::escape_value`], a inversa que o repo já define.

use crate::codec::nibble_collapse;
use crate::model::{ControlKind, Dictionary};
use crate::pedalboard::{archetype_of, family_of, slug, BoardView, KnobSpec, SlotSpec};
use crate::session::StatePage;

/// Tamanho do corpo ANTES do decode (o header de 4B já foi removido).
const CORPO_CRU_PG0_7: usize = 192;
/// Tamanho do corpo da pg8 antes do decode (4B header + 28B = 32B no fio).
const CORPO_CRU_PG8: usize = 28;
/// Tamanho do header: `[pp u16BE][00][PG]`.
const HEADER: usize = 4;
/// Bytes do nome na pg0 DEPOIS do decode (offset fixo, 198/198).
const NOME_OFFSET: usize = 2;
/// Espaço do nome na pg0 (offset 2..14). **12, não 16:** a cadeia de slots
/// começa no offset 14 (medido em 198/198) — ver o layout no topo do módulo.
const NOME_LEN: usize = 12;

/// O que o decode recusou. Cada variante vira texto legível — um `Err` sem
/// motivo não é diagnóstico para quem lê o relatório de campo.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DecodeError {
    /// A página não tem o tamanho que a família declara.
    #[error("pagina {pagina}: {got}B (esperado {esperado}B)")]
    TamanhoInesperado {
        /// Página 0..=8.
        pagina: u8,
        /// Tamanho esperado do frame cru.
        esperado: usize,
        /// Tamanho lido.
        got: usize,
    },

    /// O byte `PG` do header não é o índice da posição (frame fora de ordem).
    #[error("pagina fora de ordem: esperada {esperada}, achada {achada}")]
    PaginaForaDeOrdem {
        /// Índice da posição no array.
        esperada: u8,
        /// Valor do byte `PG` no header.
        achada: u8,
    },

    /// Um byte do corpo não é um nibble (0..=0x0F): o encoding não é
    /// uniforme aqui, e mascarar com `& 0x0F` esconderia a corrupção.
    #[error("pagina {pagina}: byte {got:#04x} fora de nibble no offset {offset}")]
    NibbleInvalido {
        /// Página 0..=8.
        pagina: u8,
        /// Offset do corpo cru.
        offset: usize,
        /// Byte lido.
        got: u8,
    },

    /// O nome não é ASCII imprimível com pad NUL.
    #[error("nome invalido: {detalhe}")]
    NomeInvalido {
        /// Por que o nome foi recusado (offset/byte).
        detalhe: String,
    },

    /// O offset provado não bateu onde devia (prova-negativa falhou).
    #[error("offset nao bate: achado {achado}, esperado {esperado}")]
    OffsetsNaoBatem {
        /// Valor lido (ou tamanho do corpo, quando é faixa fora).
        achado: usize,
        /// Offset/faixa esperada.
        esperado: usize,
    },

    /// O artefato de offsets embutido não é o formato que o código espera.
    #[error("artefato de offsets invalido: {detalhe}")]
    ArtefatoInvalido {
        /// Por que o JSON foi recusado (parse ou encoding).
        detalhe: String,
    },

    /// O efeitoCode lido dos bytes não existe no dicionário — recusa em vez
    /// de montar slot sem identidade (R1).
    #[error("effectCode {code:#010x} fora do dicionario")]
    EfeitoDesconhecido {
        /// O u32 lido do fio.
        code: u32,
    },
}

/// As 9 páginas de um preset, já nibble-decodificadas.
///
/// Não é `Serialize`: o domínio é bytes decodificados, não um DTO. Quem
/// precisa de JSON usa os acessores (`nome`, `slots`).
#[derive(Debug, Clone, PartialEq)]
pub struct Paginas {
    /// Corpos decodificados (8 × 96B + 1 × 14B), na ordem da página.
    corpos: [Vec<u8>; 9],
}

impl Paginas {
    /// O corpo decodificado da página `pagina` (0..=8).
    pub fn corpo(&self, pagina: usize) -> &[u8] {
        &self.corpos[pagina]
    }
}

/// Acha o primeiro byte que não é nibble, para o diagnóstico.
///
/// Só roda no caminho de ERRO (o decode strict já recusou) — e existe porque
/// [`crate::codec::nibble_collapse`] devolve `InvalidShape` sem posição, e um
/// "par 06 a5" sem página/offset não serve para achar o que corrompeu.
fn primeiro_byte_ruim(cru: &[u8]) -> Option<(usize, u8)> {
    cru.iter()
        .enumerate()
        .find(|(_, &b)| b > 0x0F)
        .map(|(i, &b)| (i, b))
}

/// Nibble-decodifica um corpo cru, reusando a primitiva strict do repo.
///
/// # Erros
/// [`DecodeError::NibbleInvalido`] com página e offset preenchidos — que é o
/// que [`crate::codec::nibble_collapse`] não dá (ele devolve `InvalidShape`
/// sem posição, e "par 06 a5" sem página/offset não serve para achar o que
/// corrompeu).
///
/// **Não existe `ParImpar` aqui de propósito.** `decode` só chama esta função
/// depois de validar `raw.len() == HEADER + 192` (ou `+ 28`) — ambos pares —
/// então um corpo ímpar é inalcançável por esta API. Uma variante de erro que
/// nenhum input alcança seria código que mente. Quem recusa comprimento ímpar
/// é o `nibble_collapse` do repo, testado em `tests/codec_wire.rs`.
fn decodifica_corpo(pagina: u8, cru: &[u8]) -> Result<Vec<u8>, DecodeError> {
    nibble_collapse(cru).map_err(|_| match primeiro_byte_ruim(cru) {
        Some((offset, got)) => DecodeError::NibbleInvalido {
            pagina,
            offset,
            got,
        },
        // Só sobra comprimento ímpar (o único outro motivo do repo recusar).
        // Inalcançável por `decode` — ver o doc acima — mas traduz o motivo em
        // vez de engolir o erro num rótulo errado como antes (`NomeInvalido`).
        None => DecodeError::TamanhoInesperado {
            pagina,
            esperado: cru.len().next_multiple_of(2),
            got: cru.len(),
        },
    })
}

/// O caminho único entre 9 frames do fio e o domínio [`Paginas`].
///
/// # Erros
/// [`DecodeError`] — tamanho fora da família ou `PG` fora de ordem na
/// estrutura; byte que não é nibble no corpo. **Nenhuma dessas falhas é
/// "melhor esforço"**: entregar corpo sem decode seria entregar lixo.
pub fn decode(paginas: &[StatePage; 9]) -> Result<Paginas, DecodeError> {
    let mut corpos: [Vec<u8>; 9] = Default::default();
    for (i, pagina) in paginas.iter().enumerate() {
        let idx = i as u8;
        let esperado = HEADER
            + if idx < 8 {
                CORPO_CRU_PG0_7
            } else {
                CORPO_CRU_PG8
            };
        if pagina.raw.len() != esperado {
            return Err(DecodeError::TamanhoInesperado {
                pagina: idx,
                esperado,
                got: pagina.raw.len(),
            });
        }
        let achada = pagina.raw[3];
        if achada != idx {
            return Err(DecodeError::PaginaForaDeOrdem {
                esperada: idx,
                achada,
            });
        }
        corpos[i] = decodifica_corpo(idx, &pagina.raw[HEADER..])?;
    }
    Ok(Paginas { corpos })
}

impl Paginas {
    /// O nome do preset: pg0, offset [`NOME_OFFSET`], [`NOME_LEN`] bytes.
    ///
    /// **A regra do pad é a prova.** Depois do primeiro NUL só pode vir NUL —
    /// aceitar `"AB\0CD"` seria servir um nome truncado para a biblioteca e
    /// para o DAW, e a falha seria invisível. Os [`NOME_LEN`] bytes têm de ser
    /// ASCII imprimível (0x20..=0x7E) antes do NUL, e o nome não pode ser
    /// vazio. O espaço tem 12 bytes porque a cadeia de slots ocupa 14..32 —
    /// aceitar 16 seria ler os slots como se fossem nome.
    ///
    /// # Erros
    /// [`DecodeError::NomeInvalido`] se o espaço do nome violar a regra.
    pub fn nome(&self) -> Result<&str, DecodeError> {
        let pg0 = &self.corpos[0];
        let espaco = pg0
            .get(NOME_OFFSET..NOME_OFFSET + NOME_LEN)
            .ok_or_else(|| DecodeError::NomeInvalido {
                detalhe: format!(
                    "pg0 com {}B, nome precisaria de {}",
                    pg0.len(),
                    NOME_OFFSET + NOME_LEN
                ),
            })?;

        let fim = espaco.iter().position(|&b| b == 0).unwrap_or(NOME_LEN);
        if fim == 0 {
            return Err(DecodeError::NomeInvalido {
                detalhe: "nome vazio".into(),
            });
        }
        for (i, &b) in espaco.iter().enumerate() {
            if i >= fim {
                if b != 0 {
                    return Err(DecodeError::NomeInvalido {
                        detalhe: format!(
                            "byte {b:#04x} depois do NUL no offset {}",
                            NOME_OFFSET + i
                        ),
                    });
                }
            } else if !(0x20..=0x7E).contains(&b) {
                return Err(DecodeError::NomeInvalido {
                    detalhe: format!("byte {b:#04x} nao imprimivel no offset {}", NOME_OFFSET + i),
                });
            }
        }
        std::str::from_utf8(&espaco[..fim]).map_err(|e| DecodeError::NomeInvalido {
            detalhe: e.to_string(),
        })
    }
}

// ---------------------------------------------------------------------------
// Fase 2 — o BoardView, a partir do artefato de layout provado pela análise.
//
// O artefato NAO e um mapa de offsets por (slot, param): isso foi tentado e
// nao funciona, porque os dados do fio sao indexados pela POSICAO na cadeia
// (ha 11 cadeias distintas nas 198 pps) e os params formam um fluxo contiguo
// que atravessa fronteiras de pagina. O artefato guarda a ESTRUTURA, e o
// offset de cada valor e CALCULADO — nunca procurado. Ver
// `analysis/map_state_pages.py` para a prova (26552/26554 em params,
// 1782/1782 em code e state).
// ---------------------------------------------------------------------------

/// O artefato embutido no binário — mesmo regime de `model::DICTIONARY_JSON`.
pub const OFFSETS_JSON: &str = include_str!("../../../analysis/state_pages_offsets.json");

/// Bloco de tamanho fixo: `count` elementos de `width` bytes, a partir de
/// `offset` na `page` (corpo já decodificado).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Bloco {
    /// Página 0..=8.
    pub page: u8,
    /// Offset no corpo decodificado.
    pub offset: usize,
    /// Quantos elementos o bloco tem.
    pub count: usize,
    /// Largura de cada elemento em bytes.
    pub width: u8,
}

/// O espaço do nome, em bytes.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct EspacoNome {
    /// Página (sempre 0).
    pub page: u8,
    /// Offset inicial.
    pub offset: usize,
    /// Tamanho em bytes.
    pub length: usize,
}

/// Um trecho do fluxo de params: `count` f32 a partir de `offset`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Faixa {
    /// Página.
    pub page: u8,
    /// Offset inicial.
    pub offset: usize,
    /// Quantos f32 este trecho cobre.
    pub count: usize,
}

/// O fluxo contíguo de params (135 f32 = 9 slots × 15).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Params {
    /// Total de f32 no fluxo.
    pub count: usize,
    /// Params por slot (15).
    pub per_slot: usize,
    /// Largura (4).
    pub width: u8,
    /// É float32 LE?
    pub float: bool,
    /// Onde o fluxo mora, página a página (gerado do próprio cálculo).
    pub span: Vec<Faixa>,
}

/// Os campos do layout que `slots()` consome.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Layout {
    /// Espaço do nome na pg0.
    pub nome: EspacoNome,
    /// Cadeia (posição → `x` do XML) — usada pela análise, não por `slots()`.
    pub cadeia: Bloco,
    /// `effectCode` por posição, na pg0.
    #[serde(rename = "effectCode")]
    pub effect_code: Bloco,
    /// `effectState` por posição, na pg6.
    #[serde(rename = "effectState")]
    pub effect_state: Bloco,
    /// Fluxo contíguo dos valores de knob.
    pub params: Params,
}

/// O artefato completo, na forma embutida.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Offsets {
    /// Só `"nibble"`.
    pub encoding: String,
    /// Só o modelo determinístico — a votação foi o método fraco.
    pub method: String,
    /// O layout.
    pub layout: Layout,
}

/// O método que a análise tem de ter usado (o gate também confere).
const METODO_ESPERADO: &str = "modelo-deterministico (offset calculado pela estrutura)";

impl Offsets {
    /// O artefato embutido, validado. Produção e teste leem a MESMA fonte —
    /// não existe caminho "só de teste".
    ///
    /// # Erros
    /// [`DecodeError::ArtefatoInvalido`] se o JSON não parsear, se o encoding
    /// não for `nibble` ou se o método não for o modelo determinístico (ou
    /// seja, se alguém regenerou o artefato com a votação fraca).
    pub fn carregado() -> Result<Self, DecodeError> {
        let o: Offsets =
            serde_json::from_str(OFFSETS_JSON).map_err(|e| DecodeError::ArtefatoInvalido {
                detalhe: e.to_string(),
            })?;
        if o.encoding != "nibble" {
            return Err(DecodeError::ArtefatoInvalido {
                detalhe: format!("encoding={}", o.encoding),
            });
        }
        if o.method != METODO_ESPERADO {
            return Err(DecodeError::ArtefatoInvalido {
                detalhe: format!("method={}", o.method),
            });
        }
        Ok(o)
    }
}

/// Lê `n` bytes LE em `corpo`, com o erro apontando a faixa que estourou.
fn le(corpo: &[u8], offset: usize, n: usize) -> Result<&[u8], DecodeError> {
    corpo
        .get(offset..offset + n)
        .ok_or(DecodeError::OffsetsNaoBatem {
            achado: corpo.len(),
            esperado: offset + n,
        })
}

fn le_u16(corpo: &[u8], offset: usize) -> Result<u16, DecodeError> {
    let b = le(corpo, offset, 2)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

fn le_u32(corpo: &[u8], offset: usize) -> Result<u32, DecodeError> {
    let b = le(corpo, offset, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn le_f32(corpo: &[u8], offset: usize) -> Result<f32, DecodeError> {
    let b = le(corpo, offset, 4)?;
    Ok(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Índice global do fluxo → `(página, offset)`.
///
/// O fluxo é contíguo e atravessa fronteiras de página; o span vem do
/// artefato (gerado pelo próprio cálculo na análise, nunca digitado à mão).
fn localiza(span: &[Faixa], idx: usize) -> Option<(u8, usize)> {
    let mut base = 0usize;
    for f in span {
        if idx < base + f.count {
            return Some((f.page, f.offset + 4 * (idx - base)));
        }
        base += f.count;
    }
    None
}

/// Formata o f32 do fio como o `.prst` representa o valor.
///
/// Os valores do fio são inteiros armazenados em float (`50.0`, `65535.0`) e
/// o XML guarda o inteiro (`"50"`). Sem isto a comparação de string falharia
/// por forma e não por conteúdo.
fn formata_valor(v: f32) -> String {
    if v.is_finite() && v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

impl Paginas {
    /// Os 9 slots do preset, reconstruídos do layout provado.
    ///
    /// Identidade visual (família, arquétipo, nome, variante) e knobs vêm do
    /// **dicionário** (R1); as páginas trazem `effectCode`, `effectState` e os
    /// valores atuais. É o mesmo caminho de `pedalboard::board_view_for`, só
    /// que a fonte é o fio e não o arquivo.
    ///
    /// `pp_type`/`pp_type_name` ficam zerados de propósito: o `ppType` casa em
    /// só 166/198 (spec §11.2), abaixo de prova, então não tem offset provado.
    /// Ícone errado na biblioteca vale mais que um chute com cara de prova —
    /// e a limitação está declarada, não escondida.
    ///
    /// # Erros
    /// [`DecodeError::NomeInvalido`], [`DecodeError::OffsetsNaoBatem`] ou
    /// [`DecodeError::EfeitoDesconhecido`]. **Nunca** devolve slot com `code`
    /// inventado: `BoardView` errado é pior que erro, porque o palco desenha
    /// e o usuário acredita.
    pub fn slots(
        &self,
        pp: u16,
        dict: &Dictionary,
        off: &Offsets,
    ) -> Result<BoardView, DecodeError> {
        let nome = self.nome()?.to_string();
        let lay = &off.layout;
        let pg0 = &self.corpos[usize::from(lay.cadeia.page)];
        let pg6 = &self.corpos[usize::from(lay.effect_state.page)];
        let mut slots = Vec::with_capacity(9);

        for pos in 0..lay.effect_code.count {
            // A cadeia diz qual `x` do arquivo ocupa cada POSICAO do fio.
            // Ela nao e cosmética: 40 das 198 pps tem cadeia trocada
            // (`(1,0,2,...)` e `(0,1,...,7,6,8)`), e sem ela o `slot` sairia
            // errado — o teste de paridade pegou exatamente isso.
            let x = le_u16(pg0, lay.cadeia.offset + pos * usize::from(lay.cadeia.width))?;
            // O effectCode esta POR POSICAO na pg0, nao por `x`.
            let code = le_u32(
                pg0,
                lay.effect_code.offset + pos * usize::from(lay.effect_code.width),
            )?;
            let state = le_u16(
                pg6,
                lay.effect_state.offset + pos * usize::from(lay.effect_state.width),
            )?;
            let state = state != 0;

            // A identidade vem do DICIONÁRIO (R1): o número do fio só vale
            // se ele nomeia um efeito conhecido. Nada de slot sem identidade.
            let nibble = ((code >> 24) & 0xFF) as u8;
            let index = code & 0x00FF_FFFF;
            let Some(algo) = dict.algorithm(nibble, index) else {
                return Err(DecodeError::EfeitoDesconhecido { code });
            };
            let family = family_of(&algo.module);

            // O VALOR e indexado pela POSICAO (o fluxo e contiguo na ordem do
            // sinal), nao pelo `x` — por isso `pos` e nao `x` aqui.
            let knobs = algo
                .controls
                .iter()
                .map(|c| {
                    let idx = pos * lay.params.per_slot + usize::from(c.pos);
                    let (page, offset) =
                        localiza(&lay.params.span, idx).ok_or(DecodeError::OffsetsNaoBatem {
                            achado: idx,
                            esperado: lay.params.count,
                        })?;
                    let corpo =
                        self.corpos
                            .get(usize::from(page))
                            .ok_or(DecodeError::OffsetsNaoBatem {
                                achado: usize::from(page),
                                esperado: 9,
                            })?;
                    let valor = formata_valor(le_f32(corpo, offset)?);
                    Ok::<KnobSpec, DecodeError>(KnobSpec {
                        name: c.name.clone(),
                        pos: c.pos,
                        kind: match c.kind {
                            ControlKind::Knob => "knob",
                            ControlKind::Switch => "switch",
                            ControlKind::Combox => "combox",
                        }
                        .to_string(),
                        range: c.range(),
                        options: c.options.clone(),
                        value: Some(valor),
                        default: c.default.clone(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            slots.push(SlotSpec {
                slot: x as u8,
                family,
                archetype: archetype_of(family),
                name: algo.name.trim().to_string(),
                variant: slug(&algo.name),
                state,
                code,
                knobs,
            });
        }

        // Mesmo contrato de `board_view_for`: ordenado por `slot` (x).
        slots.sort_by_key(|s| s.slot);

        Ok(BoardView {
            pp,
            name: nome,
            pp_type: 0,
            pp_type_name: String::new(),
            slots,
        })
    }
}
