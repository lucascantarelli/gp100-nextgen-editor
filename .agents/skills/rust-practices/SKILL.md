---
name: rust-practices
description: Boas práticas de Rust para gp100-core/gp100-cli — gates de qualidade (fmt/clippy/test/doc-tests), política de CI, estilo de documentação PT-BR, erros tipados, testes vetorizados e política de features.
metadata:
  category: development
---

# Boas práticas Rust (gp100-core / gp100-cli)

Use ao escrever ou revisar QUALQUER código Rust do projeto (issues M0.x do
ROADMAP, skill `core-dev`). Complementa as regras R1–R4 do ROADMAP e os ADRs
pré-assinados de `docs/DECISIONS.md` — em conflito, valem R1–R4 e os ADRs.

## Gates de qualidade (toda mudança passa pelos 4)

```bash
# Se o terminal novo não achar cargo: fix permanente = scripts/add_cargo_path.ps1
# (feito em 29/09 — vale para QUALQUER terminal novo); workaround de sessão =
# export abaixo, só para sessões abertas antes do fix.
export PATH="/c/Users/Canta/.cargo/bin:$PATH"
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test          # inclui os DOC-TESTS (exemplos de doc-comments compilam e rodam)
cargo build
```

## CI (GitHub Actions) — mesma régua do local

O CI (`.github/workflows/ci.yml`) roda os MESMOS gates por push/PR em runner
`windows-latest` (o pin gnu do `rust-toolchain.toml` não compila no Linux).
Armadilhas já mordidas — não reabrir:

- **Runner `windows-latest` é obrigatório** por causa do pin gnu; não
  "simplificar" para ubuntu.
- Instalar a toolchain pinada com `rustup toolchain install <pin> --component
  rustfmt --component clippy`. **`--component` é REPETÍVEL**: escrever
  `--component rustfmt clippy` faz o rustup ler `clippy` como uma toolchain
  (`invalid toolchain name: 'clippy'`).
- O perfil do rustup (default ou minimal) **não inclui** componentes; os
  fmt/clippy pré-instalados no runner pertencem à toolchain **msvc** dele e não
  servem ao pin gnu — sem o `--component` explícito, `cargo fmt` falha com
  "'cargo-fmt.exe' is not installed for the toolchain ...".
