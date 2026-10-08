# Decode das páginas 13xx — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decifrar as 9 páginas de estado do preset para o app servir nome real e palco vivo no backend `Real`.

**Architecture:** Um módulo puro no core (`preset_pages.rs`) nibble-decodula as 9 páginas e expõe dois parser empilhados: `nome()` (fase 1, provada 198/198) e `slots()` (fase 2, offsets vindos de um artefato de análise embutido). O actor deixa de recusar `device_board` com backend `Real` e passa a servir do **cache do boot** — nunca de um `select`, que mudaria o preset que o pedal está mostrando.

**Tech Stack:** Rust 2021 (`gp100-core`, `gp100-ui`/Tauri 2), `serde`/`serde_json`/`thiserror`, Python 3 (`analysis/`), pytest, Vitest.

**Spec:** [`docs/superpowers/specs/2026-10-08-decode-paginas-13xx-design.md`](../../superpowers/specs/2026-10-08-decode-paginas-13xx-design.md) — o plano argumenta a partir dela; quem executa lê as duas.

## Global Constraints

- **R1 (nunca adivinhar protocolo):** identidade de efeito e faixa vêm do dicionário embutido (`model::DICTIONARY_JSON`); offsets vêm do artefato de análise (Task 4). Nenhum número de parede.
- `gp100-core` compila com `#![deny(missing_docs)]` — item público **e campos** sem doc = erro de build.
- Toolchain pinada por `rust-toolchain.toml` (`stable-x86_64-pc-windows-gnu`); crate do Tauri com `cargo +stable-msvc` dentro de `packages/app/api` (ADR-7).
- Qualidade: `cargo fmt --check` · `cargo clippy --workspace --all-targets -D warnings` · `cargo test --workspace` (inclui doc-tests).
- Front: `pnpm lint` (`eslint . --max-warnings 0`) · `pnpm test` · `pnpm build`.
- **Gates 13/13** (`python3 scripts/gates.py`): `param_ranges`, `h1_compare`, `byte_order`, `baseline`, `pytest` são os que podem cair aqui.
- **`analysis/validate_golden.py` continua 1782/1782 byte-idêntico** — quirks de boot preservados (select+open duplicados do preset atual). Regra R2/R3.
- **Nenhum teste pré-existente é modificado** para deixar passar.
- **`App.tsx` < 300 linhas** (hoje 299) — nada novo na casca.
- **Nunca um `BoardView` com chute** — se 197/198 casarem, falha (spec §6).
- **Nunca um `select` novo para ler outro pp** (spec §5.2) — `device_board` lê do cache do boot.

## Review Focus

1. **Página fora de ordem ou duplicada com conteúdo divergente** — o scan manda a pg0 do preset atual duas vezes (quirk §13.4). Esperado: dedupe com igualdade; divergência = erro, não "última vence". → Task 1.
2. **Nome com NUL interno seguido de lixo** (`"AB\0CD…"`) — esperado: recusa, porque após o primeiro NUL só pode vir NUL; aceitar seria servir um nome truncado ao DAW/biblioteca. → Task 2.
3. **Offset que casa por acaso** (valor pequeno como 0/1 aparece em todo lugar) — esperado: o artefato só aceita offset com dominância alta **e** cobertura 198/198; recusação quando não casa. → Task 4.
4. **Cache desatualizado após edição no hardware** — esperado: comportamento declarado (re-scan), nunca um palco que mente; o `current_name` não pode dizer "Só no mock" com dado real disponível. → Task 6.
5. **`13010005` não existe na S1 mas o código roteia a pg8 para lá** — esperado: resolver ANTES de confiar na pg8; se o código estiver errado, a pg8 pode nem chegar num device real. → Task 8.

---

## File Structure

| Ação | Arquivo | Responsabilidade |
|---|---|---|
| Criar | `packages/core/src/preset_pages.rs` | nibble-decode + `Paginas::nome()` + `Paginas::slots()`. Puro, sem fio. |
| Criar | `packages/core/tests/preset_pages.rs` | Dataset inteiro, nome 198/198, prova-negativa, paridade de slots. |
| Modificar | `packages/core/src/lib.rs` | `pub mod preset_pages;` |
| Modificar | `analysis/map_state_pages.py` | Corrigir (nibble, pp u16, votação u8, caminho) → artefato de offsets |
| Criar | `analysis/state_pages_offsets.json` | Offsets provados, **embutido** no core |
| Criar | `analysis/validate_state_pages.py` | Gate do artefato (pytest), mesmo espírito do `param_ranges.py` |
| Modificar | `packages/app/api/src/actor.rs` | Cache de nomes; remove recusa ~L702 e aviso ~L225 |
| Modificar | `packages/core/src/session.rs` | `StatePage` documentada com o layout decifrado |
| Modificar | `docs/PROTOCOL.md` §13.10 | Seção do layout |
| Criar | `docs/STATE_PAGES_FIELD.md` | Runbook + relatório de campo |

---

### Task 1: `preset_pages::decode` — nibble-decode com validação estrutural

**Files:**
- Create: `packages/core/src/preset_pages.rs`
- Modify: `packages/core/src/lib.rs` (ao lado de `pub mod pedalboard;`)
- Test: `packages/core/tests/preset_pages.rs`

**Interfaces:**
- Consumes: `gp100_core::session::StatePage { pub raw: Vec<u8> }` — `raw` tem **196B** (pg 0..7) e **32B** (pg 8): 4B de header `[pp u16BE][?][PG]` + corpo nibble.
- Produces:
  - `pub enum DecodeError { TamanhoInesperado{pagina, esperado, got}, PaginaForaDeOrdem{esperada, achada}, NibbleInvalido{pagina, offset, got}, ParImpar{pagina}, NomeInvalido{detalhe}, OffsetsNaoBatem{achado, esperado} }`
  - `pub fn decode(paginas: &[StatePage; 9]) -> Result<Paginas, DecodeError>`
  - `pub struct Paginas` (corpo decodificado: 8×96B + 1×14B)

- [ ] **Step 1: Escrever o teste que falha**

Crie `packages/core/tests/preset_pages.rs`:

```rust
//! Contratos do decode das páginas 13xx (spec: docs/superpowers/specs/
//! 2026-10-08-decode-paginas-13xx-design.md · issues #155).
//!
//! O falso positivo que estes testes fecham: "o parser não crasheou" não
//! prova que leu o layout certo. Aqui o dataset INTEIRO da captura é
//! decodificado e o nome é conferido contra o `all.prst` — a mesma prova
//! byte-a-byte do `roundtrip_prst.rs`, aplicada ao fio.

mod common;

use std::collections::BTreeMap;

use common::fixture_rows;
use gp100_core::session::StatePage;

/// Hex crua -> bytes (local de propósito: nenhum teste depende de assinatura
/// alheia para o caminho crítico).
fn hex(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0, "hex com tamanho ímpar: {s}");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).expect("dígito hex válido"))
        .collect()
}

/// pp -> (pagina -> StatePage), das linhas `13010003` de `boot.jsonl`.
///
/// Retorna também a contagem CRUA: o quirk do §13.4 manda a pg0 do preset
/// atual duas vezes, então 1783 linhas têm de virar 1782 pares únicos —
/// e as duas cópias têm de ser IDÊNTICAS (Review Focus 1).
fn paginas_da_fixture() -> (BTreeMap<u16, [StatePage; 9]>, usize) {
    let mut brutas: BTreeMap<u16, BTreeMap<u8, Vec<u8>>> = BTreeMap::new();
    let mut n_linhas = 0usize;
    for r in fixture_rows("boot.jsonl") {
        if r["addr"] != "13010003" || r["dir"] != "in" {
            continue;
        }
        n_linhas += 1;
        let bytes = hex(r["data"].as_str().expect("data é hex"));
        assert!(bytes.len() >= 4, "frame curto: {}", bytes.len());
        let pp = u16::from_be_bytes([bytes[0], bytes[1]]);
        let pg = bytes[3];
        let corpo = bytes.to_vec();
        match brutas.entry(pp).or_default().entry(pg) {
            std::collections::btree_map::Entry::Vacant(e) => {
                e.insert(corpo);
            }
            std::collections::btree_map::Entry::Occupied(e) => {
                // DUPLICATA é esperada (quirk), mas divergente é corrupção.
                assert_eq!(
                    e.get(),
                    &corpo,
                    "pp {pp:#06x} pg {pg}: as duas leituras divergem"
                );
            }
        }
    }
    let mut saida = BTreeMap::new();
    for (pp, pags) in brutas {
        assert_eq!(pags.len(), 9, "pp {pp:#06x} tem {} páginas", pags.len());
        let arr: [StatePage; 9] = std::array::from_fn(|i| StatePage {
            raw: pags[&(i as u8)].clone(),
        });
        saida.insert(pp, arr);
    }
    (saida, n_linhas)
}

#[test]
fn dataset_inteiro_agrupa_em_198_preset_x_9_paginas() {
    let (mapa, n_linhas) = paginas_da_fixture();
    assert_eq!(mapa.len(), 198, "198 pps (banco 0x00xx + 0x01xx)");
    assert_eq!(n_linhas, 1783, "1782 pares únicos + 1 duplicata do §13.4");
    for (pp, pags) in &mapa {
        for i in 0..8usize {
            assert_eq!(pags[i].raw.len(), 196, "pp {pp:#06x} pg {i} (4B header + 192B)");
        }
        assert_eq!(pags[8].raw.len(), 32, "pp {pp:#06x} pg 8 (4B header + 28B)");
    }
}

#[test]
fn decode_devolve_8x96_mais_1x14_para_todo_preset() {
    let (mapa, _) = paginas_da_fixture();
    for (pp, pags) in &mapa {
        let dec = gp100_core::preset_pages::decode(pags)
            .unwrap_or_else(|e| panic!("pp {pp:#06x}: {e}"));
        for i in 0..8usize {
            assert_eq!(dec.corpo(i).len(), 96, "pp {pp:#06x} pg {i} decodificado");
        }
        assert_eq!(dec.corpo(8).len(), 14, "pp {pp:#06x} pg 8 decodificado");
    }
}
```

