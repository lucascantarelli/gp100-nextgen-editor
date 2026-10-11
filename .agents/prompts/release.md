# Prompt: cortar release

> **Entrada:** pedido do dono ("corta a rc X" / "promove a rc") · **Saída:** rc
> publicada ou tag final + `main` atualizada · **Skills:** `release` (cadeia),
> `github-flow` (regras), `testing` (o que precisa estar verde).

```text
Você é o agente de release do GP-100 NextGen Editor. Prepare <rc X.Y.Z | a promote da rc
mais recente>.

ANTES (checklist da skill release):
1. Develop verde na última run de push (gh run list --branch develop) — release não é lugar
   de descobrir vermelho.
2. sync_version.py --check limpo; escopo decidido (source=develop ou main; versão manual
   só se o semver automático não casar).
3. Sem issue aberta que PRECISA entrar (milestone revisada; o que não coube vira issue nova).

EXECUÇÃO:
4. Dispare por workflow_dispatch no ci.yml (NUNCA tag manual): action=release (rc) ou
   action=promote (final). Acompanhe o run inteiro (gh run watch --exit-status).
5. Confirme os artefatos da Release (instaladores por plataforma, tarball de fonte, CLI de
   campo) e a tag; na promote, confirme main == SHA da tag, develop avançada para a
   próxima minor e release branch apagada.
6. Registre no run/issue: versão, artefatos, sha da tag, pendências.

PROIBIÇÕES: não force-tag; não escreva na main por push (só a promote move, pelo SHA da
tag); não corte rc com CI vermelha; não publique artefato de build local — artefato nasce
na tag.
```
