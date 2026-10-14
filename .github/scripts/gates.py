#!/usr/bin/env python3
"""gates.py — roda TODOS os gates de script do repo de uma vez.

POR QUE ISTO EXISTE. Os gates de script vivem espalhados em varios jobs do
`ci.yml` e em varios lugares do `docs/CONTRIBUTING.md`. Rodar "o gate" antes de
commitar significava saber quais eram, e esquecer um significava descobrir o
problema na CI. Este script e a lista: um comando, todos os gates, e a ordem
que a CI usa.

    uv run python .github/scripts/gates.py            # todos
    uv run python .github/scripts/gates.py --list     # so a lista
    uv run python .github/scripts/gates.py check_bundle validate   # so estes

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


ROOT = Path(__file__).resolve().parent.parent.parent

# (nome, comando, porque roda assim). A ordem e a mesma do `ci.yml`.
GATES: list[tuple[str, list[str], str]] = [
    (
        "validate_workflows",
        [sys.executable, ".github/scripts/validate_workflows.py"],
        "sintaxe + contratos do pipeline + ordem dos jobs",
    ),
    (
        "check_bundle",
        ["uv", "run", "--no-project", "--with", "jsonschema", "python",
         ".github/scripts/check_bundle.py"],
        "schema do Tauri, icones, .deb/PKGBUILD, tarball de fonte",
    ),
    (
        "check_issue_templates",
        [sys.executable, ".github/scripts/check_issue_templates.py"],
        "os templates de issue cumprem o contrato (secoes obrigatorias + prompt, #170)",
    ),
    (
        "sync_version",
        [sys.executable, ".github/scripts/sync_version.py", "--check"],
        "os 5 manifests de versao em sincronia",
    ),
    (
        "check_deadcode",
        [sys.executable, ".github/scripts/check_deadcode.py"],
        "todo export de src/ tem consumidor fora do arquivo",
    ),
    (
        "check_module_size",
        [sys.executable, ".github/scripts/check_module_size.py"],
        "orcamento de tamanho do front (App.tsx < 300; docs/dev/ARCHITECTURE.md §4)",
    ),
    (
        "byte_order",
        [sys.executable, "tools/check_byte_order.py"],
        "o effectCode e u32 LE no fio real — se alguem escrever BE, os decoders mentem",
    ),
    (
        "param_ranges",
        [sys.executable, "tools/param_ranges.py", "--check"],
        "a trava de conteudo do knob (#110) bate com o dicionario e com as 92 amostras reais",
    ),
    (
        "state_pages",
        [sys.executable, "tools/validate_state_pages.py"],
        "o layout das paginas 13xx casa no modelo deterministico (#155) — o artefato e embutido no binario",
    ),
    (
        "baseline",
        [sys.executable, "tools/baseline.py", "--check"],
        "a baseline do golden tem versao, hash E motivo (R2/R3)",
    ),
    (
        "pytest",
        ["uv", "run", "pytest", "-q"],
        "trava da especificacao (provas A-E do golden + suite do H3)",
    ),
    (
        "check_base_images",
        ["uv", "run", "python", ".github/scripts/check_base_images.py"],
        "toda FROM/COPY --from resolve o default de ARG",
    ),
    (
        "simulate_release",
        [sys.executable, ".github/scripts/simulate_release.py"],
        "cadeia rc -> promote executada de verdade (LENTO: ~2 min; no CI só "
        "por escopo — schedule, tag/dispatch e PR de release, #246)",
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