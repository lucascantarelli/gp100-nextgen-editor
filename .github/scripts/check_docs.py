#!/usr/bin/env python3
"""check_docs.py — higiene da documentacao (`docs/**`, `README.md`, `knowledge.md`).

POR QUE ISTO EXISTE. O `docs/INDEX.md` §4 escreve tres convencoes (banner de status
com data; documento obsoleto com 📜 e ponteiro; contador so no §6) e a skill
`docs-sync` repete a regra do contador (regra 7) — e nenhuma delas tinha gate: o repo
dependia de alguem reler as 55 fontes (13.431 linhas, medido em 15/10/2026). Rodando
ESTE script contra `origin/develop` ANTES do conserto: **37 violacoes** — 36 de banner
em **24 arquivos** (faltava marcador ou data) e 2 contadores no `knowledge.md` — sobre
**27 contadores fora do §6** e **0 link morto** (o repo esta limpo hoje; a checagem de
link e guarda de regressao, nao conserto). Depois do conserto: **0 violacoes** e 25
contadores congelados no baseline.

TRES CHECAGENS:
 1. LINK — link relativo e ancora (`arquivo.md#secao`) resolvem; a ancora e conferida
    contra os cabecalhos do alvo. Ancora de LINHA (`#L42`) nao e ancora de markdown.
 2. BANNER — as primeiras linhas trazem marcador (atual/historico/parcial) E data; e o
    marcador de HISTORICO exige PONTEIRO para o substituto (§4.4 do INDEX: documento
    obsoleto nao e apagado, ganha o banner apontando para onde o assunto vive agora).
 3. CONTADOR — numero de teste/suite/arquivo/job/gate/baseline/icone (e cobertura %)
    fora do `INDEX.md` §6. `docs/analises/**` (evidencia com data) e `docs/arquivo/**`
    (historico) NAO contam. O que ja existia entrou como BASELINE congelado, no padrao do
    `check_module_size`: "o teto e o tamanho da medicao — a regra e nao crescer mais".

FALSO POSITIVO QUE O PADRAO EVITA: `99,57%` do `PROTOCOL.md` e metrica de protocolo, nao
cobertura — porcentagem so conta com vocabulario de cobertura ao lado.

Uso: python3 .github/scripts/check_docs.py            (exit 1 se houver violacao)
     python3 .github/scripts/check_docs.py --list      (so o resumo)
     python3 .github/scripts/check_docs.py --baseline  (imprime a BASELINE_CONTADOR sugerida)
"""
from __future__ import annotations

import pathlib
import re
import sys

try:  # console cp1252 do Windows: sem isto o proprio gate morre no relatorio
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[union-attr]
except (AttributeError, OSError):  # pragma: no cover
    pass

# `parents[2]`: este arquivo mora em `.github/scripts/` (armadilha do #168).
RAIZ = pathlib.Path(__file__).resolve().parents[2]

# Documentos com convencao de banner/casa unica. `docs/arquivo/` entra: a convencao 4 da
# §4 do INDEX cobra o banner de HISTORICO no proprio arquivado (e o diretorio ja diz isso).
FONTES = sorted(p for p in (RAIZ / "docs").glob("**/*.md"))
FONTES += [RAIZ / "README.md", RAIZ / "knowledge.md"]

# Prosa de terceiro (dumps de ferramenta) pode citar caminho de fora; hoje a lista esta vazia.
CAMINHOS_DE_PROPOSITO: dict[str, str] = {}

# Contador congelado: arquivo -> (ocorrencias medidas em 15/10/2026, por que NAO se corrige agora).
# Sao o "antes" da Era Hardware (diagnostico com data) ou dado de plano que nao pertence ao §6.
BASELINE_CONTADOR: dict[str, tuple[int, str]] = {
    "docs/dev/UI_TEST_PLAN.md": (
        12,
        "plano com as medicoes POR FASE da era anterior (o 'antes' da Era Hardware), datadas; "
        "reescrever e decisao de conteudo, nao de gate",
    ),
    "docs/planejamento/ROADMAP.md": (
        6,
        "tabela de DIAGNOSTICO da era anterior: o retrato que justificou a Era Hardware, com data",
    ),
    "docs/CONTRIBUTING.md": (
        2,
        "citacao historica em prosa explicativa (14 testes de CLI por modo; '1777 linhas e 29 "
        "jobs' do incidente que gerou a #169)",
    ),
    "docs/referencias/UI_REFERENCE.md": (
        2,
        "referencia de design citando a contagem de baselines de uma medicao antiga",
    ),
    "docs/dev/DECISIONS.md": (
        1,
        "ADR citando a contagem de jobs da epoca da decisao",
    ),
    "docs/dev/UI_PLAN.md": (
        1,
        "plano antigo de UI citando a contagem de testes da epoca",
    ),
    "docs/referencias/REAL_DEVICE_GAP.md": (
        1,
        "diagnostico do gap do aparelho citando a contagem de jobs da medicao",
    ),
}

MARCA = re.compile("✅|📜|⚠️")
DATA = re.compile(r"\d{2}/\d{2}/\d{4}|\d{4}-\d{2}-\d{2}")
LINK = re.compile(r"\[[^\]]*\]\(([^)\s]+)\)")
# Ponteiro para o substituto (§4.4): link markdown para `.md` ou caminho entre backticks.
PONTEIRO = re.compile(r"\]\([^)]*\.md|`[^`]*\.md")
CABECALHO = re.compile(r"^#{1,6}\s+(.*)")

