---
name: testing
description: O que a suíte da Era HW testa e como — unidades vs contratos em Rust (mock por default, evidência de campo), vitest + gate de cobertura 85%, e2e Playwright (shell, visual, webview real), round-trip .prst e o pytest de spec que morre com HW-3. Use ao escrever, revisar ou diagnosticar teste em qualquer camada, e para decidir onde um teste novo vive.
metadata:
  category: quality
---

# Testes (Era Hardware)

Complementa `rust-practices` (organização dos testes Rust) e `ui-ux-practices`
(gates do front). A régua do projeto: **teste novo acompanha código novo** — é
padrão de DoD de toda issue, não sugestão.

## Onde cada teste vive (regra de decisão)

| Camada | Unidade (interno) | Contrato (caixa-preta) |
|---|---|---|
| Rust | `#[cfg(test)] mod tests` dentro de `src/` — internals só visíveis de lá | `packages/core/tests/<assunto>.rs` — API `pub` contra EVIDÊNCIA (fixtures, golden de FORMA, round-trip) |
| UI | `packages/app/ui/tests/*.test.tsx` (vitest + jsdom) | idem — papel/label/foco/estado de tela |
| e2e | — | `packages/app/ui/e2e/*.spec.ts` (Playwright) + `tauri.smoke.mjs` (webview real) |

Nunca mova unitário para `tests/` "por organização": `tests/` compila como
crate externo e não enxerga internals (detalhe na skill `rust-practices`).

## O que a suíte Rust prova (mock por default)

- O transporte padrão é o **`MockDevice`** (`packages/core/src/transport/mock.rs`)
  — determinístico, sem hardware. `RealDevice` só entra com a feature
  `real-device` (ver skill `hardware-dev`).
- Contratos pesados em `packages/core/tests/`: consumo do golden de FORMA
  (`golden_consumer`), gate de escrita (`write_gate` — a trava ADR-5 como
  feature de compilação), boot (`boot_mock`), round-trip `.prst` byte-idêntico
  (`roundtrip_prst` — regra R4), guarda de dados canônicos
  (`guard_dados_canonicos`), envelope/framing (`wire_envelope`), páginas de
  preset (`pp_doc`, `pp_decimal`).
- Gates: `cargo fmt --check` · `cargo clippy --workspace --all-targets --
  -D warnings` · `cargo test` (inclui doc-tests) — ver `rust-practices`.

## Front (vitest + Playwright)

- `pnpm test` (vitest/jsdom): componentes, tokens, a11y (papel/label/foco),
  i18n, hooks; **`pnpm test:coverage` é GATE de 85%** statements/functions/lines
  (`packages/app/ui/vite.config.ts`) — roda em job próprio (`Cobertura · UI`).
- e2e Playwright roda do repo (fonte dos roteiros: `docs/UI_TEST_PLAN.md`,
  declarado no `playwright.config.ts`):
  - `shell.roteiros.spec.ts` · `estados.spec.ts` · `ipc.edge.spec.ts` — shell;
  - `visual.spec.ts` — baselines POR PLATAFORMA; comparar é o default (falha no
    que faltar); regenerar **só** pelo input `update-snapshots` do
    `workflow_dispatch` (`--update-snapshots=all` é o único caminho de escrita);
  - `e2e/tauri.smoke.mjs` — webview REAL (xvfb + `tauri-driver`, container
    `ci-linux`), prova o shell Tauri de verdade.
- Rodar local: `cd packages/app/ui && pnpm exec playwright test` (o config sobe
  o dev server). Não dependa de quebra de linha exata (fonte do SO muda
  métricas — ver `ui-ux-practices`).

## Pytest de spec (`uv run pytest`)

A suíte de `scripts/tests/` (casa desde #163) valida a especificação
reconstruída (golden, baseline, knob_map, faixas ADR-10, paridade de fixtures)
e os juízes de campo H1/H2 — o que ela cobre está em `docs/TESTING.md` §1.
**Não acrescente teste novo lá sem decisão registrada**: o que prova só o
processo de RE sai na auditoria da suíte (contrato da #166); dado de runtime
tem contrato em `packages/*/tests/`.

## Ao diagnosticar falha

1. Diga QUAL camada falhou (unidade, contrato, cobertura, e2e, visual, smoke) —
   cada uma tem gate e dono diferentes.
2. Nunca "conserte" ajustando expectativa para o comportamento novo sem
   confirmar com a fonte (aparelho/PROTOCOL/UI_TEST_PLAN): número que diminui
   pode ser honesto (lição da baseline v1.1 — 77 → 14 provas porque 63 mediam
   bytes fabricados).
3. Falha de baseline visual só vira ✅ com diff revisado e regeneração
   documentada na PR.
