#!/usr/bin/env python3
"""h1_compare.py — a FASE C do gate H1 deixa de ser manual (#21).

POR QUE ISTO EXISTE. O `scripts/h1_field.sh` automatizou as FASES A e B
(rodar o roteiro de leitura). A FASE C — comparar o log de campo com a
referencia do mock, classificar a divergencia nos 3 niveis do
`docs/H1_CHECKLIST.md` §4 e escrever a tabela do §5 — continuava sendo um
lembrete em texto para o humano fazer na mao, DEPOIS de desligar a
pedaleira. E e o pior momento possivel para improvisar: o operador esta com
o device na mao, o roteiro ja rodou, e a unica coisa que falta decidir e se
o H1 passou.

Tres niveis, e eles NAO se confundem:

  nivel 1 FRAMING      a ORDEM de (dir, func, addr) bate com a referencia?
                       Divergir aqui e FALHA DE PROTOCOLO -> fluxo R3.
  nivel 3 ESTRUTURAL   o TAMANHO de cada payload bate com o §13?
                       Divergir aqui tambem e FALHA -> fluxo R3.
  nivel 2 CONTEUDO     framing e estrutura iguais, so os BYTES mudam.
                       ESPERADO por desenho: o device real nao e o `all.prst`.

POR QUE O NIVEL 3 NAO COMPARA CONTRA O MOCK. Se os tres niveis usassem a
referencia do mock como unica fonte, o gate passaria junto com o mock: um
`MockDevice` que passasse a responder pagina de 197B geraria uma referencia
de 197B, e o "divergente" em campo viraria "identico ao mock". O nivel 3 e
por isso conferido contra a tabela `FORMAS_13` abaixo, transcrita do §13 do
`docs/PROTOCOL.md` — a especo viva, nao o codigo que a implementa. O
`--self-check` roda essa conferencia no proprio conjunto de referencia, e e
gate no CI: e o que impede mock e §13 de sairem de sincronia em silencio.

O B1 (`info`) entra como `sem-evidencia`, e nao como "ok". No device real o
`info` NAO emite trafego de fio nenhum (ele so identifica o transporte; o
estado vem das leituras B2/B3), entao o `b1_info.jsonl` de campo e um
arquivo vazio por construcao. Um veredito "verde" sobre um log vazio seria
teto que nunca dispara — que e o mesmo defeito do outro lado.

    python3 scripts/h1_compare.py <dir-da-sessao>      # Fase C de campo
    python3 scripts/h1_compare.py <dir> --markdown      # so a tabela do §5
    python3 scripts/h1_compare.py --self-check          # gate de CI

Saida: 0 = so nivel 2 (o H1 passou, com estado a documentar); 1 = ha
divergencia `bloqueia-H1` (PARAR e fluxo R3); 2 = erro de uso.
"""

from __future__ import annotations

import argparse
import io
import json
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REFERENCIA = ROOT / "analysis" / "h1_reference"

# O console do Windows e cp1252 e o texto deste script tem § e acentos: sem
# isto, `print` morre com UnicodeEncodeError. O `hasattr` e o mesmo de
# `simulate_release.py` — sob o pytest o stdout capturado nao tem
# `reconfigure`, e o import do modulo nao pode quebrar por causa disso.
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

# Saida 2 = erro de uso, 1 = ha `bloqueia-H1`. Mesma convencao de
# `gp100-cli` (main.rs): 2 e uso/politica, 1 e veredito negativo.
EXIT_BLOQUEIA = 1
EXIT_USO = 2
# Saida 3 = DRIFT DE CONTEUDO no rehearsal: os frames casam em forma e
# ordem, mas os bytes nao. Separate da 1 porque a resposta e outra — a 1
# pede o fluxo R3 (mudanca de forma e mudanca de baseline), e a 3 pede
# `refresh-reference --forcar` (a referencia e que esta velha). O runbook
# decide por este codigo, entao ele nao pode ser adivinhado no shell.
EXIT_DRIFT = 3


