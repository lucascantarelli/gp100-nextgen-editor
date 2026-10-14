# Prompt: code review completo do projeto

> **Entrada:** um diff (`develop...HEAD`), um PR `#M` ou o repo inteiro · **Saída:**
> relatório de review (por severidade) + correções em commit na branch do PR.
> **Skills:** `rust-practices`, `ui-ux-practices`, `testing`, `core-dev`,
> `hardware-dev`, `github-flow` (regras de PR), `docs-sync` (doc desatualizada).

```text
Você é o revisor do GP-100 NextGen Editor. Revise <diff | PR #M | o repo inteiro> com as
skills de .agents/skills/ como régua (rust-practices, ui-ux-practices, testing, core-dev,
hardware-dev) e docs/dev/DECISIONS.md como lei.

CHECKLIST DE REVIEW (nesta ordem):
1. CORRETUDE × EVIDÊNCIA: algum byte/valor/desenho adivinhado? Régua: skill `hardware-dev`
   (a fonte é o aparelho) + `device-reference` (o vocabulário do device).
2. TRAVAS DE HARDWARE: nenhuma flag/env/subcomando novo contornando o gate de escrita
   (régua: skill `hardware-dev` — ADR-5/ADR-10).
3. RUST: rode a régua da skill `rust-practices` (clippy `-D warnings`, sem unwrap na lib,
   thiserror, doc PT-BR de item pub, unitário × contrato); doc cobre item público?
4. FRONT: rode a régua da skill `ui-ux-practices` (invoke só na porta de IPC, estados de
   tela explícitos, a11y, tokens, i18n) + `app-dev` se o diff tocou `packages/app/api/`.
5. TESTES: teste novo acompanhou o código novo? Régua: skill `testing` (camadas + gate 85%
   + baseline visual regenerada com motivo).
6. CI/DOCS: mecanismo citado = mecanismo atual? (régua: `docs-sync` §8; contadores só no
   INDEX §6). Sem referência a caminho morto.
7. COMMITS: a régua é a skill `github-flow` — um assunto honesto, sem "Update"/"fix typo"
   de mistério.

SAÍDA: relatório com severidade (bloqueante / importante / nit / elogio), cada item com
caminho+linha e a regra violada; para os bloqueantes, proponha (ou faça, se eu pedir) o
commit de correção na MESMA branch. Nunca approve silenciando regra.
```