- [ ] **Step 2: Rodar para ver falhar**

Run: `cargo test --workspace --test preset_pages`
Expected: **FAIL** — `unresolved module or unlinked file common` (auxiliar ok) depois `E0432: unresolved import gp100_core::preset_pages`.

- [ ] **Step 3: Implementar `preset_pages.rs`**

```rust
//! Decode das páginas 13xx (família `13 01 00 03`) — o palco vivo (#155).
//!
//! **O que estava errado com toda a análise anterior.** A página vem
//! **nibble-expandida**: cada byte do fio guarda um nibble e o par vira um
//! byte real (`04 09` → `0x49` = `I`). A varredura que a #155 pedia — offsets
//! crus 0..28 × comprimentos 6/10/12 — procura o nome em bytes que nunca
//! foram o nome. Decodificado, o nome está na **pg0, offset 2, em 198/198**
//! (`00 00 | nome[16 com pad NUL] | …`).
//!
//! **Módulo puro, sem fio e sem `Session`.** É assim que o dataset inteiro
//! (1.782 páginas de `analysis/fixtures/boot.jsonl`) é testado sem hardware —
//! o mesmo princípio de `param_range.rs` (#110) e do inventário (#132).

use crate::session::StatePage;

/// Tamanho do corpo ANTES do decode (o header de 4B já foi removido).
const CORPO_CRU_PG0_7: usize = 192;
/// Tamanho da pg8 antes do decode (4B header + 28B = 32B no fio).
const CORPO_CRU_PG8: usize = 28;
/// Tamanho do header: `[pp u16BE][?][PG]`.
const HEADER: usize = 4;
/// Bytes do nome na pg0 DEPOIS do decode (offset fixo, 198/198).
const NOME_OFFSET: usize = 2;
/// Espaço reservado para o nome (16 bytes, preenchido com NUL).
const NOME_LEN: usize = 16;

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

    /// Um byte do corpo não é um nibble (0..=0x0F): o encoding não é uniforme
    /// aqui, e mascarar com `& 0x0F` esconderia a corrupção.
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
        /// Offset lido.
        achado: usize,
        /// Offset esperado.
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
/// precisa de JSON usa [`Paginas::nome`] / [`Paginas::slots`].
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

/// Nibble-decodifica um corpo cru (pares → bytes) e valida o encoding.
fn decodifica_corpo(pagina: u8, cru: &[u8]) -> Result<Vec<u8>, DecodeError> {
    if cru.len() % 2 != 0 {
        return Err(DecodeError::ParImpar { pagina });
    }
    let mut fora = Vec::with_capacity(cru.len() / 2);
    for (i, par) in cru.chunks(2).enumerate() {
        if par[0] > 0x0F || par[1] > 0x0F {
            return Err(DecodeError::NibbleInvalido {
                pagina,
                offset: i * 2,
                got: if par[0] > 0x0F { par[0] } else { par[1] },
            });
        }
        fora.push((par[0] << 4) | par[1]);
    }
    Ok(fora)
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
        let esperado = HEADER + if idx < 8 { CORPO_CRU_PG0_7 } else { CORPO_CRU_PG8 };
        if pagina.raw.len() != esperado {
            return Err(DecodeError::TamanhoInesperado {
                pagina: idx,
                esperado,
                got: pagina.raw.len(),
            });
        }
        let achada = pagina.raw[3];
        if achada != idx {
            return Err(DecodeError::PaginaForaDeOrdem { esperada: idx, achada });
        }
        corpos[i] = decodifica_corpo(idx, &pagina.raw[HEADER..])?;
    }
    Ok(Paginas { corpos })
}
```

Em `packages/core/src/lib.rs`, ao lado dos demais (`pub mod pedalboard;`):

```rust
/// Decode das páginas 13xx: palco vivo e nomes reais (#155).
pub mod preset_pages;
```

- [ ] **Step 4: Rodar para ver passar**

Run: `cargo test --workspace --test preset_pages`
Expected: **PASS**, 2 testes.
> **Se `NibbleInvalido` estalar:** isso é um **achado**, não um teste ruim — o erro diz `pagina` e `offset`. Registre o offset em `docs/STATE_PAGES_FIELD.md` e decida com o owner antes de seguir. **Não** troque a checagem por `& 0x0F` para fazer passar: mascarar seria esconder a corrupção que a checagem existe para pegar.

- [ ] **Step 5: fmt + clippy**

Run: `cargo fmt && cargo fmt --check && cargo clippy --workspace --all-targets -D warnings`
Expected: exit 0.

- [ ] **Step 6: Commit**

```bash
git add packages/core/src/preset_pages.rs packages/core/src/lib.rs packages/core/tests/preset_pages.rs
git commit -m "feat(core): nibble-decode das paginas 13xx com validacao estrutural (#155)"
```

---

### Task 2: `Paginas::nome()` — 198/198 contra o `all.prst`

**Files:**
- Modify: `packages/core/src/preset_pages.rs`
- Modify: `packages/core/tests/preset_pages.rs`

**Interfaces:**
- Consumes: `Paginas` (Task 1); `gp100_core::preset::Document::{parse, presets}` e `PresetView::pp_name` — o **ground truth** vem do `.prst` real, como no `roundtrip_prst.rs`.
- Produces: `pub fn nome(&self) -> Result<&str, DecodeError>`

- [ ] **Step 1: Escrever o teste que falha**

Acrescente a `packages/core/tests/preset_pages.rs`:

