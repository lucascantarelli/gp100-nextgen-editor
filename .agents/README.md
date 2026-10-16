# `.agents/` — o harness do agente (Era Hardware)

Este diretório é o que faz qualquer agente (eu, você, o próximo) entrar neste
repo e trabalhar **sem briefing extra**. Três camadas + um mapa + uma memória:

```
.agents/
├── README.md          # este mapa: ordem de entrada + fontes únicas por regra
├── agents/            # QUEM faz (papéis da equipe: triagem→implementador→revisor→verificador)
├── skills/            # COMO se faz (sob demanda, disparadas pela `description`)
│   ├── github-flow/ · core-dev/ · rust-practices/ · app-dev/
│   ├── ui-ux-practices/ · testing/ · hardware-dev/
│   └── device-reference/ · release/ · docs-sync/
└── prompts/           # O QUE executar (templates para colar; vinculam skills)
    ├── implementar-issue.md · executar-issues-em-sequencia.md · corrigir-bug.md
    └── code-review-completo.md · auditoria.md · release.md
```

O **catálogo detalhado** (o que cada skill/prompt faz) vive no `docs/INDEX.md`
§"Infra do agente"; o **gatilho** de cada skill é a `description` do `SKILL.md`.
Aqui fica só a ordem de entrada — para não manter um terceiro catálogo que apodrece.

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
5. **O papel que você está exercendo** — `.agents/agents/README.md` diz qual contrato
   ler (triagem, implementador, revisor, verificador) e em que estado (`flow:*`) a
   demanda entra e sai. Um papel, um turno: não misture implementar e revisar.

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
- **Uma casa por regra:** regra transversal mora em UMA skill e as outras
  **apontam** (a tabela abaixo). Regra repetida em 3 arquivos sincroniza em 3 e
  apodrece nos outros — o modo de falha que o §8 da `docs-sync` combate.
- **O harness tem gate próprio:** `python3 .github/scripts/check_harness.py`
  (ou `gates.py check_harness`) cobra caminho que resolve, `description` que
  dispara, contrato de papel com índice e a tabela de fontes únicas acima.
  Rode ao mexer aqui: a leitura manual já deu "está tudo certo" duas vezes com
  defeito no arquivo (#249).

## Fontes únicas por regra (não duplicar)

| Assunto | Casa (dono) | Os demais |
|---|---|---|
| O aparelho é a fonte; medir, não deduzir (ADR-13) | `hardware-dev` | apontam |
| Ciclo issue→branch→PR→CI→merge | `github-flow` (+ prompt `implementar-issue`) | apontam |
| Conventional commits + footer Codebuff | `github-flow` | apontam |
| Onde cada teste vive (unitário `src/` × contrato `tests/`) | `rust-practices` (visão por camada: `testing`) | apontam |
| Contadores (testes/cobertura/jobs) só no `INDEX.md` §6 | `docs-sync` | apontam |
| Armadilhas de host (cargo/PATH, toolchain pinada) | `knowledge.md` | apontam |
| Cadeia de release rc→promote→tag | `release` (+ prompt `release`) | apontam |
| Camada do app (`packages/app/api`: actor, commands, DTO/IPC) | `app-dev` | apontam |
| Estrutura/limites das camadas do front | `docs/dev/ARCHITECTURE.md` | apontam |
| Papéis da equipe + estado da demanda (`flow:*`) | `.agents/agents/README.md` | apontam |
| Gates locais por área (antes do PR) | `docs/CONTRIBUTING.md` §4 | `AGENTS.md` indexa |

## Modo ausente do dono (#202)

Quando o dono diz "estarei ausente, execute as ações recomendadas", o agente
opera **sem perguntar** — mas com a linha divisória escrita:

1. **Fila:** execute as issues ABERTAS do milestone na ordem de prioridade
   (formato do prompt `executar-issues-em-sequencia.md`), UMA por PR.
2. **Rotineiro — executa direto:** commit, push, PR, gates locais, re-run de
   CI flake (verde prévio + falha de ambiente), merge squash com **CI verde**,
   delete de branch **já mergeada**, comentário de entrega na issue.
3. **Crítico — comenta NA ISSUE e passa adiante** (NÃO executa):
   - merge com gate vermelho (mesmo "provavelmente flake");
   - escopo novo / criação de issue de produto;
   - delete de trabalho **não mergeado** (branch, stash, arquivo);
   - release, tag ou qualquer push em `main`;
   - fechar issue cujo PR não foi mergeado.
4. **Fim da sessão:** relatório no resumo para o dono — fechadas / em aberto /
   bloqueadas (com o motivo e o link da issue comentada).

O modo não muda gates nem escopo: só remove a espera por aprovação nas
rotinas. Toda decisão crítica continua registrada na issue, que é a memória
do projeto.
