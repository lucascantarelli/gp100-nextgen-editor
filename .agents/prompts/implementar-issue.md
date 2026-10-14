# Prompt: implementar uma issue (canônico)

> **Entrada:** `#N` (issue do GitHub, milestone v1.1.0) · **Saída:** PR para
> `develop` com `Closes #N`, CI verde e issue fechada pelo workflow de fechamento.
> **Skills:** `github-flow` (ciclo), `hardware-dev` (se tocar device),
> `core-dev`/`rust-practices` (Rust), `ui-ux-practices`/`testing` (front),
> `docs-sync` (fechamento), `device-reference` (vocabulário).
> **Template de issue correspondente:** `feature.yml` ou `refactor.yml` (as
> seções do corpo são a matéria-prima deste prompt).

```text
Você é o agente do GP-100 NextGen Editor (repo local, branch develop). Execute a issue #N.

ANTES DE CODAR: leia a issue INTEIRA (contexto medido, escopo, fora de escopo, DoD e o
"🤖 prompt do agente" dela); leia as skills citadas no corpo (normalmente github-flow +
a de área); leia os docs referenciados (INDEX.md rota; GP100_DEVICE.md se tocar
funcionalidade; DECISIONS.md se tocar estrutura). Se a issue não tem prompt auto-suficiente,
complete o faltante com o que existir no repo e REGISTRE o que assumiu no corpo do PR.

PREMISSA: o aparelho real é a fonte (ADR-13). Dúvida de comportamento do device se RESOLVE
medindo no aparelho (o procedimento e as travas estão na skill hardware-dev) — nunca por
suposição do mock ou do manual de marketing. Escrita no aparelho só com as travas (ADR-5 + gate H2).

TAREFA: implemente o escopo da issue (nada fora dele — o que aparecer vira issue nova com
evidência). Teste novo acompanha código novo — régua da skill `testing` (camadas, gate de
cobertura do front). Commits conventional com o footer Codebuff.

FECHAMENTO: branch feature/N-slug → gates LOCAIS da área → push → PR para develop com o
template preenchido (tabela Antes→Depois, gates executados, ambiente) e `Closes #N` no
CORPO → acompanhe a CI até verde (gh pr checks --watch) → revisão do próprio diff → merge
squash → o workflow Issues · Fechamento fecha a issue → docs-sync (commit separado, mesma
PR: INDEX/ROADMAP/knowledge quando aplicável).

REGISTRO DE EXECUÇÃO: ao final, escreva na issue (comentário) o que foi entregue, as
medidas/links de run e o que ficou de fora — o histórico vive na issue, não em doc.
```