# ══════════════════════════════════════════════════ nivel 3: as formas do §13
#
# (dir, func, addr) -> tamanhos de payload ACEITOS, em bytes. Transcrito do
# §13.10 (bloco 13xx) e do §13.12 (tabela dos 20 User IRs), e conferido
# contra o `docs/H1_CHECKLIST.md` §4 ("meta6 6B, paginas 196/32B, fim 4B").
#
# `13010003` aceita 196 E 32 de proposito: o §13.10 diz que a pagina 8 vem
# truncada em 32B quando o preset e o ultimo do banco. Um jogo de dados
# unico aqui reprovaria uma sessao legitima.
FORMAS_13: dict[tuple[str, str, str], tuple[int, ...]] = {
    # §13.10 — dump-preset: select -> meta6 -> abre -> avanca pg0..7 -> pg8.
    ("out", "11", "13010000"): (2,),   # select: [pp u16 BE]
    ("in", "12", "13010001"): (6,),    # meta6: [pp] 0c 1c 01 40
    ("out", "12", "13010002"): (3,),    # abre o bloco: [pp u16 BE] 01
    ("out", "12", "13010004"): (5,),    # avanca: [pp u16 BE] [PG u16 BE] 01
    ("in", "12", "13010003"): (196, 32),  # pagina; 32B = ultima do banco
    ("in", "12", "13010005"): (4,),     # fim: so o header de 4B
    # §13.12 — a tabela dos 20 slots de User IR: request de 1B, resposta 75B.
    ("out", "11", "12001002"): (1,),
    ("in", "12", "12001002"): (75,),
}


@dataclass(frozen=True)
class Frame:
    dir: str
    func: str
    addr: str
    data: str

    @property
    def endpoint(self) -> tuple[str, str, str]:
        return (self.dir, self.func, self.addr)

    @property
    def tamanho(self) -> int:
        return len(self.data) // 2


@dataclass
class Passo:
    """Um passo do roteiro e onde achar o log dele em cada tipo de sessao.

    `campo` e o que o `h1_field.sh field` grava; `ensaio` e o que o
    `h1_field.sh rehearsal` grava. Os dois existem porque o runbook usa
    prefixos diferentes — e um juiz que so conhecesse um deles reprovaria a
    sessao por um motivo de nome de arquivo, que nao e divergencia.
    """

    nome: str
    ref: str
    campo: tuple[str, ...]
    ensaio: tuple[str, ...] = ()
    framing: bool = True

    def achar(self, sessao: Path, ensaio: bool) -> Path | None:
        padroes = self.ensaio if ensaio and self.ensaio else self.campo
        for padrao in padroes:
            achado = sorted(sessao.glob(padrao))
            if achado:
                return achado[0]
        return None


# Os passos sao os que a FASE B produz. `framing=False` no B1 porque nao ha
# referencia de framing para `info` — e nao porque o passo seja opcional.
PASSOS: tuple[Passo, ...] = (
    Passo(
        "B1 info",
        "",
        campo=("b1_info.jsonl", "b1_info_r2.jsonl"),
        ensaio=("b1_info.jsonl",),
        framing=False,
    ),
    Passo(
        "B2 list-user-irs",
        "mock_list_irs.jsonl",
        campo=("b2_list.jsonl", "b2_list_r2.jsonl", "mock_list_irs.jsonl"),
        ensaio=("b2_list.jsonl", "mock_list_irs.jsonl"),
    ),
    Passo(
        "B3 dump-preset 0x0000",
        "mock_dump_0x0000.jsonl",
        campo=("b3_dump.jsonl", "b3_dump_r2.jsonl", "mock_dump_0x0000.jsonl"),
        ensaio=("b_dump_0x0000.jsonl", "mock_dump_0x0000.jsonl"),
    ),
    Passo(
        "B4 dump-preset 0x0031",
        "mock_dump_0x0031.jsonl",
        campo=("b4_dump_0x0031.jsonl", "b4_dump_0x0031_r2.jsonl", "mock_dump_0x0031.jsonl"),
        ensaio=("b_dump_0x0031.jsonl", "mock_dump_0x0031.jsonl"),
    ),
    Passo(
        "B4 dump-preset 0x0062",
        "mock_dump_0x0062.jsonl",
        campo=("b4_dump_0x0062.jsonl", "b4_dump_0x0062_r2.jsonl", "mock_dump_0x0062.jsonl"),
        ensaio=("b_dump_0x0062.jsonl", "mock_dump_0x0062.jsonl"),
    ),
)


