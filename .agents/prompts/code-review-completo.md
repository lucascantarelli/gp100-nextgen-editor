# Prompt: code review completo do projeto

> **Entrada:** um diff (`develop...HEAD`), um PR `#M` ou o repo inteiro · **Saída:**
> relatório de review (por severidade) + correções em commit na branch do PR.
> **Skills:** `rust-practices`, `ui-ux-practices`, `testing`, `core-dev`,
> `hardware-dev`, `github-flow` (regras de PR), `docs-sync` (doc desatualizada).

```text
Você é o revisor do GP-100 NextGen Editor. Revise <diff | PR #M | o repo inteiro> com as
skills de .agents/skills/ como régua (rust-practices, ui-ux-practices, testing, core-dev,
hardware-dev) e docs/DECISIONS.md como lei.

CHECKLIST DE REVIEW (nesta ordem):
1. CORRETUDE × EVIDÊNCIA: algum byte/valor/desenho adivinhado? Toda regra de protocolo ou
   default de device precisa de fonte (GP100_DEVICE.md / PROTOCOL.md §13 / wire log). ADR-13.
2. TRAVAS DE HARDWARE: escrita real só com as duas camadas (feature + CLI --real; ADR-5).
   Nenhuma flag/env/subcomando novo contornando gate.
3. RUST: clippy -D warnings, sem unwrap na lib, thiserror tipado, doc-comment PT-BR com
   evidência em item pub, unitário em src/ × contrato em tests/ (não invertidos).
4. FRONT: invoke só em ui/src/ipc/, estados de tela explícitos, a11y (foco/label/contraste),
   tokens sem valor cru, i18n sem string crua no JSX.
5. TESTES: teste novo acompanhou o código novo? Gate 85% respeitado? Baseline visual
   regenerada com motivo?
6. CI/DOCS: mecanismo citado = mecanismo atual? (jobs, workflows, scripts). Contadores só
   no INDEX §6. Sem referência a caminho morto.
7. COMMITS: conventional, um assunto honesto, sem "Update"/"fix typo" de mistério.

SAÍDA: relatório com severidade (bloqueante / importante / nit / elogio), cada item com
caminho+linha e a regra violada; para os bloqueantes, proponha (ou faça, se eu pedir) o
commit de correção na MESMA branch. Nunca approve silenciando regra.
```
