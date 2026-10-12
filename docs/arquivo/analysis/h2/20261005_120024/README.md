# Sessão H2 real — 05/10/2026, GP-100 V2.1

Evidência do gate H2 de escrita real. O veredito está em
[docs/H2_REPORT.md](../../../docs/H2_REPORT.md); este README só diz **qual log
é o quê** e **em que ordem as coisas aconteceram**.

## Linha do tempo (todas as vezes em 05/10/2026)

| # | Hora | O quê | Log | Aparelho |
|---|---|---|---|---|
| 1 | 12:00 | `dump-preset 0x0000` — linha de base | `antes.jsonl` | saudável (10 OUT / 10 IN) |
| 2 | 12:0x | **F1 com `99.5`** | `f1_setparam.jsonl` | **TRAVOU** (assert de firmware) |
| 3 | — | F2 com o aparelho já travado | `f2_save.jsonl` | sem efeito — 9 frames saíram, nada gravou |
| 4 | — | F3 com o aparelho já travado | `f3_upload.jsonl`, `f3_upload_retry.jsonl` | timeout; **slot 2 ficou vazio** |
| 5 | — | sondas (2× `list-user-irs`, 1× `dump-preset`, + pausas) | `rb_list1..3`, `rb_dump*` | tudo em timeout |
| 6 | 12:09 | **power-cycle físico** | — | volta sozinho, sem perda |
| 7 | 12:1x | **F3 limpo** (slot 2, 295 chunks) | `f3_sozinho.jsonl` | **OK**, 296 ACKs |
| 8 | — | read-back do preset | `rb_dump_depois.jsonl` | **byte-a-byte igual ao "antes"** |
| 9 | 12:1x | **F1 com `99.5` de novo, isolado** | `iso_f1.jsonl` + `iso_f1_sonda.jsonl` | **TRAVOU DE NOVO** — culpa do F1 confirmada |
| 10 | 12:2x | power-cycle | — | volta |
| 11 | 12:2x | **F1 com `15.0`** | `f1_setparam_15.jsonl` | **OK** — sondas responderam na hora |
| 12 | 12:2x | **F2 limpo** | `f2_save_15.jsonl` | **OK** — sondas responderam na hora |
| 13 | 12:2x | read-back final | `rb_dump_final2.jsonl` | knob `99.0 → 15.0` na página 0 |

## Pastas

- **`verde/`** — os três fluxos que **passaram**, com os nomes canônicos que o
  juiz (`scripts/h2_compare.py`) consome. É esta pasta que dá `CONFORME` nos
  três fluxos.
- **`e99/`** — os mesmos três fluxos com o valor `99.5`, preservados porque são
  a evidência do travamento. **Não** é a sessão verde; é o relatório do bug.
- **`poll/`** — as sondas de 10 s durante a recuperação, cada tentativa em um
  arquivo. Mostram a porta ausente e depois o retorno.

## Scripts (reproduzem os achados)

| Script | O que prova |
|---|---|
| `decode_knobs.py` | As 92 amostras reais de knob da Suite. Para o par exato do F1 (`slot 3` / `0x0700006e` / ctrl 0) a faixa é **15.0 … 99.0** — o `99.5` do runbook estava acima. |
| `decode_dump_diff.py` | Compara `antes.jsonl` com `rb_dump_final2.jsonl` e mostra o `f32` do knob mudando **99.0 → 15.0** na página 0, mais o CRC recalculado. |

```bash
python analysis/h2/20261005_120024/decode_knobs.py
python analysis/h2/20261005_120024/decode_dump_diff.py
python scripts/h2_compare.py analysis/h2/20261005_120024/verde --markdown
```

## Duas armadilhas desta sessão

1. **D6 não é device travado.** Depois de um power-cycle, a *primeira*
   transação costuma tomar timeout e a segunda responde sem nada ter mudado.
   Um timeout só não prova nada — foi preciso repetir várias vezes para
   distinguir as duas coisas.
2. **`IN: 0` no log não é falha.** F1 e F2 são fire-and-forget por projeto
   (D3/D4). O `IN: 0` deles é o comportamento esperado; o do F3, não.