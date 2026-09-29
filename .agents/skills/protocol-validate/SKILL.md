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
uv run pytest                                  # idem + hash da baseline + fixtures (P4)
```
A suíte pytest inclui `test_fixtures_parity`, que RERODA `make_fixtures.py` e
exige paridade 100% com o golden — se as capturas mudaram, regenerar antes.

## Critérios de aceite (o que é "verde")
- **Prova A (accounting):** IN = 100% obrigatório; OUT ≥ 99,5% (as sobras são
  fragmentos SEM-HDR do ring buffer do proxy — aceitáveis e esperadas).
- **Provas B–E (geração):** 100% obrigatório — knobs 89/89+3/3, boot 2299/2299,
  save 77/77, IR 1186/1186.
- **Hash da baseline:** o sha256 do golden gravado no §13 e nos testes deve
  bater EXATAMENTE (ver lição de EOL abaixo).
- **knob_map:** 13/14 OK; a exceção conhecida é CAB/Mic (controle oculto, fora do
  parameters.json) — não é falha.
- Qualquer número abaixo disso = **FALHA**, não "quase".

## ⚠️ Baseline = BYTES (lição EOL, mordida de 29/09)

O hash da baseline cobre **os bytes do arquivo, incluindo finais de linha**.
O golden foi gravado no disco Windows com **CRLF** e o §13 congelou o hash
DESSES bytes (`0426d6a8…`). Um checkout que converta EOLs desloca o hash **sem
mudar um byte lógico** — a CI chegou a falhar por isso (`18ccf068…` no blob LF
do repo).

Regras permanentes:
1. `docs/protocol_golden.json` está protegido com `-text` no `.gitattributes`:
   **nunca remover** a linha nem renormalizar o arquivo (é a spec executável).
2. Um **bump de baseline** (fluxo R2/R3) também atualiza: o hash no §13 do
   PROTOCOL.md, `GOLDEN_BASELINE_SHA` em `make_fixtures.py`, o assert em
   `test_protocol.py` e os exemplos em `DECISIONS.md`/INDEX — os 5 lugares
   citam o MESMO hash.
3. Se um hash de baseline falhar, PRIMEIRA suspeita é EOL/bytes, não conteúdo:
   comparar `git show :docs/protocol_golden.json | sha256sum` com o disco antes
   de qualquer conclusão dramática.
4. Scripts de análise usam caminhos **relativos à raiz do repo** (nunca
   absolutos tipo `D:\GP-100 app` — quebrou a CI no runner, mordida 29/09).

## Ao falhar
1. Identificar a prova e o template/grupo ofensor (o validador imprime diffs).
2. NÃO ajustar números no validador para "passar" — isso é fraude de regressão.
3. Causas legítimas: captura nova (→ skill `spec-baseline`), bug em
   `build_golden.py` (corrigir o gerador, regenerar, revalidar), fixture
   corrompida, ou EOL/bytes (seção acima).
4. Reportar cobertura no formato do RESUMO (A/B/C/D/E) ao encerrar a tarefa.
