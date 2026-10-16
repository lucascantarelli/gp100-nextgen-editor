#!/usr/bin/env python3
"""check_docs_modulos.py — o documento que DESCREVE um modulo acompanha o modulo (#275).

POR QUE ISTO EXISTE. O P7 ("doc e harness acompanham, na MESMA PR que muda
comportamento") vale para quem lembra. Depois do merge ninguem mede nada: um PR que
mexe na superficie de um modulo e NAO toca no documento que o descreve passa verde, e
o documento apodrece ate alguem reler as 13 mil linhas de `docs/**`. E a mesma classe
do #71 na documentacao — a regra existe (INDEX §4, skill `docs-sync`, P7) e o juiz nao.

O gate do #270 responde outra pergunta: ele pega LINK morto (estrutura). Um documento
pode citar um caminho que existe e descrever uma API que nao existe mais: link vivo,
conteudo morto. Medido em 15/10/2026 contra `origin/develop`: 40 docs vivos citando
78 caminhos de codigo, 8 pares em que o modulo andou depois do doc — e **4 desses com
mudanca de SUPERFICIE**, em 2 documentos (`docs/tecnico/PACKAGING.md` e
`docs/planejamento/RELEASE_PLAN.md`). Os outros 4 passam: mexeram corpo, nao contrato.

O QUE ELE MEDE, e por que assim:

  * CITACAO, nao mapa escrito a mao. O conjunto vem do proprio documento (os caminhos
    de codigo que ele cita): um doc nao pode escapar por omissao, e nao ha segunda
    lista para divergir. Isencao existe, mas e DECLARADA com motivo (`ISENCOES`) e
    aparece no diff — isencao orfa (o doc nao cita mais o caminho) reprova.
  * ANCESTRALIDADE, nao relogio. "O modulo mudou depois do doc" e
    `git merge-base --is-ancestor <commit do doc> <commit do modulo>`: dois commits do
    mesmo dia nao tem ordem, e mtime nao existe no git.
  * SUPERFICIE, nao corpo. So cobra quando o diff do modulo (desde a revisao do doc)
    mexe em linha de assinatura/export/`impl`/constante/seletor. Refatoracao interna
    nao obriga documento — e o gate que cobra tudo vira ruido, e ruido vira isencao
    em massa.
  * APONTA O DOCUMENTO. A violacao diz qual doc, qual modulo, quantas linhas de
    superficie e os dois shas — o produto e o nome do arquivo que alguem tem de
    reler, nao a contagem.
  * PREVISAO DO SQUASH (#282). O gate roda na arvore do PR (merge sintetico) e a
    pergunta que ele faz no `develop` — `--is-ancestor(doc, modulo)` — NAO TEM
    RESPOSTA quando os dois commits vivem em linhas PARALELAS: medido no PR #260,
    o doc tinha por ultimo toque `b117765` (que chegou na branch pelo merge da
    base) e o modulo `a06c7fc` (o corte, escrito antes daquele merge) —
    `is-ancestor(b117765, a06c7fc)` e FALSO, o gate le "o modulo nao andou depois
    do doc" e a CI do PR fica verde; no squash o modulo passa a ter por ultimo
    toque o commit do merge, descendente de `b117765`, e a `develop` fica verde
    so ate o merge (run 38900637312 vermelha). Duas `develop` vermelhas na mesma
    sessao (#277/#279), cada uma custando um PR de docs-sync. Em modo PR
    (`PR_BASE`, que a CI passa) o par e medido DENTRO DO RANGE: cobra quando o PR
    toca a superficie do modulo e o doc nao acompanhou no mesmo range — o mesmo
    regime que o #264 deu ao gate de movimento, e o vermelho chega no PR, onde
    ainda da para consertar.

LIMITES DECLARADOS (o que este gate NAO ve):
  * citacao SEM caminho (`session.rs` solto) nao entra: o gate nao adivinha o modulo.
    E uma escolha MEDIDA, nao preguica: aceitar o nome unico do arquivo subiria de 78
    para 290 pares em 15/10 e obrigaria o `INDEX.md` (o mapa cita os ~50 scripts por
    nome) e o `DECISIONS.md` a acompanhar a superficie de TODO script — isso e ruido,
    e ruido vira isencao em massa, que e o mesmo que nao ter gate;
  * dado e configuracao (`.json`, `.toml`, `.yml`) ficam fora: mudam por dado, nao por
    contrato descrito;
  * `docs/arquivo/**` (historico) e `docs/analises/**` (evidencia datada) sao
    excluidos por GENERO — eles descrevem o repo numa data, envelhecer e o trabalho
    deles (a mesma isencao que o `check_docs.py` usa para o contador);
  * link morto, banner e contador sao do `check_docs.py` (#270) — aqui nao se duplica.

Uso: python3 .github/scripts/check_docs_modulos.py          (exit 1 se houver violacao)
     python3 .github/scripts/check_docs_modulos.py --list   (os pares medidos, com veredito)
     python3 .github/scripts/check_docs_modulos.py --base <sha>   (regime do PR: previsao do squash)
A base do PR vem do ambiente (`PR_BASE`), como no `check_move_only.py`; sem ela o gate
roda no regime da `develop` (ancestralidade). Local: passe `--base`.
"""
from __future__ import annotations

