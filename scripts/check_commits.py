#!/usr/bin/env python3
"""
Gate de mensagens de commit (conventional commits).

Por que um script e não um `if:` no YAML: a faixa a checar muda com o evento
(push usa `before..HEAD`, PR usa a base do PR) e a lista de commits aceitos é
regras —rules que Testing Business aterram mais rápido em Python do que
embutidas num bloco bash de 20 linhas.

Padrão aceito: `tipo(escopo)?!?: assunto`
  tipo ∈ feat | fix | perf | revert | deps | docs | chore | ci | test | refactor | style | build
O `!` marca breaking change (o version-bump lê o mesmo formato).

Merges são ignorados (`--no-merges`): em `pull_request` o GitHub faz o checkout
de um commit SINTETICO (`Merge <head> into <base>`) que nao e de ninguem — ele
nao pode reprovar o PR de quem escreveu certo. Nos merges reais o squash do
GitFlow ja deixou a mensagem no padrao, entao nada se perde.

Uso: python3 scripts/check_commits.py   (env: BEFORE, PR_BASE)
"""
from __future__ import annotations

import os
import re
import subprocess

# `deps` e `build` bumps de patch (version-bump); `style`/`refactor` não bumpam.
PATTERN = re.compile(r"^[a-z][a-z0-9_-]*(\([^\)]+\))?!?: .+")


def sh(*args: str) -> str:
    return subprocess.run(args, stdout=subprocess.PIPE, text=True).stdout


def exists(rev: str) -> bool:
    """`git cat-file -e` não escreve NADA no stdout quando dá certo — o sinal é
    o exit code. Ler stdout (o que o código fazia) dava `False` sempre e jogava
    o gate no `HEAD~1..HEAD`, deixando passar qualquer push com 2 commits ruins.
    """
    return (
        subprocess.run(["git", "cat-file", "-e", f"{rev}^{{commit}}"]).returncode == 0
    )


def main() -> int:
    pr_base = os.environ.get("PR_BASE", "")
    before = os.environ.get("BEFORE", "")

    if pr_base:
        rng = f"{pr_base}..HEAD"
    elif before and set(before) != {"0"} and exists(before):
        rng = f"{before}..HEAD"
    else:
        rng = "HEAD~1..HEAD"

    subjects = [
        s
        for s in sh("git", "log", "--format=%s", "--no-merges", rng).splitlines()
        if s.strip()
    ]
    if not subjects:
        print("nenhum commit na faixa — nada a checar")
        return 0

    bad: list[str] = []
    for subject in subjects:
        if PATTERN.match(subject):
            print(f"OK  {subject}")
        else:
            print(f"FORA DO PADRÃO  {subject}")
            bad.append(subject)

    if bad:
        print(f"\n{len(bad)} commit(s) fora do padrão `tipo(escopo)?: assunto` — ex.: feat(ui): …")
        return 1
    print(f"\n{len(subjects)} commit(s) no padrão ({rng})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())