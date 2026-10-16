---
name: rust-practices
description: Boas práticas de Rust para os crates gp100-core e gp100-cli (packages/core, packages/cli) — gates de qualidade (fmt/clippy/test/doc-tests), política de CI, estilo de documentação PT-BR, erros tipados, testes vetorizados e política de features.
metadata:
  category: development
---

# Boas práticas Rust (gp100-core / gp100-cli)

Use ao escrever ou revisar QUALQUER código Rust do projeto (issues do milestone
`v1.1.0`, skill `core-dev`). Complementa os princípios P1–P7 do
`docs/planejamento/ROADMAP.md` §1.2 e os ADRs de `docs/dev/DECISIONS.md` — em
conflito, valem P1–P7 e os ADRs.

## Gates de qualidade (toda mudança passa pelos 4)

```bash
# Terminal novo sem cargo? As armadilhas de host (fix permanente + workaround de
# sessão) têm UMA casa: `knowledge.md` §"Armadilhas deste host" — não redescubra.
export PATH="/c/Users/Canta/.cargo/bin:$PATH"
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test          # inclui os DOC-TESTS (exemplos de doc-comments compilam e rodam)
cargo build
```

## CI (GitHub Actions) — mesma régua do local

A CI é o pipeline `.github/workflows/ci.yml` + os seis reutilizáveis de
Distribuição/Release (`dist-*.yml`/`release-*.yml`, `workflow_call` — #245: o
corpo saiu do `ci.yml`, que ficou com o grafo) + o workflow próprio de fechamento
`issues-fechamento.yml` (#169) + o `visual-baselines.yml`, workflow de
manutenção read-only que regenera as baselines `-linux` por dispatch (#204:
saiu do ci.yml porque o `default: release` do dispatch cortava tag junto com
regeneração). Os gates Rust são **três jobs separados**, não um `rust` único:

| Job | Gate | Matriz |
|---|---|---|
| `Lint · Rust (fmt)` | `cargo fmt --check` | — |
| `Lint · Rust (clippy · …)` | `cargo clippy --all-targets -- -D warnings` | core/cli/ui-rust |
| `Compilação · Rust (cargo check · …)` | `cargo check` | core/cli/ui-rust |
| `Testes · Rust (cargo test · …)` | `cargo test` + build do binário de campo | core/cli/ui-rust |

Matriz de OS (#169): **core e cli em 4 OS** — windows-latest, ubuntu-24.04,
macos-15-intel e **archlinux** (container `archlinux:base-devel` no runner
ubuntu; o Arch é o Linux rolling da matriz). **ui-rust em 2 OS** — Windows
(MSVC) + Linux no container `ci-linux` (issue #41); o macOS saiu do ui-rust de
propósito: compilar o shell Tauri lá não pegava classe de bug própria.

**Quando roda:** `push` em `develop`/`main` e em tags `v*`; branch de trabalho
**não** dispara push — entra só por `pull_request` (a #68 restringiu `on.push`
a integration porque `feature/**` + `pull_request` davam duas runs por push).
O filtro é de *caminho* (`.github/scripts/ci_plan.py`): PR que só mexe em `docs/` não
sobe Rust nem front. A lista de caminhos Rust sai dos `members` do `Cargo.toml` da
**raiz** do repo e **falha alto** se ele não for legível (#261: antes, um caminho errado
virava lista vazia e o PR que mexia só em `packages/core` rodava a CI **sem** lint,
compile e teste de Rust — e ficava verde).

Armadilhas mordidas — não reabrir:

- **Toolchain por OS/projeto (ADR-8)**: o pin gnu do `rust-toolchain.toml` da
  raiz vale para o host Windows; a CI define `RUSTUP_TOOLCHAIN` por OS/projeto
  (gnu na raiz, msvc em `packages/app/api`, stable fora do Windows), no
  `env:` do JOB (não via GITHUB_ENV: o probe `rustc -vV` do rust-cache roda
  dentro da composite e não enxerga a var exportada — o pin gnu da raiz vence
  e aparece "target tuple in channel name" nas annotations) — sem isso,
  qualquer cargo no Linux morre com o mesmo erro. Não "simplificar" para
  runner único.
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
- Sem `unsafe` na lib. Caso legítimo apareça (ex.: WinMM real), novo ADR antes.
- Magic number de protocolo NUNCA inline: vira `const` documentada ou entra no
  golden. Dados de protocolo vêm de `docs/referencias/protocol_golden.json`; replay
  usa `packages/core/data/fixtures/` — nunca hardcode de captura.
- Feature `real-device`: default **ON no app** (`gp100-ui`, #165 — `tauri dev`
  fala com o aparelho) e **OFF no core/CLI/testes**; nada de I/O de device fora
  do módulo `transport` (ADR-4). Nos testes o caminho default é o `MockDevice`.
- Tipos novos quando baratos: newtype para ids (pp, slot, página) com métodos
  de conversão BE/LE concentrados no codec (ADR-1) — espalhar conversões é bug.

## Regime de bytes (`.gitattributes -text`)

Arquivo cuja identidade é **byte-a-byte** (hash de baseline, round-trip `.prst`,
comparação de replay) entra no `.gitattributes` com **`-text` ANTES do
primeiro commit** — EOL faz parte dos bytes, e a normalização de EOL do git
quebra a identidade silenciosamente (disco CRLF × blob LF; mordido 2x na CI
em 29/09: golden e .prst).

- Classes na lista (ver seção comentada no `.gitattributes`):
  1. especificação executável (`docs/referencias/protocol_golden.json`);
  2. patches de referência (`files/patches/*.prst` — round-trip);
  3. evidência de campo (`docs/arquivo/analysis/captures/*.jsonl`, `ir_slot*.bin`);
  4. fixtures de replay (`packages/core/data/fixtures/*.jsonl` — o replay
     compara byte a byte).
- **NÃO** entra: produto editável/regenerável (docs, scripts, knob_map,
  parameters.json — identidade semântica, não byte).
- Ao tocar na lista: `git add --renormalize .` e conferir
  `git show :<caminho> | sha256sum` × `sha256sum <caminho>`.
- Teste que depende de bytes de arquivo (round-trip/replay) deve ler o arquivo **do
  repo** (checkout limpo) — nunca de cópia local fora do git.

## Testes (modelo híbrido, análogo ao pytest)

### Regra de organização (EXPLÍCITA — a pergunta que sempre volta)

**É correto — e esperado — existir teste dentro de `src/`.** Não é dívida
nem inconsistência a "limpar". A divisão obrigatória:

1. **Unitários → `#[cfg(test)] mod tests` DENTRO do módulo em `src/`**:
   servem para internals e peças privadas. Um `tests/x.rs` compila como
   crate EXTERNO e só enxerga o que é `pub` — internals são INVISÍVEIS de
   lá; testá-los fora exigiria torná-los pub (piora a API). `#[cfg(test)]`
   não entra no build de release (zero custo) e a CI cobre os dois lados
   no mesmo `cargo test`.
2. **Contratos → `packages/core/tests/<assunto>.rs`** (crate `gp100-core`;
   caixa-preta, só API
   `pub`): provam comportamento contra EVIDÊNCIA de campo — golden,
   fixtures, round-trip `.prst`. Cada arquivo é um binário de teste; helpers
   em `tests/common/mod.rs` (≈ conftest.py; o nome `mod.rs` evita binário).
   Rodar um: `cargo test --test codec_wire`.
3. **Regra de decisão:** usa só API pública → `tests/`; precisa de
   internals → `src/`. NÃO mover unitário para `tests/` "por organização":
   perde o acesso sem ganhar nada. Teste novo acompanha código novo
   (padrão de DoD das issues).

### Exemplo vivo (codec de fio)

```text
packages/core/            # crate gp100-core
├── src/
│   ├── lib.rs                  # SEM unitários: consts/erros/envelope — os 3
│   │                           #   migraram p/ tests/wire_envelope.rs
│   └── codec.rs                # 7 unitários #[cfg(test)]: primitivas
│                               #   nibble strict, rejeição de slot,
│                               #   vetores mínimos (Bog RedM @ 15.0)
│                               #   — usam internals (addr4, name_field,
│                               #   envelope), invisíveis de tests/
└── tests/                      # caixa-preta, só API pub
    ├── common/
    │   └── mod.rs              #    ≈ conftest.py (mod.rs evita binário)
    ├── codec_wire.rs           # 4 contratos vs fixtures BYTE A BYTE:
    │                           #   92 knobs + 2 saves + 1186 frames IR
    ├── golden_consumer.rs
    ├── model_dictionary.rs     #    ≈ tests/test_model_dictionary.py
    ├── roundtrip_prst.rs
    └── wire_envelope.rs        # envelope §13.1 (ex-unitários do lib.rs)
```

- A prova PESADA contra fixture é SEMPRE contrato em `tests/` (arquivo
  lido do repo via `CARGO_MANIFEST_DIR` → raiz; regime de bytes).
- Dentro de `src/` fica só o vetor MÍNIMO que documenta a regra local do
  módulo — se um unitário precisa da fixture inteira, é sinal de que é
  contrato e deve migrar para `tests/`.

### Mapa pytest → Rust

| pytest | Rust |
|---|---|
| `tests/test_x.py` importa o pacote | `tests/x.rs` usa `gp100_core::…` (API pub) |
| `conftest.py` / fixtures | `tests/common/mod.rs` |
| `pytest tests/test_x.py` | `cargo test --test x` |
| acesso a internals | só no `#[cfg(test)] mod tests` dentro de `src/` |
| `@pytest.mark.parametrize` | `#[test]` + loop (ou crate `rstest`) |

- **Doc-tests**: exemplos de doc-comments executáveis (documentação viva);
  regras de framing/endianness viram doc-tests.
- Vetorizados com os exemplos do golden / fixtures; propriedade
  mínima: "gerado casa com o próprio exemplo do template".

## Armadilhas deste host

Toolchain pinada, PATH do cargo, `python3`/uv, console cp1252, EOL e CJK acidental:
**fonte única** é `knowledge.md` §"Armadilhas deste host" (leia antes de brigar com o
ambiente). Aqui fica só o comando de sessão do bloco acima.