import os
import pathlib
import re
import subprocess
import sys

try:  # console cp1252 do Windows: sem isto o proprio gate morre no relatorio
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[union-attr]
except (AttributeError, OSError):  # pragma: no cover
    pass

# `parents[2]`: este arquivo mora em `.github/scripts/` (armadilha do #168).
RAIZ = pathlib.Path(__file__).resolve().parents[2]

# ── documentos VIVOS: os que descrevem o repo de hoje ────────────────────────
FONTES = [
    "docs/**/*.md",
    ".agents/**/*.md",
    "README.md",
    "knowledge.md",
]
# Historico e evidencia datada: envelhecer e o trabalho deles (genero, nao excecao).
GENERO_FORA = ("docs/arquivo/", "docs/analises/")

# ── o que conta como MODULO: codigo-fonte, com caminho ───────────────────────
# Dado/config (`.json`, `.toml`, `.yml`) fica fora: muda por dado, nao por contrato
# descrito. Caminho sem diretorio (`session.rs` solto) tambem: o gate nao adivinha.
CODIGO = re.compile(
    r"(?<![\w./-])((?:packages|tools|\.github|packaging)/[\w./-]+"
    r"\.(?:rs|ts|tsx|js|mjs|py|css))"
)

# A SUPERFICIE de cada linguagem: o que um documento descreve. Corpo de funcao,
# comentario, reindentacao e formatacao NAO entram (nao obrigam documento).
SUPERFICIE = {
    ".rs": re.compile(r"^[+-]\s*(pub\b|impl\b)"),
    ".ts": re.compile(r"^[+-]\s*(export\b|declare\b|interface\b|type\b|enum\b|class\b)"),
    ".tsx": re.compile(r"^[+-]\s*(export\b|declare\b|interface\b|type\b|enum\b|class\b)"),
    ".js": re.compile(r"^[+-]\s*(export\b|function\b|class\b)"),
    ".mjs": re.compile(r"^[+-]\s*(export\b|function\b|class\b)"),
    # `def` publico (sem `_`), classe e constante de modulo; decorator tambem e contrato.
    ".py": re.compile(r"^[+-]\s*(def [a-z]|class [A-Z]|@|[A-Z][A-Z0-9_]* =)"),
    # Seletor, custom property ou at-rule: o vocabulario que um doc de design descreve.
    ".css": re.compile(r"^[+-]\s*([.:@#]|--)"),
}

# ── ISENCOES: par (doc, modulo) que e CITACAO, nao descricao ─────────────────
# Vazio de proposito: hoje nenhum par precisa. O uso e a excecao declarada — o
# motivo entra no diff e o revisor o le. Isencao orfa (o doc nao cita mais o
# caminho, ou ele sumiu) REPROVA: sem isso a isencao vira deposito.
ISENCOES: dict[tuple[str, str], str] = {}

VIOLACOES: list[str] = []

