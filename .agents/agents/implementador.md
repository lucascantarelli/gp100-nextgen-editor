# Papel: implementador

> Contrato de papel (`.agents/agents/README.md`). É o trabalho normal do agente da
> sessão: pega uma issue e entrega um PR.

- **Estado de entrada**: issue com `flow:pronto` (se não estiver pronto, devolve para
  triagem — não completa o escopo por conta própria). Ao começar: **`flow:execucao`**.
- **Estado de saída**: PR aberto contra **`develop`** com gates verdes, `Closes #N`
  **no corpo**, `flow:revisao`.
- **Autonomia**: branch, código, teste, commit, push, PR (MCP `github.create_pull_request`
  ou `gh pr create --base develop`). **Não** faz merge.
- **Skills**: carregar a skill da área antes de tocar código (`core-dev`, `app-dev`,
  `hardware-dev`, `ui-ux-practices`, `rust-practices`) + `testing`.

## Passo a passo

1. **Ler a issue inteira** (não só o prompt): escopo, fora de escopo, garantias de
   não-regressão e plano de teste são lei. Surpresas: escrever na issue e parar.
2. **Branch** a partir de `develop` (base do GitFlow do repo — `main` só recebe release):
   `<tipo>/<N>-<slug>` (ex.: `fix/231-mapa-ir-valor`).
3. **Teste primeiro quando o plano de teste permite**: escrever o teste que falha,
   ver falhar, implementar, ver passar. Guardar as duas saídas — é isso que vai no PR.
4. **Código no menor caminho que resolve.** Sem refatoração de vizinhança, sem
   dependência nova sem justificar na issue, sem renomear o que a issue não pediu.
5. **Rodar os gates LOCAIS da área** — a tabela por área está em `docs/CONTRIBUTING.md`
   §4 (e o índice curto em `AGENTS.md`). Se um falha,
   consertar a causa. **Proibido**: `--no-verify`, relaxar asserção, `#[ignore]`,
   `skip`, ou editar a expectativa do teste para o código passar.
6. **Commit convencional** (`fix`/`feat`/`refactor`/`docs`/`test`/`chore`), um assunto
   por commit. Se o commit só move código, declarar o trailer `Move-only:` (a regra
   é da skill `github-flow`) — sem o trailer o CI reprova o movimento.
7. **PR** para `develop` com o template preenchido: o que foi medido, tabela de gates
   **com a saída real**, risco e o que ficou de fora. `Closes #N` no corpo — e só no
   PR que conclui a issue.
8. **Mover o estado**: `flow:pronto` → `flow:revisao`. Se o CI ainda roda, esperar e
   colar o link do run no PR.

## Regras de reentrada (o que faz este papel repetir trabalho)

- Falha de CI que o implementador consegue reproduzir: corrigir e **um** push novo.
- `changes requested` do revisor: ler a lista inteira, corrigir tudo, um push.
- Depois de **duas** rodadas no mesmo portão: parar e escrever o diagnóstico na issue.
  Não empilhar tentativa sobre tentativa — é assim que a sessão come o orçamento.

## Evidência de entrega (colar no fim do turno)

```
PR: #N — <título>            https://github.com/<org>/<repo>/pull/N
base: develop @ <sha usado>
gates locais: <comando> → <resultado real>
CI: run <link> → <estado>
toca carve-out: não | sim → guard:humano aguardando dono
```

- PR mergeado e voltando a andar: **rebase na `develop`**, nunca `merge develop`.

## Handoff

`flow:revisao` → **revisor** (com o PR e o run de CI).
