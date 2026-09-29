---
name: rust-practices
description: Boas práticas de Rust para gp100-core/gp100-cli — gates de qualidade (fmt/clippy/test/doc-tests), estilo de documentação PT-BR, erros tipados, testes vetorizados e política de features.
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
# (uma vez, como admin); workaround de sessão = export abaixo.
export PATH="/c/Users/Canta/.cargo/bin:$PATH"
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test          # inclui os DOC-TESTS (exemplos de doc-comments compilam e rodam)
cargo build
```

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

## Testes

- Teste novo acompanha código novo (padrão de DoD das issues).
- Vetorizados com os exemplos do golden; propriedade mínima: "gerado casa com
  o próprio exemplo do template" (M0.3).
- Regras de framing/endianness viram doc-tests (documentação viva).

## Armadilhas deste host (ver knowledge.md)

- Toolchain PINADA `stable-x86_64-pc-windows-gnu` em `rust-toolchain.toml` — o
  host NÃO tem MSVC Build Tools e o `link` do PATH é o GNU coreutils; não
  trocar de alvo nem apagar o arquivo.
- PATH do cargo: se terminal novo não rodar `cargo`, fix permanente =
  `scripts/add_cargo_path.ps1` como admin (uma vez); workaround de sessão =
  `export PATH="/c/Users/Canta/.cargo/bin:$PATH"`.
