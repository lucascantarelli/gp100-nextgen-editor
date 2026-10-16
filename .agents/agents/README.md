# `.agents/agents/` — os papéis da equipe (harness v2)

> **Status:** ✅ atual (15/10/2026) · **Leia antes:** `knowledge.md` → `.agents/README.md` → este arquivo.
>
> **A frase que resume o desenho:** **papel é arquivo LIDO, não agente CARREGADO.** Nada aqui
> depende de o cliente carregar um agente: depende de `read_files`, que existe em qualquer
> harness. É o que torna este diretório útil hoje e no dia em que o cliente suportar agentes
> de projeto.

## 1. O que o cliente carrega — MEDIDO, não suposto (15/10/2026)

**O cliente NÃO carrega agente definido no projeto:** as três sondas foram recusadas
(`.agents/<id>.ts`, `.agents/agents/<id>.ts`, `.agents/agents/<id>.md`) e a única spawnável é a
**injetada** (`researcher-web`, até 6 em paralelo), junto dos MCPs `github`, `playwright` e
`filesystem`. A medição inteira — sondas, ferramentas de cada MCP, a prova de que o arquivo não
estava quebrado e os limites declarados — está em
[`docs/analises/harness-v2-limite-medido-2026-10-15.md`](../../docs/analises/harness-v2-limite-medido-2026-10-15.md).

## 2. O que isso decide (o contorno)

O contorno não é "esperar o cliente": é **tirar do cliente toda a responsabilidade pela equipe**.

| O que a equipe precisa | Onde mora (funciona hoje, em qualquer harness) | Não depende de |
|---|---|---|
| Quem faz o quê | estes contratos de papel (arquivos) | cliente carregar agente |
| Como se faz | `.agents/skills/*/SKILL.md` (conhecimento, sob demanda) | — |
| Ordem de execução | `.agents/prompts/*` (entradas canônicas) | — |
| Estado da demanda | **labels `flow:*` no GitHub** + comentário de handoff | cliente |
| Prova de que ficou pronto | `.github/scripts/gates.py` + CI | cliente |
| Fato externo citado | agente injetado `researcher-web` (o único disponível) | — |
| GitHub (issue, PR, review, checks) | MCP `github`; `gh` CLI é alternativa | `gh` instalado |
| Interface do dono (front no navegador) | MCP `playwright` | — (a janela Tauri nativa NÃO é alcançável por MCP) |
| Paralelismo | **`git worktree`** (uma árvore por issue) + processos em background | subagentes de código |

**Consequência aceita e escrita:** o *revisor independente* hoje é independência **por contrato**
(não escreveu o código, não pode editar nada, olha o diff e a issue), não por modelo distinto —
porque **não existe** subagente de código para spawnar. Está registrado para ninguém achar que
há uma segunda mente no circuito.

## 3. Os papéis

Fatia vertical (o que existe agora) — cada contrato diz quando entra, o que lê, o que devolve e
o que **não** faz:

| Papel | Estado (`flow:*`) | Ferramentas mínimas | Devolve |
|---|---|---|---|
| [`triagem.md`](triagem.md) | — → `flow:triagem` → `flow:plano` → `flow:pronto` | `gh`/MCP github (issue+labels), `code_search`, `read_files`, `web_search` | issue auto-suficiente (template + 🤖 prompt) e o estado seguinte |
| [`implementador.md`](implementador.md) | `flow:pronto` → `flow:execucao` | `read_files`/`write_file`/`str_replace`, `run_terminal_command`, `gh` | branch + PR com tabela de gates |
| [`revisor.md`](revisor.md) | `flow:revisao` | `read_files`, `code_search`, `gh` (diff/checks); **nenhuma** ferramenta de escrita | review por severidade + veredito |
| [`verificador.md`](verificador.md) | `flow:verificacao` → `flow:integrado` | `run_terminal_command`, `gh` (checks/runs) | bloco de evidência (comando → saída, contagens, links) |

**Segunda onda** (contrato próprio, um PR por papel, herdando este molde): `arquiteto` (ADR/§4.1,
fronteiras de módulo) · `seguranca` (SAST, dependências, segredos, hardening de workflow) ·
`devops` (release, artefatos, cache/custo) · `docs` (`INDEX`/`knowledge`/`ROADMAP` + `docs-sync`) ·
`performance` (latência MIDI — #239/#240) · `campo` (gate H5, submetido à trava de escrita e à
skill `hardware-dev`).

