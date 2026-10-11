# Prompt: executar a lista de issues em sequência

> **Entrada:** milestone (padrão `v1.1.0`) ou lista explícita de `#N` · **Saída:**
> uma PR por issue, na ordem, com o tracker atualizado.
> **Skills:** `github-flow`, `docs-sync` + a skill de área de cada issue.
> **Uso:** o dono pede "executa as issues abertas"; o agente processa UMA por vez.

```text
Você é o agente do GP-100 NextGen Editor (repo local, develop). Execute as issues ABERTAS
do milestone v1.1.0 (ou a lista que eu passar), UMA por vez, na ordem de prioridade.

1. Inventário: gh issue list --milestone v1.1.0 --state open --json number,title,labels —
   monte a fila (priority:high primeiro; épicas só quando eu pedir explicitamente).
2. Para CADA issue, na fila:
   a. Leia a issue inteira. Se ela tem o "🤖 prompt do agente" no corpo, ele É o passo a
      passo (formato canônico: .agents/prompts/implementar-issue.md).
   b. Execute o ciclo completo: branch → implementação + testes → gates locais → push →
      PR (Closes #N) → CI verde → merge squash → issue fechada pelo workflow → docs-sync.
   c. Registre na issue o que foi entregue (comentário com medidas/links de run).
   d. PARE entre issues: cada uma é um PR revisável; nunca junte duas issues num PR.
3. Descoberta no caminho (bug, gap de hardware, dívida) vira issue nova NA HORA
   (templates .github/ISSUE_TEMPLATE/) e entra no fim da fila — não é "resolvido de
   passagem".
4. Bloqueio real (falta de hardware, decisão de escopo, gate vermelho que exige escolha):
   PARE e reporte — não contorne gate, não inventie escopo.
Ao final: resumo da fila (fechadas / em aberto / bloqueadas, com motivo) e o estado da
develop (última run verde).
```
