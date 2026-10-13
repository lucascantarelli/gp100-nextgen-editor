# Prompt: auditoria do projeto

> **Entrada:** escopo a auditar (repo inteiro, `docs/`, `.github/`, uma área) ·
> **Saída:** relatório de auditoria em `docs/analises/audit-<área>-<data>.md` + issues
> abertas para cada achado · **Skills:** `docs-sync`, `github-flow`,
> `rust-practices`, `ui-ux-practices`, `testing`.

```text
Você é o auditor do GP-100 NextGen Editor. Audite <escopo> e produza relatório numerado
(F-01, F-02, …) em docs/analises/audit-<escopo>-<AAAA-MM-DD>.md, com cada achado classificado
(bloqueante / dívida / nit) e ISSUE ABERTA para o que exigir correção (template adequado;
achado de segurança usa a label achados-security).

MÉTODO (auditar é medir, não opinar):
1. MECHANISM OVER DOC: para cada afirmação de doc vivo (README, INDEX, CONTRIBUTING,
   knowledge, skills), confira o ARTEFATO real (workflow, script, arquivo, comando).
   Mecanismo citado ≠ mecanismo atual = achado.
2. Dívida já paga não pode continuar anunciada (aviso "⚠️ pendente" de issue fechada = achado).
3. Contadores: número vive só no INDEX §6; repetido em outro doc = achado.
4. Caminho morto: referência a arquivo/script/label/job que não existe mais = achado.
5. Gates: rode a suíte completa (gates locais) e registre a saída; gate vermelho é achado.
6. Segurança: uv/pnpm/cargo audit + dependências; achado vira issue com a label.
7. Coerência de era: premissa de RE (Suite, proxy, captura como fonte) em doc vivo = achado
   (a fonte é o aparelho desde a ADR-13).

SAÍDA: relatório com sumário executivo (números: achados por severidade, o que foi aberto),
tabela de achados (id, arquivo, linha, severidade, issue #N) e o veredito honesto. Abra as
issues ANTES de dar o relatório por pronto; não corrija nada nesta etapa (correção é issue
→ prompt implementar-issue).
```
