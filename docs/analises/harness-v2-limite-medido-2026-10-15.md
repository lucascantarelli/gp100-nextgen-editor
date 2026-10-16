# 🔎 MEDIÇÃO — o que o cliente carrega (harness v2) — 2026-10-15

> ✅ **atual** (medição, 15/10/2026) · **Leia com:** [`docs/INDEX.md`](../INDEX.md) §6 (contadores)
> e [`docs/analises/audit-harness-2026-10-14.md`](audit-harness-2026-10-14.md) (os 12 achados do harness).
> **Escopo:** o que este cliente injeta (agentes e MCPs) e o que ele recusa carregar do projeto.
> **Método:** quatro sondas medidas em sessão, com o formato que a doc oficial publica — não
> inferência, não leitura de manual.

## 1. O que o cliente carrega — MEDIDO, não suposto (15/10/2026)

Antes de desenhar a equipe, a pergunta que decide a arquitetura: **este cliente carrega agente
definido no projeto?** Testado com quatro sondas, no formato que a documentação oficial do
Codebuff (a base do Freebuff) publica:

| Layout testado | id da sonda | Resultado medido |
|---|---|---|
| `.agents/<id>.ts` (o caminho documentado) | `poc-pesquisador` | ❌ `Agent "poc-pesquisador" is not available to spawn` |
| `.agents/agents/<id>.ts` | `poc-pesquisador-dir` | ❌ `not available to spawn` |
| `.agents/agents/<id>.md` (layout de subagente Markdown) | `poc-pesquisador-md` | ❌ `not available to spawn` |
| agente **injetado pelo cliente** | `researcher-web` | ✅ única spawnável · até **6** em paralelo por chamada |

**O que o cliente INJETA (e aí sim cross-sessão):** agentes (`researcher-web` — o único) e **MCPs**
conectados ao repo: `github` (26 ferramentas: `create_issue`, `list_issues`, `create_pull_request`,
`create_pull_request_review`, `get_pull_request_status`…), `playwright` (25: `browser_navigate`,
`browser_snapshot`, `browser_take_screenshot`, `browser_console_messages`…) e `filesystem`. Chamada
de prova: `github.list_issues(labels=["flow:pronto"])` → **funcionou** (resposta `[]`). É por aí que a
nossa equipe toca o GitHub quando o `gh` não existir na máquina.

**O que estava correto nas sondas** (para não confundir "não carrega" com "arquivo quebrado"):
o registro TypeScript da sonda era válido (`esbuild --format=esm` → exit **0**), com os
campos exatamente como a doc publica (`id`, `displayName`, `model`, `toolNames`, `spawnerPrompt`,
`systemPrompt`, `instructionsPrompt`). O arquivo é preservado como registro da forma em
[`docs/arquivo/harness-poc-agente-projeto.ts`](../arquivo/harness-poc-agente-projeto.ts).

**Limites da medição, ditos de propósito:**
- o cliente pode varrer `.agents/*.ts` **só no start** — as três recusas valem para *esta sessão*;
  reiniciar o app é o teste que fecha a questão (a doc de skills do próprio produto manda
  reiniciar quando uma skill não aparece);
- a documentação do **Freebuff** não afirma em lugar nenhum que o Desktop carrega agente de
  projeto (quem documenta a forma é o **Codebuff**; o `/init` dele cria `.agents/types/`, que
  **não existe** neste repo);
- o `model` do agente é um **ID opaco** no estado deste cliente (`"model": "m-096e75164d"` em
  `~/.config/freebuff-desktop/state.json`) — não há slug público para espelhar.

## 2. O que a medição decide

O contorno não é "esperar o cliente": é tirar do cliente toda a responsabilidade pela equipe —
papel é **arquivo lido** (`.agents/agents/`), estado da demanda é **label `flow:*` no GitHub**, e
paralelismo é `git worktree`. A consequência aceita está escrita em
[`../../.agents/agents/README.md`](../../.agents/agents/README.md) §2 e §5: o *revisor
independente* é independência **por contrato**, não por modelo distinto, porque não existe
subagente de código para spawnar.