**Fronteira papel × skill (não duplicar):** o papel diz **quem faz e o que devolve**; a skill diz
**como se faz**. Se você está escrevendo regra de área num contrato de papel, ela está no lugar
errado — a regra transversal tem **uma casa** (tabela em `.agents/README.md`) e o papel **aponta o
caminho** dela.

## 4. Workflow (o ciclo, com o cliente que temos)

```
demanda → triagem (issue + labels + 🤖 prompt)
        → flow:pronto → implementador (branch, PR, gates locais)
        → flow:revisao → revisor (veredito; 1 correção automática se `muda`)
        → flow:verificacao → verificador (CI verde + bloco de evidência)
        → flow:integrado (squash) → docs-sync
        … qualquer etapa pode terminar em flow:bloqueado (com o impasse na issue)
```

- **Handoff (formato único, colado como comentário na issue):** `ESTADO` · `ARTEFATOS` ·
  `COMANDOS + SAÍDA` · `DECISÕES` · `BLOQUEIOS` · `PRÓXIMO PASSO`.
- **Fila:** `priority:high` → `medium` → `low`; épica só a pedido; descoberta vira issue **na hora**.
- **Autonomia:** o papel com `flow:verificacao` verde **integra (squash) e registra o merge** —
  exceto nos **carve-outs** (`.github/**`, `.agents/**`, `AGENTS.md`, `DECISIONS.md`, release/tag/
  `main`, segredos, migração destrutiva, escopo de produto novo) e no que estiver marcado
  `guard:humano`.
- **Anti-loop:** **1 correção** por falha; erro repetido ou diff inalterado ⇒ `flow:bloqueado` e
  o impasse vai para a issue.

## 5. Quando o cliente carregar agentes de projeto

Se um dia as sondas da §1 passarem a devolver ✅, o adaptador é **fino** — um arquivo por papel,
sem duplicar conhecimento:

```ts
// .agents/<id>.ts — adaptador do papel `revisor` (hoje NÃO é carregado; ver §1)
{
  id: 'revisor',
  displayName: 'Revisor',
  toolNames: [
    'read_files', 'code_search', 'run_terminal_command',
    'github/list_pull_requests',   // MCP entra qualificado: servidor/ferramenta
  ],                                // escrever arquivo NÃO entra: este papel não edita
  mcpServers: { github: { /* config de .agents/mcp.json */ } },
  spawnerPrompt: 'Spawn no estado flow:revisao de uma issue deste repo.',
  systemPrompt: 'Leia .agents/agents/revisor.md e siga o contrato; não edite arquivo nenhum.',
  instructionsPrompt: 'Contrato: .agents/agents/revisor.md · skills: .agents/skills/{github-flow,testing,rust-practices}.',
}
```

A regra: **o `.ts` não carrega conhecimento**, só aponta (`systemPrompt` + caminhos). Assim o dia
em que o cliente suportar agentes, os contratos de hoje viram o `systemPrompt` de amanhã — e nada
precisa ser reescrito.

## 6. Execução paralela — `git worktree`

Substituto do paralelismo por subagente, que este cliente não oferece (§1). O repo **já
usa** a prática (`git worktree list` mostra as árvores irmãs criadas em sessões passadas).

```bash
git worktree add "../gp100-231" -b fix/231-mapa-ir-valor develop   # uma árvore por issue
uv sync --all-groups                       # deps Python POR ÁRVORE (não são compartilhadas)
pnpm --dir packages/app/ui install          # o lockfile vive em packages/app/ui
git worktree remove ../gp100-231            # árvore é descartável; a branch não (só após o merge)
```

O dev server da árvore precisa de **porta própria** (o default do Vite é 5173, a do checkout
principal): `--port`/`--strictPort` são flags do Vite e o `dev` do front é `vite` puro
(`packages/app/ui/package.json`), sem wrapper que as descarte.

O que evita o retrabalho que o paralelismo cria: **um dono por arquivo** (dois PRs no mesmo
arquivo = conflito no squash: paralelize por assunto disjunto, não por "tarefas pequenas");
**carve-out não paraleliza** (`.github/**`, `.agents/**`, release: serial, com o dono); **cada
árvore tem o seu `target/` do Rust** (o 1º `cargo test` de cada árvore paga o build inteiro: o
Rust pesado vai em série); e **o gate lê o repo onde roda** (`python3 .github/scripts/gates.py`
na árvore errada é evidência do trabalho errado).
