> 📜 **histórico** · registro de 15/10/2026 · **Leia com:** o §6 do [`INDEX`](../INDEX.md) (a
> régua de hoje, medida em `develop`) e os documentos que citavam estes números — eles agora
> **apontam para cá** em vez de repetir o contador.

# Medições datadas que saíram dos documentos vivos (#270)

**Por que este arquivo existe.** A regra 7 da skill `docs-sync` — e a convenção 3 da §4 do
[`INDEX`](../INDEX.md) — diz que **contador vive no §6 do INDEX**. O gate
`.github/scripts/check_docs.py` (issue [#270](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/270),
PR #271) transformou a regra em juiz; como o repo já tinha números de diagnóstico espalhados,
ele entrou no ar com **25 contadores congelados no baseline** — e um baseline é depósito de
dívida invisível, não conserto: enquanto o número mora no doc vivo, ele envelhece em silêncio
(foi o que aconteceu entre o §6, que dizia **18** ícones, e o `knowledge.md`, que dizia **16**).

O dono mandou a dívida ser paga (15/10/2026). O critério, um a um: **medida de hoje vai para o
§6** (a régua, com o comando); **medição datada vem para cá** (evidência com data, o gênero que
a §4.5 do [`INDEX`](../INDEX.md) já define para `docs/analises/`). Nada foi apagado — o
`BASELINE_CONTADOR` nasceu **vazio** e os 25 números estão abaixo, com a origem.

## 1. Diagnóstico da virada (medido 09/10/2026)

Origem: `docs/planejamento/ROADMAP.md` §1.1 — o mesmo retrato está no corpo do épico
[#171](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/171), que é a medição
publicada na data.

| Sintoma | Medição de 09/10 |
|---|---|
| Suíte inflada | Rust **309** testes em **36** suítes; front **513** em **38** arquivos; e2e **90** em **12** arquivos, **96** baselines; pytest **92**; gates **13** |
| Repo espalhado | `docs/` com **30** arquivos sem categoria (além de `docs/arquivo/analysis/` com 60+ itens, `scripts/` com 22 itens e `files/` com ~90 MB locais) |
| CI fora da ordem | **26** jobs num workflow só |

## 2. Rodadas de teste manual da UI (set/out, era RE)

Origem: `docs/dev/UI_TEST_PLAN.md`, nos blocos `EXECUÇÃO — #NN (data)` de cada rodada.

| Rodada | Data | Medição |
|---|---|---|
| #11 LibraryPanel (a biblioteca comanda o pedalboard) | 02/10 | **144** unit (**12** arquivos) · **77** e2e · **89,9%** stmts / **86,6%** branch (gate 85) · **18** baselines win32 regeneradas |
| #19 Effects list (trocar o efeito dentro do pedal) | 02/10 | **155** unit (**13** arquivos) · **78** e2e · **90,3%** stmts / **86,5%** branch (gate 85); `effects.ts` e `PedalModal.tsx` em 100% de linhas |
| U-6/U-7 (regressão visual + smoke do shell Tauri) | 30/09 | **15** baselines em `visual.spec.ts` (board/looper/biblioteca + erro-boot/erro-preset × 1440/1280/1024) |
| Gate de coverage 85% (Vitest + @vitest/coverage-v8) | 01/10 | unit **27 → 77**; branches **81,6%** medido, sem gate |
| Página em viewport única (palco/rodapé chassis) | — | **48** baselines regeneradas · **71/71** e2e |
| Rodada do looper (deck em `<svg>`) | — | **141** unit · **53** e2e funcionais · cobertura **90,1%/85,9%** |
| #30 i18n pt-BR/en/es/zh | 03/10 | **178** unit (**15** arquivos, +14 de i18n) · **78** e2e · cobertura **91,7%/86,7%** (gate 85%) |

## 3. Números de prosa (medições pontuais)

| Onde era citado | Medição | Data/origem |
|---|---|---|
| `docs/CONTRIBUTING.md` §4b | **14** testes de CLI **em cada modo** (`cargo test --workspace` roda os dois) | 14/10/2026 |
| `docs/CONTRIBUTING.md` §5 (workflows reutilizáveis) | o `ci.yml` tinha **1777** linhas e **29** jobs antes da #245 | 13/10/2026 (antes da #245) |
| `docs/dev/DECISIONS.md` ADR-8 (Dependabot) | CI dispara em cada PR com **14** jobs, cache quente | época do ADR (09/2026) |
| `docs/dev/UI_PLAN.md` M1.1 | DeviceActor D8 provado por **10** testes do crate `gp100-ui` na CI 3-OS | 30/09/2026 |
| `docs/referencias/REAL_DEVICE_GAP.md` (cobertura do CoreMIDI) | run `37389032681` com **48** jobs, zero vermelhos | 05/10/2026 (passo 6b) |
| `docs/referencias/UI_REFERENCE.md` §7.8/§7.9 | a #25 renovou **18** baselines de `board-*`/`lib-*` | 02/10/2026 |

## 4. Como usar este registro

- **Contador de hoje** → §6 do [`INDEX`](../INDEX.md): lá o número vem com o **comando** e a
  **data**, e é medido em `develop` (regra 9 da `docs-sync`).
- **Contador datado** (de uma rodada, de uma run, de uma era) → **aqui**, com a data ao lado.
  No doc vivo ele envelhece sem avisar; aqui ele é história com proveniência.
- **Contador novo em doc vivo** → o gate reprova. Ou o doc conserta, ou o número entra neste
  arquivo (com a data) e o doc vivo aponta para cá.
