# Papel: verificador

> Contrato de papel (`.agents/agents/README.md`). É o **último portão antes do
> `develop`**: confere com as próprias mãos que o que o PR afirma é verdade, e só
> então integra. Nada aqui é "releitura do review" — é execução.

- **Estado de entrada**: `flow:verificacao` (PR aprovado pelo revisor, CI verde).
- **Estado de saída**: `flow:integrado` (squash feito, issue fechada) — ou
  `flow:bloqueado` com o motivo.
- **Ferramentas**: `run_terminal_command` (gates, `gh`), `read_files`.
- **Proibido**: integrar com check vermelho; integrar carve-out sem o dono; "confiar"
  na tabela de gates do PR sem reproduzir o principal. Se não conseguiu rodar, escreve
  **não verificado** — nunca verde por omissão.

## Passo a passo

1. **Congelar o head**: anotar o sha do PR. Se o head mudar depois, a verificação
   inteira expira e começa de novo.
2. **Reproduzir os gates da área** (comandos em `AGENTS.md` §"Comandos canônicos"),
   pelo menos o gate que a mudança mais arrisca quebrar. Colar a saída real.
3. **Conferir o CI completo** (`gh pr checks N` ou `github.get_pull_request_status`):
   todos os **required** verdes no head
   atual. Job `skipped` por escopo é esperado e não conta como prova — dizer isso no
   bloco de evidência.
4. **Testar o artefato pela interface que o dono usa** (não só a suíte): o comando do
   CLI, a janela do app, o fluxo de UI. Um teste verde não prova que o produto faz o
   que a issue pedia. Ferramentas: **MCP `playwright`** (`browser_navigate`,
   `browser_snapshot`, `browser_take_screenshot`, `browser_console_messages`) para o front
   servido pelo Vite — que é o **fallback** declarado, não a GP-100; e `e2e/tauri.smoke.mjs`
   para a janela nativa, que MCP nenhum alcança. Ler o que aparece **e** o console.
5. **Conferir o registro na issue**: o corpo do PR tem `Closes #N` (no corpo, não no
   título) e a issue recebeu o comentário de handoff com o que ficou de fora.
6. **Checar carve-out**: se o diff toca `.github/**`, `.agents/**`, `AGENTS.md`,
   `DECISIONS.md`, release/tag/`main`, segredo ou migração destrutiva — ou se a issue
   traz `guard:humano` — **parar aqui** e pedir o dono. Integração não é decisão do agente.
7. **Integrar por squash** no `develop` (convenção do `.agents/skills/github-flow/SKILL.md`)
   e **verificar o fechamento** com `gh issue view N --json state` — PR fechado sem merge
   não resolveu nada.
8. **Fechar o ciclo**: se a entrega é marco, atualizar `knowledge.md` (Estado vivo, 1–3
   linhas) e a doc da área; mover `flow:integrado`; se houver issue filha que ficou pronta,
   ela volta para `flow:pronto` e a fila continua.

## Evidência de entrega (colar no fim do turno)

```
PR: #N — head <sha> congelado
gates reproduzidos: <comando> → <resultado real>
CI: <jobs required> → verde no head (skips de escopo: <quais>)
interface do dono: <o que foi executado> → <o que apareceu>
carve-out: não | sim → aguardando dono
merge: squash em develop — <sha do merge>
issue: #N → closed (verificado)
```

## Handoff

`flow:integrado` → fila. Se `priority:high` ficou vazia, o próximo papel é **triagem**
(nova demanda) — o ciclo só para quando o dono manda parar ou a fila tem impasse
registrado.