@dataclass
class Achado:
    """Uma linha da tabela do §5."""

    passo: str
    endpoint: str
    nivel: int
    esperado: str
    obtido: str
    hipotese: str
    severidade: str


@dataclass
class Resultado:
    passo: str
    estado: str  # "ok" | "bloqueia" | "estado" | "sem-evidencia"
    achados: list[Achado] = field(default_factory=list)
    detalhe: str = ""


# ═══════════════════════════════════════════════════════════════ leitura

def ler_log(caminho: Path) -> list[Frame]:
    """Le um log P4 (`{"s","dir","func","addr","data"}`, 1 frame por linha)."""
    frames: list[Frame] = []
    with io.open(caminho, encoding="utf-8") as fh:
        for linha in fh:
            linha = linha.strip()
            if not linha:
                continue
            try:
                bruto = json.loads(linha)
            except json.JSONDecodeError as exc:
                raise ValueError(f"{caminho.name}: linha nao-JSON ({exc})") from exc
            faltando = {"dir", "func", "addr", "data"} - bruto.keys()
            if faltando:
                raise ValueError(
                    f"{caminho.name}: linha sem {sorted(faltando)} (schema P4)"
                )
            frames.append(
                Frame(bruto["dir"], bruto["func"], bruto["addr"], bruto["data"])
            )
    return frames


def _rotulo(endpoint: tuple[str, str, str]) -> str:
    return f"{endpoint[1]}/{endpoint[2]}"


def _encurta(hexa: str) -> str:
    return hexa if len(hexa) <= 16 else hexa[:16] + "..."


# ═══════════════════════════════════════════════════════ nivel 3 (formas)

def checar_formas(frames: list[Frame], passo: str) -> list[Achado]:
    """Confere cada payload contra `FORMAS_13` (o §13, nao o mock).

    Agrega por endpoint: um dump-preset tem 8 paginas no mesmo endereco, e
    8 linhas iguais na tabela do §5 escondem o que importa — que TODAS as 8
    paginas estão fora da forma. Uma linha com a contagem diz mais.
    """
    desconhecidos: dict[tuple[str, str, str], list[int]] = {}
    fora_da_forma: dict[tuple[str, str, str], list[int]] = {}
    for f in frames:
        aceitos = FORMAS_13.get(f.endpoint)
        if aceitos is None:
            desconhecidos.setdefault(f.endpoint, []).append(f.tamanho)
        elif f.tamanho not in aceitos:
            fora_da_forma.setdefault(f.endpoint, []).append(f.tamanho)

    achados: list[Achado] = []
    for endpoint, tamanhos in desconhecidos.items():
        n = len(tamanhos)
        achados.append(
            Achado(
                passo,
                f"{endpoint[0]} {_rotulo(endpoint)}",
                1,
                "endpoint no vocabulario do §13",
                f"{n} frame(s) em {tamanhos[0]}B",
                "endpoint fora do §13 — ou o device fala um endereco novo, "
                "ou o log tem lixo de framing",
                "bloqueia-H1",
            )
        )
    for endpoint, tamanhos in fora_da_forma.items():
        aceitos = FORMAS_13[endpoint]
        n = len(tamanhos)
        distintos = sorted(set(tamanhos))
        achados.append(
            Achado(
                passo,
                f"{endpoint[0]} {_rotulo(endpoint)}",
                3,
                "/".join(str(a) for a in aceitos) + "B",
                f"{n} frame(s) em "
                + ("/".join(str(t) for t in distintos) + "B"),
                f"payload fora da forma do §13 (aceita {'/'.join(str(a) for a in aceitos)}B)",
                "bloqueia-H1",
            )
        )
    return achados


# ══════════════════════════════════════════════════ nivel 1 (framing)

