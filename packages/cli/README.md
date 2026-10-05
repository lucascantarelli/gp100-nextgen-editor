# gp100-cli — operação por linha de comando (bin Rust) · **CONGELADO**

> **Não é mais o produto.** O produto é o app (`packages/app/`), e hoje ele
> expõe um **superset** do que este binário expõe (incluindo o SnapTone, que
> o CLI nunca teve). Decisão do owner em 05/10: manter, mas congelar.
>
> **Capability nova NÃO entra aqui** — vai para o `app/api` ou não existe.
> Correção de bug entra, divergência permanente não. O binário continua
> compilando, testando e no gate: é o executável com que o core é exercitado
> nos gates H1–H3, e `--log`/`--dry-run` são o insumo do juiz de campo.
> O porquê da decisão está no cabeçalho de
> [`src/main.rs`](src/main.rs) e em `docs/REAL_DEVICE_GAP.md` §6.

Binário de operação/demonstração sobre o [gp100-core](../core). Fala com o
**MockDevice** por default (ADR-4/ADR-5): nenhum byte vai ao hardware. O modo
`--real` (USB-MIDI real via midir/WinMM/ALSA) exige build com a feature
`real-device` **e** dupla confirmação — política de hardware da VISION §7.

## Uso

```bash
cargo run -p gp100-cli -- info                        # estado do mock
cargo run -p gp100-cli -- list-user-irs               # tabela dos 20 slots
cargo run -p gp100-cli -- dump-preset 0x0007          # select + 9 páginas (hex)
cargo run -p gp100-cli -- set-param 3 0x0700006e 0 15.0 --dry-run
cargo run -p gp100-cli -- save 0x0007 6 "Blink OD" --dry-run
cargo run -p gp100-cli -- list-user-irs --log session.jsonl  # frames no schema P4
```

- Escritas (`set-param`/`save`) exigem `--dry-run` nesta fase — escrita real só
  pós-gate H2 (`WRITE_VERIFIED`).
- `--log` grava OUT/IN no MESMO schema das fixtures P4
  (`{"s","dir","func","addr","data"}`), consumível por `analysis/decode_wire.py`
  e pelos testes de replay — é o insumo do
  [H1_CHECKLIST](../../docs/H1_CHECKLIST.md) em campo.
- Códigos de saída: `2` = uso/política · `3` = erro de protocolo.

## Build de campo

```bash
cargo build --release -p gp100-cli --features real-device
```
