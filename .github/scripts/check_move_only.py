#!/usr/bin/env python3
"""check_move_only.py — o gate da auditoria de movimento (#263/#264).

POR QUE ISTO EXISTE. Um corte grande e o momento em que "movi" e "reescrevi de
passagem" ficam indistinguiveis no diff: 1 395 linhas saem de um arquivo e
aparecem em oito. Nenhum revisor confere isso linha a linha, e a suite de testes
prova COMPORTAMENTO — nao acusa comentario perdido, doc trocado nem um `if`
invertido que ainda passa. O corte do `session.rs` (#257/PR #260) precisou de uma
auditoria escrita a mao para provar o que o PR prometia; este gate e essa
auditoria, permanente.

O CONTRATO. Quem declara movimento puro declara no proprio commit, com um trailer
repetivel (um por arquivo de origem):

    Move-only: packages/core/src/session.rs

Sem trailer, o gate nao audita nada — custo zero para todo PR que nao e corte.

AS DUAS CLAUSULAS (por commit declarado, contra o PAI dele):

- **nao perdeu** — toda linha nao vazia da origem (no pai) existe nos arquivos
  tocados pelo commit, com a MESMA multiplicidade. A unica tolerancia e o
  qualificador de visibilidade que um corte pela metade do arquivo realmente
  precisa (`    fn tx_req(` -> `    pub(super) fn tx_req(`).
- **nao reescreveu** — toda linha ADICIONADA que nao casa com uma linha da origem
  tem de ser ANDAIME (vazia, comentario, atributo, `use`/`mod`, `impl ... {`,
  `}`). Qualquer outra linha adicionada reprova nomeando `arquivo:linha`: e esta
  clausula que impede um "corte" que na verdade mudou logica.

A clausula (b) vale para arquivos do MESMO tipo da origem (Rust -> Rust): mexer
em `docs/` no mesmo PR — ou no arquivo que ficou na origem — nao entra na conta,
porque nao e o destino do movimento.

SQUASH E MERGE DO GITHUB. `Squash and merge` nao entrega o diff do commit que
declarou: ele COLAPSA os commits do PR em um, e o diff do commit resultante e o PR
INTEIRO (o trailer sobrevive porque `squash_merge_commit_message: COMMIT_MESSAGES`
— a config deste repo — poe as mensagens dos commits no corpo do squash). Auditar
isso como se fosse UM movimento atribui a mudanca de qualquer commit IRMAO ao
movimento: medido, um squash limpo reprovava por uma linha escrita por outro
commit do PR — e no `develop` nao ha "tire o trailer" que conserte, porque o
commit que declarou nao existe mais. Por isso o commit de squash (a linha de
hifens que o GitHub poe antes do bloco de co-autoria) e o commit de merge (>= 2
pais) sao CASOS DE MOVIMENTO VALIDOS, com a auditoria separada por clausula:

- **nao perdeu** continua inteira, contra o pai (ou o primeiro pai): e a promessa
  central, e a unica que o colapso nao destroi;
- **nao reescreveu** continua coberta, porque uma linha reescrita sai da origem —
  a clausula (a) a acusa como PERDIDA;
- so a clausula do ANDAIME (conteudo NOVO) vira AVISO, porque a atribuicao
  linha->commit morre no colapso: a linha nova e do movimento ou de um commit
  irmao, e o diff colapsado nao diz qual. O AVISO conta e nomeia (fica na saida
  da CI), mas nao reprova — por commit, no PR, a clausula foi cobrada inteira.

O squash de UM commit nao tem o separador e nao precisa dele: o diff daquele
commit E o diff do movimento, e a auditoria sai estrita como sempre.

PREVISAO DO SQUASH. O squash junta os commits do PR, entao um commit IRMAO pode
perder uma linha que o commit do movimento deixou no lugar: o PR passava (o irmao
nao declara nada) e o `develop` reprovava depois do MERGE, quando o commit que
declarou nao existe mais. Por isso o gate audita tambem o RANGE INTEIRO como um
squash previsto — as origens declaradas no range contra a base dele, com a mesma
regra do squash (o "nao perdeu" inteiro, o andaime como AVISO) — e o vermelho
chega no PR, onde ainda da para consertar. A previsao nao roda quando o tip JA e
o squash/merge (seria a mesma auditoria, e a linha sairia duplicada) nem no modo
`--declared`. Ela usa a base do PR; se a base andar antes do merge, atualize a
branch (rebase) e o gate reavalia contra a base que o merge vai usar.

RANGE. Igual ao `check_commits.py`: em PR vale `PR_BASE`, em push vale `BEFORE`, e
sem nenhum dos dois o ultimo commit (`HEAD~1..HEAD`). `--from/--to` para uso local.
MERGES entram: um merge pode carregar o trailer, e ignorar o commit (o
`--no-merges` do `check_commits.py` existe para as MENSAGENS) deixava um movimento
declarado num merge sem auditoria nenhuma.

MODO RETROATIVO. `--declared CAMINHO` (repetivel) audita o range inteiro contra a
base dele, sem depender de trailer — para squash/merge, em que o commit original
nao existe mais, e para conferir um corte ja mergeado. O tip do range decide a
regra: sendo squash/merge, o diff auditado e o PR inteiro e a clausula do andaime
vira AVISO; sendo commit, e estrita.

USO.
    python3 .github/scripts/check_move_only.py
    python3 .github/scripts/check_move_only.py --from develop --to HEAD
    python3 .github/scripts/check_move_only.py --declared packages/core/src/session.rs --from develop

SAIDA. Uma linha por origem auditada: `N linha(s) de item, todas presentes` mais o
andaime contado, o rotulo `(squash)`/`(merge)` quando o diff auditado e o PR
inteiro e a contagem de AVISOs; na falha, as linhas perdidas e as que sairam do
andaime.
Exit: 0 ok/nada declarado · 1 violacao · 2 erro de uso (origem/base irresolvivel).
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent.parent

# `Move-only: <caminho>` — o trailer. Case-insensitive no nome para nao reprovar
# por teimosia de teclado; o caminho e o do REPO (como o diff mostra).
TRAILER = re.compile(r"^\s*move-only:\s*(\S.*?)\s*$", re.IGNORECASE)

# O que um corte pode PRECISAR acrescentar na assinatura (cruzar modulo).
VISIBILIDADE = ("pub", "pub(super)", "pub(crate)", "pub(self)")

# O andaime do arquivo novo: o que o movimento pode CRIAR. Fora daqui e conteudo.
ANDAIME = (
    re.compile(r"^(//!|///|//)"),  # comentario (doc de modulo, de item, solto)
    re.compile(r"^#!?\[.*\]$"),  # atributo / atributo interno
    re.compile(r"^(pub(\([^)]*\))?\s+)?(use|mod)\b"),  # imports e declaracao de modulo
    re.compile(r"^impl\b.*\{\s*$"),  # abertura do `impl` que agrupa os metodos
    re.compile(r"^\}\s*$"),  # fechamento
)

# `use` pode ocupar VARIAS linhas (o rustfmt quebra a lista quando nao cabe).
# A primeira linha o `ANDAIME` ja reconhece; as de dentro da lista e o `};` de
# fechamento so valem como andaime ENQUANTO a lista esta aberta — por isso o
# estado, e nao so o padrao: linha de codigo que termina em virgula nao vira
# andaime por acidente.
ABRE_USE = re.compile(r"^(pub(\([^)]*\))?\s+)?use\b")

# A marca do SQUASH de varios commits: o GitHub separa a lista das mensagens do PR
# do bloco de co-autoria agregado com uma linha de hifens (medido em `38bba33`, o
# squash do corte do `actor.rs`: nove). O squash de UM commit nao tem essa linha —
# e nao precisa dela, pelo motivo do cabecalho. O sinal so decide a FORMA da
# auditoria, nunca SE ela acontece: quem declara movimento continua sendo o
# trailer, e um commit normal segue estrito.
SEPARADOR_SQUASH = re.compile(r"^-{5,}$")


class UsoErro(Exception):
    """Nao da para auditar (origem/base irresolvivel) — exit 2, nao violacao."""


def git(raiz: Path, *args: str) -> str:
    """Roda git na raiz; stdout como texto. Falha de git vira erro de USO."""
    r = subprocess.run(
        ["git", *args], cwd=raiz, stdout=subprocess.PIPE, stderr=subprocess.PIPE
    )
    if r.returncode != 0:
        erro = r.stderr.decode("utf-8", "replace").strip()
        raise UsoErro(f"git {' '.join(args)}: {erro}")
    return r.stdout.decode("utf-8", "replace")


def existe(raiz: Path, rev: str) -> bool:
    """Existe a REV? (`git cat-file -e` nao escreve nada quando da certo.)"""
    r = subprocess.run(
        ["git", "cat-file", "-e", f"{rev}^{{commit}}"],
        cwd=raiz,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return r.returncode == 0


def existe_arquivo(raiz: Path, rev: str, caminho: str) -> bool:
    r = subprocess.run(
        ["git", "cat-file", "-e", f"{rev}:{caminho}"],
        cwd=raiz,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return r.returncode == 0


def conteudo(raiz: Path, rev: str, caminho: str) -> list[str]:
    """As linhas de um arquivo numa rev — erro de USO se ele nao existe la."""
    if not existe_arquivo(raiz, rev, caminho):
        raise UsoErro(f"origem declarada nao existe em {rev}: {caminho}")
    return git(raiz, "show", f"{rev}:{caminho}").splitlines()


def relevantes(linhas: list[str]) -> list[str]:
    """Linhas de conteudo: sem espaco no fim, sem as vazias (andaime)."""
    return [l.rstrip() for l in linhas if l.strip()]


VIS_PREFIXO = re.compile(r"^(\s*)(pub(\([^)]*\))?\s+)(.*)$")


def variantes(linha: str) -> list[str]:
    """As formas equivalentes de uma linha, nos DOIS sentidos.

    O corte pode acrescentar o qualificador (origem `fn tx_req(` -> destino
    `pub(super) fn tx_req(`) e a comparacao precisa funcionar de qualquer lado:
    quem procura e a linha da origem, quem e procurado e a do destino (e
    vice-versa, na clausula do andaime).
    """
    m = re.match(r"^(\s*)(.*)$", linha or "")
    indent, corpo = m.group(1), m.group(2)
    formas = [f"{indent}{vis} {corpo}" for vis in VISIBILIDADE]
    sem_prefixo = VIS_PREFIXO.match(linha or "")
    if sem_prefixo:
        formas.append(f"{sem_prefixo.group(1)}{sem_prefixo.group(4)}")
    return formas


def casa(linha: str, disponivel: Counter) -> str | None:
    """Consome a linha no destino: exata, ou com a visibilidade do corte."""
    for candidata in [linha, *variantes(linha)]:
        if disponivel.get(candidata, 0) > 0:
            disponivel[candidata] -= 1
            return candidata
    return None


def adicionadas(raiz: Path, rev_a: str, rev_b: str, caminho: str) -> list[str]:
    """As linhas que o diff ACRESCENTOU naquele arquivo (`--unified=0`)."""
    out = git(raiz, "diff", "--unified=0", rev_a, rev_b, "--", caminho)
    linhas = [
        l[1:].rstrip()
        for l in out.splitlines()
        if l.startswith("+") and not l.startswith("+++")
    ]
    return [l for l in linhas if l.strip()]


@dataclass
class Andaime:
    """Classificador do andaime, com o estado do `use` de varias linhas."""

    dentro_use: bool = False

    def aceita(self, linha: str) -> bool:
        if self.dentro_use:
            if linha.endswith("};"):
                self.dentro_use = False
                return True
            return linha.endswith(",")  # item da lista de import
        if ABRE_USE.match(linha):
            self.dentro_use = linha.rstrip().endswith("{")
            return True
        return any(p.match(linha) for p in ANDAIME)


@dataclass
class Resultado:
    """A auditoria de UMA origem (um trailer, ou um `--declared`)."""

    rotulo: str  # sha curto do commit, ou o range (modo --declared)
    origem: str
    linhas_item: int
    andaime: int
    declarada: bool = False  # veio de `--declared` (range), nao de um trailer
    previsao: bool = False  # e a auditoria do SQUASH PREVISTO do range
    estrutura: str = "commit"  # como o commit foi CRIADO: commit | squash | merge
    faltando: list[str] = field(default_factory=list)
    fora: list[tuple[str, str]] = field(default_factory=list)
    # Conteudo novo que um squash/merge nao deixa ATRIBUIR ao movimento. Conta e
    # nomeia, mas nao reprova (ver o cabecalho).
    avisos: list[tuple[str, str]] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        return not self.faltando and not self.fora


def estrutura_do_commit(raiz: Path, rev: str) -> str:
    """Como o commit foi CRIADO — 'commit', 'squash' ou 'merge'.

    Nao e curiosidade: decide o que a clausula do andaime pode AFIRMAR. Num
    squash de VARIOS commits e num merge o diff do commit e o PR inteiro, e nao
    o movimento que foi declarado — a linha nova pode ser de um commit irmao.
    Squash de UM commit nao tem o separador, e nao precisa: o diff dele E o
    diff do movimento.
    """
    if len(git(raiz, "rev-list", "--parents", "-1", rev).split()) > 2:
        return "merge"
    mensagem = git(raiz, "show", "-s", "--format=%B", rev)
    if any(SEPARADOR_SQUASH.match(l) for l in mensagem.splitlines()):
        return "squash"
    return "commit"


def audita(
    raiz: Path,
    origem: str,
    rev_origem: str,
    rev_destino: str,
    rotulo: str,
    declarada: bool = False,
    previsao: bool = False,
    estrutura: str = "commit",
) -> Resultado:
    """Audita um movimento declarado: a origem sai de `rev_origem`, o texto tem de
    estar no que o diff `rev_origem..rev_destino` tocou.

    `estrutura` (o que `estrutura_do_commit` diz do destino) decide so a clausula
    do andaime: em squash/merge o achado e AVISO, porque o diff e o PR inteiro.
    """
    origem_linhas = relevantes(conteudo(raiz, rev_origem, origem))
    tocados = [c for c in git(raiz, "diff", "--name-only", rev_origem, rev_destino).splitlines() if c]

    # (a) NAO PERDEU — a busca e nos arquivos TOCADOS pelo diff (a origem pode
    # continuar existindo, num corte pela metade: so o que saiu precisa aparecer).
    disponivel: Counter = Counter()
    for caminho in tocados:
        if existe_arquivo(raiz, rev_destino, caminho):
            disponivel.update(relevantes(conteudo(raiz, rev_destino, caminho)))

    faltando: list[str] = []
    for linha in origem_linhas:
        if casa(linha, disponivel) is None:
            faltando.append(linha)

    # (b) NAO REESCREVEU — so nos arquivos do MESMO tipo da origem.
    sufixo = Path(origem).suffix
    restante: Counter = Counter(origem_linhas)
    andaime = Andaime()
    criadas = 0
    # O colapso (squash/merge) junta N commits do PR num diff so: a linha nova
    # pode ser do movimento ou de um IRMAO, e o diff nao diz qual. Ela vira
    # AVISO — dita e contada, sem reprovar. No PR, por commit, foi cobrada
    # inteira; a clausula (a) segue estrita aqui porque o colapso nao a toca.
    colapsado = estrutura != "commit"
    fora: list[tuple[str, str]] = []
    avisos: list[tuple[str, str]] = []
    for caminho in tocados:
        if Path(caminho).suffix != sufixo or not existe_arquivo(raiz, rev_destino, caminho):
            continue
        for linha in adicionadas(raiz, rev_origem, rev_destino, caminho):
            if casa(linha, restante) is not None:
                continue
            if andaime.aceita(linha):
                criadas += 1
                continue
            (avisos if colapsado else fora).append((caminho, linha))

    return Resultado(
        rotulo=rotulo,
        origem=origem,
        linhas_item=len(origem_linhas),
        andaime=criadas,
        declarada=declarada,
        previsao=previsao,
        estrutura=estrutura,
        faltando=faltando,
        fora=fora,
        avisos=avisos,
    )


def declara_movimento(raiz: Path, rev: str) -> list[str]:
    """Os caminhos declarados pelos trailers `Move-only:` daquele commit."""
    origens: list[str] = []
    for linha in git(raiz, "show", "-s", "--format=%B", rev).splitlines():
        m = TRAILER.match(linha)
        if m:
            origens.append(m.group(1))
    return origens


def resolve_range(raiz: Path, args) -> tuple[str, str, bool]:
    """(base, tip, explicito) — explicito = o usuario passou `--from`."""
    if args.de:
        return args.de, args.ate or "HEAD", True
    ate = args.ate or "HEAD"
    pr_base = os.environ.get("PR_BASE", "")
    before = os.environ.get("BEFORE", "")
    if pr_base:
        return pr_base, ate, False
    if before and set(before) != {"0"} and existe(raiz, before):
        return before, ate, False
    return "HEAD~1", ate, False


def commits_do_range(raiz: Path, base: str, tip: str, explicito: bool) -> list[str]:
    """Os hashes do range — MERGES ENTRAM. Base irresolvivel: erro de uso se foi o
    usuario que a pediu; senao (historico curto) audita so o tip.

    O `--no-merges` daqui saiu: ele existe no `check_commits.py` para as MENSAGENS
    (o commit sintetico que o GitHub cria no PR nao e de ninguem), mas na auditoria
    de movimento ele so fazia um movimento declarado num merge nunca ser lido. Um
    merge sem trailer nao audita nada — o filtro de verdade e o trailer.
    """
    if not existe(raiz, base):
        if explicito:
            raise UsoErro(f"base irresolvivel: {base}")
        return [git(raiz, "rev-list", "-1", tip).strip()]
    return [s for s in git(raiz, "rev-list", f"{base}..{tip}").split() if s]


def relata_avisos(r: Resultado) -> None:
    """O AVISO de um squash/merge: nomeia o conteudo nao ATRIBUIVEL, sem reprovar.

    Sai no stdout (e informacao da auditoria, nao erro) e sai SEMPRE — o achado
    nao pode sumir so porque a clausula (a) passou.
    """
    if not r.avisos:
        return
    onde = "num squash previsto" if r.previsao else f"num commit de {r.estrutura}"
    print(
        f"  AVISO ({len(r.avisos)}) - conteudo novo {onde}, que junta os commits do\n"
        "  PR num diff so: nao da para ATRIBUIR ao movimento nem a um commit irmao.\n"
        "  Nao reprova aqui; por commit, no PR, a clausula foi cobrada."
    )
    for caminho, linha in r.avisos[:20]:
        print(f"    - {caminho}: {linha}")
    if len(r.avisos) > 20:
        print(f"    ... (+{len(r.avisos) - 20})")


def relata(resultados: list[Resultado]) -> int:
    if not resultados:
        print("[check_move_only] nenhum commit declara Move-only — nada a auditar")
        return 0
    falhou = False
    for r in resultados:
        # ASCII na saida: o console do Windows nao e UTF-8 e o gate tem de ser
        # legivel no terminal de quem for consertar.
        if r.previsao:
            como = "previsto"
        else:
            como = "declarado" if r.declarada else "Move-only"
        # O rotulo do colapso fica na linha: quem le a CI precisa saber que o diff
        # auditado e o PR inteiro, e nao o commit que declarou o movimento.
        colapso = f" ({r.estrutura})" if r.estrutura != "commit" else ""
        resumo = (
            f"{r.linhas_item} linha(s) de item, {len(r.faltando)} perdida(s), "
            f"{r.andaime} de andaime, {len(r.fora)} fora do andaime"
        )
        if r.avisos:
            resumo += f", {len(r.avisos)} aviso(s)"
        if r.ok:
            print(f"[check_move_only] {r.rotulo} {como}{colapso}: {r.origem} -> {resumo}")
            relata_avisos(r)
            continue
        falhou = True
        print(
            f"[check_move_only] {r.rotulo} {como}{colapso}: {r.origem} -> FALHA ({resumo})",
            file=sys.stderr,
        )
        if r.faltando:
            print(f"  PERDIDAS ({len(r.faltando)}) - a origem tem estas linhas:", file=sys.stderr)
            for linha in r.faltando[:20]:
                print(f"    - {linha}", file=sys.stderr)
            if len(r.faltando) > 20:
                print(f"    ... (+{len(r.faltando) - 20})", file=sys.stderr)
        if r.fora:
            print(
                f"  FORA DO ANDAIME ({len(r.fora)}) - conteudo NOVO num movimento:",
                file=sys.stderr,
            )
            for caminho, linha in r.fora[:20]:
                print(f"    - {caminho}: {linha}", file=sys.stderr)
            if len(r.fora) > 20:
                print(f"    ... (+{len(r.fora) - 20})", file=sys.stderr)
        relata_avisos(r)
    if falhou:
        print(
            "\n[check_move_only] Movimento puro nao perde nem reescreve linha: se houve\n"
            "  mudanca de conteudo, tire o trailer do commit (e diga no PR o que mudou).",
            file=sys.stderr,
        )
        if any(r.previsao for r in resultados):
            print(
                "  O que falhou e a PREVISAO do squash: o `develop` vai reprovar assim\n"
                "  depois do merge, e la o commit que declarou nao existe mais. Reponha\n"
                "  a linha no PR (se a base andou, atualize a branch e o gate reavalia).",
                file=sys.stderr,
            )
        if any(r.estrutura != "commit" and not r.previsao for r in resultados):
            # Conselho que se possa SEGUIR: no `develop` o commit que declarou o
            # movimento nao existe mais, entao "tire o trailer" nao e opcao — a
            # saida e repor a linha (ou consertar no PR, antes do merge).
            print(
                "  O achado esta num commit de SQUASH/MERGE (o diff e o PR inteiro e o\n"
                "  commit que declarou nao existe mais): reponha a linha perdida, ou,\n"
                "  se o PR realmente mudou logica, o trailer nao deveria ter entrado\n"
                "  (tire-o no PR, antes do merge).",
                file=sys.stderr,
            )
        return 1
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--from", dest="de", default="", help="base do range (default: PR_BASE/BEFORE/HEAD~1)")
    ap.add_argument("--to", dest="ate", default="", help="tip do range (default: HEAD)")
    ap.add_argument(
        "--declared",
        action="append",
        default=[],
        metavar="CAMINHO",
        help="audita o range inteiro contra a base dele (retroativo; sem trailer)",
    )
    ap.add_argument("--raiz", default="", help="raiz do repo (default: a do script)")
    args = ap.parse_args()

    raiz = Path(args.raiz).resolve() if args.raiz else RAIZ
    try:
        base, tip, explicito = resolve_range(raiz, args)
        if args.declared:
            rotulo = f"{base}..{tip}"
            # O modo retroativo audita o RANGE: a forma do TIP e que diz se o
            # diff auditado e um movimento ou o PR inteiro (squash/merge).
            estrutura = estrutura_do_commit(raiz, tip)
            resultados = [
                audita(raiz, o, base, tip, rotulo, declarada=True, estrutura=estrutura)
                for o in args.declared
            ]
        else:
            resultados = []
            origens: list[str] = []  # a ordem de declaracao, sem repetir
            chaves: set[tuple[str, str, str]] = set()  # (origem, base, destino) ja auditado
            for sha in commits_do_range(raiz, base, tip, explicito):
                for origem in declara_movimento(raiz, sha):
                    pai = f"{sha}^"
                    if not existe(raiz, pai):
                        raise UsoErro(
                            f"{sha[:7]} declara Move-only e nao tem pai: movimento se le contra o pai"
                        )
                    if origem not in origens:
                        origens.append(origem)
                    # A chave usa sha RESOLVIDO: comparar `HEAD` com o sha de
                    # verdade foi o que fez a previsao nao se reconhecer como
                    # repeticao no push do `develop` (a linha saia duas vezes).
                    chaves.add(
                        (origem, git(raiz, "rev-parse", pai).strip(), sha)
                    )
                    resultados.append(
                        audita(
                            raiz,
                            origem,
                            pai,
                            sha,
                            sha[:7],
                            estrutura=estrutura_do_commit(raiz, sha),
                        )
                    )
            # A PREVISAO DO SQUASH: o range inteiro auditado como o squash que o
            # GitHub vai criar. E o que pega o commit IRMAO que perde uma linha
            # depois do movimento — hoje o PR passa e o `develop` reprova. So
            # entra quando o range NAO e ja o squash (ai seria a mesma
            # auditoria, dita duas vezes — o push do `develop`).
            cheio = {rev: git(raiz, "rev-parse", rev).strip() for rev in (base, tip)}
            curto = {
                rev: git(raiz, "rev-parse", "--short", rev).strip() for rev in (base, tip)
            }
            for origem in origens:
                if (origem, cheio[base], cheio[tip]) in chaves:
                    continue
                resultados.append(
                    audita(
                        raiz,
                        origem,
                        cheio[base],
                        cheio[tip],
                        f"{curto[base]}..{curto[tip]}",
                        previsao=True,
                        estrutura="squash",
                    )
                )
    except UsoErro as e:
        print(f"[check_move_only] {e}", file=sys.stderr)
        return 2

    return relata(resultados)


if __name__ == "__main__":
    sys.exit(main())