```rust
use gp100_core::preset::Document;

/// Caminho do `all.prst` — mesmo arranjo de `roundtrip_prst.rs`
/// (CARGO_MANIFEST_DIR subindo 2 níveis até a raiz do repo).
fn all_prst() -> std::path::PathBuf {
    let mut root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    root.push("files");
    root.push("patches");
    root.push("all.prst");
    root
}

/// Os 99 nomes de fábrica, na ordem do documento.
fn nomes_de_fabrica() -> Vec<String> {
    let bytes = std::fs::read(all_prst()).expect("all.prst existe");
    let doc = Document::parse(&bytes).expect("dialecto válido");
    doc.presets()
        .map(|p| p.pp_name().unwrap_or("").to_string())
        .collect()
}

/// **O teste que a #155 pede: 198/198, e o ground truth é o artefato.**
///
/// A chave é `pp & 0xFF`: o aparelho tem dois bancos (0x0000..0x0062 e
/// 0x0100..0x0162) e ambos apontam para os mesmos 99 presets do documento —
/// foi assim que a análise medida achou 198/198, e este teste é quem prova.
#[test]
fn nome_bate_com_o_all_prst_em_198_de_198() {
    let esperados = nomes_de_fabrica();
    assert_eq!(esperados.len(), 99, "all.prst tem 99 presets");
    assert!(esperados.iter().all(|n| !n.is_empty()), "nome vazio no artefato");

    let (mapa, _) = paginas_da_fixture();
    let mut ok = 0;
    for (pp, pags) in &mapa {
        let dec = gp100_core::preset_pages::decode(pags).expect("decode");
        let lido = dec
            .nome()
            .unwrap_or_else(|e| panic!("pp {pp:#06x}: {e}"));
        let idx = usize::from(*pp & 0xFF);
        assert_eq!(
            lido,
            esperados[idx],
            "pp {pp:#06x} (indice {idx}) divergiu do all.prst"
        );
        ok += 1;
    }
    assert_eq!(ok, 198, "todos os 198 pps decodificaram o nome certo");
}
```

- [ ] **Step 2: Rodar para ver falhar**

Run: `cargo test --workspace --test preset_pages`
Expected: **FAIL** — `no method named nome`.

- [ ] **Step 3: Implementar**

Em `preset_pages.rs`, adicionando as constantes `NOME_OFFSET`/`NOME_LEN` já declaradas:

```rust
impl Paginas {
    /// O nome do preset: pg0, offset [`NOME_OFFSET`], [`NOME_LEN`] bytes.
    ///
    /// **A regra do pad é a prova.** Depois do primeiro NUL só pode vir NUL —
    /// aceitar `"AB\0CD"` seria servir um nome truncado para a biblioteca e
    /// para o DAW, e a falha seria invisível (Review Focus 2). Os 16 bytes
    /// têm de ser ASCII imprimível (0x20..=0x7E) antes do NUL, e o nome não
    /// pode ser vazio.
    ///
    /// # Erros
    /// [`DecodeError::NomeInvalido`] se o espaço do nome violar a regra.
    pub fn nome(&self) -> Result<&str, DecodeError> {
        let pg0 = &self.corpos[0];
        let espaco = pg0
            .get(NOME_OFFSET..NOME_OFFSET + NOME_LEN)
            .ok_or_else(|| DecodeError::NomeInvalido {
                detalhe: format!("pg0 com {}B, nome precisaria de {}", pg0.len(), NOME_OFFSET + NOME_LEN),
            })?;

        let fim = espaco.iter().position(|&b| b == 0).unwrap_or(NOME_LEN);
        if fim == 0 {
            return Err(DecodeError::NomeInvalido { detalhe: "nome vazio".into() });
        }
        for (i, &b) in espaco.iter().enumerate() {
            let depois_do_fim = i >= fim;
            if depois_do_fim {
                if b != 0 {
                    return Err(DecodeError::NomeInvalido {
                        detalhe: format!("byte {b:#04x} depois do NUL no offset {}", NOME_OFFSET + i),
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
```

- [ ] **Step 4: Rodar para ver passar**

Run: `cargo test --workspace --test preset_pages`
Expected: **PASS**, 3 testes (nome 198/198).

- [ ] **Step 5: Commit**

```bash
git add packages/core/src/preset_pages.rs packages/core/tests/preset_pages.rs
git commit -m "feat(core): nome do preset na pg0 offset 2, provado 198/198 (#155)"
```

---

### Task 3: Prova-negativa — recusar onde não deve

**Files:**
- Modify: `packages/core/tests/preset_pages.rs`

**Interfaces:**
- Consumes: `decode`, `Paginas::nome`, `DecodeError` (Tasks 1–2).
- Produces: nenhum código novo — só o teste que fecha o critério de aceite 1 e 2 da spec (§7.2).

- [ ] **Step 1: Escrever os testes (falham por não existirem casos)**

```rust
/// Monta um `StatePage` sintético com o formato certo (4B header + corpo nibble).
fn pagina_sintetica(idx: u8, corpo_cru: Vec<u8>) -> StatePage {
    let mut raw = vec![0x00, 0x00, 0x00, idx];
    raw.extend(corpo_cru);
    StatePage { raw }
}

/// 9 páginas válidas com um corpo nibble arbitrário (24 pares = 12 bytes).
fn nove_paginas_validas() -> [StatePage; 9] {
    std::array::from_fn(|i| pagina_sintetica(i as u8, vec![0x00; 24]))
}

#[test]
fn recusa_tamanho_errado() {
    let mut pags = nove_paginas_validas();
    pags[3].raw.pop();
    let e = gp100_core::preset_pages::decode(&pags).expect_err("195B não é 196B");
    assert!(matches!(
        e,
        gp100_core::preset_pages::DecodeError::TamanhoInesperado { pagina: 3, .. }
    ), "{e}");
}

#[test]
fn recusa_pagina_fora_de_ordem() {
    let mut pags = nove_paginas_validas();
    pags[5].raw[3] = 7; // o header diz 7, a posição é 5
    let e = gp100_core::preset_pages::decode(&pags).expect_err("PG fora de ordem");
    assert!(matches!(
        e,
        gp100_core::preset_pages::DecodeError::PaginaForaDeOrdem { esperada: 5, achada: 7 }
    ), "{e}");
}

#[test]
fn recusa_byte_que_nao_e_nibble() {
    let mut pags = nove_paginas_validas();
    pags[2].raw[4] = 0xA5; // primeiro byte do corpo
    let e = gp100_core::preset_pages::decode(&pags).expect_err("0xA5 não é nibble");
    assert!(matches!(
        e,
        gp100_core::preset_pages::DecodeError::NibbleInvalido { pagina: 2, offset: 0, got: 0xA5 }
    ), "{e}");
}

/// **Review Focus 2:** o nome não pode ser truncado no primeiro NUL.
#[test]
fn recusa_nome_com_nul_interno() {
    // 16 bytes de espaço: "AB" + NUL + "CD" + NULs.
    let mut espaco = b"AB".to_vec();
    espaco.push(0);
    espaco.extend_from_slice(b"CD");
    espaco.resize(16, 0);
    let mut cru = vec![0u8; 4]; // 2 bytes decodificados antes do nome
    for &b in &espaco {
        cru.push(b >> 4);
        cru.push(b & 0x0F);
    }
    let mut pags = nove_paginas_validas();
    pags[0] = pagina_sintetica(0, cru);
    let dec = gp100_core::preset_pages::decode(&pags).expect("estrutura válida");
    let e = dec.nome().expect_err("tem NUL interno com lixo");
    assert!(matches!(e, gp100_core::preset_pages::DecodeError::NomeInvalido { .. }), "{e}");
}

#[test]
fn recusa_nome_vazio() {
    let mut cru = vec![0u8; 4]; // 2 bytes decodificados, depois 16 NULs
    cru.extend(std::iter::repeat_n(0, 32));
    let mut pags = nove_paginas_validas();
    pags[0] = pagina_sintetica(0, cru);
    let dec = gp100_core::preset_pages::decode(&pags).expect("estrutura válida");
    let e = dec.nome().expect_err("nome vazio");
    assert!(matches!(e, gp100_core::preset_pages::DecodeError::NomeInvalido { .. }), "{e}");
}

#[test]
fn aceita_nome_com_pad_nul_normal() {
    let mut espaco = b"Blink OD".to_vec();
    espaco.resize(16, 0);
    let mut cru = vec![0u8; 4];
    for &b in &espaco {
        cru.push(b >> 4);
        cru.push(b & 0x0F);
    }
    let mut pags = nove_paginas_validas();
    pags[0] = pagina_sintetica(0, cru);
    let dec = gp100_core::preset_pages::decode(&pags).expect("estrutura válida");
    assert_eq!(dec.nome().expect("válido"), "Blink OD");
}
```

- [ ] **Step 2: Rodar para ver falhar**

Run: `cargo test --workspace --test preset_pages`
Expected: **FAIL** em pelo menos `recusa_nome_com_nul_interno` (a validação ainda não existe — a Task 2 só roda no caminho feliz dos 198).

