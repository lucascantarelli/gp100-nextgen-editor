# 🔬 PROVENANCE — quem gera o quê em `analysis/`

> Este arquivo existe porque o `analysis/check_orphans.py` encontrou scripts que
> **ninguém referencia e que não são lixo**: são os geradores dos artefatos
> versionados. Sem esta tabela, quem apaga um `.json` daqui não tem por onde
> descobrir a ferramenta que o reconstrói — e o caminho vira "reflectir a RE na
> mão", que é exatamente o que este diretório existe para evitar.

Rode `python analysis/check_orphans.py` para ver a lista viva.

## Geradores de artefato VERSIONADO

| Script | Gera | O que é o artefato |
|---|---|---|
| `parse_algorithm_xml.py` | `algorithm_dict.json` · `algorithm_dict.csv` | O dicionário de algoritmos/parâmetros destilado do `algorithm.xml` oficial |
| `validate_knob_map.py` | `knob_map.json` | O mapa knob→fio da sessão 3 (89 edits) cruzado com `parameters.json` |
| `build_golden.py` | `docs/protocol_golden.json` | **A especificação executável** — ver `docs/PROTOCOL.md` §13 |
| `make_fixtures.py` | `analysis/fixtures/*.jsonl` + `manifest.json` | As fixtures que o `gp100-core` usa nos replays |
| `baseline.py` | `baseline.json` | A baseline do golden (versão + hash + motivo) — gate R2/R3 |
| `derive_save_ops.py` | — (deriva os ops do §13.12) | A sequência `00020000` do `save` |

## Verificadores (não geram; provam)

| Script | Prova |
|---|---|
| `check_byte_order.py` | O `effectCode` é **u32 LE** no fio, contra a captura real — **é gate** (`gates.py`) |
| `validate_golden.py` | O golden reproduz as capturas (prova A–E) |
| `check_orphans.py` | Ninguém ficou sem referência (este arquivo é a resposta a ele) |
| `validate_core_capture.py` | O `gp100-core` bate com a captura no replay |

## Reedição / extração (produzem intermediário LOCAL, não versionado)

| Script | Lê | Escreve |
|---|---|---|
| `exe_strings.py` | `nsis_app/GP-100.exe` | `exe_strings.txt` (local, ~3 MB — não versionado de propósito) |
| `manual_decode.py` | `files/*.pdf` | `manual_decoded.txt` (local) |
| `manual_settings.py` · `manual_tuner.py` | `manual_streams.txt` | stdout (recortes por keyword, para o inventário da UI) |
| `disasm_*.py` · `rtti_disasm.py` · `xrefs_opcodes.py` · `find_817_imm.py` | binários do firmware/exe | stdout (análise estática) |
| `dump_*.py` · `flow_dump.py` · `probe_session4.py` · `recon_session3.py` | capturas | stdout (leitura das sessões) |
| `decode_*.py` · `wirelog.py` | capturas/fio | stdout |

> **Por que estes ficam, se ninguém os chama?** Porque são o rastro auditável da
> engenharia reversa: qualquer número do `PROTOCOL.md` tem um comando que o
> reproduz. Um deles que você *apague* transforma uma afirmação verificável em
> folclore. O que **não** fica é o que não tem entrada nem saída — foi o caso do
> `parse_preset_xml.py`, removido em 05/10/2026: imprimia uma árvore XML que
> qualquer editor abre, e nenhum artefato dependia dele.

## Caminhos fixos (armadilha de Windows)

Vários scripts daqui abrem caminhos **relativos ao cwd** (`open("analysis/...")`).
Rode-os da raiz do repo:

```bash
python analysis/parse_algorithm_xml.py     # ok: cwd = raiz
cd analysis && python parse_algorithm_xml.py   # quebra
```

Os que importam para o CI/gates usam caminho absoluto derivado de `__file__`
(ver `validate_knob_map.py` e `check_byte_order.py`).