def checar_framing(real: list[Frame], ref: list[Frame], passo: str) -> list[Achado]:
    """Compara a ORDEM de (dir, func, addr). Conteudo fora — é nivel 2."""
    achados: list[Achado] = []
    for i in range(max(len(real), len(ref))):
        a = real[i] if i < len(real) else None
        b = ref[i] if i < len(ref) else None
        ma = f"{a.dir} {_rotulo(a.endpoint)}" if a else "(fim do log real)"
        mb = f"{b.dir} {_rotulo(b.endpoint)}" if b else "(fim do log do mock)"
        if ma != mb:
            achados.append(
                Achado(
                    passo,
                    mb,
                    1,
                    f"frame {i}: {mb}",
                    f"frame {i}: {ma}",
                    "ordem/endpoint do fio divergente do mock",
                    "bloqueia-H1",
                )
            )
            # Uma insercao ou remocao desloca tudo o que vem depois; reportar
            # as N linhas seguintes como se fossem N bugs distintos polui a
            # tabela do §5 e esconde o primeiro achado, que e o que importa.
            break
    return achados


# ══════════════════════════════════════════════════ nivel 2 (conteudo)

def checar_conteudo(
    real: list[Frame], ref: list[Frame], passo: str, ensaio: bool
) -> list[Achado]:
    """Framing e estrutura iguais, bytes diferentes.

    O significado depende de QUEM falou. No `field`, e o device real, e
    nivel 2 e ESPERADO. No `rehearsal`, o outro lado e o proprio
    `MockDevice` — e ai uma diferenca de conteudo nao pode ser "estado do
    device": e o mock ter saido de sincronia com a referencia commitada,
    que e um artefato DERIVADO dele. Sem essa distincao o operador recebe
    um "esperado, documente" sobre bytes que nao tem nada a ver com a
    pedaleira — e a linha vai parar no H1_REPORT como afirmação sobre o
    device. E por isso que o rehearsal e um gate de verdade, e nao um
    aquecimento.
    """
    if len(real) != len(ref):
        return []
    por_endpoint: dict[tuple[str, str, str], list[tuple[str, str]]] = {}
    for a, b in zip(real, ref):
        if a.data == b.data:
            continue
        por_endpoint.setdefault(a.endpoint, []).append((b.data, a.data))
    achados: list[Achado] = []
    for endpoint, pares in por_endpoint.items():
        esperado, obtido = pares[0]
        mesmo_tamanho = all(len(e) == len(o) for e, o in pares)
        sufixo = "" if mesmo_tamanho else " (TAMANHO tambem difere — checar)"
        if ensaio:
            hipotese = (
                "DRIFT: o mock atual diverge da referencia commitada — o §13 "
                "ou a referencia estao errados; regere a referencia ou corrija "
                "o mock. NAO e estado de device."
            )
            severidade = "bloqueia-H1"
        else:
            hipotese = "estado do device != all.prst" + sufixo
            severidade = "estado"
        achados.append(
            Achado(
                passo,
                f"{endpoint[0]} {_rotulo(endpoint)}",
                2,
                f"{len(pares)} frame(s) da referencia: {_encurta(esperado)}",
                f"{len(pares)} frame(s) do device: {_encurta(obtido)}",
                hipotese,
                severidade,
            )
        )
    return achados


# ════════════════════════════════════════════════════════ classificacao

def classificar(passo: Passo, sessao: Path, ensaio: bool) -> Resultado:
    log = passo.achar(sessao, ensaio)
    if log is None:
        return Resultado(passo.nome, "sem-evidencia", detalhe="log nao encontrado")
    frames = ler_log(log)
    if not frames:
        vazio = (
            f"{log.name} sem nenhum frame — `info --real` nao emite trafego de "
            "fio; o estado real vem das leituras B2/B3"
            if not passo.framing
            else (
                f"{log.name} sem nenhum frame — nenhum byte de fio gravado. "
                "Timeout/InvalidShape? repetir 1x (D6) antes de classificar"
            )
        )
        return Resultado(passo.nome, "sem-evidencia", detalhe=vazio)

    achados = checar_formas(frames, passo.nome)
    if passo.framing and passo.ref:
        ref = ler_log(REFERENCIA / passo.ref)
        achados += checar_framing(frames, ref, passo.nome)
        if not achados:
            achados += checar_conteudo(frames, ref, passo.nome, ensaio)

    if any(a.severidade == "bloqueia-H1" for a in achados):
        estado = "bloqueia"
    elif achados:
        estado = "estado"
    else:
        estado = "ok"
    return Resultado(passo.nome, estado, achados, detalhe=log.name)


