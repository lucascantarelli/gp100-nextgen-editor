#!/usr/bin/env python3
"""h2_compare.py — a FASE C do gate H2 deixa de ser manual (#22).

POR QUE ISTO EXISTE (E POR QUE ELE NAO E O h1_compare DE NOVO). O
`scripts/h1_compare.py` e um COMPARADOR: ele confronta o log de campo com a
referencia do mock e separa tres niveis de divergencia. O H2 nao tem o que
comparar. Nao existe referencia de mock para "o preset que o dono salvou" —
o estado do aparelho e do dono, e nao de um conjunto de fixtures. O que o
§13 afirma sobre escrita nao e um valor, e um INVARIANTE: quantos frames
saem, em que ordem, com que tamanho, e se cada chunk recebe o ACK dele.

Por isso o juiz do H2 e um CONFERIDOR DE INVARIANTES, e a diferenca muda o
que ele pode reprovar:

  - o H1 reprova quando o fio diverge do mock;
  - o H2 reprova quando o fio diverge do §13.

E o que sobra para o humano e o que o §13 NAO consegue provar: se o knob
girou, se o nome persistiu, se o slot ficou com o IR. Isso e o DISPLAY
(ADR-6/D4: o `set-param` e fire-and-forget e nao tem read-back no fio), e e
por isso que o juiz imprime o valor que o display DEVERIA mostrar — para a
verificacao ser uma COMPARACAO e nao um "pareceu que".

OS TRES INVARIANTES (transcritos do §13, nao lidos do codigo que os
implementa — o mesmo cuidado do `FORMAS_13` do H1):

  F1 set-param (§13.11, D4)  1 frame OUT, ZERO IN. O endereco carrega o
                              slot da CADEIA em `10SS0002`, e `SS` vai de
                              01 a 09 — nao e o slot de IR (0..=19).
  F2 save (§13.12, D3)        9 frames OUT, ZERO IN: 5 do meta
                              (`11000000` `11000004` `11000005` `11000007`
                              `12000002`) e o ciclo de ops da S4 em
                              `00020000` com op 0,0,1,1 nesta ordem.
  F3 upload-ir (§13.7)        1 BEGIN em `10050001` e depois N chunks em
                              `12001002`, cada um com o ACK em `12001002`.
                              O FIM do upload e a DUPLICACAO do ultimo
                              chunk (idx `0x226` na captura) — nao ha
                              commit no fio.

O `ZERO IN` do F1 e do F2 nao e detalhe: e o D3/D4. Um IN inesperado ali
nao e confirmacao, e o device empurrando algo por conta propria — e o log
que separa as duas coisas.

    python3 scripts/h2_compare.py <dir-da-sessao>       # Fase C de campo
    python3 scripts/h2_compare.py <dir> --markdown       # so a tabela do §6
    python3 scripts/h2_compare.py --self-check           # gate de CI

Saida: 0 = os 3 fluxos obedeceram o §13 (o veredito do display ainda e do
operador); 1 = ha divergencia `bloqueia-H2` (PARAR, e nao repetir o fluxo);
2 = erro de uso.
"""

from __future__ import annotations

import argparse
import io
import json
import re
import struct
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# A referencia dos 3 fluxos, gerada pelo `h2_field.sh rehearsal` contra o
# `MockDevice` e versionada no repo. E o que ancora a `FORMAS_13` (ver
# `self_check`).
REFERENCIA = ROOT / "analysis" / "h2_reference"

# O console do Windows e cp1252 e o texto deste script tem §: sem isto,
# `print` morre com UnicodeEncodeError. O `hasattr` e o mesmo de
# `h1_compare.py` — sob o pytest o stdout capturado nao tem `reconfigure`.
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

# Saida 2 = erro de uso, 1 = ha `bloqueia-H2`. Mesma convencao de
# `h1_compare.py` e do `gp100-cli`.
EXIT_BLOQUEIA = 1
EXIT_USO = 2

# O endereco do set-param carrega o slot da cadeia: `10SS0002` com SS de
# `01` a `09`. E um endereco COM VARIAVEL, e nao um endereco fixo — por isso
# ele nao cabe inteiro na `FORMAS_13` e tem regra propria (abaixo).
RE_SET_PARAM = re.compile(r"^10(0[1-9])0002$")