# Cache por (raiz, caminho): o teste roda em repositorios sinteticos, e a chave
# inclui a raiz para um repo nao herdar o sha de outro.
_CACHE: dict[tuple[str, str], tuple[str, str]] = {}


def viola(msg: str) -> None:
    VIOLACOES.append(msg)


def rel(p: pathlib.Path) -> str:
    return str(p.relative_to(RAIZ)).replace("\\", "/")


def _git(*args: str) -> str:
    saida = subprocess.run(
        ["git", *args],
        cwd=RAIZ,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    return saida.stdout


def docs_vivos() -> list[pathlib.Path]:
    """Os documentos vivos, ordenados (fora os de genero historico/evidencia)."""
    achados: list[pathlib.Path] = []
    for padrao in FONTES:
        achados += list(RAIZ.glob(padrao))
    return sorted(
        {
            p
            for p in achados
            if p.is_file()
            and not any(rel(p).startswith(fora) for fora in GENERO_FORA)
        }
    )


def citacoes(texto: str) -> set[str]:
    """Os caminhos de CODIGO que o texto cita e que existem no repo."""
    return {c for c in CODIGO.findall(texto) if (RAIZ / c).exists()}


def ultimo_commit(caminho: str) -> tuple[str, str]:
    """(sha, data) do ultimo commit que tocou `caminho` — ('', '') quando nao ha."""
    chave = (str(RAIZ), caminho)
    if chave not in _CACHE:
        saida = _git("log", "-1", "--format=%H\t%cd", "--date=short", "--", caminho)
        campos = saida.strip().split("\t")
        _CACHE[chave] = (campos[0], campos[1]) if len(campos) > 1 else ("", "")
    return _CACHE[chave]


def ancestral(anterior: str, posterior: str) -> bool:
    """O commit `anterior` esta no passado de `posterior`? (ordem, nao relogio.)"""
    return (
        subprocess.run(
            ["git", "merge-base", "--is-ancestor", anterior, posterior],
            cwd=RAIZ,
            capture_output=True,
        ).returncode
        == 0
    )


def revisao_no_range(base: str, caminho: str) -> str:
    """O commit MAIS NOVO do range `base..HEAD` que tocou `caminho` ('', nenhum).

    E a leitura da previsao do squash (#282): dentro do range os commits estao na
    MESMA linha, entao a pergunta "o doc acompanhou o modulo?" tem resposta — o
    que a ancestralidade contra o `develop` nao dava.
    """
    saida = _git("log", "--format=%H", f"{base}..HEAD", "--", caminho)
    return saida.strip().splitlines()[0] if saida.strip() else ""


def superficie_desde(revisao: str, modulo: str) -> list[str]:
    """Linhas de SUPERFICIE que o modulo mexeu desde `revisao` (ate o HEAD)."""
    padrao = SUPERFICIE.get(pathlib.Path(modulo).suffix)
    if padrao is None:  # pragma: no cover — CODIGO so casa as extensoes do mapa
        return []
    diff = _git("diff", revisao, "HEAD", "--", modulo)
    return [linha for linha in diff.splitlines() if padrao.match(linha)]


def pares() -> list[tuple[str, str]]:
    """(doc, modulo) de tudo que o repo cita hoje — a materia-prima do contrato."""
    medidos = {
        (rel(d), c) for d in docs_vivos() for c in citacoes(d.read_text(encoding="utf-8", errors="replace"))
    }
    return sorted(medidos)


def checa_pares(medidos: list[tuple[str, str]], base: str = "") -> None:
    for doc, modulo in medidos:
        if (doc, modulo) in ISENCOES:
            continue
        if base and checa_previsao(doc, modulo, base):
            continue
        doc_sha, doc_data = ultimo_commit(doc)
        mod_sha, mod_data = ultimo_commit(modulo)
        if not doc_sha or not mod_sha or doc_sha == mod_sha:
            continue  # mesmo commit (ou arquivo novo): acompanhou por construcao
        if not ancestral(doc_sha, mod_sha):
            continue  # o doc veio DEPOIS do modulo: esta mais novo, nao mais velho
        linhas = superficie_desde(doc_sha, modulo)
        if linhas:
            viola(
                f"VELHO  {doc}: descreve {modulo}, que mexeu na superficie "
                f"({len(linhas)} linha(s)) depois da ultima revisao do doc "
                f"({doc_data} {doc_sha[:7]} -> {mod_data} {mod_sha[:7]}) — "
                "releia o documento (a data do banner conta como revisao, INDEX §4 item 1) "
                "ou declare ISENCOES[(doc, modulo)] com o motivo"
            )


def checa_previsao(doc: str, modulo: str, base: str) -> bool:
    """Modo PR (#282): o range decide. Devolve True quando o par ja foi cobrado.

    Em modo PR a ancestralidade por arquivo nao tem resposta (linhas paralelas):
    o par e cobrado quando o RANGE toca o modulo e o doc NAO acompanhou no mesmo
    range — ou acompanhou ANTES do ultimo toque do modulo, que e o mesmo veredito
    que a `develop` dara depois do squash.
    """
    mod_range = revisao_no_range(base, modulo)
    if not mod_range:
        return False  # o PR nao toca o modulo: nao e trabalho dele
    doc_range = revisao_no_range(base, doc)
    if doc_range and ancestral(mod_range, doc_range):
        return False  # o doc acompanhou o modulo DENTRO do PR
    doc_sha, doc_data = ultimo_commit(doc)
    revisao = doc_sha or base
    linhas = superficie_desde(revisao, modulo)
    if linhas:
        viola(
            f"PREVISTO  {doc}: o PR mexe na superficie de {modulo} ({len(linhas)} "
            f"linha(s)) e o documento nao acompanhou no mesmo range (ultima revisao: "
            f"{doc_data} {doc_sha[:7] or base[:7]}) — depois do squash a `develop` fica "
            "vermelha: releia o documento neste PR ou declare ISENCOES[(doc, modulo)] "
            "com o motivo"
        )
        return True
    return False


def checa_isencoes(medidos: list[tuple[str, str]]) -> None:
    for (doc, modulo), motivo in sorted(ISENCOES.items()):
        if not motivo.strip():
            viola(f"ISENCAO  {doc} -> {modulo}: sem motivo — isencao e excecao declarada, nao deposito")
        if (doc, modulo) not in medidos:
            viola(
                f"ISENCAO  {doc} -> {modulo}: orfa — o doc nao cita mais esse caminho "
                "(ou ele nao existe): a isencao ficou sem materia, tire a linha"
            )


def base_do_range(argv: list[str]) -> str:
    """A base contra a qual o squash vai ser criado: `--base`, senao `PR_BASE`.

    Sem ela o gate roda no regime do `develop` (ancestralidade) — e e assim que o
    push e o schedule rodam.
    """
    if "--base" in argv:
        return argv[argv.index("--base") + 1]
    return os.environ.get("PR_BASE", "").strip()


def main(argv: list[str]) -> int:
    medidos = pares()
    base = base_do_range(argv)
    checa_pares(medidos, base)
    checa_isencoes(medidos)

    docs = {d for d, _ in medidos}
    regime = f"previsao do squash ({base[:7]})" if base else "ancestralidade (develop)"
    print(
        f"docs vivos: {len(docs_vivos())} | citando codigo: {len(docs)} | "
        f"pares: {len(medidos)} | regime: {regime}"
    )

    if "--list" in argv:
        for doc, modulo in medidos:
            doc_sha, _ = ultimo_commit(doc)
            mod_sha, _ = ultimo_commit(modulo)
            if (doc, modulo) in ISENCOES:
                veredito = "isento"
            elif not doc_sha or not mod_sha or doc_sha == mod_sha or not ancestral(doc_sha, mod_sha):
                veredito = "ok"
            elif superficie_desde(doc_sha, modulo):
                veredito = "VELHO"
            else:
                veredito = "ok (corpo)"
            print(f"  {veredito:10} {doc} -> {modulo}")
        return 1 if VIOLACOES else 0

    print(f"violacoes: {len(VIOLACOES)}")
    for v in VIOLACOES:
        print(f"  {v}")
    return 1 if VIOLACOES else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
