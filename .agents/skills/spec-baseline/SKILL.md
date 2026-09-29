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
- O gp100-core valida o hash da baseline no build de release (issue M0.3).
- Divergência device real vs baseline = parar fluxo e abrir captura (R3).