# ══════════════════════════════════════ nivel 3: as formas do §13 (escrita)
#
# (dir, func, addr) -> tamanhos de payload ACEITOS, em bytes. Transcrito do
# §13.11 (knob), do §13.12 (save: os 5 do meta e o ciclo de ops) e do §13.7
# (upload: BEGIN, chunk de 33B e ACK de 75B).
#
# O tamanho do payload vem da contagem de BYTES REAIS do envelope, e nao do
# sysEx inteiro: o `--log` do CLI grava `data` sem o header e sem o F7, e e
# contra essa coluna que o operador vai conferir com o §13 na mao.
FORMAS_13: dict[tuple[str, str, str], tuple[int, ...]] = {
    # §13.11 — set-param: 10B reais (code u32 LE + ctrl + 0x00 + f32 LE)
    # expandidos em nibbles = 20B. O endereco tem o slot e fica de fora.
    ("out", "12", "SET_PARAM"): (20,),
    # §13.12 — save: 5 writes do meta + o ciclo de ops.
    ("out", "12", "11000000"): (20,),  # zeros4 + pp u16 + zeros2 + nome 12B
    ("out", "12", "11000004"): (20,),  # reservado
    ("out", "12", "11000005"): (4,),   # ppType u32 BE + zeros2
    ("out", "12", "11000007"): (50,),  # reservado
    ("out", "12", "12000002"): (8,),   # fecha o bloco: zeros4 + pp u16 + zeros2
    ("out", "12", "00020000"): (8,),   # op: zeros4 + op u16 BE + zeros2
    # §13.7 — upload de IR: BEGIN cru, chunk de 33B e o ACK de 4B.
    #
    # O ACK e de 4B (`[slot][idx u16 BE][01]`, o `ir_chunk_ack_payload` do
    # core) e NAO os 75B que a tabela do H1 registra no MESMO endereco: la
    # quem responde com 75B e a LISTA dos 20 slots (`12001002` de 1B, §13.12),
    # que e outra mensagem. Duas respostas diferentes no mesmo endereco e a
    # razao de o self-check conferir as formas contra uma referencia gerada
    # pelo mock, e nao contra a memoria de quem escreve o juiz.
    ("out", "12", "10050001"): (8,),
    ("out", "12", "12001002"): (33,),
    ("in", "12", "12001002"): (4,),
}

# A ordem dos 5 writes do meta (§13.12) e o ciclo de ops da S4 (D3):
# op 0 com o meta, op 0 de novo, op 1 duas vezes. A ordem E o invariante —
# um save com os mesmos 9 frames fora de ordem gravou outra coisa.
ORDEM_SAVE: tuple[str, ...] = (
    "11000000",
    "11000004",
    "11000005",
    "11000007",
    "12000002",
    "00020000",
    "00020000",
    "00020000",
    "00020000",
)
# Os 4 valores de op, na ordem em que saem: 0, 0, 1, 1 (BE, ADR-1).
OPS_SAVE: tuple[int, ...] = (0, 0, 1, 1)


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

    @property
    def bytes(self) -> bytes:
        return bytes.fromhex(self.data)


