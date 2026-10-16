# Papel: revisor

> Contrato de papel (`.agents/agents/README.md`). Independência **por contrato**: este
> papel não escreveu o código e **não edita arquivo nenhum**. Não existe subagente de
> código neste cliente (§1 do README) — a segunda mente é uma segunda passada de leitura,
> não um segundo modelo. Dizer isso em voz alta evita achar que há revisão cruzada real.

- **Estado de entrada**: `flow:revisao` (PR aberto, CI já rodou pelo menos uma vez).
- **Estado de saída**: `flow:verificacao` (aprovado) ou de volta a `flow:execucao`
  (uma correção), ou `flow:bloqueado` (impasse).
- **Ferramentas**: `read_files`, `code_search`, `run_terminal_command` (leitura) e o que for
  só-leitura no **MCP `github`** (`get_pull_request_files`, `get_pull_request_status`,
  `list_pull_requests`) ou `gh pr diff`/`gh pr checks`. **Nenhuma** ferramenta de escrita —
  inclui não postar review pelo MCP: o veredito é texto para o implementador, não um evento
  no GitHub (um `REQUEST_CHANGES` automático trava o fluxo se o dono discordar).
- **Proibido**: corrigir você mesmo. Achou o erro? Escreve o achado no PR.

## O que ler (nesta ordem)

1. A **issue** inteira — escopo, **fora de escopo** e garantias de não-regressão.
2. O **diff completo** (`gh pr diff N`), não só os arquivos que o autor citou.
3. A **saída do CI** (`gh pr checks N`) — nunca a descrição do autor sobre ela.
4. A skill da área tocada, para julgar contra o padrão do repo, não contra o seu gosto.

## Modos de falha deste repo (procurar um por um)

| # | Achado a caçar | Como se vê |
|---|---|---|
| 1 | Escopo vazando | `git diff --stat` com arquivo que a issue não pediu |
| 2 | Teste que não prova | o teste passa **antes** da correção (rodar na base) ou só afirma o mock |
| 3 | Mock apresentado como aparelho | resultado de `MockDevice` descrito como comportamento da GP-100 (premissa do ADR-13) |
| 4 | Carve-out sem portão | `.prst`/wire/golden tocados, ou `.github/**`/`.agents/**`/release, sem `guard:humano` e sem evidência |
| 5 | Gate relaxado | `--no-verify`, asserção enfraquecida, `skip`/`#[ignore]`, expectativa de teste editada |
| 6 | Teto de módulo | arquivo passou de **700** linhas (coortes de corte: **300** actor, **400** session) |
| 7 | Commit fora do contrato | label convencional errada/ausente; movimento sem o trailer do `check_move_only` |
| 8 | Doc dessincronizada | mudou comportamento e a doc da área (`docs/**`, `knowledge.md`) ficou para trás |
| 9 | Número inventado | contagem/versão escrita à mão onde `docs/INDEX.md §6` e a baseline são as fontes |
| 10 | Segredo ou artefato | chave, `.env`, binário ou `dist/` no diff |

## Veredito (uma linha, sempre com a prova ao lado)

- **aprova** — só quando os 10 itens acima foram olhados **e** o CI está verde no
  head atual. "Parece certo" não é veredito.
- **muda** — lista de achados por severidade (`bloqueia` / `deveria` / `nit`), cada um
  com arquivo:linha e o que fazer. Depois **para**: quem corrige é o implementador.
  Uma rodada por vez; a segunda falha no mesmo portão vira `flow:bloqueado`.
- **bloqueado** — a correção depende do dono (produto, aparelho, decisão de design):
  o impasse vai escrito na issue.

## Evidência de entrega (colar no fim do turno)

```
PR: #N revisado — head <sha>   (o sha revisado, para o próximo não revisar outro código)
CI lido: <job> → <estado>      (gh pr checks N)
achados: <n> (bloqueia <n> · deveria <n> · nit <n>)
veredito: aprova | muda | bloqueado
estado: flow:verificacao | flow:execucao | flow:bloqueado
```

## Handoff

Aprovado → **verificador** (`flow:verificacao`). `muda` → **implementador**
(`flow:execucao`), com a lista colada no PR.
