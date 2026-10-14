---
name: core-dev
description: Implementação nos crates gp100-core e gp100-cli (packages/core e packages/cli) em Rust na Era Hardware — o aparelho real é a fonte (ADR-13), round-trip .prst sagrado, mock por default nos testes, travas de escrita. Use ao executar issue de core/cli (uma por vez), dentro das decisões de docs/dev/DECISIONS.md.
metadata:
  category: development
---

# Desenvolvimento do gp100-core (Rust)

Use para executar as issues de core/cli (`packages/core`, `packages/cli`) no GitHub (milestone `v1.1.0`),
UMA por vez, na ordem — o ciclo issue→branch→PR→merge é a skill `github-flow`,
e o formato de execução canônico é o prompt `.agents/prompts/implementar-issue.md`.

## Regras de ouro (não negociáveis)
1. **Nunca adivinhar protocolo**: todo byte vem de `docs/referencias/protocol_golden.json`
   (baseline com hash no §13 do PROTOCOL.md) e das narrativas §13.10–13.12.
   Premissa ausente? Parar e registrar a dúvida — nunca "resolver" com palpite.
   A premissa de campo é da skill `hardware-dev` (procedimento e travas lá;
   aqui só se consome o byte já medido).
1a. **Fonte dinâmica vs. snapshot**: em runtime o app valida CONTEÚDO contra o
   schema dos objetos (`preset_pages`, `codec`) — nunca contra bytes do golden ou
   do `all.prst` (isso é snapshot de análise, só serve em teste). O `wait_for`
   valida só a FORMA (`match_shape`: endereço + comprimento/by-len); o nome do
   preset termina no primeiro NUL (rabo stale após o NUL é legítimo).
2. **Golden é spec de FORMA, não fonte de valor**:
   `docs/referencias/protocol_golden.json` descreve o FORMATO do fio; cada espaço de VALOR
   (default de knob, faixa, semântica) tem o próprio modelo/teste, com a evidência
   de campo que a skill `hardware-dev` exige (precedentes: knobs #110, pp #148).
   Mudar a baseline é processo raro e provado.
3. **Round-trip sagrado**: o modelo `.prst` só está pronto quando os 3 arquivos
   de `files/patches/` regeneram byte-idênticos.
4. **Aparelho no app, mock no teste (#165)**: no `gp100-ui` a feature
   `real-device` é **default** (`tauri dev` abre o GP-100; `--no-default-features`
   = app sem transporte); o CLI segue mock por default e exige
   `--real --i-know-what-im-doing` para hardware, com `--mock-device` de selo de
   teste (política de hardware VISION §7).
5. **Qualidade**: `cargo clippy -- -D warnings` + `cargo test` verdes a cada issue;
   teste novo sempre acompanha código novo (onde cada teste vive: skill
   `testing`).
6. **Encerramento pelo ciclo da skill `github-flow`** — é ela que manda em
   PR/merge/issue; marco com CI vermelha não é marco.
7. **Commit segue o padrão que a CI cobra** — formato, exemplos e footer vivem na
   skill `github-flow`, fonte única do ciclo; o labelling antigo de fase (prefixo
   de marco antes do assunto) NÃO vale mais.
8. **Modo real não se adiciona fora do gate**: subcomando que "precisaria" de
   device real = parar e registrar ADR/ROADMAP — nunca flag ad-hoc nem bypass. A
   política (as duas camadas e o anti-brick) é da skill `hardware-dev`.

## Checklist por issue
1. Ler a issue no GitHub (DoD define "pronto", não o feeling); branch/PR pela
   skill `github-flow` (fonte única do ciclo issue→PR→CI→merge).
2. Ler as **ADRs** (`docs/dev/DECISIONS.md`) antes de escolher estrutura/API.
3. Implementar + testes vetorizados com exemplos do golden.
4. `cargo fmt && cargo clippy -- -D warnings && cargo test` — gates e estilo
   detalhados em `.agents/skills/rust-practices/SKILL.md` (doc-comments PT-BR
   com evidência, doc-tests, `deny(missing_docs)` no core, sem unwrap na lib).
5. Fechar pelo ciclo da skill `github-flow`; SÓ ENTÃO sincronizar `knowledge.md`
   (1 linha) e o status da fase no ROADMAP pela skill `docs-sync`.

## Estrutura alvo do workspace
```
packages/core/       # crate gp100-core — lib: model, preset, golden, codec,
                     #      transport, session
packages/cli/        # crate gp100-cli — bin: subcomandos info/list-user-irs/dump-preset/...
packages/library/    # crate gp100-library — SQLite da biblioteca persistente (ADR-9, #26)
packages/app/api/    # crate gp100-ui — ponte Tauri 2 (skill `app-dev`)
rust-toolchain.toml  # pin de toolchain (ADR-8)
```

## Armadilhas conhecidas
- effectCode no fio = u32 **LE** (b0=LSB do índice, b3=nibble); nos .prst é decimal.
- pp/PG/CRC no fio = **BE**; float32 de valor = **LE**.
- Payloads de objeto (set-param, IR) = **nibble-expandidos**, hi-nibble primeiro.
- **`state_page(pg)` devolve `raw[3] == PG+1` (§13.10)**: um laço 0..8
  alimentando `preset_pages::decode` derruba com "página fora de ordem:
  esperada 0, achada 1" — o slot 0 vem do `open`, não do req de página.
  Quem relê as 9 páginas em ciclo usa `Session::reload_paginas_correntes`
  (a sequência do boot; #164/ADR-15, `PROTOCOL.md` §13.15).
- Log de captura é append-only e tem truncamentos SEM-HDR (ring buffer) — as
  fixtures de `packages/core/data/fixtures/` já tratam isso; não "corrigir" no codec.
- Armadilhas deste host (PATH, toolchain, `python3`/uv, EOL, porta MIDI): a
  **fonte única** é `knowledge.md` §"Armadilhas deste host" — não redescubra no
  meio da issue.
- Movimentação/organização de testes: regra explícita na skill `rust-practices`.
