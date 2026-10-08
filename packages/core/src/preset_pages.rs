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

use crate::codec::nibble_collapse;
use crate::session::StatePage;

/// Tamanho do corpo ANTES do decode (o header de 4B já foi removido).
const CORPO_CRU_PG0_7: usize = 192;
/// Tamanho do corpo da pg8 antes do decode (4B header + 28B = 32B no fio).
const CORPO_CRU_PG8: usize = 28;
/// Tamanho do header: `[pp u16BE][00][PG]`.
const HEADER: usize = 4;

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

    /// Corpo com número ímpar de bytes — não fecha em pares.
    #[error("pagina {pagina}: corpo com tamanho ímpar")]
    ParImpar {
        /// Página 0..=8.
        pagina: u8,
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
/// [`DecodeError::ParImpar`] ou [`DecodeError::NibbleInvalido`] — com a
/// página e o offset preenchidos, que é o que a primitiva não dá.
fn decodifica_corpo(pagina: u8, cru: &[u8]) -> Result<Vec<u8>, DecodeError> {
    if !cru.len().is_multiple_of(2) {
        return Err(DecodeError::ParImpar { pagina });
    }
    nibble_collapse(cru).map_err(|e| match primeiro_byte_ruim(cru) {
        Some((offset, got)) => DecodeError::NibbleInvalido {
            pagina,
            offset,
            got,
        },
        // Só pode ser a forma do erro mudar (o comprimento já foi checado):
        // nesse caso o erro do repo é mais honesto do que inventar aqui.
        None => DecodeError::NomeInvalido {
            detalhe: e.to_string(),
        },
    })
}

/// O caminho único entre 9 frames do fio e o domínio [`Paginas`].
///
/// # Erros
/// [`DecodeError`] — tamanho fora da família, `PG` fora de ordem, corpo que
/// não fecha em pares ou byte que não é nibble. **Nenhuma dessas falhas é
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

// `Paginas::nome()` entra na Task 2, com o teste que primeiro falha — o
// TDD de cada task exige ver o teste cair antes de existir o método.
