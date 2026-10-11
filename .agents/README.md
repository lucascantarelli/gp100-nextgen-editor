# `.agents/` — o harness do agente (Era Hardware)

Este diretório é o que faz qualquer agente (eu, você, o próximo) entrar neste
repo e trabalhar **sem briefing extra**. Três peças + uma memória:

```
.agents/
├── README.md          # este mapa (estrutura e ordem de entrada)
├── skills/            # COMO se faz (sob demanda, disparadas pela description)
│   ├── github-flow/       ciclo issue → branch → PR → CI verde → fechamento
│   ├── core-dev/          Rust do core/cli dentro das decisões (ADR-13, R4)
│   ├── rust-practices/    gates, estilo, organização de testes Rust
│   ├── ui-ux-practices/   front (tokens, a11y, estados, gates)
│   ├── testing/           o que a suíte testa e onde cada teste vive
│   ├── hardware-dev/      aparelho real: leitura livre, escrita travada, evidência
│   ├── device-reference/  o QUE a GP-100 é (vocabulário → docs/GP100_DEVICE.md)
│   ├── release/           cadeia rc → promote → tag e artefatos
│   └── docs-sync/         sincronização de doc ao fechar marco
└── prompts/           # O QUE executar (templates para colar; vinculam skills)
    ├── implementar-issue.md        # canônico — executa uma issue
    ├── executar-issues-em-sequencia.md  # a fila do milestone, uma por PR
    ├── corrigir-bug.md             # diagnóstico → fix → teste de regressão
    ├── code-review-completo.md     # revisão com a régua das skills
    ├── auditoria.md                # medir, relatar, abrir issues
    └── release.md                  # rc/promote com checklist
```

## Como entrar no repo (ordem que funciona)

1. **`knowledge.md`** (raiz) — premissa (o aparelho é a fonte), armadilhas de
   ambiente que já morderam, o que NÃO fazer. Ler inteiro: é curto de propósito.
2. **`docs/INDEX.md`** — mapa: qual doc responde qual pergunta. Não adivinhe
   caminho; pergunte ao INDEX.
3. **A issue** (`gh issue view #N`) — o corpo dela traz contexto medido, escopo,
   DoD e o **🤖 prompt do agente** pronto para colar. Se você veio por pedido
   direto do dono, procure a issue correspondente; se não existir, crie antes
   de codar (`.github/ISSUE_TEMPLATE/`).
4. **A skill da área** — o campo `description` de cada `SKILL.md` diz quando
   usá-la; skills de processo (`github-flow`, `docs-sync`) valem sempre.

## Regras de vida do harness

- **Issue manda**: DoD dela define "pronto"; doc não duplica plano de issue.
- **Uma issue = um PR**; `Closes #N` no corpo; CI verde antes de merge; o
  fechamento é automático (workflow próprio desde a #169).
- **Nada de RE**: a especificação não é reconstruída de Suite/proxy — o
  aparelho é a fonte (ADR-13). Skills da Era RE estão arquivadas em
  `docs/arquivo/skills-era-RE/` (história, não processo).
- **Nada de escrever no aparelho** fora das travas (ADR-5 + gate H2) — ver
  `hardware-dev`.
- Skill nova só se justificar (dispara quando a description casa com uma tarefa
  recorrente); skill morta vai para `docs/arquivo/`, nunca é apagada.
- Ao fechar marco: `docs-sync` (commit próprio na mesma PR) mantém
  INDEX/ROADMAP/knowledge honestos.