def tabela(resultados: list[Resultado]) -> str:
    """As linhas da tabela do §5, prontas para colar no H1_REPORT."""
    linhas = [
        "| # | Passo | Endpoint (func/addr) | Nível | Esperado (golden/mock) "
        "| Obtido (hex curto) | Hipótese | Severidade | Ação |",
        "|---|---|---|---|---|---|---|---|---|",
    ]
    n = 0
    for r in resultados:
        for a in r.achados:
            n += 1
            acao = {
                "bloqueia-H1": "fluxo R3 (§6)",
                "estado": "documentar",
                "info": "conferir",
            }[a.severidade]
            linhas.append(
                f"| {n} | {a.passo} | {a.endpoint} | {a.nivel} | {a.esperado} | "
                f"{a.obtido} | {a.hipotese} | {a.severidade} | {acao} |"
            )
    return "\n".join(linhas)


def veredito(resultados: list[Resultado], ensaio: bool = False) -> int:
    """Imprime o veredito e devolve o CÓDIGO DE SAÍDA.

    O código é o contrato com o `h1_field.sh`, e ele separa duas coisas que
    nao podem ser tratadas igual:

      1  nivel 1/3 (forma/framing) — o device nao esta falando o §13. Não se
         copia referência nenhuma: e o fluxo R3.
      3  nivel 2 no ENSAIO — o mock saiu de sincronia com a referencia
         commitada. A forma esta certa; os bytes mudaram. E o caso em que a
         referencia é que está velha, e `--forcar` resolve.
      0  campo com só nivel 2 — estado do device, que é o esperado.
    """
    achados = [a for r in resultados for a in r.achados]
    de_forma = [a for a in achados if a.nivel in (1, 3)]
    de_conteudo = [a for a in achados if a.nivel == 2]
    sem = [r for r in resultados if r.estado == "sem-evidencia"]

    print("=" * 68)
    for r in resultados:
        marca = {
            "ok": "OK",
            "estado": "ESTADO" if not ensaio else "DRIFT",
            "bloqueia": "BLOQUEIA",
            "sem-evidencia": "SEM EVIDENCIA",
        }[r.estado]
        print(f"  [{marca:14}] {r.passo}" + (f" — {r.detalhe}" if r.detalhe else ""))
        for a in r.achados:
            print(f"       n{a.nivel} {a.endpoint}: {a.esperado} != {a.obtido}")
            print(f"            {a.hipotese} [{a.severidade}]")
    print("=" * 68)

    if de_forma:
        print(
            f"H1 BLOQUEADO: {len(de_forma)} divergencia(s) de nivel 1/3 "
            f"(forma ou framing).\n"
            "  PARAR o roteiro (§7 do checklist). Nao improvisar em campo:\n"
            "  preservar a evidencia -> decodificar o log no PC -> se confirmar,\n"
            "  fluxo R3 (§6): build_golden -> validate 100% -> bump de baseline\n"
            "  nos 5 lugares -> retomar o codigo e refazer o H1 do zero."
        )
        return EXIT_BLOQUEIA

    if ensaio and de_conteudo:
        print(
            f"DRIFT: {len(de_conteudo)} divergencia(s) de CONTENDO entre o mock e\n"
            "  a referencia commitada. A forma bate com o §13; os bytes nao.\n"
            "  Isto NAO e estado de device — e a referencia que esta velha (ou o\n"
            "  mock que mudou de semantica). Confira o diff e regere com:\n"
            "      ./scripts/h1_field.sh refresh-reference --forcar"
        )
        return EXIT_DRIFT

    print(
        "H1 com niveis 1 e 3 limpos em todos os passos com evidencia. "
        f"{len(de_conteudo)} divergencia(s) de CONTENDO (nivel 2) e "
        f"{len(sem)} passo(s) sem evidencia — ESPERADO por desenho (o device "
        "real nao e o all.prst)."
    )
    if sem:
        print("  Passos SEM EVIDENCIA nao contam como 'passaram': leia o display.")
    print("  Copie a tabela do §5 para o docs/H1_REPORT.md e arquive a sessao.")
    return 0


