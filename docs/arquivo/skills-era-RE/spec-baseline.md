---
name: spec-baseline
description: Congela ou altera a baseline da especificação do protocolo (protocol_golden.json + parameters.json), com hashes sha256 e processo formal R2/R3 do ROADMAP.
metadata:
  category: governance
---

# Baseline da especificação de protocolo

Use quando: (a) congelar a baseline pela primeira vez; (b) uma captura nova mudou
o protocolo e o golden foi regenerado; (c) bump de versão da baseline.

## Processo (SEMPRE nesta ordem)
1. Garantir `validate_golden.py` a 100% nas 5 provas (prova B–E são gerativas).
2. Calcular sha256 de `docs/protocol_golden.json` e `analysis/parameters.json`.
3. Atualizar o bloco **Baseline vX.Y** no banner do `docs/PROTOCOL.md` §13:
   - novos hashes;
   - data;
   - justificativa de 1–3 linhas (o que mudou e por quê — cite a captura/issue).
4. Atualizar o `_meta` do golden (`"baseline": "X.Y"`).
5. Atualizar `knowledge.md` (estado vivo) e a issue correspondente no ROADMAP
   (P1/H3) com ✅ + data.

## Regras
- NUNCA altere o golden sem captura que o justifique (regra R2: captura →
  `build_golden.py` → `validate_golden.py` 100% → bump).
- **Hoje**, quem valida o hash da baseline é o pytest (`GOLDEN_BASELINE_SHA` em
  `make_fixtures.py` + assert em `test_protocol.py`; detalhes dos 5 lugares em
  `protocol-validate`). O gp100-core NÃO valida hash no build (o `golden.rs`
  cita a baseline só em doc-comment) — se um dia isso existir, é decisão nova
  (ADR), não fato atual.
- Divergência device real vs baseline = parar fluxo e abrir captura (R3).
