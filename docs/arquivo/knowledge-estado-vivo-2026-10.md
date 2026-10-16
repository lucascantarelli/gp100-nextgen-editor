> 📜 **histórico** (arquivado 16/10/2026, #269) — os marcos **14/10 → 09/10** do "Estado vivo"
> do [`knowledge.md`](../../knowledge.md), que passou a guardar só os três mais recentes.
> Nada aqui é regra nem estado atual: é o registro de como o repo chegou ao estado de hoje.
> O estado vivo vigente é o do próprio `knowledge.md`.

## Marcos arquivados (14/10 → 09/10)

- 14/10 — **#243 (actor fatiado) entregue**: `packages/app/api/src/actor.rs`
  saiu de 2 702 linhas para um módulo (`actor.rs` = handle + spawn, sob o teto de
  600) + `actor/`: `estado` (o que a thread possui), `relogio` (loop com prazos +
  read-back), `fila` (`Request`), `tipos` (DTOs do IPC), `backends`,
  `{leitura,escrita,boot,diagnostico}` e `tests`. Comportamento idêntico: 27
  contratos do actor antes e depois, mesmos testes do crate (contagem no `docs/INDEX.md` §6). O par
  `sink`+`debounce` virou `Cadencia` (escrita marca o estado e avisa a tela
  junto) e o orçamento do crate ficou declarado em `docs/dev/ARCHITECTURE.md`
  §4.1 — a cobrança automática do Rust é a **#248**.
- 14/10 — **CI enxuta (#245/#246)**: Distribuição e Release saíram do
  `ci.yml` (1777 → ~1350 linhas) para 6 workflows `workflow_call`, com o
  `ci.yml` ficando com o grafo, o nome dos jobs e a matriz de OS. O `env:` de
  topo do caller NÃO atravessa o `workflow_call` (nem é contexto do `with:`) —
  runner/args vêm por `with:` e o resto (toolchain do CLI, `BOT_NAME`/
  `BOT_EMAIL`) mora no arquivo chamado. Check de job reutilizável que RODA sai
  como `<caller> / <job>`; `skipped` (todo PR) sai com o nome do caller, então
  required check de PR não muda. E o `simulate_release` (~2 min) só roda por
  escopo (`scope-ci`): schedule, tag/dispatch e PR de release.
- 13/10 — **#168 (`scripts/` dissolvida)**: CI/gates → `.github/scripts/`;
  spec/dados → `tools/` (suíte Spec em  `tools/tests/`); empacotamento →
  `packaging/`; RE → `docs/arquivo/scripts-era-RE/`. `uv run python
  .github/scripts/gates.py` roda a suíte de gates (contagem no INDEX §6). Armadilhas novas: root em
  `.github/scripts/` é `parents[2]` de `__file__`; `simulate_release` resolve
  o git-bash via `shutil.which` (System32/WSL precede o PATH) e traduz
  `python3`→`python` no sandbox Windows.
- 13/10 — **#167 (docs categorizada)**: `docs/` virou árvore (`dev/` `tecnico/`
  `planejamento/` `analises/` `referencias/` `arquivo/`) com TODOS os caminhos
  do repo atualizados (sed + link checker); `SECURITY.md` da raiz virou ponteiro
  para `docs/tecnico/seguranca.md`; golden bump v1.3 só de proveniência;
  `files/` triado no disco (instalador/DebugView/logs fora — 97→16 MB).
- 13/10 — **#164 (sync bilateral) entregue**: estado tipado único no actor
  (`idle/sync/pending/failed/offline`; `device_info` semeia e `device://sync`
  transiciona), read-back pós-escrita (debounce 800 ms) + refresh de 10 s pela
  sequência do boot (`reload_paginas_correntes` — `PROTOCOL.md` §13.15,
  ADR-15), chip na navbar com ação reler/reconectar e reconciliação "o
  aparelho vence". Limite honesto: knob físico não emite SysEx — é poll,
  não push.
- 13/10 — **regeneração de baseline virou workflow dedicado** (#204):
  `visual-baselines.yml` (`workflow_dispatch` sem inputs, `contents: read`)
  regenera as baselines `-linux`; o input `update-snapshots` saiu do `ci.yml`.
  Motivo: o `default: release` do input `action` fazia o dispatch de baseline
  cortar rc junto (runs 38070949933 e a do incidente rc.5 de 13/10 — revertido).
  Contrato dos 3 workflows de pipeline + dos 6 reutilizáveis de dist/release
  (#245) no `validate_workflows.py` (contratos 14/15).
- 12/10 — **Navbar enxuta (diretriz do dono)**: cluster de PATCH (◀ ▶ + nome)
  removido da navbar — troca vive na LISTA e o rótulo no display LED do palco;
  DeviceBadge sem miniatura SVG (dot `.live-dot`/`.idle-dot` + status honesto).
- 11/10 — **#170 (harness do agente) em execução**: skills da Era HW
  (`hardware-dev`, `testing`, `release` novas; 5 da Era RE arquivadas em
  `docs/arquivo/skills-era-RE/`), `.agents/prompts/` com 6 templates, templates
  de issue com contrato obrigatório (gate `check_issue_templates.py`) e este
  arquivo reescrito. O knowledge da era antiga ficou em
  `docs/arquivo/knowledge-era-RE_2026-10.md`.
- 11/10 — **#169 (CI na ordem do dono) ✅**: `ci.yml` com estágios em grafo
  (validação → lint → compilação → infra → segurança → testes → cobertura →
  distribuição → release), Relatório por último, Fechamento em
  `issues-fechamento.yml` próprio, Arch Linux na matriz via container.
- 10/10 — **`docs/referencias/GP100_DEVICE.md` é a fonte canônica do device** (vocabulário:
  User Patch ≠ User IR ≠ Setlist; fila de lacunas de leitura). O aparelho está
  ligado nesta máquina e é consultável por sessão (ver `hardware-dev`).
- 09/10 — **Era Hardware aberta** (épico #171, milestone v1.1.0); roadmap
  reescrito, era anterior arquivada em `docs/arquivo/`.
