# 📜 `docs/arquivo/analysis/` — a Era de RE, em cápsula

> **Histórico, não processo.** Esta pasta É o antigo `analysis/`, movida aqui
> inteira na dissolução da **#163** (11/10/2026) **preservando a estrutura**:
> tudo que os relatórios H1–H4, o `PROTOCOL.md` §13 e o golden citam continua
> encontrável trocando o prefixo `analysis/` → `docs/arquivo/analysis/`.
> As casas do que continua vivo estão no **ADR-14** (`docs/dev/DECISIONS.md`).

## Para onde foi o que era vivo

| Casa | O que entra | Por quê |
|---|---|---|
| `packages/core/data/` | `parameters.json`, `state_pages_offsets.json`, `fx_map.json`, `fixtures/` | dado consumido por runtime/teste de contrato, com origem declarada (#178) |
| `scripts/` | `build_golden.py`, `validate_golden.py`, `baseline.py`, `wirelog.py`, `validate_core_capture.py`, `make_fixtures.py`, `param_ranges.py`, `validate_state_pages.py`, `validate_knob_map.py`, `check_byte_order.py`, `map_state_pages.py`, `dump_*.py`, `tests/` | roda hoje (CI, `gates.py`, pytest, fluxo R2/R3) e só precisa de insumo versionado |
| **aqui** | o resto: decoders, RE estática, proxy, geradores com insumo local não versionado, manual, catálogo, achados e **evidência dos gates** | proveniência: de onde veio cada número, para sempre |

## Regras desta cápsula

1. **Não é insumo de nada.** `guard_dados_canonicos.rs` reprova qualquer
   `analysis/` dentro de `packages/` — incluindo este caminho novo.
2. **Os arquivos internos seguem citando `analysis/` entre si.** É o marco da
   época (regra `docs-sync` 8: histórico preserva o marco); quem copiar um
   caminho de cá para fora é que corrige.
3. **Bytes congelados.** As regras `-text` do `.gitattributes` migraram junto
   (capturas, `h1_reference/`, `h2/`). EOL aqui é parte do byte: conversão
   quebra hash de baseline e round-trip.
4. **Não versionar lixo novo.** `suite_local/`, `manual_v18.*`, `fw_strings.txt`,
   `exe_strings.txt`, `nsis_app/` e companhia continuam ignorados pelo
   `.gitignore`; a sessão H4 (`h4/`) é evidência local não rastreada, como
   ficou desde o PR #178.

## O que ainda se faz daqui

Só o que a cápsula serve de **proveniência**:

```bash
# a proveniência do golden (o que as provas A–E leram)
ls docs/arquivo/analysis/captures/session{1,2,3,4}.jsonl
# a origem do dicionário (payload oficial da Suite V1.5.1)
head -20 docs/arquivo/analysis/algorithm.xml
```

O que **roda** está em `scripts/` (mapa no `docs/INDEX.md` §3) e o fluxo de
captura nova é `--log` do `gp100-cli` → `scripts/wirelog.py` →
`scripts/build_golden.py` → `scripts/validate_golden.py` →
`scripts/baseline.py bump` (R2/R3).