def desnibelar(payload: bytes) -> bytes:
    """Volta dos bytes de nibble para os bytes reais.

    O `nibble_expand` do core (ADR-1) transforma CADA byte real em DOIS
    bytes, um por nibble: o real `0x6e` vira `06 0e`. O inverso e
    `real = (payload[2i] << 4) | payload[2i+1]`.

    Isto importa porque o set-param e o unico dos 3 fluxos com payload
    expandido — e o unico em que o valor a conferir no display mora dentro
    dele. Ler os ultimos 4 bytes crus daria o nibble, nao o f32.
    """
    return bytes(
        (payload[2 * i] << 4) | payload[2 * i + 1] for i in range(len(payload) // 2)
    )


@dataclass
class Achado:
    """Uma linha da tabela do §6 do H2_CHECKLIST."""

    fluxo: str
    endpoint: str
    invariant: str
    esperado: str
    obtido: str
    hipotese: str
    severidade: str


@dataclass
class Resultado:
    fluxo: str
    estado: str  # "ok" | "bloqueia" | "sem-evidencia"
    achados: list[Achado] = field(default_factory=list)
    # O que o DISPLAY deveria mostrar depois deste fluxo (nivel 2: a
    # verificacao do operador, nao a do fio). Vazio = o §13 nao fixa um
    # valor, e o checklist manda ver no display sem referencia previa.
    display: str = ""
    # Porque o fluxo nao tem veredito: "log nao encontrado", "log vazio" ou
    # o texto do erro de leitura. Sem isto, `sem-evidencia` e `bloqueia` por
    # log corrompido apareceriam igual na saida, e o operador nao saberia
    # se faltou o passo ou se o arquivo estragou.
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


def _formas_de(f: Frame) -> tuple[int, ...] | None:
    """A forma aceita por um frame, resolvendo o endereco variavel do F1."""
    if RE_SET_PARAM.match(f.addr):
        return FORMAS_13[("out", "12", "SET_PARAM")]
    return FORMAS_13.get(f.endpoint)


def _no_vocabulario(f: Frame) -> bool:
    return _formas_de(f) is not None


# ═══════════════════════════════ nivel 3: a forma de cada payload (§13)

def checar_formas(frames: list[Frame], fluxo: str) -> list[Achado]:
    """Confere cada payload contra `FORMAS_13` (o §13, nao o codigo).

    Agrega por endpoint: um upload de IR tem centenas de chunks no mesmo
    endereco, e centenas de linhas iguais na tabela escondem o que importa —
    que TODOS os payloads estão fora da forma. Uma linha com a contagem diz
    mais do que 296 linhas iguais.
    """
    desconhecidos: dict[tuple[str, str, str], list[int]] = {}
    fora_da_forma: dict[tuple[str, str, str], list[int]] = {}
    for f in frames:
        aceitos = _formas_de(f)
        chave = f.endpoint
        if aceitos is None:
            desconhecidos.setdefault(chave, []).append(f.tamanho)
        elif f.tamanho not in aceitos:
            fora_da_forma.setdefault(chave, []).append(f.tamanho)

    achados: list[Achado] = []
    for endpoint, tamanhos in desconhecidos.items():
        achados.append(
            Achado(
                fluxo,
                f"{endpoint[0]} {_rotulo(endpoint)}",
                "vocabulario",
                "endpoint no §13 (escrita)",
                f"{len(tamanhos)} frame(s) em {tamanhos[0]}B",
                "endpoint fora do §13 — ou o device fala um endereco novo, ou o "
                "log tem lixo de framing",
                "bloqueia-H2",
            )
        )
    for endpoint, tamanhos in fora_da_forma.items():
        aceitos = _formas_de(Frame(endpoint[0], endpoint[1], endpoint[2], ""))
        n = len(tamanhos)
        distintos = sorted(set(tamanhos))
        achados.append(
            Achado(
                fluxo,
                f"{endpoint[0]} {_rotulo(endpoint)}",
                "forma do payload",
                "/".join(str(a) for a in aceitos) + "B",
                f"{n} frame(s) em " + "/".join(str(t) for t in distintos) + "B",
                f"payload fora da forma do §13 (aceita "
                f"{'/'.join(str(a) for a in aceitos)}B)",
                "bloqueia-H2",
            )
        )
    return achados


# ═══════════════════════════════ nivel 1: os invariantes de cada fluxo

def checar_f1(frames: list[Frame]) -> list[Achado]:
    """set-param: 1 frame OUT e ZERO IN (§13.11, D4).

    O read-back nao existe no fio: o `set_param` e fire-and-forget e a captura
    nao tem resposta. Um IN aqui nao e confirmacao — e o device empurrando
    algo por conta propria, e precisa de registro, mas NAO pode ser lido
    como "o knob aceitou".
    """
    achados: list[Achado] = []
    outs = [f for f in frames if f.dir == "out"]
    ins = [f for f in frames if f.dir == "in"]
    if len(outs) != 1:
        achados.append(
            Achado(
                "F1 set-param",
                "out",
                "contagem de writes",
                "1 frame OUT",
                f"{len(outs)} frame(s) OUT",
                "um knob e UM frame — mais de um significa dois efeitos "
                "escritos sem o operador pedir (§13.11)",
                "bloqueia-H2",
            )
        )
    for f in ins:
        achados.append(
            Achado(
                "F1 set-param",
                f"in {_rotulo(f.endpoint)}",
                "fire-and-forget",
                "ZERO IN (D4)",
                "1 frame IN",
                "o set-param nao tem resposta no fio (D4) — um IN aqui nao e "
                "confirmacao, e o device reagindo por conta propria",
                "documenta",
            )
        )
    return achados


def checar_f2(frames: list[Frame]) -> list[Achado]:
    """save: 9 OUT na ordem do §13.12 e ZERO IN (D3)."""
    achados: list[Achado] = []
    outs = [f for f in frames if f.dir == "out"]
    ins = [f for f in frames if f.dir == "in"]

    if len(outs) != len(ORDEM_SAVE):
        achados.append(
            Achado(
                "F2 save",
                "out",
                "contagem de writes",
                f"{len(ORDEM_SAVE)} frames OUT (5 meta + 4 ops)",
                f"{len(outs)} frame(s) OUT",
                "o save e a escrita que PERSISTE: faltou frame, o preset nao "
                "gravou tudo; sobrou, gravou alem do §13",
                "bloqueia-H2",
            )
        )
    else:
        for i, (f, addr_esperado) in enumerate(zip(outs, ORDEM_SAVE)):
            if f.addr != addr_esperado:
                achados.append(
                    Achado(
                        "F2 save",
                        f"out {_rotulo(f.endpoint)}",
                        "ordem dos writes",
                        f"frame {i}: 12/{addr_esperado}",
                        f"frame {i}: 12/{f.addr}",
                        "a ordem do §12 e o invariante: 5 do meta e o ciclo "
                        "op0,op0,op1,op1 da S4 (D3)",
                        "bloqueia-H2",
                    )
                )
                # Uma troca desloca tudo o que vem depois; reportar as N
                # linhas seguintes como N bugs distintos polui a tabela e
                # esconde o primeiro achado, que e o que importa.
                break
        for i, (f, op) in enumerate(zip(outs[5:], OPS_SAVE)):
            payload = f.bytes
            # `op` vive em `[4..5]` BE (ADR-1: campo de enderecamento e BE).
            obtido = int.from_bytes(payload[4:6], "big") if len(payload) >= 6 else -1
            if obtido != op:
                achados.append(
                    Achado(
                        "F2 save",
                        "out 12/00020000",
                        "valor da op",
                        f"op {i} = {op}",
                        f"op {i} = {obtido}",
                        "o ciclo de ops da S4 e 0,0,1,1; op errada grava um "
                        "preset diferente do que o operador pediu",
                        "bloqueia-H2",
                    )
                )
                break
    for f in ins:
        achados.append(
            Achado(
                "F2 save",
                f"in {_rotulo(f.endpoint)}",
                "fire-and-forget",
                "ZERO IN (D3)",
                "1 frame IN",
                "o save nao tem resposta (D3) — burst tardio NAO e confirmacao, "
                "e oquirk de fim de sessao que a S2 ja registrou",
                "documenta",
            )
        )
    return achados


def checar_f3(frames: list[Frame]) -> list[Achado]:
    """upload-ir: 1 BEGIN, N chunks, N ACKs e a duplicacao do ultimo (§13.7).

    O ACK por chunk e o que impede "um aparelho meio gravado por minutos" sem
    ninguem avisar: se um chunk entrou sem ACK, o device nao recebeu aquele
    pedaco e o IR fica com um buraco no meio.
    """
    achados: list[Achado] = []
    begins = [f for f in frames if f.dir == "out" and f.addr == "10050001"]
    outs = [f for f in frames if f.dir == "out" and f.addr == "12001002"]
    ins = [f for f in frames if f.dir == "in" and f.addr == "12001002"]

    if len(begins) != 1:
        achados.append(
            Achado(
                "F3 upload-ir",
                "out 12/10050001",
                "contagem de BEGIN",
                "1 frame OUT",
                f"{len(begins)} frame(s) OUT",
                "o BEGIN abre a reserva do slot (§13.7); sem ele o device nao "
                "sabe para qual slot os chunks sao",
                "bloqueia-H2",
            )
        )
    if len(outs) == 0:
        achados.append(
            Achado(
                "F3 upload-ir",
                "out 12/12001002",
                "contagem de chunks",
                ">=1 chunk OUT",
                "0 chunks OUT",
                "upload sem chunk nao grava nada — o slot fica como estava",
                "bloqueia-H2",
            )
        )
    if len(ins) != len(outs):
        achados.append(
            Achado(
                "F3 upload-ir",
                "in 12/12001002",
                "ACK por chunk",
                f"{len(outs)} ACK(s) para {len(outs)} chunk(s)",
                f"{len(ins)} ACK(s)",
                "chunk sem ACK nao chegou inteiro: o IR fica com um buraco no "
                "meio e o aparelho nao avisa (§13.7)",
                "bloqueia-H2",
            )
        )
    # O FIM do upload e a DUPLICACAO do ultimo chunk, nao um commit: o device
    # fecha no ultimo ACK quando ve o idx repetido. Sem a duplicacao, o
    # arquivo pode ter ficado sem a ultima pagina.
    if len(outs) >= 2:
        ultimo = outs[-1].bytes
        anterior = outs[-2].bytes
        if ultimo[1:3] != anterior[1:3]:
            achados.append(
                Achado(
                    "F3 upload-ir",
                    "out 12/12001002",
                    "fecho por duplicacao",
                    f"ultimo chunk repete o idx {anterior[1:3].hex()}",
                    f"ultimo chunk tem idx {ultimo[1:3].hex()}",
                    "o §13.7 fecha o upload na DUPLICACAO do ultimo chunk "
                    "(idx 0x226 na captura) — sem ela o device pode ter "
                    "descartado a ultima pagina",
                    "bloqueia-H2",
                )
            )
    return achados


# ═══════════════════════════ nivel 2: o que o DISPLAY deveria mostrar

def _nome_do_display(payload: bytes) -> str:
    """Nome no meta do save: `[8..20]`, preenchido com espaco (§13.12)."""
    bruto = payload[8:20].decode("latin-1", errors="replace")
    return bruto.rstrip(" \x00")


def _slot_do_begin(payload: bytes) -> int:
    """O BEGIN e cru: `00 [slot] 00 00 01 00 00 0a` (§13.7)."""
    return payload[1] if len(payload) >= 2 else -1


def display_esperado(fluxo: str, frames: list[Frame]) -> str:
    """O que o operador DEVE ver no display depois deste fluxo.

    Sai de dentro do proprio log de fio, e nao de um arquivo de entrada: o
    valor que o operador digitou e o que esta no payload, e comparar os dois
    e o que separa "grava o que eu pedi" de "grava alguma coisa".
    """
    outs = [f for f in frames if f.dir == "out"]
    if fluxo == "F1 set-param":
        if not outs:
            return ""
        real = desnibelar(outs[0].bytes)
        if len(real) < 10:
            return ""
        # Os 10B reais sao `code u32 LE | ctrl | 0x00 | value f32 LE`, e o
        # valor esta nos ultimos 4 (ADR-1: o valor e LE como os outros
        # campos de parametro).
        valor = struct.unpack("<f", real[6:10])[0]
        return f"o valor do knob deve mostrar {valor}"
    if fluxo == "F2 save":
        metas = [f for f in outs if f.addr == "11000000"]
        if not metas:
            return ""
        nome = _nome_do_display(metas[0].bytes)
        return f"o preset deve mostrar o nome '{nome}' (e manter depois de sair e voltar)"
    if fluxo == "F3 upload-ir":
        begins = [f for f in outs if f.addr == "10050001"]
        if not begins:
            return ""
        slot = _slot_do_begin(begins[0].bytes)
        return f"o slot {slot} deve mostrar o nome do IR, e `list-user-irs` nao pode devolve-lo vazio"
    return ""


# ═══════════════════════════════════════════════════ os tres fluxos do §3

@dataclass(frozen=True)
class Fluxo:
    nome: str
    rotulo: str
    padroes: tuple[str, ...]
    invariante: object  # callable(list[Frame]) -> list[Achado]


FLUXOS: tuple[Fluxo, ...] = (
    Fluxo(
        "F1 set-param",
        "set-param (knob, §13.11)",
        ("f1_setparam.jsonl", "f1_setparam_r2.jsonl"),
        checar_f1,
    ),
    Fluxo(
        "F2 save",
        "save (§13.12)",
        ("f2_save.jsonl", "f2_save_r2.jsonl"),
        checar_f2,
    ),
    Fluxo(
        "F3 upload-ir",
        "upload-ir (§13.7)",
        ("f3_upload.jsonl", "f3_upload_r2.jsonl"),
        checar_f3,
    ),
)


def achar_log(sessao: Path, fluxo: Fluxo) -> Path | None:
    for padrao in fluxo.padroes:
        achado = sorted(sessao.glob(padrao))
        if achado:
            return achado[0]
    return None


def conferir_fluxo(fluxo: Fluxo, caminho: Path | None) -> Resultado:
    """Aplica os TRES niveis a um fluxo. A ordem importa (ver `conferir`).

    O nivel 3 vem primeiro: se um payload esta fora da forma, a leitura dos
    bytes dele para extrair o nome ou a op e leitura de lixo, e o achado de
    forma e o que o operador precisa ver primeiro.
    """
    if caminho is None:
        return Resultado(fluxo.nome, "sem-evidencia", detalhe="log nao encontrado")
    try:
        frames = ler_log(caminho)
    except ValueError as exc:
        return Resultado(
            fluxo.nome,
            "bloqueia",
            [
                Achado(
                    fluxo.nome,
                    "-",
                    "leitura do log",
                    "log P4 valido",
                    str(exc),
                    "log corrompido: sem ele nao ha prova de nada",
                    "bloqueia-H2",
                )
            ],
            detalhe="log corrompido",
        )
    if not frames:
        # Log vazio NAO e "ok": e a mesma coisa que um teto que nunca
        # dispara. O operador precisa saber que nao gravou evidencia.
        return Resultado(fluxo.nome, "sem-evidencia", detalhe="log vazio")

    achados = checar_formas(frames, fluxo.nome)
    if achados:
        return Resultado(fluxo.nome, "bloqueia", achados)
    achados = fluxo.invariante(frames)  # type: ignore[operator]
    estado = "bloqueia" if any(a.severidade == "bloqueia-H2" for a in achados) else "ok"
    return Resultado(
        fluxo.nome,
        estado,
        achados,
        display=display_esperado(fluxo.nome, frames),
    )


def conferir(sessao: Path) -> list[Resultado]:
    return [conferir_fluxo(f, achar_log(sessao, f)) for f in FLUXOS]


# ═══════════════════════════════════════════════════════════════ relatorio

def tabela(resultados: list[Resultado]) -> str:
    """A tabela do §6 do H2_CHECKLIST, em Markdown."""
    linhas = [
        "| # | Fluxo (§13) | Invariante do fio | Obtido | Veredito do operador |",
        "|---|---|---|---|---|",
    ]
    for r in resultados:
        coluna_fluxo = rotulo_de(r.fluxo)
        if r.estado == "sem-evidencia":
            invariante, obtido = r.detalhe, "sem log"
        elif r.estado == "bloqueia":
            primeiros = r.achados[0]
            invariante = f"{primeiros.invariant}: {primeiros.esperado}"
            obtido = primeiros.obtido
        else:
            invariante, obtido = "conforme", "conforme"
        linhas.append(
            f"| {r.fluxo} | {coluna_fluxo} | {invariante} | {obtido} | ______ |"
        )
    return "\n".join(linhas)


def rotulo_de(fluxo_nome: str) -> str:
    """O nome curto do fluxo, como o H2_CHECKLIST §3-5 o chama."""
    for f in FLUXOS:
        if f.nome == fluxo_nome:
            return f.rotulo
    return fluxo_nome


def self_check() -> int:
    """Confere a `FORMAS_13` contra a referencia do mock — gate de CI.

    A `FORMAS_13` e uma TRANSCRICAO do §13, e transcricao sem conferencia e
    comentario. Este gate existe porque a transcricao ja errou uma vez: o
    ACK do chunk de IR foi escrito com 75B, que e o tamanho da RESPOSTA DA
    LISTA dos 20 slots (`12001002` de 1B, §13.12) — a mesma mensagem, outra
   outra resposta So conferindo contra o que o `MockDevice` realmente emits e que o
    erro aparece antes de o operador estar com a pedaleira na mao.

    As duas direcoes importam: um endpoint da referencia fora da tabela
    reprova (a tabela envelheceu), e um endpoint da tabela que a referencia
    nao produz reprova tambem (sobra que dariia "ok" em campo para algo que
    o device nunca disse).
    """
    falhas: list[str] = []

    # 1. O vocabulario tem que cobrir os endpoints de escrita do §13 e nenhum
    #    endereco a mais (um endereco sobrando aqui e um falso "ok" em campo).
    for chave in FORMAS_13:
        if chave == ("out", "12", "SET_PARAM"):
            continue
        if chave[0] not in ("in", "out") or chave[1] != "12":
            falhas.append(f"FORMAS_13 com endpoint de fora do §13.12: {chave}")
    for addr, tam in (
        ("11000000", 20),
        ("11000004", 20),
        ("11000005", 4),
        ("11000007", 50),
        ("12000002", 8),
        ("00020000", 8),
        ("10050001", 8),
        ("12001002", 33),
    ):
        chave = ("out", "12", addr)
        if chave not in FORMAS_13:
            falhas.append(f"FORMAS_13 sem o endpoint de escrita {addr} (§13)")
        elif FORMAS_13[chave] != (tam,):
            falhas.append(f"FORMAS_13[{addr}] = {FORMAS_13[chave]}, §13 diz {tam}B")

    # 1b. A transcricao contra a REFERENCIA do mock (o que o device emite).
    if not REFERENCIA.is_dir():
        falhas.append(
            f"referencia de ensaio ausente: {REFERENCIA} — gere com "
            "`scripts/h2_field.sh rehearsal` e versione o resultado"
        )
    else:
        vistos: dict[tuple[str, str, str], set[int]] = {}
        for fluxo in FLUXOS:
            achado = achar_log(REFERENCIA, fluxo)
            if achado is None:
                falhas.append(
                    f"referencia sem o log de {fluxo.nome} ({achado}) — o "
                    "self-check nao tem contra o que conferir a tabela"
                )
                continue
            for f in ler_log(achado):
                vistos.setdefault(f.endpoint, set()).add(f.tamanho)
        for endpoint, tamanhos in sorted(vistos.items()):
            chave = (
                ("out", "12", "SET_PARAM")
                if endpoint[0] == "out" and RE_SET_PARAM.match(endpoint[2])
                else endpoint
            )
            aceitos = FORMAS_13.get(chave)
            if aceitos is None:
                falhas.append(
                    f"a referencia emite {endpoint[0]} {endpoint[1]}/{endpoint[2]} "
                    "e a FORMAS_13 nao conhece — a tabela envelheceu"
                )
            elif not tamanhos <= set(aceitos):
                falhas.append(
                    f"FORMAS_13[{endpoint[2]}] aceita {aceitos}B e a referencia "
                    f"emitiu {sorted(tamanhos)}B"
                )

    # 2. O endereco do set-param tem de aceitar 1..=9 e recusar 0, 10 e "ab".
    #    O formato e `10{slot:02x}0002`, entao o slot 9 e `10090002` e o 10
    #    (que nao existe) seria `100a0002`.
    for slot in range(1, 10):
        bom = f"10{slot:02x}0002"
        if not RE_SET_PARAM.match(bom):
            falhas.append(f"RE_SET_PARAM recusa o slot valido {slot} ({bom}, §13.11)")
    for ruim in ("10000002", "100a0002", "10000000", "100100020"):
        if RE_SET_PARAM.match(ruim):
            falhas.append(f"RE_SET_PARAM aceita {ruim}, que nao e slot 1..=9")

    # 3. A ordem do save tem 5 do meta + 4 ops, e as ops sao 0,0,1,1.
    if len(ORDEM_SAVE) != 9:
        falhas.append(f"ORDEM_SAVE com {len(ORDEM_SAVE)} frames, o §13 diz 9")
    if ORDEM_SAVE[:5] != ("11000000", "11000004", "11000005", "11000007", "12000002"):
        falhas.append(f"ORDEM_SAVE com meta errado: {ORDEM_SAVE[:5]}")
    if OPS_SAVE != (0, 0, 1, 1):
        falhas.append(f"OPS_SAVE = {OPS_SAVE}, a S4 capturou 0,0,1,1")

    if falhas:
        for f in falhas:
            print(f"[self-check] {f}", file=sys.stderr)
        print("[self-check] a transcricao do §13 esta fora de sincronia", file=sys.stderr)
        return 1
    print("[self-check] a transcricao do §13 confere com o protocolo")
    return 0


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("sessao", nargs="?", help="diretorio da sessao de campo")
    ap.add_argument("--markdown", action="store_true", help="so a tabela do §6")
    ap.add_argument("--self-check", action="store_true", help="gate de CI")
    args = ap.parse_args(argv)

    if args.self_check:
        return self_check()
    if not args.sessao:
        ap.print_usage(sys.stderr)
        return EXIT_USO

    sessao = Path(args.sessao)
    if not sessao.is_dir():
        print(f"[h2_compare] sessao nao encontrada: {sessao}", file=sys.stderr)
        return EXIT_USO

    resultados = conferir(sessao)
    if args.markdown:
        print(tabela(resultados))
        return 0

    print("=" * 68)
    print("  FASE C do H2 — os invariantes do §13 nos 3 fluxos")
    print("=" * 68)
    for r in resultados:
        if r.estado == "sem-evidencia":
            print(f"  [SEM EVIDENCIA] {r.fluxo}: {r.detalhe}")
            continue
        if r.estado == "bloqueia":
            print(f"  [PARAR         ] {r.fluxo}")
            for a in r.achados:
                print(f"      - {a.invariant}: esperava {a.esperado}, veio {a.obtido}")
                print(f"        {a.hipotese}")
            continue
        print(f"  [CONFORME      ] {r.fluxo}")
        if r.display:
            print(f"      display: {r.display}")
    print("=" * 68)

    bloqueia = [r for r in resultados if r.estado == "bloqueia"]
    sem_evidencia = [r for r in resultados if r.estado == "sem-evidencia"]
    conformes = [r for r in resultados if r.estado == "ok"]

    if bloqueia:
        print(f"{len(bloqueia)} fluxo(s) com divergencia de §13 — PARAR.")
        print("  Nao repetir o fluxo as cegas, nao trocar de slot. Registrar a")
        print("  divergencia: se o device respondeu diferente do §13 e PROTOCOLO")
        print("  (fluxo R3 do H1_CHECKLIST §6); se respondeu o §13 mas o efeito nao")
        print("  foi o esperado, e COMPORTAMENTO e vira issue nova — nao se")
        print("  consertar mexendo no codec (R1).")
        return EXIT_BLOQUEIA
    if sem_evidencia:
        print(f"{len(conformes)}/{len(resultados)} fluxo(s) conferem o §13; "
              f"{len(sem_evidencia)} sem evidencia.")
        print("  Log sem evidencia NAO conta como passou: se o comando nao rodou,")
        print("  o fluxo nao foi verificado. Repita o passo, nao o gate.")
        return EXIT_BLOQUEIA
    print(f"os 3 fluxos obedeceram o §13. O VEREDITO DO DISPLAY ainda e do")
    print("operador: o fio nao sabe se o knob girou nem se o nome persistiu")
    print("(D3/D4: sao fire-and-forget, sem read-back). Copie a tabela do §6")
    print("para o docs/H2_REPORT.md com o veredito de cada um.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