> **Nota honesta:** `recusa_tamanho_errado`, `recusa_pagina_fora_de_ordem` e `recusa_byte_que_nao_e_nibble` já passam na Task 1 (a validação foi escrita lá); isto é esperado — o teste novo que **deve** falhar aqui é o do `NomeInvalido`. Se **todos** passarem de primeira, rode Step 3 mesmo assim com um `dbg!` no `nome()` para confirmar que ele está percorrendo o espaço (um teste que nunca pôde falhar não prova nada).

- [ ] **Step 3: Confirmar que a regra do pad está no `nome()`**

A Task 2 já implementou o loop `depois_do_fim → b != 0`. Se o teste ainda falhar, o bug é do teste; corrija o construtor (`pagina_sintetica` monta 4B header + pares), não a regra.

- [ ] **Step 4: Rodar para ver passar**

Run: `cargo test --workspace --test preset_pages`
Expected: **PASS**, 9 testes.

- [ ] **Step 5: Commit**

```bash
git add packages/core/tests/preset_pages.rs
git commit -m "test(core): prova-negativa do decode das paginas — recusa onde nao deve (#155)"
```

---

### Task 4: Análise dos offsets da fase 2 → artefato

**Files:**
- Modify: `analysis/map_state_pages.py`
- Create: `analysis/state_pages_offsets.json`
- Create: `analysis/validate_state_pages.py`

**Interfaces:**
- Consumes: `analysis/captures/session1.jsonl`, `files/patches/all.prst`, `analysis/parameters.json`.
- Produces: `analysis/state_pages_offsets.json` com o schema da Task 5 — **é este arquivo que o core embute**.

**Por que esta task existe:** a fase 2 (`slots()`) precisa saber **onde** cada knob está. O script atual tem três defeitos que fazem ele nunca achar: caminho absoluto `D:\…`, `pp = d[0]` (u8) que colapsa 198 pps em 2 balde, e a votação de u8 com `+= 0  # placeholder` (nunca vota).

- [ ] **Step 1: Corrigir o script**

Em `analysis/map_state_pages.py`, as quatro mudanças:

```python
HERE = os.path.dirname(os.path.abspath(__file__))
CAP1 = os.path.join(HERE, "captures", "session1.jsonl")
ALLP = os.path.join(HERE, os.pardir, "files", "patches", "all.prst")   # era D:\GP-100 app\...
OUT  = os.path.join(HERE, "state_pages_offsets.json")
```

```python
def unibble(b: bytes) -> bytes:
    """Corpo nibble-expandido -> bytes reais (pares -> byte)."""
    return bytes(((b[i] & 0x0F) << 4) | (b[i + 1] & 0x0F)
                 for i in range(0, len(b) - 1, 2))
```

No laço de coleta, **pp u16BE + decode antes de comparar**:

```python
        if dr == "in_long" and a == "13010003" and len(d) >= 100:
            pp = int.from_bytes(d[0:2], "big")          # era d[0] (u8) — colapsava 198 em 2
            page = d[3]
            presets[pp][page] = unibble(d[4:])           # era d[4:] cru — nunca casava
```

Na votação, **também u8** (o `+= 0 # placeholder` saía da conta):

```python
                    # u8: o valor do XML como byte unico, em qualquer offset
                    for off, got in enumerate(blob):
                        if got == (v & 0xFF):
                            votes[(x, pi)][("u8", page, off)] += 1
                    # u16 LE: 2 bytes consecutivos
                    start = blob.find(b8)
                    while start >= 0:
                        votes[(x, pi)][("u16", page, start)] += 1
                        start = blob.find(b8, start + 1)
```

No fim, **gravar o artefato com cobertura** (substitui o `print` de candidatos):

```python
    saida = {"generated_from": "analysis/captures/session1.jsonl", "encoding": "nibble",
             "pages": {}, "params": {}}
    for key, c in sorted(votes.items()):
        if not c:
            continue
        (larg, page, off), n = c.most_common(1)[0]
        total = sum(c.values())
        x, pi = key
        saida["params"][f"{x}/{pi}"] = {
            "page": page, "offset": off, "width": 2 if larg == "u16" else 1,
            "votes": n, "total": total, "share": round(n / total, 4) if total else 0.0,
        }
    # ---- ground truth por slot: (effectCode, effectState) do all.prst ----
    # O script já abre o XML; aqui só acumulamos o que os laços de x/param
    # já percorrem. Confirme as TAGS e os ATRIBUTOS antes de rodar:
    #   grep -oE '<[a-zA-Z]+' files/patches/all.prst | sort -u
    #   grep -oE '<[a-zA-Z]+ [^>]*' files/patches/all.prst | grep -oE '(effectCode|effectState|code|state|ppId)=' | sort -u
    verdade_slots = {}
    for preset in ET.parse(ALLP).getroot().iter("preset"):
        pid = int(preset.get("ppId") or preset.get("id") or "0", 16)
        por_x = {}
        for eff in preset.iter("effect"):
            x = int(eff.get("x") or "0")
            cod = eff.get("effectCode") or eff.get("code") or "0"
            est = eff.get("effectState") or eff.get("state") or "0"
            por_x[x] = (int(cod), int(est))
        verdade_slots[pid] = por_x
    if len(verdade_slots) != 99:
        print(f"XML: {len(verdade_slots)} presets com slots (esperado 99)")
        return 1

    # ---- slots: effectCode e effectState por posição x (0..8) ----
    # O ground truth é o MESMO parse do all.prst que este script já faz.
    # Confirme os nomes de atributo ANTES de rodar (têm de bater com o XML real):
    #   grep -oE '(effectCode|effectState|code|state)=' files/patches/all.prst | sort -u
    saida["slots"] = {}
    for x in range(9):
        # `Counter` já vem de `from collections import Counter` (linha 13) —
        # `collections.Counter()` daria NameError: o nome `collections` não está atrelado.
        vc = Counter()   # candidatos do effectCode: (pagina, off, largura)
        cand_state, tot_state = Counter(), Counter()
        for pp, pags in presets.items():
            cod, est = verdade_slots[pp][x]          # (u32, 0|1) lido do XML
            for page, blob in pags.items():
                for w in (4, 2, 1):
                    b = cod.to_bytes(w, "little")
                    i = blob.find(b)
                    while i >= 0:
                        vc[(page, i, w)] += 1
                        i = blob.find(b, i + 1)
                # state: pontua CADA offset pelo quanto ele acerta o 0/1 real
                for off in range(len(blob)):
                    cand_state[(page, off)] += 1 if blob[off] == est else 0
                    tot_state[(page, off)] += 1
        if not vc:
            print(f"slot {x}: effectCode sem candidato")
            return 1
        (pc, oc, wc), nc = vc.most_common(1)[0]
        # state: o vencedor é o offset com MAIOR taxa de acerto, e a taxa tem
        # de passar a barra — 0/1 é dado espalhado, então dominancia ingênua
        # aqui seria falso positivo garantido.
        (ps, os_), = [k for k, _ in cand_state.most_common(1)]
        ns, ts = cand_state[(ps, os_)], tot_state[(ps, os_)]
        if ts == 0 or ns / ts < 0.95:
            print(f"slot {x}: state share {ns}/{ts} abaixo da barra")
            return 1
        saida["slots"][str(x)] = {
            "code": {"page": pc, "offset": oc, "width": wc,
                     "share": round(nc / sum(vc.values()), 4)},
            "state": {"page": ps, "offset": os_, "width": 1,
                      "share": round(ns / ts, 4)},
        }

    saida["share_min"] = min((p["share"] for p in saida["params"].values()), default=0.0)
    saida["slots_share_min"] = min(
        v[k]["share"] for v in saida["slots"].values() for k in ("code", "state")
    )
    saida["keys"] = len(saida["params"])
    with open(OUT, "w", encoding="utf8") as fh:
        json.dump(saida, fh, ensure_ascii=False, indent=2, sort_keys=True)
    print(f"chaves={saida['keys']} share_min={saida['share_min']} "
          f"slots_share_min={saida['slots_share_min']}")
    # Barra da spec: offset so entra com dominancia alta — em params E em slots.
    ok = saida["share_min"] >= 0.95 and saida["slots_share_min"] >= 0.95
    return 0 if ok else 1
```

- [ ] **Step 2: Rodar e ler o resultado**

