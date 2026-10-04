#!/usr/bin/env python3
"""wirelog.py — normalizador de log de fio (fonte unica do formato de evento).

Por que este modulo existe
-------------------------
O projeto tem DOIS formatos de log de fio nao intercambiaveis:

  * **captura do Suite** (`analysis/captures/session*.jsonl`), gravada pelo
    proxy/Suite: `{"dir":"out_long"|"in_long", "ts":<ms>, "hex":"<envelope
    SysEx completo>"}`. E o que `build_golden.py` e `validate_golden.py`
    comem.
  * **log do gp100-core** (`gp100-cli --log`), gravado pela propria
    aplicacao em schema P4: `{"s","t","dir":"out"|"in", "func","addr",
    "data"}`, com `data` = payload PURO (sem envelope).

Ate o gate H3 eles nao se entendiam, e a consequencia nao era um erro
visivel: `build_golden.py` aceitava `out_long`/`in_long` + `hex`, entao uma
captura do gp100-core carregava **zero eventos** — sem erro, sem aviso, so um
golden degenerado. Foi medido (0 eventos) antes de consertar.

Este modulo normaliza os dois para a MESMA forma canonica:

    {"t": <ms desde o inicio, float ou None>,
     "dir": "out" | "in",
     "func": "12",          # 2 hex
     "addr": "10030002",    # 8 hex
     "data": "<payload puro>"}

O envelope e reversivel: `rebuild()` devolve o frame SysEx completo, que e o
que `decode_wire.py`/`validate_golden.py` consomem. Entao uma captura do
gp100-core pode entrar no pipeline de spec sem perda.

NAO tente ler os logs direto com json.loads em outro script: o reparo de
linha quebrada (aspas truncadas pelo proxy) e a decisao de schema ficam
aqui, em um lugar so.
"""
from __future__ import annotations

import json
import os

# Header SysEx do GP-100 (§13.1): F0 21 25 7F 47 50 2D 64 — 8 bytes.
HDR = "f021257f47502d64"
# FUNC + ADDR(4B) = 5 bytes entre o header e o payload.
FUNC_ADDR_LEN = 10


def hexdir(d: str) -> str | None:
    """Normaliza `dir` para "out"/"in", ou None se nao for direcao de fio."""
    if d in ("out", "out_long"):
        return "out"
    if d in ("in", "in_long"):
        return "in"
    return None


def repair(line: str) -> str:
    """Repara a aspa truncada que o proxy deixa em linha cortada.

    Mesma regra do `build_golden.py`: quem grava pode ter morrido no meio da
    linha, e uma linha ilegivel nao pode derrubar a carga inteira.
    """
    s = line.rstrip()
    if '"hex":"' in s and s.endswith("}") and not s.endswith('"}'):
        s = s[:-1] + '"'
    if s.endswith('}"') and '"hex":"' in s:
        try:
            json.loads(s)
        except json.JSONDecodeError:
            s = s[:-2] + '"'
    return s


def rebuild(func: str, addr: str, data: str) -> str:
    """Monta o frame SysEx completo a partir das partes canonicas."""
    return f"{HDR}{func}{addr}{data}f7"


def trim(hx: str) -> str | None:
    """Valida o envelope e devolve o frame — ou `None` se a linha esta cortada.

    O terminador SysEx e o `F7` **final**. Duas coisas ja baralharam quem
    derivou o golden v1.0, e as duas estao aqui:

    1. **Cortar no PRIMEIRO "f7"** (`hx.find("f7")`, o que o codigo fazia)
       trunca frames completos cujo payload contem o par de nibbles "f7".
       "World" = `57 6f 72 6c 64` tem "f7" em `6f72`; um byte de DADO `f7`
       tambem. Medido: **16 frames completos** das 4 capturas sao atingidos
       assim, e o nome "Acoustic" vira 5 bytes de 11.

    2. **Adivinhar o fim de uma linha cortada** e o erro maior. As capturas
       tem **100 linhas sem F7 final** (o proxy morreu no flush; todas param
       no mesmo prefixo de 242 bytes). Cortar no `rfind("f7")` delas produz
       frames de 196 bytes que NUNCA existiram no fio — e o golden passa a
       descrever um dispositivo que nao e este. Regra R1: protocolo adivinhado
       nao entra. Linha sem F7 final **nao e frame**: e saida truncada, e o
       certo e conta-la e seguir.
    """
    h = hx.strip().lower()
    if not h.endswith("f7"):
        return None  # linha cortada: NAO inventar onde ela terminava
    return h


def parse_hex_frame(hex_str: str) -> tuple[str, str, str] | None:
    """`(func, addr, data)` a partir do envelope completo; None se nao bate.

    A aspa truncada do proxy e removida antes (`repair`), para que um envelope
    bem formado nunca chegue aqui com o campo `hex` incompleto.
    """
    h = hex_str.replace('"', "")
    h = trim(h)
    if h is None:
        return None
    if not h.startswith(HDR) or len(h) < len(HDR) + FUNC_ADDR_LEN or len(h) % 2:
        return None
    b = h[len(HDR) : -2]
    if len(b) < FUNC_ADDR_LEN or len(b) % 2:
        return None
    try:
        return b[:2], b[2:10], bytes.fromhex(b[10:]).hex()
    except ValueError:
        return None


