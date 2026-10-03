#!/usr/bin/env python3
"""gates.py — roda TODOS os gates de script do repo de uma vez.

POR QUE ISTO EXISTE. Os gates de script vivem espalhados em varios jobs do
`ci.yml` e em varios lugares do `docs/CONTRIBUTING.md`. Rodar "o gate" antes de
commitar significava saber quais eram, e esquecer um significava descobrir o
problema na CI. Este script e a lista: um comando, todos os gates, e a ordem
que a CI usa.

    python3 scripts/gates.py            # todos
    python3 scripts/gates.py --list     # so a lista
    python3 scripts/gates.py check_bundle validate   # so estes

Cada gate roda no seu proprio ambiente (alguns precisam de `uv --with`, outros
nao), e a saida e resumida: o gate imprime o que tem de imprimir -- este
arquivo nao esconde erro de gate atras de um "falhou: 1".
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent

# (nome, comando, porque roda assim). A ordem e a mesma do `ci.yml`.
GATES: list[tuple[str, list[str], str]] = [
    (
        "validate_workflows",
        [sys.executable, "scripts/validate_workflows.py"],
        "sintaxe + contratos do pipeline + ordem dos jobs",
    ),
    (
        "check_bundle",
        ["uv", "run", "--no-project", "--with", "jsonschema", "python",
         "scripts/check_bundle.py"],
        "schema do Tauri, icones, .deb/PKGBUILD, tarball de fonte",
    ),
    (
        "sync_version",
        [sys.executable, "scripts/sync_version.py", "--check"],
        "os 5 manifests de versao em sincronia",
    ),
    (
        "pytest",
        ["uv", "run", "pytest", "-q"],
        "trava da especificacao (provas A-E do golden)",
    ),
    (
        "check_base_images",
        ["uv", "run", "python", "scripts/check_base_images.py"],
        "toda FROM/COPY --from resolve o default de ARG",
    ),
    (
        "simulate_release",
        [sys.executable, "scripts/simulate_release.py"],
        "cadeia rc -> promote executada de verdade (LENTO: ~2 min)",
    ),
]


def log(msg: str) -> None:
    print(f"[gates] {msg}", file=sys.stderr)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("nomes", nargs="*", help="gates especificos (default: todos)")
    ap.add_argument("--list", action="store_true", help="lista e sai")
    args = ap.parse_args()

    if args.list:
        for nome, _, motivo in GATES:
            print(f"{nome:22} {motivo}")
        return 0

    conhecidos = {g[0] for g in GATES}
    desconhecidos = [n for n in args.nomes if n not in conhecidos]
    if desconhecidos:
        log(f"gate desconhecido: {', '.join(desconhecidos)}")
        log(f"disponiveis: {', '.join(n for n, _, _ in GATES)}")
        return 2

    selecionados = [g for g in GATES if not args.nomes or g[0] in args.nomes]

    faltando = sorted({cmd[0] for _, cmd, _ in selecionados if not shutil.which(cmd[0])})
    if faltando:
        log(f"ferramenta ausente no PATH: {', '.join(faltando)}")
        return 2

    reprovados: list[str] = []
    for nome, cmd, motivo in selecionados:
        log(f"{nome} — {motivo}")
        if subprocess.run(cmd, cwd=ROOT).returncode != 0:
            reprovados.append(nome)

    print()
    if reprovados:
        log(f"REPROVADO: {', '.join(reprovados)}")
        return 1
    log(f"todos os {len(selecionados)} gates passaram")
    return 0


if __name__ == "__main__":
    sys.exit(main())