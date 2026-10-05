#!/usr/bin/env python3
"""check_orphans.py — o que ninguém referencia.

Varre os scripts de `analysis/` e `scripts/` e procura o nome de cada um em
QUALQUER lugar que possa legitimamente consumi-lo: docs, skills (.agents),
workflows do CI, gates.py, o próprio código, e a configuração do repo.

Um script sem referência não é lixo automaticamente — o `analysis/` guarda o
rastro da engenharia reversa, e um script de sessão pode valer como evidência.
O que este relatório faz é separar "ninguém usa" de "não sei para que serve".

**A referência esperada é o [`analysis/PROVENANCE.md`](PROVENANCE.md)**, que
mapeia cada script para o artefato que ele gera. Isso é de propósito: o valor
deste check não é o zero de hoje — é pegar o script que alguém adicionar
amanhã **sem** ponteiro nenhum, que é como os 6 anteriores chegaram aqui.
Se ele acusar, a resposta normal é acrescentar a linha no PROVENANCE; apagar
só se o script não tiver entrada nem saída (foi o caso do `parse_preset_xml.py`).

Uso:  python analysis/check_orphans.py
"""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# onde uma referência vale como "alguém usa isto"
FONTES = [
    *ROOT.glob("docs/**/*.md"),
    *ROOT.glob(".agents/**/*"),
    *ROOT.glob(".github/**/*.yml"),
    *ROOT.glob("scripts/**/*"),
    *ROOT.glob("packages/**/*.rs"),
    *ROOT.glob("packages/**/*.ts"),
    *ROOT.glob("packages/**/*.tsx"),
    *ROOT.glob("analysis/*.py"),
    *ROOT.glob("analysis/*.md"),
    *ROOT.glob("analysis/tests/**/*"),
    ROOT / "README.md",
    ROOT / "knowledge.md",
    ROOT / "CONTRIBUTING.md",
    ROOT / "pyproject.toml",
    ROOT / "Makefile",
]

def carregar_fontes() -> str:
    pedacos = []
    for f in FONTES:
        if not f.is_file():
            continue
        try:
            pedacos.append(f.read_text(encoding="utf-8", errors="replace"))
        except OSError:
            continue
    return "\n".join(pedacos)


def main() -> int:
    corpus = carregar_fontes()
    alvos = sorted(
        [p for p in (ROOT / "scripts").glob("*.py")]
        + [p for p in (ROOT / "scripts").glob("*.ps1")]
        + [p for p in (ROOT / "scripts").glob("*.sh")]
        + [p for p in (ROOT / "analysis").glob("*.py")]
    )

    orfaos = []
    for p in alvos:
        nome = p.name
        # conta ocorrências fora do próprio arquivo
        ocorrencias = corpus.count(nome)
        proprio = 0
        if nome in p.read_text(encoding="utf-8", errors="replace"):
            proprio = 1  # o docstring geralmente repete o próprio nome
        usos = ocorrencias - proprio
        if usos <= 0:
            orfaos.append((p.relative_to(ROOT).as_posix(), p.stat().st_size))

    print(f"scripts varridos: {len(alvos)}")
    print(f"sem NENHUMA referência fora de si mesmos: {len(orfaos)}\n")
    if orfaos:
        print(f"{'arquivo':<48} {'bytes':>8}")
        print("-" * 58)
        for nome, tam in orfaos:
            print(f"{nome:<48} {tam:>8}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