# ══════════════════════════════════════════════════════════ self-check

def self_check() -> int:
    """Gate de CI: o conjunto de referencia esta de acordo com o §13?

    Duas invariantes, e as duas IMPORTAM porque nenhuma delas aparece em
    nenhum outro lugar do pipeline:

      1. todo payload da referencia tem a forma que o §13 diz;
      2. os tres `mock_dump_*` tem o MESMO framing, porque o dump-preset
         fala com o device por pp e o framing nao pode depender do pp — so
         o conteudo muda entre eles.
    """
    problemas: list[str] = []

    dumps: list[Path] = sorted(REFERENCIA.glob("mock_dump_*.jsonl"))
    if len(dumps) < 2:
        problemas.append(
            f"esperado >= 2 mock_dump_*.jsonl em {REFERENCIA.name}, achei {len(dumps)}"
        )
    for caminho in sorted(REFERENCIA.glob("*.jsonl")):
        for a in checar_formas(ler_log(caminho), caminho.name):
            problemas.append(
                f"{caminho.name}: nivel 3 — {a.endpoint} esperava {a.esperado}, "
                f"veio {a.obtido}"
            )

    if len(dumps) >= 2:
        base = [(f.dir, f.func, f.addr) for f in ler_log(dumps[0])]
        for outro in dumps[1:]:
            framing = [(f.dir, f.func, f.addr) for f in ler_log(outro)]
            if framing != base:
                problemas.append(
                    f"{dumps[0].name} e {outro.name} tem framing diferente — "
                    "o dump-preset nao pode depender do pp"
                )

    if problemas:
        print("[h1_compare] self-check REPROVADO:")
        for p in problemas:
            print(f"  - {p}")
        return EXIT_BLOQUEIA
    print(
        f"[h1_compare] self-check OK: {len(list(REFERENCIA.glob('*.jsonl')))} "
        f"referencias em conformidade com o §13 "
        f"({len(FORMAS_13)} endpoints, {len(dumps)} dumps com framing igual)"
    )
    return 0


# ════════════════════════════════════════════════════════════════ main

def main() -> int:
    ap = argparse.ArgumentParser(
        description="Fase C do gate H1: classifica a sessao e emite a tabela do §5."
    )
    ap.add_argument(
        "sessao",
        nargs="?",
        type=Path,
        help="diretorio da sessao (analysis/h1_field_<tag> ou h1_rehearsal_*)",
    )
    ap.add_argument(
        "--ensaio",
        action="store_true",
        help="os arquivos sao do rehearsal (prefixo b_), nao do campo (b2_/b3_/b4_)",
    )
    ap.add_argument("--markdown", action="store_true", help="imprime so a tabela do §5")
    ap.add_argument(
        "--self-check",
        action="store_true",
        help="confere o conjunto de referencia contra o §13 (gate de CI)",
    )
    args = ap.parse_args()

    if args.self_check:
        return self_check()
    if args.sessao is None:
        ap.print_usage(sys.stderr)
        print(
            "\nuso: h1_compare.py <dir-da-sessao> [--ensaio] [--markdown]\n"
            "     h1_compare.py --self-check",
            file=sys.stderr,
        )
        return EXIT_USO
    if not args.sessao.is_dir():
        print(f"[!] nao e um diretorio: {args.sessao}", file=sys.stderr)
        return EXIT_USO

    resultados = [classificar(p, args.sessao, args.ensaio) for p in PASSOS]
    if args.markdown:
        print(tabela(resultados))
        print()
    return veredito(resultados, args.ensaio)


if __name__ == "__main__":
    sys.exit(main())