# O contador e o que a §6 do INDEX possui. Porcentagem so conta com vocabulario de cobertura.
CONTADOR = re.compile(
    r"(?<![\w.])(\d{2,})\s*(?:em\s*)?"
    r"(testes|su[íi]tes|arquivos|jobs|workflows|gates|baselines|targets|[íi]cones)\b"
    r"|\b(\d{1,3}[.,]\d{1,2})\s*%[^\n]{0,40}(cobertura|stmts|branch|funcs|linhas)"
    r"|(cobertura|stmts|branch|funcs)[^\n]{0,40}\b(\d{1,3}[.,]\d{1,2})\s*%",
    re.IGNORECASE,
)

VIOLACOES: list[str] = []
CONTADOS: dict[str, int] = {}


def viola(secao: str, msg: str) -> None:
    VIOLACOES.append(f"[{secao}] {msg}")


def rel(p: pathlib.Path) -> str:
    return str(p.relative_to(RAIZ)).replace("\\", "/")


def slug(texto: str) -> str:
    """Slug de cabecalho no padrao do GitHub (aproximado): minusculo, pontuacao fora."""
    return re.sub(r"[^\w\- ]", "", texto.strip().lower()).replace(" ", "-")


def cabecalhos(p: pathlib.Path) -> set[str]:
    achados = set()
    for linha in p.read_text(encoding="utf-8", errors="replace").splitlines():
        m = CABECALHO.match(linha)
        if m:
            achados.add(slug(m.group(1)))
    return achados


def checa_links(fontes: dict[pathlib.Path, str]) -> None:
    for p, texto in fontes.items():
        for m in LINK.finditer(texto):
            alvo = m.group(1)
            if alvo.startswith(("http://", "https://", "mailto:", "//")):
                continue
            caminho, _, ancora = alvo.partition("#")
            if caminho in CAMINHOS_DE_PROPOSITO or alvo in CAMINHOS_DE_PROPOSITO:
                continue
            destino = None
            if caminho:
                for base in (p.parent, RAIZ):
                    c = (base / caminho).resolve()
                    if c.exists():
                        destino = c
                        break
                if destino is None:
                    viola("LINK", f"{rel(p)} -> {alvo} (arquivo nao existe)")
                    continue
            elif not ancora:
                continue
            if not ancora or ancora.startswith("L") or destino is None:
                continue  # `#L42`/`#L42-L50` sao ancoras de LINHA, nao de markdown
            if destino.is_file() and destino.suffix == ".md":
                if slug(ancora) not in cabecalhos(destino):
                    viola("LINK", f"{rel(p)} -> {alvo} (ancora nao existe no alvo)")


def tem_frontmatter(texto: str) -> bool:
    """Arquivo que abre com bloco `---` e artefato COM cabecalho de maquina.

    Os 5 arquivos de `docs/arquivo/skills-era-RE/` sao skills arquivadas: o
    front-matter (`name`/`description`) e funcional e um banner markdown ANTES dele
    invalidaria o bloco. Nao e excecao de conveniencia — e a unica leitura possivel.
    """
    return texto.startswith("---\n") and "\n---\n" in texto[:2000]


def checa_banner(fontes: dict[pathlib.Path, str]) -> None:
    for p, texto in fontes.items():
        if tem_frontmatter(texto):
            continue
        cabeca = "\n".join(texto.splitlines()[:12])
        if not MARCA.search(cabeca):
            viola("BANNER", f"{rel(p)}: sem marcador de status (atual/historico/parcial) nas primeiras linhas")
        if not DATA.search(cabeca):
            viola("BANNER", f"{rel(p)}: sem data de revisao nas primeiras linhas")
        marca = MARCA.search(cabeca)
        if marca and marca.group(0) == "📜" and not PONTEIRO.search(cabeca):
            viola(
                "BANNER",
                f"{rel(p)}: banner de HISTORICO sem ponteiro para o substituto "
                "(§4.4 do INDEX: o arquivado aponta para onde o assunto vive agora)",
            )


def checa_contador(p: pathlib.Path, texto: str) -> None:
    relativo = rel(p)
    if relativo == "docs/INDEX.md" or "/analises/" in relativo or "/arquivo/" in relativo:
        return
    n = len(CONTADOR.findall(texto))
    if not n:
        return
    CONTADOS[relativo] = n
    teto, motivo = BASELINE_CONTADOR.get(relativo, (0, ""))
    if n > teto:
        extra = f" (baseline congelado: {teto} — {motivo})" if teto else ""
        viola(
            "CONTADOR",
            f"{relativo}: {n} contador(es) fora do `docs/INDEX.md` §6{extra}",
        )


def main(argv: list[str]) -> int:
    fontes = {p: p.read_text(encoding="utf-8", errors="replace") for p in FONTES if p.is_file()}
    faltando = [rel(p) for p in FONTES if not p.is_file()]
    if faltando:
        viola("INVENTARIO", f"arquivo listado e ausente: {', '.join(faltando)}")
    checa_links(fontes)
    checa_banner(fontes)
    for p, texto in fontes.items():
        checa_contador(p, texto)

    if "--baseline" in argv:
        for arq in sorted(CONTADOS, key=lambda a: -CONTADOS[a]):
            print(f'    "{arq}": ({CONTADOS[arq]}, "motivo a escrever"),')
        return 0

    print(f"docs: {len(fontes)} arquivos | contadores fora do §6: {sum(CONTADOS.values())}")
    if "--list" in argv:
        for arq in sorted(CONTADOS, key=lambda a: -CONTADOS[a]):
            teto = BASELINE_CONTADOR.get(arq, (0, ""))[0]
            print(f"  {CONTADOS[arq]:>3} (teto {teto})  {arq}")
        return 0
    print(f"violacoes: {len(VIOLACOES)}")
    for v in VIOLACOES:
        print(f"  {v}")
    return 1 if VIOLACOES else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
