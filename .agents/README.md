# `.agents/` — o harness do agente (Era Hardware)

Este diretório é o que faz qualquer agente (eu, você, o próximo) entrar neste
repo e trabalhar **sem briefing extra**:

```
.agents/
├── README.md   # este mapa: ordem de entrada + fontes únicas por regra
├── agents/     # QUEM faz (triagem→implementador→revisor→verificador)
├── skills/     # COMO se faz (10 skills, sob demanda pela `description`)
└── prompts/    # O QUE executar (6 templates; vinculam skills)
```

O **catálogo detalhado** (o que cada skill/prompt faz) vive no `docs/INDEX.md`
§"Infra do agente"; o **gatilho** de cada skill é a `description` do `SKILL.md`.
Aqui fica só a ordem de entrada — para não manter um terceiro catálogo que apodrece.

## Como entrar no repo (ordem que funciona)

1. **`knowledge.md`** (raiz) — premissa (a dona dela é a `hardware-dev`), armadilhas
   de ambiente que já morderam e o estado vivo. Ler inteiro: é curto de propósito.
2. **`docs/INDEX.md`** — mapa: qual doc responde qual pergunta. Não adivinhe caminho.
3. **A issue** (`gh issue view #N`) — contexto medido, escopo, DoD e o `🤖 prompt`. Pedido
   direto do dono sem issue? Crie a issue antes de codar (`.github/ISSUE_TEMPLATE/`).
4. **A skill da área** — o gatilho é a `description` do `SKILL.md`; `github-flow` e
   `docs-sync` valem sempre.
5. **O papel da vez** — `.agents/agents/README.md`: qual contrato ler (triagem,
   implementador, revisor, verificador) e em que `flow:*` a demanda entra e sai. Um papel,
   um turno: não misture implementar e revisar.

## Regras de vida do harness

- **Issue manda**: o DoD dela define "pronto"; doc não duplica plano de issue.
- **Ciclo**: skill `github-flow` — uma issue = um PR, `Closes #N` no corpo, CI verde antes
  do merge, fechamento automático e `docs-sync` em commit próprio na mesma PR.
- **Nada de apagar**: skill, doc ou era morta vai para `docs/arquivo/` com banner; a lista do
  que NÃO se faz é a do `AGENTS.md` §5, e a premissa é da `hardware-dev`.
- **Uma casa por regra:** regra transversal mora em UMA casa (skill ou arquivo de entrada) e
  as outras **apontam** (tabela abaixo, gerada pelo gate). Repetida em 3, sincroniza em 3 e
  apodrece nos outros — o §8 da `docs-sync` combate.
- **O harness tem gate próprio:** `.github/scripts/check_harness.py` cobra caminho que
  resolve, `description` que dispara, contrato de papel com índice e a tabela acima. Rode ao
  mexer aqui: a leitura manual já deu "está tudo certo" duas vezes com defeito (#249).

## Fontes únicas por regra (não duplicar)

A tabela é **saída do gate**: quem manda é o `REGRAS` do
`.github/scripts/check_harness.py`, que cobra **uma casa por regra** (§4) — a dona
enuncia, as outras apontam. Editar a tabela à mão reprova; para mudar uma linha, mude a
regra no gate e rode `check_harness.py --tabela`.

<!-- fontes-unica:inicio (gerada por `.github/scripts/check_harness.py --tabela`; editar a mao reprova) -->
| Assunto | Casa (dono) | Os demais |
|---|---|---|
| O aparelho é a fonte; medir, não deduzir (ADR-13) | `hardware-dev` | apontam |
| Ciclo issue→branch→PR→CI→merge | `github-flow` | apontam |
| Conventional commits + footer Codebuff | `github-flow` | apontam |
| Onde cada teste vive (unitário `src/` × contrato `tests/`) | `rust-practices` + `testing` | apontam |
| Contadores (testes/cobertura/jobs) só no `INDEX.md` §6 | `docs-sync` | apontam |
| Armadilhas de host (cargo/PATH, toolchain pinada) | `knowledge.md` | apontam |
| Cadeia de release rc→promote→tag | `release` | apontam |
| Camada do app (`packages/app/api`: actor, commands, DTO/IPC) | `app-dev` | apontam |
| Estrutura e limites das camadas do front | `docs/dev/ARCHITECTURE.md` | apontam |
| Papéis da equipe + estado da demanda (`flow:*`) | `.agents/agents/README.md` | apontam |
| Modo ausente do dono (#202) | `.agents/README.md` | apontam |
| O que NÃO fazer (lista única) | `AGENTS.md` | apontam |
| Gates locais por área (antes do PR) | `docs/CONTRIBUTING.md` | apontam |
<!-- fontes-unica:fim -->

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