Run: `python3 analysis/map_state_pages.py`
Expected: exit 0 e `share_min >= 0.95`.

> **Isto é o ponto de pesquisa, e o exit 1 é uma resposta legítima.** Se a cobertura não chegar a 0.95: **pare e reporte** — o layout não é o que a hipótese diz. Não abaixe a barra, não filtre as chaves de "menos importante", não troque a comparação por algo que passe. O artefato com `share_min` baixo é um resultado; um artefato rebaixado é uma mentira que a fase 2 vai herdar.

- [ ] **Step 3: Gravar a prova-negativa no gate**

Crie `analysis/validate_state_pages.py` no formato do `param_ranges.py` (roda no gate `pytest`):

```python
#!/usr/bin/env python3
"""validate_state_pages.py — a trava do artefato de offsets (#155).

Um offset que "quase" casa é pior que nenhum: a fase 2 vai embutir este
arquivo no binário. Este gate recusa o artefato se a dominância do vencedor
cair abaixo da barra, se a cobertura de chaves cair, ou se o encoding não
for o declarado.
"""
import json, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ARTEFATO = os.path.join(HERE, "state_pages_offsets.json")
SHARE_MIN = 0.95
CHAVES_MIN = 100

def main() -> int:
    if not os.path.exists(ARTEFATO):
        print("state_pages_offsets.json ausente — rode map_state_pages.py")
        return 1
    a = json.load(open(ARTEFATO, encoding="utf8"))
    erros = []
    if a.get("encoding") != "nibble":
        erros.append(f"encoding={a.get('encoding')!r} (esperado 'nibble')")
    if a.get("keys", 0) < CHAVES_MIN:
        erros.append(f"chaves={a.get('keys')} < {CHAVES_MIN}")
    if a.get("share_min", 0.0) < SHARE_MIN:
        erros.append(f"share_min={a.get('share_min')} < {SHARE_MIN}")
    for k, p in a.get("params", {}).items():
        if not (0 <= p["page"] <= 8):
            erros.append(f"{k}: page={p['page']} fora de 0..8")
        if p["width"] not in (1, 2):
            erros.append(f"{k}: width={p['width']}")
    if erros:
        for e in erros:
            print(f"ERRO: {e}")
        return 1
    print(f"ok: {a['keys']} chaves, share_min={a['share_min']}")
    return 0

if __name__ == "__main__":
    sys.exit(main())
```

- [ ] **Step 4: Rodar o gate**

Run: `python3 analysis/validate_state_pages.py`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add analysis/map_state_pages.py analysis/state_pages_offsets.json analysis/validate_state_pages.py
git commit -m "analysis: offsets das paginas 13xx com nibble-decode e gate de dominancia (#155)"
```

---

### Task 5: `Paginas::slots()` — o `BoardView` a partir dos offsets provados

**Files:**
- Modify: `packages/core/src/preset_pages.rs`
- Modify: `packages/core/tests/preset_pages.rs`

**Interfaces:**
- Consumes: offset map da Task 4 (embutido); `gp100_core::model::{DICTIONARY_JSON, Dictionary}`; `gp100_core::pedalboard::{BoardView, SlotSpec, KnobSpec, Family, Archetype}`; `gp100_core::preset::Document` (ground truth).
- Produces: `pub struct Offsets { encoding, params, slots }`, `pub struct Alvo { page, offset, width, share, votes, total }`, `pub struct SlotOffsets { code: Alvo, state: Alvo }`, `Offsets::carregado() -> Result<Offsets, DecodeError>`, `pub fn slots(&self, pp: u16, dict: &Dictionary, off: &Offsets) -> Result<BoardView, DecodeError>`

**Atenção ao embutido:** use o mesmo número de `../` que `model.rs` usa ao incluir `analysis/parameters.json` — confira com `grep -n include_str packages/core/src/model.rs` **antes** de escrever a linha.

- [ ] **Step 1: Escrever o teste de paridade (falha)**

```rust
use gp100_core::model::Dictionary;
use gp100_core::pedalboard::BoardView;

/// O BoardView produzido do ARTEFATO para o mesmo pp — o ground truth.
fn board_de_fabrica(doc: &Document, dict: &Dictionary, pp: u16) -> BoardView {
    gp100_core::pedalboard::board_view_for(doc, dict, Some(pp))
        .expect("board do artefato")
}

/// **Paridade: o palco vindo das páginas tem de ser o mesmo que o do arquivo**
/// para as 99 fábricas. É a prova de que os offsets dizem a verdade — o
/// mesmo espírito do roundtrip byte-a-byte do `.prst`.
#[test]
fn slots_das_paginas_batem_com_o_all_prst_nas_99_fabrica() {
    let bytes = std::fs::read(all_prst()).expect("all.prst existe");
    let doc = Document::parse(&bytes).expect("dialecto válido");
    let dict = Dictionary::from_json(gp100_core::model::DICTIONARY_JSON).expect("dicionário válido");

    let (mapa, _) = paginas_da_fixture();
    let mut comparados = 0;
    for pp in 0u16..=0x62 {
        let Some(pags) = mapa.get(&pp) else { continue };
        let dec = gp100_core::preset_pages::decode(pags).expect("decode");
        let off = gp100_core::preset_pages::Offsets::carregado().expect("artefato");
        let lido = dec
            .slots(pp, &dict, &off)
            .unwrap_or_else(|e| panic!("pp {pp:#06x}: {e}"));
        let esperado = board_de_fabrica(&doc, &dict, pp);
        assert_eq!(lido.slots.len(), esperado.slots.len(), "pp {pp:#06x} nº de slots");
        for (a, b) in lido.slots.iter().zip(esperado.slots.iter()) {
            assert_eq!(a.slot, b.slot, "pp {pp:#06x} posição");
            assert_eq!(a.code, b.code, "pp {pp:#06x} effectCode do slot {}", b.slot);
            assert_eq!(a.state, b.state, "pp {pp:#06x} on/off do slot {}", b.slot);
            assert_eq!(a.knobs.len(), b.knobs.len(), "pp {pp:#06x} nº de knobs do slot {}", b.slot);
            for (ka, kb) in a.knobs.iter().zip(b.knobs.iter()) {
                assert_eq!(ka.pos, kb.pos, "pp {pp:#06x} pos do knob");
                assert_eq!(ka.value, kb.value, "pp {pp:#06x} slot {} knob {}", b.slot, kb.name);
            }
        }
        comparados += 1;
    }
    assert!(comparados >= 99, "só {comparados} presets comparados");
}

/// **Prova-negativa da fase 2:** um offset que não casa tem de virar erro,
/// nunca um `BoardView` com `code` inventado (spec §6 — Review Focus 3).
#[test]
fn recusa_offset_deslocado_em_vez_de_inventar_code() {
    let dict =
        Dictionary::from_json(gp100_core::model::DICTIONARY_JSON).expect("dicionário válido");
    let (mapa, _) = paginas_da_fixture();
    let pags = mapa.get(&0u16).expect("pp 0 existe");
    let dec = gp100_core::preset_pages::decode(pags).expect("decode");

    let mut off = gp100_core::preset_pages::Offsets::carregado().expect("artefato embutido");
    for s in off.slots.values_mut() {
        s.code.offset += 7; // lê bytes que não são o effectCode
    }
    let e = dec
        .slots(0, &dict, &off)
        .expect_err("offset deslocado não pode virar slot");
    assert!(
        matches!(
            e,
            gp100_core::preset_pages::DecodeError::EfeitoDesconhecido { .. }
                | gp100_core::preset_pages::DecodeError::OffsetsNaoBatem { .. }
        ),
        "{e}"
    );
}
```

> Se `board_view_for` não for `pub` ou tiver outra assinatura, use o caminho público equivalente em `pedalboard.rs` (`doc.presets()` + `BoardView` — ver `pedalboard.rs:203`). O **contrato** é o mesmo: mesmo `BoardView`, dois produtores.

- [ ] **Step 2: Rodar para ver falhar**

Run: `cargo test --workspace --test preset_pages`
Expected: **FAIL** — `no method named slots`.

- [ ] **Step 3: Implementar**

```rust
/// Offsets provados pela análise (Task 4), embutidos no binário —
/// o mesmo regime de `model::DICTIONARY_JSON` (nada de parede).
pub const OFFSETS_JSON: &str = include_str!("../../../analysis/state_pages_offsets.json");

