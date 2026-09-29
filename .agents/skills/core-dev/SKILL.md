---
name: core-dev
description: Implementa as issues M0.1–M0.7 do gp100-core em Rust dentro das decisões de P5 (golden-file como insumo, round-trip sagrado, mock por default).
metadata:
  category: development
---

# Desenvolvimento do gp100-core (Rust)

Use para executar as issues M0.1–M0.7 do `docs/ROADMAP.md`, UMA por vez, na ordem.

## Regras de ouro (não negociáveis)
1. **Nunca adivinhar protocolo**: todo byte vem de `docs/protocol_golden.json`
   (baseline com hash no §13 do PROTOCOL.md) e das narrativas §13.10–13.12.
   Premissa ausente? Parar e registrar a dúvida — nunca "resolver" com palpite.
2. **Golden é insumo, não produto**: `docs/protocol_golden.json` é lido/embedado
   pelo core; mudar a spec é trabalho da skill `spec-baseline` + `capture-analyze`.
3. **Round-trip sagrado (R4)**: o modelo `.prst` só está pronto quando os 3 arquivos
   de `files/patches/` regeneram byte-idênticos.
4. **Mock por default**: `RealDevice` fica atrás de feature `real-device`;
   o CLI exige `--real --i-know-what-im-doing` (política de hardware VISION §7).
5. **Qualidade**: `cargo clippy -- -D warnings` + `cargo test` verdes a cada issue;
   teste novo sempre acompanha código novo (padrão de DoD da issue).

## Checklist por issue (M0.x)
1. Ler a issue no ROADMAP (DoD define "pronto", não o feeling).
2. Ler P5 (`docs/DECISIONS.md`) antes de escolher estrutura/API.
3. Implementar + testes vetorizados com exemplos do golden.
4. `cargo fmt && cargo clippy -- -D warnings && cargo test` — gates e estilo
   detalhados em `.agents/skills/rust-practices/SKILL.md` (doc-comments PT-BR
   com evidência, doc-tests, `deny(missing_docs)` no core, sem unwrap na lib).
5. Marcar ✅ na issue do ROADMAP (com data) e atualizar `knowledge.md` (1 linha).

## Estrutura alvo do workspace
```
gp100-core/          # lib: model (M0.1), preset (M0.2), golden (M0.3),
                     #      codec (M0.4), transport (M0.5), session (M0.6)
gp100-cli/           # bin: subcomandos info/list-user-irs/dump-preset/... (M0.7)
rust-toolchain.toml  # stable pinado (P2)
```

## Armadilhas conhecidas
- effectCode no fio = u32 **LE** (b0=LSB do índice, b3=nibble); nos .prst é decimal.
- pp/PG/CRC no fio = **BE**; float32 de valor = **LE**.
- Payloads de objeto (set-param, IR) = **nibble-expandidos**, hi-nibble primeiro.
- Log de captura é append-only e tem truncamentos SEM-HDR (ring buffer) — fixtures
  já tratam isso (P4); não "corrigir" no codec.
- Terminal novo sem cargo no PATH: fix permanente `scripts/add_cargo_path.ps1`
  (1x, admin) ou `export PATH="/c/Users/Canta/.cargo/bin:$PATH"` na sessão.
