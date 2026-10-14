#!/usr/bin/env python3
"""check_module_size.py — o gate do orçamento de tamanho (front #82 · Rust #248).

POR QUE ISTO EXISTE. O critério da #82 ("App.tsx abaixo de 300 linhas") é um
número. Número sem gate é número que se cumpre no dia do PR e se esquece no mês
seguinte — e o ganho real da #82 era preventivo: cada marco novo adiciona estado
ao App, e o App volta a inchar. Um teto que ninguém cobra é teto que volta a
ser 514.

O mesmo vale do outro lado do IPC, e foi o que a #248 fechou: `actor.rs` chegou
a 2 702 linhas **sem freio automático** (F-07/F-09 da auditoria de 14/10). O
Rust não tinha equivalente ao gate do front — nenhum `.rs` de `packages/*/src`
era medido.

O gate é deliberadamente burro: conta LINHAS FISICAS. Ele não julga qualidade,
não lê AST e não tenta adivinhar intenção. O que ele faz é tornar visível, no
momento do erro, a hora em que um arquivo passou do combinado — e o mapa de onde
cada camada pode (e não pode) crescer está em `docs/dev/ARCHITECTURE.md` §1/§4
(front) e §4.1 (crate do app, ADR-7).

TETO. O número é o teto de LINHAS (inclusive comentários e linhas em branco).
Contar só código daria um número que sobe e desce com o estilo de formatação, e
um gate que oscila sozinho é gate que a gente desliga.

DUAS FAMILIAS DE TETO:

- **genérico** — vale para o que não está declarado: 700 no `components/` do
  front (acima disso, ou falta extrair regra ou falta extrair subcomponente) e
  700 em qualquer `.rs` de `packages/*/src` (acima disso o módulo tem mais de um
  assunto). As coortes têm teto PRÓPRIO, abaixo do genérico, porque ali o
  critério é mais estreito: 300 no actor (#243) e 400 em `session/` (#262).
- **declarado** — o arquivo que SABEMOS passar do genérico e cujo corte está
  escrito como issue (`TETOS` para o front, `TETOS_RUST` para o Rust). A coluna
  do "por que" não é decoração: quando alguém quiser subir um teto, o motivo do
  teto antigo é a primeira coisa que ele precisa ler. Arquivo declarado e
  AUSENTE é erro do gate — senão "apaguei o arquivo" vira a forma mais barata de
  aprovar.

FORA DO ORÇAMENTO. Arquivo de teste (`tests.rs`, ou dentro de um diretório
`tests/`) não é medido: ele cresce com a COBERTURA, não com funcionalidade —
medir isso só ensinaria a escrever menos teste.

EXCEÇÃO. `--allow` recebe `caminho:linhas`. A chave é o caminho relativo à raiz
do repo para `.rs` (`packages/core/src/session.rs`) e relativo a
`packages/app/ui` para o front (`src/components/Foo.tsx`) — as duas formas não
colidem. Ela é para o arquivo que SABEMOS que vai passar do teto e cujo corte
está escrito como issue. Entrada nova aqui é decisão consciente, não
esquecimento — e o gate imprime quantas exceções estão ativas, porque uma lista
que ninguém vê deixa de ser revisão.

USO.
    python3 .github/scripts/check_module_size.py
    python3 .github/scripts/check_module_size.py --allow packages/core/src/golden.rs:900
    python3 .github/scripts/check_module_size.py --allow src/components/Foo.tsx:820
    python3 .github/scripts/check_module_size.py --list
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
UI = ROOT / "packages" / "app" / "ui"

# (caminho relativo a packages/app/ui, teto de linhas, POR QUE este teto).
TETOS: list[tuple[str, int, str]] = [
    ("src/App.tsx", 300, "casca: acima disso, estado de sessao voltou para a casca"),
    ("src/components/LooperPanel.tsx", 700, "apresentacao grande; a FSM ja saiu para looper/fsm.ts"),
    ("src/ipc/device.ts", 600, "a porta cresce por operacao de device, nao por feature de UI"),
]

# Teto generico do front: acima disso, ou falta extrair regra de dominio
# (looper/, tuner/) ou falta extrair subcomponente.
TETO_PADRAO = 700

# (caminho relativo a RAIZ do repo, teto de linhas, POR QUE este teto).
#
# Os `congelado` foram medidos em 14/10/2026 (#248): ja estavam acima do teto
# generico quando o gate passou a olhar o Rust, e o teto deles e EXATAMENTE o
# tamanho da medicao — a regra e "nao cresce mais", nao "pode crescer ate aqui".
# O corte de cada um esta escrito como issue (o numero esta no motivo); subir o
# teto sem cortar e o que este arquivo existe para tornar visivel.
TETOS_RUST: list[tuple[str, int, str]] = [
    ("packages/app/api/src/actor.rs", 600, "#243: handle + spawn; coorte nova vira modulo, nao linha"),
    ("packages/app/api/src/commands.rs", 900, "#243: porta do IPC; o corte por coorte e divida declarada"),
    ("packages/app/api/src/lib.rs", 400, "#243: run()/AppState/registro do generate_handler!"),
    ("packages/core/src/golden.rs", 829, "congelado: golden do fio (R1-R3); corte na #257"),
    ("packages/core/src/preset.rs", 901, "congelado: modelo de preset/decodificacao; corte na #257"),
    (
        "packages/core/src/session.rs",
        200,
        "#257: o HANDLE (o que a Session POSSUI + o ciclo de vida); metodo novo vira modulo do assunto",
    ),
    (
        "packages/core/src/transport/mock.rs",
        350,
        "#257: o HANDLE (o que o MockDevice POSSUI + o ciclo de vida) + o `mod tests` inline "
        "(um `mod tests` em `mock/tests.rs` e `clippy::module_inception`); metodo novo vira modulo do assunto",
    ),
    ("packages/cli/src/main.rs", 1226, "congelado: CLI de campo; corte na #257"),
]

# Teto generico do Rust (qualquer `.rs` de `packages/*/src` nao declarado).
TETO_RUST_PADRAO = 700

# Coorte do actor (#243 / ARCHITECTURE §4.1): `Request` -> `Session` de UM
# assunto. O teto e menor que o generico de proposito — coorte que passa de 300
# linhas quase sempre sao duas coortes no mesmo arquivo.
TETO_COORTE_ACTOR = 300
DIR_COORTE_ACTOR = "packages/app/api/src/actor/"

# Coorte do `session` (#262 / ARCHITECTURE §4.1): os modulos que o corte da #257
# tirou do `session.rs` sao UM assunto cada (a transacao, o boot, a escrita, a
# leitura...), e o teto e menor que o generico pelo mesmo motivo do actor.
#
# O numero sai da MEDIDA: o maior assunto legitimo hoje e a `escrita` (347 = as
# cinco operacoes de escrita do §13.7/§13.11/§13.12/§5); a coorte do actor e 300
# sobre o maior modulo dela (244, +23%), entao 400 sobre 347 e a mesma folga
# (+15%). Acima disso o modulo virou dois — o primeiro candidato e separar
# IR/SnapTone da `escrita`.
TETO_COORTE_SESSAO = 400
DIR_COORTE_SESSAO = "packages/core/src/session/"

# Arquivo de teste nao entra no orcamento (cresce com a cobertura, nao com
# funcionalidade) — por nome e por diretorio.
NOMES_DE_TESTE = {"tests.rs", "test.rs"}


def conta_linhas(caminho: Path) -> int:
    """Linhas físicas do arquivo. Tolera o `\\r` do checkout do Windows."""
    with caminho.open("r", encoding="utf-8", errors="replace") as fh:
        return sum(1 for _ in fh)


def e_teste(rel: str) -> bool:
    """Arquivo de teste (fora do orçamento): `tests.rs` ou dentro de `tests/`."""
    partes = Path(rel).parts
    return partes[-1] in NOMES_DE_TESTE or "tests" in partes[:-1]


def teto_rust(rel: str, declarados: dict[str, int]) -> int:
    """Teto de um `.rs`: declarado > coorte do actor > coorte do session > genérico.

    O declarado vem primeiro porque é decisão registrada para UM arquivo: é ele
    que sustenta o handle da FSM (`session.rs` = 200) — sem essa precedência, a
    coorte do diretório engoliria o teto mais estreito.
    """
    if rel in declarados:
        return declarados[rel]
    if rel.startswith(DIR_COORTE_ACTOR):
        return TETO_COORTE_ACTOR
    if rel.startswith(DIR_COORTE_SESSAO):
        return TETO_COORTE_SESSAO
    return TETO_RUST_PADRAO


def avalia_rust(
    raiz: Path,
    tetos: list[tuple[str, int, str]] | None = None,
    excecoes: dict[str, int] | None = None,
) -> tuple[list[tuple[str, int, int]], list[str]]:
    """Mede os `.rs` de `packages/*/src` e devolve (excessos, ausentes).

    Separado do `main` porque o teste do gate (`tools/tests/test_check_module_size.py`)
    precisa medir uma árvore própria: inflar arquivo de verdade para provar que o
    gate reprova é o tipo de teste que ninguém roda duas vezes.
    """
    declarados = {rel: teto for rel, teto, _ in (tetos if tetos is not None else TETOS_RUST)}
    tetos_usados = dict(declarados)
    tetos_usados.update(excecoes or {})
    excessos: list[tuple[str, int, int]] = []
    ausentes: list[str] = []

    raizes = sorted({*raiz.glob("packages/*/src"), *raiz.glob("packages/*/*/src")})
    for base in raizes:
        for caminho in sorted(base.rglob("*.rs")):
            rel = caminho.relative_to(raiz).as_posix()
            if e_teste(rel):
                continue
            teto = teto_rust(rel, tetos_usados)
            linhas = conta_linhas(caminho)
            if linhas > teto:
                excessos.append((rel, linhas, teto))

    # Declarado e ausente: o teto sumiu junto com o arquivo.
    for rel in declarados:
        if not (raiz / rel).is_file():
            ausentes.append(rel)

    return excessos, ausentes


def _parse_allow(itens: list[str]) -> dict[str, int] | None:
    excecoes: dict[str, int] = {}
    for item in itens:
        if ":" not in item:
            print(f"[check_module_size] --allow invalido (use caminho:linhas): {item}", file=sys.stderr)
            return None
        rel, _, n = item.rpartition(":")
        if not n.isdigit():
            print(f"[check_module_size] --allow invalido (linhas nao numericas): {item}", file=sys.stderr)
            return None
        excecoes[rel] = int(n)
    return excecoes


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
        print("front (relativo a packages/app/ui)")
        for rel, teto, motivo in TETOS:
            print(f"  {rel:<32} {teto:>5}  {motivo}")
        print(f"  {'(restante)':<32} {TETO_PADRAO:>5}  regra padrao de components/")
        print("Rust (relativo a raiz do repo)")
        for rel, teto, motivo in TETOS_RUST:
            print(f"  {rel:<44} {teto:>5}  {motivo}")
        print(f"  {DIR_COORTE_ACTOR + '<coorte>.rs':<44} {TETO_COORTE_ACTOR:>5}  coorte do actor (#243, ARCHITECTURE 4.1)")
        print(f"  {DIR_COORTE_SESSAO + '<modulo>.rs':<44} {TETO_COORTE_SESSAO:>5}  coorte do session (#262, ARCHITECTURE 4.1)")
        print(f"  {'(demais .rs de packages/*/src)':<44} {TETO_RUST_PADRAO:>5}  regra padrao do Rust")
        print(f"  {'(testes: tests.rs, tests/)':<44} {'-':>5}  fora do orcamento: cresce com a cobertura")
        return 0

    excecoes = _parse_allow(args.allow)
    if excecoes is None:
        return 2

    faltando = [raiz for raiz in (UI, ROOT) if not raiz.is_dir()]
    if faltando:
        for raiz in faltando:
            print(f"[check_module_size] raiz nao encontrada: {raiz}", file=sys.stderr)
        return 2

    excessos: list[tuple[str, int, int]] = []
    ausentes: list[str] = []

    # ── front ────────────────────────────────────────────────────────────
    def checa(rel: str, teto: int, obrigatorio: bool) -> None:
        caminho = UI / rel
        if not caminho.is_file():
            if obrigatorio:
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

    # ── Rust (#248) ──────────────────────────────────────────────────────
    rust_excesso, rust_ausente = avalia_rust(ROOT, excecoes=excecoes)
    excessos += rust_excesso
    ausentes += rust_ausente

    if ausentes:
        print("[check_module_size] declarado no orcamento e AUSENTE:", file=sys.stderr)
        for rel in ausentes:
            print(f"  {rel} — o teto dele sumiu junto com o arquivo", file=sys.stderr)
        print(
            "[check_module_size] Ou o arquivo foi renomeado (atualize o gate), ou "
            "ele saiu do orcamento de proposito (atualize docs/dev/ARCHITECTURE.md 4/4.1).",
            file=sys.stderr,
        )
        return 1

    if excessos:
        print(
            "[check_module_size] Acima do teto (docs/dev/ARCHITECTURE.md §4/§4.1):",
            file=sys.stderr,
        )
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
    print("[check_module_size] front e Rust dentro do orcamento", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