- `dtolnay/rust-toolchain` sonda `rustc -vV` **antes** de instalar e morre no
  shim do rustup (canal pinado ainda inexistente no runner: "target tuple in
  channel name") — usar install explícito.
- CI vermelho nunca é contornado: sem relaxar `-D warnings`, sem pular job.

## Estilo de documentação (obrigatório)

1. Todo crate/módulo abre com `//!` explicando papel, fase do ROADMAP e as
   fontes de verdade (golden / §13 / ADRs).
2. Todo item `pub` (const, fn, struct, trait) tem doc-comment `///` em PT-BR:
   o que é, DE ONDE vem a evidência (§13/golden/fixture) e armadilhas.
3. Comentário `//` dentro de função explica o PORQUÊ da decisão, não o óbvio
   do código; passo a passo numerado onde houver sequência (ex.: guarda de
   política no CLI).
4. Exemplos em doc-comments são EXECUTÁVEIS (doc-tests) — documentação que
   mente quebra o `cargo test`.
5. `gp100-core` compila com `#![deny(missing_docs)]`: item público sem doc =
   erro de build (a política é imposta pelo compilador, não pela boa vontade).

## Código

- Erros = `thiserror` com `ProtocolError` tipado (ADR-2). Sem
  `unwrap`/`expect`/`panic!` na lib (allowed só em testes).
- Sem `unsafe` no M0. Caso legítimo apareça (ex.: WinMM real), novo ADR antes.
- Magic number de protocolo NUNCA inline: vira `const` documentada ou entra no
  golden (R1). Dados de protocolo vêm de `docs/protocol_golden.json`; replay
  usa `analysis/fixtures/` (P4) — nunca hardcode de captura.
- Feature `real-device` default OFF; nada de I/O de device fora do módulo
  `transport` (ADR-4). Mock é o caminho default de tudo.
- Tipos novos quando baratos: newtype para ids (pp, slot, página) com métodos
  de conversão BE/LE concentrados no codec (ADR-1) — espalhar conversões é bug.

## Regime de bytes (`.gitattributes -text`)

Arquivo cuja identidade é **byte-a-byte** (hash de baseline, round-trip R4,
comparação de replay) entra no `.gitattributes` com **`-text` ANTES do
primeiro commit** — EOL faz parte dos bytes, e a normalização de EOL do git
quebra a identidade silenciosamente (disco CRLF × blob LF; mordido 2x na CI
em 29/09: golden e .prst).

- Classes na lista (ver seção comentada no `.gitattributes`):
  1. especificação executável (`docs/protocol_golden.json`);
  2. patches de referência (`files/patches/*.prst`, R4);
  3. evidência de campo (`analysis/captures/*.jsonl`, `ir_slot*.bin`);
  4. fixtures de replay (`analysis/fixtures/*.jsonl` — o replay do M0.6
     compara byte a byte).
- **NÃO** entra: produto editável/regenerável (docs, scripts, knob_map,
  parameters.json — identidade semântica, não byte).
- Ao tocar na lista: `git add --renormalize .` e conferir
  `git show :<caminho> | sha256sum` × `sha256sum <caminho>`.
- Teste que depende de bytes de arquivo (R4/replay) deve ler o arquivo **do
  repo** (checkout limpo) — nunca de cópia local fora do git.

## Testes (modelo híbrido, análogo ao pytest)

```text
gp100-core/
├── src/                        # código + testes UNITÁRIOS (internals)
│   └── lib.rs                  #    #[cfg(test)] mod tests { ... }
└── tests/                      # testes de CONTRATO (caixa-preta, só API pub)
    ├── common/
    │   └── mod.rs              #    ≈ conftest.py (mod.rs evita binário)
    └── model_dictionary.rs     #    ≈ tests/test_model_dictionary.py
```

| pytest | Rust |
|---|---|
| `tests/test_x.py` importa o pacote | `tests/x.rs` usa `gp100_core::…` (API pub) |
| `conftest.py` / fixtures | `tests/common/mod.rs` |
| `pytest tests/test_x.py` | `cargo test --test x` |
| acesso a internals | só no `#[cfg(test)] mod tests` dentro de `src/` |
| `@pytest.mark.parametrize` | `#[test]` + loop (ou crate `rstest`) |

- **Unitários** (`#[cfg(test)] mod tests` dentro de `src/`): internals e
  peças privadas (codec/FSM). Acesso total ao módulo.
- **Integração/contrato** (`gp100-core/tests/<assunto>.rs`): caixa-preta,
  só API `pub` — cada arquivo é um binário de teste; helpers em
  `tests/common/mod.rs` (≈ conftest.py; o nome `mod.rs` evita binário).
  Rodar um arquivo: `cargo test --test model_dictionary`.
- **Doc-tests**: exemplos de doc-comments executáveis (documentação viva).
- Regra de decisão: usa só API pública → `tests/`; precisa de internals →
  `src/`. Teste novo acompanha código novo (padrão de DoD das issues).
- Vetorizados com os exemplos do golden / fixtures do P4; propriedade
  mínima: "gerado casa com o próprio exemplo do template" (M0.3).
- Regras de framing/endianness viram doc-tests.

## Armadilhas deste host (ver knowledge.md)

- Toolchain PINADA `stable-x86_64-pc-windows-gnu` em `rust-toolchain.toml` — o
  host NÃO tem MSVC Build Tools e o `link` do PATH é o GNU coreutils; não
  trocar de alvo nem apagar o arquivo.
- PATH do cargo: resolvido no sistema em 29/09 (`scripts/add_cargo_path.ps1`);
  workaround de sessão só para terminais abertos antes do fix:
  `export PATH="/c/Users/Canta/.cargo/bin:$PATH"`.
