---
name: protocol-validate
description: Roda e interpreta a suíte de validação do protocolo (validate_golden 5 provas + validate_knob_map) como gate de qualquer mudança de spec, decoder ou captura.
metadata:
  category: quality
---

# Validação do protocolo (gate de qualidade)

Use antes de: merge/PR que toque `docs/protocol_golden.json`, `PROTOCOL.md`,
decoders, capturas ou o gp100-core; após rodar `build_golden.py`; após qualquer
captura nova; e no fim das issues M0.6/M0.8.

## Como rodar
```bash
uv run python analysis/validate_golden.py      # 5 provas
uv run python analysis/validate_knob_map.py    # envelope do knob vs dicionário/.prst
uv run pytest                                  # após P3: mesma coisa + gate de CI
```

## Critérios de aceite (o que é "verde")
- **Prova A (accounting):** IN = 100% obrigatório; OUT ≥ 99,5% (as sobras são
  fragmentos SEM-HDR do ring buffer do proxy — aceitáveis e esperadas).
- **Provas B–E (geração):** 100% obrigatório — knobs 89/89+3/3, boot 2299/2299,
  save 77/77, IR 1186/1186.
- **knob_map:** 13/14 OK; a exceção conhecida é CAB/Mic (controle oculto, fora do
  parameters.json) — não é falha.
- Qualquer número abaixo disso = **FALHA**, não "quase".

## Ao falhar
1. Identificar a prova e o template/grupo ofensor (o validador imprime diffs).
2. NÃO ajustar números no validador para "passar" — isso é fraude de regressão.
3. Causas legítimas: captura nova (→ skill `spec-baseline`), bug em
   `build_golden.py` (corrigir o gerador, regenerar, revalidar), fixture corrompida.
4. Reportar cobertura no formato do RESUMO (A/B/C/D/E) ao encerrar a tarefa.
