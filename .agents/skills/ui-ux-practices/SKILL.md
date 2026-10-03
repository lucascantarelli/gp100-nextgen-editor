---
name: ui-ux-practices
description: Boas práticas de UI/UX para o front (ui/ do workspace, M1+) — design tokens (escala de Fibonacci sobre base 4, alinhamento 8pt), acessibilidade WCAG 2.2 (contraste AA, foco visível 2px/3:1), padrões de desktop (Tauri 2), gates de qualidade (pnpm lint/test/build) e a regra R1 aplicada ao front.
metadata:
  category: development
---

# Boas práticas UI/UX (ui/ — Tauri 2 + React/TS)

Use ao escrever ou revisar QUALQUER código de front do projeto (issues do
GitHub, milestone `v1.0.0`; fases UI no ROADMAP/UI_PLAN — branch/PR pela skill
`github-flow`). Complementa a skill `rust-practices` (gates do lado Rust) e os
ADRs de `docs/DECISIONS.md`. Fontes: [docs/UI_DESIGN.md](../../../docs/UI_DESIGN.md)
(tokens — FONTE ÚNICA de valores), [docs/UI_PLAN.md](../../../docs/UI_PLAN.md)
(arquitetura/issues) e as práticas citadas em cada seção (WCAG 2.2, Fitts,
Miller, escala 8pt/Fibonacci).

## R1 no front (a UI nunca fala protocolo)

1. Nenhum componente importa `gp100-core` direto — a única porta de entrada é
   o IPC do Tauri (`invoke("…")` + eventos). Lint de IMPORT bloqueia violação.
2. Nenhum byte de protocolo, endereço, hex de template ou timing no front:
   o que chega do backend são TIPOS (DTOs gerados/espelhados em TS) e estados
   ("conectado", "preset carregado"), não frames.
3. `invoke` centralizado em `ui/src/ipc/` (um módulo por domínio: device,
   presets, ir) — componentes nunca chamam `invoke` espalhado.

## Design tokens (FONTE ÚNICA = docs/UI_DESIGN.md + ui/src/design/tokens.ts)

- **Escala de Fibonacci sobre base 4** para ESPAÇOS: 4 · 8 · 12 · 20 · 32 · 52
  · 84 (F4–F13, arredondados ao múltiplo de 4). Nada de valor "meio termo"
  fora da escala (o linter de design.css bloqueia px avulsos em
  padding/margin/gap — usar `var(--space-N)`).
- **Cores**: todo par foreground/background citado na UI atinge contraste
  **AA 4.5:1** (texto) / **3:1** (UI e gráficos, WCAG 1.4.11); tokens
  documentados com os rácios medidos. Nunca cor como ÚNICO indicador de
  estado (ícone/texto junto — WCAG 1.4.1).
- **Foco visível (WCAG 2.4.13)**: todo foco de teclado = outline **2px** com
  contraste **3:1** contra o entorno + `:focus-visible` (nunca remover outline
  sem substituto). Token `--focus-ring`.
- **Alvo de toque**: mínimo **32×32 CSS px** para tudo clicável (desktop:
  acurácia do mouse + touch em telas híbridas; Fitts — alvos grandes e nas
  bordas valem mais).
- **Tipografia**: escala modular (1.125) sobre 13px base de UI; corpo de texto
  longo nunca abaixo de 12px; altura de linha ≥1.5 em texto corrido.
- **Dark/light**: apenas os tokens trocam (tema = camada CSS); nenhum valor
  cru em componente.

## UX de desktop (padrões que o projeto segue)

- **Estados de TELA explícitos** (`idle/loading/empty/error/ready`) modelados
  como união TS em cada recurso — nada de spinner eterno ou tela branca;
  erro SEMPRE com ação de recuperação (reconectar, reabrir).
- **Operações longas** (boot, upload IR): progresso visível + botão de
  cancelar (quando o backend suportar); nada bloqueia a janela inteira.
- **Ação destrutiva** (salvar em cima de preset, apagar): dupla confirmação
  com RESUMO do efeito (espelha a política do CLI — VISION §7.2).
- **Teclado**: toda ação tem atalho; ordem de foco = ordem visual (WCAG
  2.4.3); `Esc` fecha modais; atalhos documentados na UI (rodapé/`?`).
- **Densidade**: painéis de knobs compactos (13px, espaçamento F4/F8), listas
  com 8pt; respiro de página F32.
- **Feedback <100ms** para hover/ativação (percepção de resposta imediata,
  Nielsen); transições ≤200ms e **respeitando
  `prefers-reduced-motion`**.

## Estrutura de código (ui/src)

```
design/       tokens.ts + design.css (tokens; nada de UI aqui)
ipc/          invoke + tipos TS por domínio (única porta p/ o backend)
components/   painéis e primitivos (TopBar, Pedalboard, Pedal, Knob, TunerPanel…)
hooks/        useBoot, useGlobalShortcuts, usePushLog (estado de sessão)
i18n/         messages.ts — NENHUMA string crua no JSX (pt-BR base; i18n = #30)
tuner/        motor do afinador (autocorrelação) — separado da UI
tests/        vitest (jsdom) · e2e/ spec do Playwright (roda do repo, na CI)
```

- Componentes **sem lógica de negócio**: recebem dados prontos e callbacks.
- Tipos TS de payload IPC em `ipc/types.ts` (espelhos manuais até o M1.0
  decidir ts-rs/specta — ADR pendente do plano).

## Gates de qualidade (toda mudança de front passa)

```bash
cd packages/app/ui
pnpm install --frozen-lockfile
pnpm tsc --noEmit  # type-check
pnpm lint          # eslint (inclui regra anti-invoke-espalhado e a11y)
pnpm test          # vitest: tokens, a11y de primitivos, estados de tela
pnpm test:coverage # GATE 85% statements/functions/lines (vite.config.ts)
pnpm build         # vite build (bundle; tsc já rodou acima)
```

- **A11y é gate**: primitivos têm testes de papel/label/foco; regressão de
  contraste/foco = falha de CI, não sugestão.
- Rust lado Tauri (`packages/app/api/`, crate `gp100-ui` — ADR-7): mesmos
  gates da `rust-practices` (fmt, clippy -D warnings, testes), no job
  `Lint`/`Compilação`/`Testes · Rust (… · ui-rust · windows-latest)`.
- CI é **um workflow só** (`.github/workflows/ci.yml`, #68). Os gates do front:
  `Lint · UI (eslint)` · `Compilação · UI (tsc + vite · <os>)` em 3-OS ·
  `Testes · UI (vitest)` · `Cobertura · UI (gate 85%)` · e os e2e
  (`Testes · E2E shell`, `· E2E visual`, `· E2E webview`). Não existe mais o
  `_validate.yml` nem jobs `front`/`ui-rust`/`e2e-visual` com esses nomes.

## Armadilhas específicas de Tauri 2 desktop

- `invoke` só existe dentro do webview — código rodando em teste de browser
  puro precisa de mock do `window.__TAURI__` (helpers em `ipc/mock.ts`).
- Janela redimensionável: layout tem de sobreviver de 800×600 até ultrawide
  (testar os 2 extremos; nada de largura fixa em px).
- Fonte do SO (Segoe UI/Inter) pode mudar métricas: nunca depender de quebra
  de linha exata; caixas com `min-width`, não `width` rígido.
- `prefers-color-scheme` define o tema inicial; a escolha do usuário persiste
  por cima (localStorage).