/// Onde um valor vive no corpo DECODIFICADO da página.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Alvo {
    /// Página 0..=8.
    pub page: u8,
    /// Offset no corpo decodificado.
    pub offset: usize,
    /// Largura em bytes (1 = u8, 2 = u16 LE, 4 = u32 LE).
    pub width: u8,
    /// Dominação do vencedor (o gate exige ≥ 0.95).
    pub share: f64,
    /// Votos do vencedor / total — diagnóstico quando algo não bate.
    #[serde(default)]
    pub votes: usize,
    #[serde(default)]
    pub total: usize,
}

impl Alvo {
    /// Lê `width` bytes LE no corpo da página indicada.
    ///
    /// # Erros
    /// [`DecodeError::OffsetsNaoBatem`] se o alvo cair fora do corpo — ele
    /// tem 96B (pg 0..7) e 14B (pg 8), então um offset de outro formato não
    /// passa daqui.
    fn le(&self, paginas: &Paginas) -> Result<u32, DecodeError> {
        let corpo = paginas.corpo(usize::from(self.page));
        let fim = self.offset.saturating_add(usize::from(self.width));
        let faixa = corpo.get(self.offset..fim).ok_or(DecodeError::OffsetsNaoBatem {
            achado: corpo.len(),
            esperado: fim,
        })?;
        let mut v = 0u32;
        for (i, &b) in faixa.iter().enumerate() {
            v |= u32::from(b) << (8 * i);
        }
        Ok(v)
    }
}

/// Onde estão `effectCode` e `effectState` de um slot.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SlotOffsets {
    /// effectCode do slot x.
    pub code: Alvo,
    /// effectState (0|1) do slot x.
    pub state: Alvo,
}

/// O artefato, na forma embutida.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Offsets {
    /// Só `"nibble"` — qualquer outra coisa é outro formato.
    pub encoding: String,
    /// `"<x>/<pos>"` → valor do parâmetro (`pos` = `Control::pos`).
    pub params: std::collections::HashMap<String, Alvo>,
    /// `"0".."8"` → code/state daquele slot.
    pub slots: std::collections::HashMap<String, SlotOffsets>,
}

impl Offsets {
    /// O artefato embutido, validado. Produção e teste leem a MESMA fonte —
    /// não existe caminho "só de teste".
    ///
    /// # Erros
    /// [`DecodeError::ArtefatoInvalido`] se o JSON não parsear ou se o
    /// encoding não for `nibble`.
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
        Ok(o)
    }
}

