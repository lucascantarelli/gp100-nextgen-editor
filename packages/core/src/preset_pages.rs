//! Decode das páginas 13xx (família `13 01 00 03`) — o palco vivo (#155).
//!
//! **O que estava errado com toda a análise anterior.** A página vem
//! **nibble-expandida**: cada byte do fio guarda um nibble e o par vira um
//! byte real (`04 09` → `0x49` = `I`). A varredura que a #155 pedia — offsets
//! crus 0..28 × comprimentos 6/10/12 — procura o nome em bytes que nunca
//! foram o nome. Decodificado, o nome está na **pg0, offset 2, em 198/198**
//! (`00 00 | nome[16, pad NUL] | …`).
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
