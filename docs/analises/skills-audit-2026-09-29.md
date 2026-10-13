# 🔍 Auditoria de skills — regras implícitas explicitadas (29/09)

> **O que é:** auditoria pontual nas 8 skills de `.agents/skills/`, motivada pela regra de
> organização de testes que estava implícita na `rust-practices` e "voltou" como dúvida em
> conversa (29/09). Critério: prática que um agente experiente já segue, mas que NÃO está
> escrita na skill = achado — é dívida de retrocesso (o próximo agente chuta errado).
>
> **Resultado:** 5 achados corrigidos (3 skills: `core-dev`, `docs-sync`, `spec-baseline`).
> Skills sem achados: `capture-analyze`, `new-session`, `protocol-validate`, `proxy-build`,
> `rust-practices` (recém-revisada, já carrega a regra de testes explícita).

## Achados corrigidos

### 1. `core-dev` — encerramento de issue incompleto (faltava CI)
**Implícito:** as issues fechadas (M0.1–M0.4) sempre terminaram com push + `gh run watch
--exit-status` até verde; a skill pedia só fmt/clippy/test locais e já marcava ✅.
**Risco:** marcar ✅ com CI vermelha (EOL/bytes, runner) e o owner descobrir depois.
**Correção:** passo 5 do checklist agora é "commit/push + `gh run watch … --exit-status`
até verde → SÓ ENTÃO ✅ no ROADMAP + knowledge".

### 2. `core-dev` — labelling de commit não escrito
**Implícito:** histórico real do repo usa `assunto: resumo` em PT-BR (ex.: `M0.4: codec
de fio — …`, `docs-sync: M0.4 ✅ …`, `golden: API tipada…`) + footer 🤖/Co-Authored-By.
**Correção:** regra registrada no checklist (passo 5), com o prefixo por tipo.

### 3. `core-dev` — gating por hardware ausente (padrão de envio)
**Implícito:** o modo real é bloqueado em DUAS camadas (CLI: `--real
--i-know-what-im-doing`; e o gate H do ROADMAP só libera escrita real pós-M0/H2,
`WRITE_VERIFIED`), mas a skill não diz o que fazer quando um subcomando novo
precisaria de device real. **Correção:** regra "modo real não se adiciona fora do
gate H; necessidade nova → ADR/ROADMAP, nunca feature flag ad-hoc".

### 4. `docs-sync` — commit da docs-sync separado
**Implícito:** M0.2/M0.3/M0.4 e a skill rust-practices foram commitados como
`docs-sync: …` SEPARADOS do commit de código (d3088c8+fc0b6e6, 6422cd1+d2fa925).
**Correção:** regra "docs-sync vira commit próprio `docs-sync: …` DEPOIS do commit
da mudança — nunca escondido dentro do commit de código".

### 5. `spec-baseline` — afirmação imprecisa (M0.3 não valida hash no build)
**Escrito:** "O gp100-core valida o hash da baseline no build de release (issue M0.3)".
**Real:** `golden.rs` cita a baseline apenas em doc-comment; quem valida o hash é o
pytest (`GOLDEN_BASELINE_SHA` + assert em `test_protocol.py`) — e `protocol-validate`
já documenta esses lugares. **Correção:** texto trocado pela verdade atual + nota de
que validação no core (se um dia existir) é decisão nova (novo ADR), não fato.

## Methodologia (para a próxima auditoria)
1. Ler a skill inteira e listar toda afirmação de processo sem comando/nome de arquivo.
2. Cruzar com o histórico real (`git log`) e com o código: o que a skill AFIRMA sobre
   o repo precisa bater com o repo (achado nº 5 nasceu de `grep` no `golden.rs`).
3. Prática seguida por ≥2 marcos consecutivos e ausente do texto = corrigir na hora.
4. Afirmação que descreve futuro como presente = separar "hoje" de "quando existir".
