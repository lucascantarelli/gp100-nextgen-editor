> 📜 **histórico** — Era de RE, arquivado na #168 · última revisão 2026-10-13.

# scripts-era-RE — o que restou de `scripts/` (#168)

Cápsula fechada em 13/10 (HW-8). A pasta `scripts/` do repo foi **dissolvida**:
cada item foi para a casa do seu ciclo de vida (ADR-16, `docs/dev/DECISIONS.md`).
O que provava a **Era de RE fechada** veio para cá — é **proveniência**, não
processo: nada aqui roda na CI nem é insumo de runtime.

| Item | O que era |
|---|---|
| `h1_field.sh` · `h1_compare.py` · `test_h1_compare.py` | Runbook do gate H1 (leitura) e juiz da Fase C (níveis 1/2/3 + tabela do §5 do `H1_REPORT.md`) |
| `h2_field.sh` · `h2_compare.py` · `test_h2_compare.py` | Runbook do gate H2 (escrita) e juiz da transcrição do H2 contra a referência de escrita (#22) |
| `wirelog_compare.py` | Núcleo comum dos juízes H1/H2 (#141): `Frame` do schema P4, parser e helpers de tabela |
| `add_cargo_path.ps1` | Fix idempotente do PATH do cargo no sistema (HKLM, admin 1×, 29/09) — o passo virou `knowledge.md` §Armadilhas |

As evidências que os juízes leem (`h1_reference/`, `h2/`, `h2_reference/`)
continuam em `docs/arquivo/analysis/`. A captura e o julgamento de campo
atuais vivem no fluxo H5 (`docs/tecnico/H5_CHECKLIST.md`) — este diretório não
é mais o caminho de nenhum gate.