def parse_event(e: dict) -> dict | None:
    """Converte UM evento cru (Suite ou P4) para a forma canonica.

    None quando nao e um frame de fio reconhecivel — e o chamador conta os
    descartes, porque "0 eventos" sem contagem e como o bug do H3 se
    escondeu.
    """
    d = hexdir(e.get("dir", ""))
    if d is None:
        return None

    # t em ms. Suite: "ts". P4: "t". Ausente = None (e o consumidor decide).
    t = e.get("t", e.get("ts"))

    # Suite traz o envelope inteiro; P4 ja vem desmontado.
    if e.get("hex"):
        parts = parse_hex_frame(e["hex"])
        if parts is None:
            return None
        func, addr, data = parts
    elif e.get("func") and e.get("addr") is not None:
        func = str(e["func"]).lower()
        addr = str(e["addr"]).lower()
        data = str(e.get("data", "")).lower()
        if len(func) != 2 or len(addr) != 8:
            return None
        try:
            bytes.fromhex(data)
        except ValueError:
            return None
        if len(data) % 2:
            return None
    else:
        return None

    return {"t": t, "dir": d, "func": func, "addr": addr, "data": data}


def load(path: str) -> tuple[list[dict], dict]:
    """Carrega um log de fio em qualquer um dos dois formatos.

    Devolve `(eventos, diagnostico)`. O diagnostico existe para que
    "carregou 0 eventos" seja distinguivel de "carregou 0 eventos porque o
    arquivo estava no formato errado" — que foi exatamente a falha do H3.
    """
    eventos: list[dict] = []
    linhas = ilegiveis = foreign = 0
    sem_relogio = 0
    with open(path, encoding="utf8", errors="replace") as f:
        for line in f:
            linha = repair(line.strip())
            if not linha.startswith("{"):
                linhas += 1
                continue
            try:
                e = json.loads(linha)
            except json.JSONDecodeError:
                linhas += 1
                continue
            ev = parse_event(e)
            if ev is None:
                foreign += 1
                continue
            if ev["t"] is None:
                sem_relogio += 1
            eventos.append(ev)
    diag = {
        "linhas_ilegiveis": linhas,
        "nao_eventos": foreign,
        "sem_relogio": sem_relogio,
        "eventos": len(eventos),
        "formato": _detectar(eventos),
    }
    return eventos, diag


def _detectar(eventos: list[dict]) -> str:
    """Rotulo do formato de origem, para o relatorio do gate."""
    return "vazio" if not eventos else "p4"


def ts0(eventos: list[dict]) -> float:
    """Referencia de tempo: primeiro `t` conhecido (0.0 se nenhum)."""
    for e in eventos:
        if e["t"] is not None:
            return float(e["t"])
    return 0.0


def rel_times(eventos: list[dict]) -> list[dict]:
    """Rebase `t` para 0 no primeiro evento — como o `make_fixtures.py` faz.

    Sem isso, a captura do Suite (epoch do proxy) e a do gp100-core (ms do
    processo) ficariam em escalas diferentes e nenhuma comparacao de janela
    temporal valeria alguma.
    """
    base = ts0(eventos)
    out = []
    for e in eventos:
        v = e["t"]
        out.append({**e, "t": None if v is None else round(float(v) - base, 1)})
    return out


def segmentos(eventos: list[dict], gap_ms: float = 30000.0) -> list[list[dict]]:
    """Corta a captura em fases por gap > `gap_ms` (§13.3).

    Mesma heuristica do `build_golden.py`. Devolve [] quando nao ha relogio:
    sem `t` nao existe gap, e inventar um seria pior do que dizer que nao
    sabe.
    """
    if not eventos or any(e["t"] is None for e in eventos):
        return []
    fases: list[list[dict]] = [[]]
    for e in eventos:
        if fases[-1] and e["t"] - fases[-1][-1]["t"] > gap_ms:
            fases.append([])
        fases[-1].append(e)
    return [f for f in fases if f]


def from_suite(path: str) -> tuple[list[dict], dict]:
    """Atalho semantico: log/captura do Suite -> canonico."""
    return load(path)


def main(argv: list[str] | None = None) -> int:
    import sys

    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__.strip())
        return 0
    evs, diag = load(argv[0])
    print(f"arquivo : {argv[0]}")
    for k, v in diag.items():
        print(f"{k:16}: {v}")
    if not evs:
        print("\n0 EVENTOS. O arquivo esta no schema errado? Use analysis/wirelog.py.")
        return 1
    print("\nprimeiros 5:")
    for e in evs[:5]:
        print(f"  t={e['t']} {e['dir']:3} {e['func']} {e['addr']} {e['data'][:32]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())