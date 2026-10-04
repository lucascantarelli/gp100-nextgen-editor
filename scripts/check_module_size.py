#!/usr/bin/env python3
"""check_module_size.py — o gate que segura o orçamento de tamanho do front (#82).

POR QUE ISTO EXISTE. O critério da #82 ("App.tsx abaixo de 300 linhas") é um
número. Número sem gate é número que se cumpre no dia do PR e se esquece no mês
seguinte — e o ganho real da #82 era preventivo: cada marco novo adiciona estado
ao App, e o App volta a inchar. Um teto que ninguém cobra é teto que volta a
ser 514.

O gate é deliberadamente burro: conta LINHAS FISICAS. Ele não julga qualidade,
não lê AST e não tenta adivinhar intenção. O que ele faz é tornar visível, no
momento do erro, a hora em que um arquivo passou do combinado — e o mapa de onde
cada camada do front pode (e não pode) crescer está em `docs/ARCHITECTURE.md` §1.

TETO. O número é o teto de LINHAS (inclusive comentários e linhas em branco).
Contar só código daria um número que sobe e desce com o estilo de formatação, e
um gate que oscila sozinho é gate que a gente desliga.

EXCEÇÃO. `--allow` recebe `caminho:linhas`. Ela é para o arquivo que SABEMOS que
vai passar do teto e cujo corte está escrito como issue. Entrada nova aqui é
decisão consciente, não esquecimento — e o gate imprime quantas exceções estão
ativas, porque uma lista que ninguém vê deixa de ser revisão.

USO.
    python3 scripts/check_module_size.py
    python3 scripts/check_module_size.py --allow src/components/Foo.tsx:820
    python3 scripts/check_module_size.py --list
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
UI = ROOT / "packages" / "app" / "ui"

# (caminho relativo a packages/app/ui, teto de linhas, POR QUE este teto).
# A coluna do "por que" não é decoração: quando alguém quiser subir um teto, o
# motivo do teto antigo é a primeira coisa que ele precisa ler.
TETOS: list[tuple[str, int, str]] = [
    ("src/App.tsx", 300, "casca: acima disso, estado de sessao voltou para a casca"),
    ("src/components/LooperPanel.tsx", 700, "apresentacao grande; a FSM ja saiu para looper/fsm.ts"),
    ("src/ipc/device.ts", 600, "a porta cresce por operacao de device, nao por feature de UI"),
]

# Teto generico para o resto: acima disso, ou falta extrair regra de dominio
# (looper/, tuner/) ou falta extrair subcomponente.
TETO_PADRAO = 700


def conta_linhas(caminho: Path) -> int:
    """Linhas físicas do arquivo. Tolera o `\r` do checkout do Windows."""
    with caminho.open("r", encoding="utf-8", errors="replace") as fh:
        return sum(1 for _ in fh)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--list", action="store_true", help="lista os tetos e sai")
    ap.add_argument(
        "--allow",
        action="append",
        default=[],
        metavar="CAMINHO:LINHAS",
        help="excecao temporaria (o corte fica como issue)",
    )
    args = ap.parse_args()

    if args.list:
        for rel, teto, motivo in TETOS:
            print(f"{rel:<32} {teto:>5}  {motivo}")
        print(f"{'(restante)':<32} {TETO_PADRAO:>5}  regra padrao de components/")
        return 0

    if not UI.is_dir():
        print(f"[check_module_size] front nao encontrado: {UI}", file=sys.stderr)
        return 2

    # excecoes: caminho -> teto substituto
    excecoes: dict[str, int] = {}
    for item in args.allow:
        if ":" not in item:
            print(f"[check_module_size] --allow invalido (use caminho:linhas): {item}", file=sys.stderr)
            return 2
        rel, _, n = item.rpartition(":")
        if not n.isdigit():
            print(f"[check_module_size] --allow invalido (linhas nao numericas): {item}", file=sys.stderr)
            return 2
        excecoes[rel] = int(n)

    tetos = {rel: teto for rel, teto, _ in TETOS}
    excessos: list[tuple[str, int, int]] = []
    ausentes: list[str] = []

    def checa(rel: str, teto: int, obrigatorio: bool) -> None:
        caminho = UI / rel
        if not caminho.is_file():
            if obrigatorio:
                # Arquivo DECLARADO e ausente: o gate nao pode passar em
                # silencio, senao "apaguei o App.tsx" vira a forma mais barata de
                # aprovar. (Para os components/ a ausencia nao e erro: o teto
                # padrao so vale para o que existe.)
                ausentes.append(rel)
            return
        linhas = conta_linhas(caminho)
        if linhas > teto:
            excessos.append((rel, linhas, teto))

    for rel, teto, _ in TETOS:
        checa(rel, excecoes.get(rel, teto), obrigatorio=True)

    for caminho in sorted((UI / "src" / "components").glob("*.tsx")):
        rel = caminho.relative_to(UI).as_posix()
        checa(rel, excecoes.get(rel, TETO_PADRAO), obrigatorio=False)

    if ausentes:
        print("[check_module_size] declarado no orcamento e AUSENTE:", file=sys.stderr)
        for rel in ausentes:
            print(f"  {rel} — o teto dele sumiu junto com o arquivo", file=sys.stderr)
        print(
            "[check_module_size] Ou o arquivo foi renomeado (atualize o gate), ou "
            "ele saiu do front de proposito (atualize docs/ARCHITECTURE.md secao 4).",
            file=sys.stderr,
        )
        return 1

    if excessos:
        print("[check_module_size] Acima do teto (docs/ARCHITECTURE.md §4):", file=sys.stderr)
        for rel, linhas, teto in excessos:
            print(f"  {rel}: {linhas} > {teto} ({linhas - teto:+d})", file=sys.stderr)
        print(
            "[check_module_size] O corte de um teto e decisao: --list mostra o motivo\n"
            "  de cada um, ou o PR cita a issue que assume a excecao.",
            file=sys.stderr,
        )
        return 1

    if excecoes:
        print(
            f"[check_module_size] {len(excecoes)} excecao(oes) em uso: "
            + ", ".join(sorted(excecoes)),
            file=sys.stderr,
        )
    print("[check_module_size] frente dentro do orcamento", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