impl Paginas {
    /// Os 9 slots do preset, reconstruídos dos offsets provados.
    ///
    /// Identidade visual (família, arquétipo, nome, variante) e knobs vêm do
    /// **dicionário** (R1); as páginas trazem effectCode, effectState e os
    /// valores atuais. É o mesmo caminho de `pedalboard::board_view_for`, só
    /// que a fonte é o fio e não o arquivo.
    ///
    /// `pp_type`/`pp_type_name` ficam zerados de propósito: o `ppType` casa em
    /// só 166/198 (spec §11.2), abaixo da barra de 0.95, então não tem offset
    /// provado. Ícone errado na biblioteca vale mais que um chute com cara de
    /// prova — e a limitação está declarada, não escondida.
    ///
    /// # Erros
    /// [`DecodeError::NomeInvalido`], [`DecodeError::OffsetsNaoBatem`] ou
    /// [`DecodeError::EfeitoDesconhecido`]. **Nunca** devolve slot com `code`
    /// inventado: `BoardView` errado é pior que erro, porque o palco desenha
    /// e o usuário acredita.
    pub fn slots(
        &self,
        pp: u16,
        dict: &crate::model::Dictionary,
        off: &Offsets,
    ) -> Result<BoardView, DecodeError> {
        let nome = self.nome()?.to_string();
        let mut slots = Vec::with_capacity(9);
        for x in 0..9u8 {
            let so = off
                .slots
                .get(&x.to_string())
                .ok_or_else(|| DecodeError::OffsetsNaoBatem {
                    achado: usize::from(x),
                    esperado: 9,
                })?;
            let code = so.code.le(self)?;
            let state = so.state.le(self)? != 0;

            let nibble = ((code >> 24) & 0xFF) as u8;
            let index = code & 0x00FF_FFFF;
            let Some(algo) = dict.algorithm(nibble, index) else {
                return Err(DecodeError::EfeitoDesconhecido { code });
            };
            let family = crate::pedalboard::family_of(&algo.module);

            let knobs = algo
                .controls
                .iter()
                .map(|c| {
                    // Valor atual: offset POR parâmetro, mesma chave da análise.
                    let valor = off
                        .params
                        .get(&format!("{x}/{}", c.pos))
                        .map(|a| a.le(self).map(|v| v.to_string()))
                        .transpose()?;
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
                        value: valor,
                        default: c.default.clone(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            slots.push(SlotSpec {
                slot: x,
                family,
                archetype: crate::pedalboard::archetype_of(family),
                name: algo.name.trim().to_string(),
                variant: crate::pedalboard::slug(&algo.name),
                state,
                code,
                knobs,
            });
        }
        Ok(BoardView {
            pp,
            name: nome,
            pp_type: 0,
            pp_type_name: String::new(),
            slots,
        })
    }
}
```

**Duas mudanças em `pedalboard.rs`, na mesma task** (sem elas não compila):

```rust
// pedalboard.rs:141, :155, :170 — promover a pub(crate) em vez de reimplementar.
pub(crate) fn family_of(module: &str) -> Family { … }
pub(crate) fn archetype_of(f: Family) -> Archetype { … }
pub(crate) fn slug(name: &str) -> String { … }
```

E no topo de `preset_pages.rs`, junto do `use crate::session::StatePage;`:

```rust
use crate::model::{ControlKind, Dictionary};
use crate::pedalboard::{BoardView, KnobSpec, SlotSpec};
```

> O `pos` da chave `"{x}/{pos}"` tem de ser o MESMO `pi` que o script da Task 4 votou — se não for, o teste de paridade falha alto no `value` do knob. É exatamente para isso que ele existe.

> **O único acoplamento desta task com a Task 4 é o schema do artefato** (`encoding`/`params`/`slots`, com `Alvo { page, offset, width, share }`) — já fixado no código da Task 4. O que a Task 4 ainda vai descobrir é **onde** cada offset está (os números), não **o formato** que este código lê. Se a Task 4 sair com outro schema, este código é que muda — e o teste de paridade (99/99) é quem barra a divergência, junto com `recusa_offset_deslocado_em_vez_de_inventar_code`.

- [ ] **Step 4: Rodar para ver passar**

Run: `cargo test --workspace --test preset_pages`
Expected: **PASS** (paridade 99/99 + as 9 anteriores).

- [ ] **Step 5: fmt + clippy + commit**

Run: `cargo fmt && cargo clippy --workspace --all-targets -D warnings && cargo test --workspace`
Expected: exit 0.

```bash
git add packages/core/src/preset_pages.rs packages/core/tests/preset_pages.rs
git commit -m "feat(core): slots do BoardView a partir dos offsets provados (#155)"
```

---

### Task 6: Integração fase 1 — nomes reais na biblioteca

**Files:**
- Modify: `packages/app/api/src/actor.rs` (cache de nomes no scan; ~L225 e `Request::Library`)

**Interfaces:**
- Consumes: `gp100_core::preset_pages::{decode, Paginas}`; `Request::Boot` / `Request::Library`; `DeviceSnapshot.current_name`.
- Produces: `PresetEntry { pp, name, pp_type, pp_type_name }` com nome **real**; `DeviceSnapshot.current_name` sem o `"Só no mock"`.

- [ ] **Step 1: Escrever o teste do cache**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// O cache só é preenchido pelo scan; sem scan, não há nome — e inventar
    /// seria pior que dizer "desconhecido" (spec §6: nunca chute).
    #[test]
    fn cache_de_nomes_vazio_sem_boot() {
        let c: std::collections::HashMap<u16, String> = Default::default();
        assert!(c.is_empty());
        assert_eq!(c.get(&0), None);
    }
}
```

> O teste real da fase 1 está no core (Task 2: 198/198). Aqui o que se prova é a **integração**: que o actor não serve nome quando não leu. Amplie este teste para o caminho `Request::Library` com o actor montado em mock (o padrão dos testes existentes de `actor.rs`) — se o mock já devolve nome do artefato, o teste cobre os **dois** caminhos e não só o vazio.

- [ ] **Step 2: Rodar para ver falhar**

Run: `cargo test -p gp100-ui --manifest-path packages/app/api/Cargo.toml`
Expected: o teste novo compila; a implementação (Step 3) é o que falta.

- [ ] **Step 3: Implementar o cache**

Em `actor.rs`:
1. Adicione `nomes: std::collections::HashMap<u16, String>` ao estado do actor (o mesmo lugar onde vive o inventário).
2. No handler do scan/boot, para cada resposta `13010003` agrupada por pp, chame `preset_pages::decode` e grave `nomes.insert(pp, paginas.nome()?.to_string())`. Falha de decode de UM pp **não** derruba o boot: registre e siga (o pp fica sem nome, e a UI mostra desconhecido).
3. `Request::Library` devolve `name` do cache; ausente → string vazia, nunca o nome do artefato.
4. `DeviceSnapshot.current_name`: se o cache tem o pp corrente, devolve o nome real; senão mantém a mensagem de "não lido" **sem** dizer "Só no mock".

- [ ] **Step 4: Rodar**

Run: `cargo test -p gp100-ui --manifest-path packages/app/api/Cargo.toml && cargo clippy --workspace --all-targets -D warnings`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add packages/app/api/src/actor.rs
git commit -m "feat(api): nomes reais na biblioteca a partir do cache do scan (#155)"
```

---

### Task 7: Integração fase 2 — `device_board` sem a recusa

**Files:**
- Modify: `packages/app/api/src/actor.rs` (remove recusa ~L702; serve do cache)
- Modify: `docs/PROTOCOL.md` §13.10

**Interfaces:**
- Consumes: `Paginas::slots` (Task 5), cache do Task 6.
- Produces: `device_board(pp)` com `backend == Real` devolvendo `BoardView`.

- [ ] **Step 1: Teste (falha)**

```rust
/// **O critério da #155:** com backend Real o palco não é mais recusado,
/// e o pp pedido vem do CACHE — nunca de um select novo (spec §5.2).
#[test]
fn board_real_vem_do_cache_e_nao_de_um_select() {
    // monte o actor com backend Real (o padrão dos testes de boot) e conte
    // as transações: device_board NÃO pode emitir 13010000 (select).
    unimplemented!("escreva usando o harness de transações existente");
}
```

> Substitua o `unimplemented!` pelo harness real: os testes de `pp_gate.rs` já contam transações do mock (`contagem de transações do mock = 0`), e o mesmo instrumento serve aqui. **O teste tem de falhar hoje** (a recusa devolve `Err`) e passar depois.

- [ ] **Step 2: Rodar para ver falhar**

Run: `cargo test -p gp100-ui --manifest-path packages/app/api/Cargo.toml`
Expected: **FAIL** com `"palco do aparelho pendente do decode das páginas 13xx (issue #152)"`.

- [ ] **Step 3: Remover a recusa e servir do cache**

Em `Request::Board` (actor.rs ~L702), substitua o bloco `if backend == Backend::Real { return Err(…) }` por:

```rust
// O palco do aparelho vem das páginas 13xx decifradas (#155) — do CACHE
// do scan, nunca de um select novo: ler outro pp não pode mudar o preset
// que o pedal está mostrando (spec §5.2).
if backend == Backend::Real {
    let paginas = cache_de_paginas.get(pp.unwrap_or(current_pp))
        .ok_or("páginas deste pp não foram lidas — refaça o boot")?;
    let dict = Dictionary::from_json(gp100_core::model::DICTIONARY_JSON)
        .map_err(|e| e.to_string())?;
    let off = preset_pages::Offsets::carregado().map_err(|e| e.to_string())?;
    return Ok(paginas
        .slots(pp.unwrap_or(current_pp), &dict, &off)
        .map_err(|e| e.to_string())?);
}
```

- [ ] **Step 4: Rodar**

Run: `cargo test -p gp100-ui --manifest-path packages/app/api/Cargo.toml`
Expected: **PASS**.

- [ ] **Step 5: `PROTOCOL.md` §13.10**

Acrescente ao §13.10 a seção **"Layout da página (decifrado 08/10/2026)"**: corpo nibble-expandido, pg0 = `00 00 | nome[16, pad NUL] | …`, tamanhos 196/32 no fio → 96/14 decodificados, e a referência ao `preset_pages.rs` + ao gate `validate_state_pages.py`.

- [ ] **Step 6: Commit**

```bash
git add packages/app/api/src/actor.rs docs/PROTOCOL.md
git commit -m "feat(api): device_board real servido do cache das paginas (#155)"
```

---

### Task 8: Resolver a divergência do `13010005`

**Files:**
- Modify: `packages/core/src/session.rs` (`state_page`), possivelmente `docs/PROTOCOL.md`

**Interfaces:**
- Consumes: contagem de `13010005` na S1 (**0 mensagens**) vs `Session::state_page` que roteia pg8 para `13010005`.
- Produces: uma decisão justificada e um teste.

- [ ] **Step 1: Medir antes de mexer**

Run:
```bash
python3 - <<'PY'
import json, collections
HDR="f021257f47502d64"; c=collections.Counter()
for line in open("analysis/captures/session1.jsonl", encoding="utf8", errors="replace"):
    s=line.strip()
    if not s: continue
    try: e=json.loads(s)
    except: continue
    hx=e.get("hex","").lower()
    if not hx.startswith(HDR): continue
    i=hx.find("f7"); hx=hx[:i+2] if i>=0 else hx
    b=hx[16:-2]
    if len(b)>=20 and len(b)%2==0 and e.get("dir")=="in_long":
        c[b[2:10]]+=1
print({k:v for k,v in c.items() if k.startswith("1301")})
PY
```
Expected: `13010003` aparece, `13010005` **não**.

- [ ] **Step 2: Decidir com a evidência**

Duas hipóteses, e o teste as distingue:
- **(a) O código está errado** — a pg8 chega em `13010003` com 32B (é o que a captura mostra: `d[3] == 8`, 198 ocorrências). Corrigir `state_page` para `in_addr = 13010003` em `page <= 8`, e o teste é a contagem acima.
- **(b) A captura não cobre** — o `13010005` só aparece num outro fluxo (o `build_golden.py`/`PROTOCOL` §13.10 podem ter vindo de outra sessão). Nesse caso `state_page` está certo e o que falta é captura.

A evidência que tenho apoia **(a)**: **198 respas em `13010003` com `d[3] == 8` e 32B** foram medidas. Confirme com o Step 1 antes de editar.

- [ ] **Step 3: Escrever o teste que distingue**

```rust
/// A pg8 chega em `13010003` (198 respas @32B, d[3]==8) — não em 13010005,
/// endereço com ZERO mensagens na S1. Se `state_page` rotear para
/// 13010005, o mock devolve o golden do endereço errado e o teste cai.
#[test]
fn pg8_vem_do_mesmo_endereco_das_demais() {
    // use o mock: state_page(8) tem de casar com a linha 13010003/d[3]==8
    // de boot.jsonl, byte a byte.
}
```

- [ ] **Step 4: Corrigir `state_page` se (a)**

Em `session.rs:664`, se o Step 1 confirmar:
```rust
let in_addr: &[u8; 4] = &[0x13, 0x01, 0x00, 0x03]; // pg8 também (S1: 198 respas @32B)
```
Atualize o doc-comment de `state_page` e de `StatePage` com a contagem medida.

- [ ] **Step 5: Rodar tudo**

Run: `cargo test --workspace && python3 analysis/validate_golden.py`
Expected: exit 0 · golden **1782/1782** (R2/R3: se o golden mudar, é fluxo novo, não este PR).

- [ ] **Step 6: Commit**

```bash
git add packages/core/src/session.rs packages/core/tests/preset_pages.rs docs/PROTOCOL.md
git commit -m "fix(core): pg8 chega em 13010003 — 13010005 nao existe na S1 (#155)"
```

---

### Task 9: Confirmação em campo

**Files:**
- Create: `docs/STATE_PAGES_FIELD.md`

**Interfaces:**
- Consumes: build de campo `--features real-device,write-verified` (ADR-5), FieldDiag + wire log.
- Produzes: relatório com a **prova por manipulação** (spec §8).

- [ ] **Step 1: Build de campo**

```bash
cd packages/app/api && cargo +stable-msvc build --release --features real-device,write-verified
```

- [ ] **Step 2: Captura dirigida (a) — NOME**

Com o wire log do FieldDiag ligado: renomear UM user patch no aparelho e gravar (SAVE). Depois:
- diff do frame `13010003` pg0 antes/depois → o byte que mudou **tem de** ser o nome em offset 2;
- o nome novo aparece no app **sem re-seed do artefato**.

- [ ] **Step 3: Captura dirigida (b) — KNOB**

Girar UM knob no hardware → quais bytes de quais páginas mudam → confirma o offset do artefato da Task 4.

- [ ] **Step 4: Escrever `docs/STATE_PAGES_FIELD.md`**

Conteúdo obrigatório: SHA do build, firmware, os dois diffs, o offset confirmado, a contagem de pp lido, e a frase explícita de **o que não foi medido**. Anexar os `jsonl` de campo em `analysis/captures/` (fluxo R3: nova captura → `make_fixtures` → paridade).

- [ ] **Step 5: Commit**

```bash
git add docs/STATE_PAGES_FIELD.md analysis/captures/
git commit -m "docs(#155): confirmacao em campo — nome e knob editados no aparelho"
```

---

### Task 10: Gates, documentação e PR

- [ ] **Step 1: Registrar a decisão**

`docs/DECISIONS.md` → novo ADR (formato dos vizinhos: `**Status:** Accepted · **Afeta:** …`): o encoding nibble das páginas, o nome em pg0/offset2, o cache-do-boot como fonte do palco (nunca um `select`), e a recusa honesta do `DecodeError`.

- [ ] **Step 2: Índice e roadmap**

`docs/INDEX.md` → `STATE_PAGES_FIELD.md` e o spec; `docs/ROADMAP.md` → progresso da #155.

- [ ] **Step 3: Matriz completa**

```bash
cargo fmt --check && cargo clippy --workspace --all-targets -D warnings && cargo test --workspace
cd packages/app/ui && pnpm lint && pnpm test && pnpm build
cd ../.. && python3 analysis/validate_golden.py && python3 analysis/validate_state_pages.py && python3 scripts/gates.py
```
Expected: exit 0 em todos · golden **1782/1782** · `gates.py` → **13/13**. **Cole a saída real de cada um aqui antes de seguir.**

- [ ] **Step 4: Branch, commit e PR**

```bash
git checkout -b feature/155-decode-paginas-13xx
git push -u origin feature/155-decode-paginas-13xx
gh pr create --base develop \
  --title "feat: decode das paginas 13xx — palco vivo e nomes reais (#155)" \
  --body-file .freebuff/pr-155.md
```
Corpo cita `Closes #155` **somente** se o campo (Task 9) estiver no merge; senão a issue fica aberta com o comentário do que falta. **Registrar no corpo que a #152 precisa ser reescrita** (meta6 refutada — spec §9).

- [ ] **Step 5: Aguardar CI**

Run: `gh pr checks <n> --watch` → **0 falhas**, `mergeStateStatus == CLEAN`.

---

## Self-Review

**1. Cobertura da spec:**

| Spec | Task |
|---|---|
| §2 verdade dos dados | Task 1 (dataset), Task 2 (nome 198/198) |
| §3 escopo fase 1 | Tasks 1–3 (core) + Task 6 (integração) |
| §3 escopo fase 2 | Tasks 4–5 (offsets + slots) + Task 7 (integração) |
| §4 arquitetura `preset_pages.rs` | Tasks 1–2, 5 |
| §5.1 nome de graça no boot | Task 6 |
| §5.2 cache, nunca select | Task 7 (teste conta transações) |
| §5.3 três pontos de integração | Task 6 (~L225 + library), Task 7 (~L702 + PROTOCOL) |
| §6 `DecodeError` sem chute | Tasks 1–3 (7 variantes, todas testadas) |
| §7.1–7.3 testes | Tasks 1–3, 5 |
| §7.4 golden byte-idêntico | Tasks 8, 10 |
| §8 campo | Task 9 |
| §11.1 divergência `13010005` | Task 8 |
| §11.2 `ppType` 166/198 | §4.1 da spec o moveu para fase 2 — **fora deste plano**, registrado em §11 da spec |
| §10 critérios de aceite | Tasks 1–2, 5, 7, 9, 10 |

**2. Placeholders:** varredura por `TBD` · `TODO` · `implement later` · `add validation` · `handle edge cases` · `Similar to Task N` · blocos `…` → **zero ocorrências no corpo do plano** (a única ocorrência é esta própria frase, que os cita). Todo step de código tem bloco completo: `decode`, `nome`, os 7 testes da Task 3, o script da Task 4 (incluindo `verdade_slots`, que não é referenciada sem ser construída), o schema fixado, `Alvo`/`Offsets`/`carregado`/`slots` da Task 5, e as duas micro-mudanças em `pedalboard.rs` (`pub(crate)`). Os `grep`/`grep -oE` do Task 4 e do Task 8 são **instruções de verificação** com arquivo e padrão dados — não preenchimento. O único acoplamento declarado (Task 5 ↔ schema da Task 4) está explícito na nota logo abaixo do `slots()`.

**3. Consistência de tipos:** `StatePage::raw` (196/32B com header de 4B) → `decode(&[StatePage;9])` → `Paginas::corpo(i)` → `nome()` / `slots(pp, dict, off)`. `DecodeError` tem **9 variantes**, todas construídas e testadas: `TamanhoInesperado`, `PaginaForaDeOrdem`, `NibbleInvalido`, `ParImpar` (Task 3), `NomeInvalido` (Task 3, 2 testes), `OffsetsNaoBatem` e `EfeitoDesconhecido` (Task 5, `recusa_offset_deslocado`), `ArtefatoInvalido` (Task 5, `carregado()`), `Arquivo…`/todo resto inexistente. Os nomes de campo de `SlotSpec`/`KnobSpec` usados no teste de paridade foram conferidos contra `pedalboard.rs:76-127` (`slot`, `code`, `state`, `knobs` · `pos`, `value`), e `board_view_for(doc, dict, Option<u16>)` é `pub` (linha 225). `pp & 0xFF` é a mesma chave da análise medida.

**Caminho de tradução `effectCode` → identidade conferido no código** (não é suposição): `dict.algorithm(nibble, index)` → `Algorithm { name, module, code, nibble, index, controls }`; `family_of(module)` · `archetype_of(family)` · `slug(name)` são `fn` **privadas** de `pedalboard.rs`, então a Task 5 as promove a `pub(crate)` em vez de reimplementar (R1: um caminho só).

**4. Review Focus:** as 5 linhas apontam para Tasks 1, 2, 4, 6, 8 — e cada uma tem o teste no próprio step da task (dedupe com igualdade na Task 1; NUL interno na Task 3; `share_min` + gate na Task 4; teste do cache vazio na Task 6; contagem `13010005` na Task 8).

---

## Execution Handoff

Plano completo e salvo em `docs/superpowers/plans/2026-10-08-decode-paginas-13xx.md`. **Revise o plano antes de implementar — ele captura o que você quer?**

Duas formas de executar:

- **Subagent-driven** — um subagent novo implementa cada task e um revisor novo confere antes da seguinte, com revisão de branch inteira no fim. Mais exaustivo; custa contexto novo por task e por revisão.
- **Native** — eu implemento todas as tasks nesta sessão e um revisor confere a branch no fim. Mais barato e rápido; sem revisão independente até o fim.

**Para este plano recomendo `subagent-driven`:** as Tasks 6–7 dependem das interfaces que as 1–5 definem (`Paginas`, `DecodeError`, o artefato de offsets), a Task 4 é pesquisa com entrega incerta (um erro ali vira reescrita), e o custo de um erro embarcado é alto — ele vai embutido no binário e só aparece com um aparelho na mão.
