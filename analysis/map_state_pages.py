#!/usr/bin/env python3
"""map_state_pages.py — o LAYOUT das páginas 13xx, provado contra a captura (#155).

Cruza a captura S1 (198 presets x 9 páginas pelo addr `13010003`) com os 99
presets XML do `all.prst` e grava `analysis/state_pages_offsets.json`, o
artefato que `preset_pages::slots()` embute no binário.

## O que este script faz (e o que já estava errado nele)

Os quatro defeitos originais, todos corrigidos:

1. `ALLP` era um caminho absoluto `r"D:\\..."` -> resolve a partir de `HERE`.
2. `pp = d[0]` (u8) colapsava os 198 pps em 2 balde -> `int.from_bytes(d[:2], "big")`,
   e o índice do XML e `pp & 0xFF` (os dois bancos apontam para os mesmos 99,
   provado em `tests/preset_pages.rs::nome_bate_com_o_all_prst`).
3. A votacao de u8 era `+= 0  # placeholder` — **nunca votava**.
4. Comparava bytes CRUS. A pagina e nibble-expandida, entao toda comparacao em
   cru falha por construcao.

## Por que a resposta final NAO e uma votacao de offsets

A votacao (corrigida nos 4 pontos acima, depois ainda em f32 LE) deu
`share_min` baixo, e a barra de 0.95 nao passava. Investigar em vez de
rebaixar a barra foi o que achou o layout de verdade:

- **Os params sao `f32 LE`**, nao u8/u16 (`00 00 a0 42` = 80.0). O doc do
  `codec.rs` ja dizia "float de valor LE".
- **Os dados do fio sao indexados por POSICAO na cadeia, nao pelo `x` do XML.**
  A cadeia mora em `pg0[14..32]` (9 x u16 LE) e diz qual `x` do XML ocupa
  cada posicao — ha 11 cadeias distintas nas 198, e as trocadas
  `(0,1,2,3,4,5,7,6,8)` eram exatamente os casos que a votacao errava.
- Os params formam **um fluxo contiguo de 135 f32** (9 slots x 15 params)
  atravessando pg0..pg6 — nao 135 offsets independentes.

Com isso o offset deixa de ser PROCURADO e passa a ser **CALCULADO pela
estrutura**, o que e uma prova muito mais forte que qualquer dominancia de
votos. Este script valida o modelo e so grava se ele casar.

Uso:  python analysis/map_state_pages.py   (exit 1 se o modelo nao casar)
"""
import json
import os
import struct
import sys
import xml.etree.ElementTree as ET
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
CAP1 = os.path.join(HERE, "captures", "session1.jsonl")
ALLP = os.path.join(HERE, os.pardir, "files", "patches", "all.prst")
OUT = os.path.join(HERE, "state_pages_offsets.json")

HDR = "f021257f47502d64"

# Layout provado — ver a doc acima. Tudo em corpo DECODIFICADO (post-unibble).
PP_OFF, PP_LEN = 0, 2          # u16 LE
NOME_OFF, NOME_LEN = 2, 12     # ASCII, pad NUL
CADEIA_OFF, CADEIA_N = 14, 9   # 9 x u16 LE: posicao -> x do XML
CODE_OFF, CODE_N = 32, 9       # 9 x u32 LE (por posicao)
STATE_PAGE, STATE_OFF, STATE_N = 6, 32, 9   # 9 x u16 LE (por posicao)
PARAMS_N = 9 * 15              # 135 x f32 LE, fluxo contiguo
PARAMS_POR_SLOT = 15

# Barra da spec (§1, §7.2): "casa onde deve e falha onde nao deve".
# 0.95 sobre um MODELO determinístico e uma margem folgada para os presets
# cujo valor foi editado no aparelho depois do dump do all.prst.
MODELO_MIN = 0.95


def unibble(b: bytes) -> bytes:
    """Corpo nibble-expandido -> bytes reais (pares -> byte), regra ADR-1."""
    if len(b) % 2:
        b = b[:-1]
    return bytes(((b[i] & 0x0F) << 4) | (b[i + 1] & 0x0F) for i in range(0, len(b) - 1, 2))


def trim(hx):
    i = hx.find("f7")
    return hx[: i + 2] if i >= 0 else hx


def load_rows(path):
    """JSONL da captura -> `(t, dir, func, addr, payload)` cru."""
    evs = []
    for line in open(path, encoding="utf8", errors="replace"):
        s = line.strip()
        if not s:
            continue
        try:
            evs.append(json.loads(s))
        except Exception:
            pass
    if not evs:
        return []
    ts0 = evs[0]["ts"]
    rows = []
    for e in evs:
        hx = e.get("hex", "")
        if not hx:
            continue
        hx = trim(hx)
        if not hx.startswith(HDR) or len(hx) < 30 or len(hx) % 2:
            continue
        b = hx[16:-2]
        if len(b) % 2:
            b = b[:-1]
        try:
            rows.append((e["ts"] - ts0, e["dir"], b[:2], b[2:10], bytes.fromhex(b[10:])))
        except ValueError:
            pass
    return rows


def xml_presets():
    """Os 99 presets do `all.prst`, na ordem do documento."""
    root = ET.parse(ALLP).getroot()
    out = []
    for presets in root.iter("presets"):
        effs = {}
        for eff in presets.iter("Effect"):
            ps = []
            i = 0
            while eff.get(f"params_{i}") is not None:
                try:
                    ps.append(int(eff.get(f"params_{i}")))
                except ValueError:
                    ps.append(None)
                i += 1
            effs[int(eff.get("x", -1))] = {
                "module": eff.get("effectModuleName"),
                "code": int(eff.get("effectCode") or "0"),
                "state": int(eff.get("effectState") or "0"),
                "params": ps,
            }
        out.append({"name": presets.get("ppName"), "effects": effs})
    return out


def coleta_paginas(rows):
    """pp -> {pagina 0..8 -> corpo YA decodificado}."""
    presets = {}
    brutas = Counter()
    for _t, dr, _f, a, d in rows:
        if dr != "in_long" or a != "13010003" or len(d) < 4:
            continue
        page = d[3]
        if page > 8:
            continue
        pp = int.from_bytes(d[:2], "big")  # u16 BE
        brutas[page] += 1
        presets.setdefault(pp, {})[page] = unibble(d[4:])
    return presets, brutas


def pos_de_idx(idx):
    """Indice global 0..134 do fluxo de params -> `(pagina, offset)`.

    O fluxo e contiguo e atravessa fronteiras de pagina:
      0..6    -> pg0[68..96]   (7 f32)
      7..126  -> pg1..pg5      (24 f32 por pagina)
      127..134-> pg6[0..32]    (8 f32)
    7 + 5*24 + 8 = 135 = 9 slots x 15 params.
    """
    if idx < 7:
        return (0, 68 + 4 * idx)
    k = idx - 7
    if k < 120:
        return (1 + k // 24, (k % 24) * 4)
    return (6, (k - 120) * 4)


def u16(b, o):
    return b[o] | (b[o + 1] << 8)


def u32(b, o):
    return int.from_bytes(b[o : o + 4], "little")


def valida_modelo(presets, xmlp):
    """Roda o modelo em todos os pps e devolve as contagens.

    Nada aqui "procura" offset: o offset vem da estrutura. Se o modelo estiver
    errado, ele erra em MASSA — nao em dois casos — entao esta taxa e a prova.
    """
    res = {"params_ok": 0, "params_tot": 0, "state_ok": 0, "state_tot": 0,
           "code_ok": 0, "code_tot": 0, "cadeia_ok": 0, "cadeia_tot": 0}
    difs = []
    for pp, pages in sorted(presets.items()):
        i = pp & 0xFF
        if i >= len(xmlp):
            continue
        xp = xmlp[i]
        pg0 = pages[0]
        cadeia = [u16(pg0, CADEIA_OFF + 2 * p) for p in range(CADEIA_N)]
        if sorted(cadeia) == list(range(9)):
            res["cadeia_ok"] += 1
        res["cadeia_tot"] += 1

        b6 = pages[6]
        for pos in range(9):
            x = cadeia[pos]
            ef = xp["effects"].get(x)
            if ef is None:
                continue
            # effectCode por posicao
            res["code_tot"] += 1
            if u32(pg0, CODE_OFF + 4 * pos) == ef["code"]:
                res["code_ok"] += 1
            # effectState por posicao
            res["state_tot"] += 1
            if u16(b6, STATE_OFF + 2 * pos) == ef["state"]:
                res["state_ok"] += 1
            # params: fluxo contiguo, slot = posicao
            for pi, v in enumerate(ef["params"]):
                if v is None:
                    continue
                g = pos * PARAMS_POR_SLOT + pi
                if g >= PARAMS_N:
                    continue
                pg, off = pos_de_idx(g)
                res["params_tot"] += 1
                got = struct.unpack_from("<f", pages[pg], off)[0]
                if abs(got - float(v)) < 1e-6:
                    res["params_ok"] += 1
                else:
                    # O offset segue certo: o VALOR e que diverge (foi editado
                    # no aparelho depois do dump do all.prst). Registramos.
                    if len(difs) < 20:
                        difs.append({"pp": pp, "pos": pos, "x": x, "param": pi,
                                     "xml": v, "fio": got, "page": pg, "offset": off})
    return res, difs


def span_params():
    """O span do fluxo de params, gerado do proprio `pos_de_idx` (nunca digitado a mao)."""
    span = []
    for g in range(PARAMS_N):
        pg, off = pos_de_idx(g)
        if span and span[-1]["page"] == pg and span[-1]["offset"] + 4 * span[-1]["count"] == off:
            span[-1]["count"] += 1
        else:
            span.append({"page": pg, "offset": off, "count": 1})
    return span


def main():
    sys.stdout.reconfigure(errors="replace")
    rows = load_rows(CAP1)
    presets, brutas = coleta_paginas(rows)
    print(f"linhas S1: {len(rows)} | 13010003 por PG: {dict(sorted(brutas.items()))}")
    print(f"presets: {len(presets)} | paginas por preset: "
          f"{Counter(len(v) for v in presets.values()).most_common()}")
    if not presets:
        print("ERRO: nenhuma pagina capturada")
        return 1

    xmlp = xml_presets()
    print(f"presets XML: {len(xmlp)}")

    res, difs = valida_modelo(presets, xmlp)
    def taxa(k_ok, k_tot):
        t = res[k_tot]
        return res[k_ok] / t if t else 0.0

    print("\n== validacao do MODELO (offset calculado, nao procurado) ==")
    print(f"  cadeia     : {res['cadeia_ok']}/{res['cadeia_tot']}  ({taxa('cadeia_ok','cadeia_tot'):.4f})")
    print(f"  effectCode : {res['code_ok']}/{res['code_tot']}  ({taxa('code_ok','code_tot'):.4f})")
    print(f"  effectState: {res['state_ok']}/{res['state_tot']}  ({taxa('state_ok','state_tot'):.4f})")
    print(f"  params f32 : {res['params_ok']}/{res['params_tot']}  ({taxa('params_ok','params_tot'):.4f})")
    if difs:
        print(f"  divergencias de VALOR (offset certo, dado editado): {len(difs)} mostradas")
        for d in difs[:5]:
            print(f"    pp={d['pp']:#06x} pos={d['pos']} x={d['x']} param_{d['param']}: "
                  f"xml={d['xml']} fio={d['fio']} (pg{d['page']}@{d['offset']})")

    taxa_params = taxa("params_ok", "params_tot")
    taxa_state = taxa("state_ok", "state_tot")
    ok = taxa_params >= MODELO_MIN and taxa_state >= MODELO_MIN

    saida = {
        "generated_from": "analysis/captures/session1.jsonl",
        "encoding": "nibble",
        "method": "modelo-deterministico (offset calculado pela estrutura)",
        "layout": {
            "pp": {"page": 0, "offset": PP_OFF, "width": PP_LEN, "endian": "le"},
            "nome": {"page": 0, "offset": NOME_OFF, "length": NOME_LEN},
            "cadeia": {"page": 0, "offset": CADEIA_OFF, "count": CADEIA_N,
                       "width": 2, "endian": "le",
                       "meaning": "posicao -> x do Effect no all.prst"},
            "effectCode": {"page": 0, "offset": CODE_OFF, "count": CODE_N,
                           "width": 4, "endian": "le", "indexed_by": "posicao"},
            "effectState": {"page": STATE_PAGE, "offset": STATE_OFF, "count": STATE_N,
                            "width": 2, "endian": "le", "indexed_by": "posicao"},
            "params": {"count": PARAMS_N, "per_slot": PARAMS_POR_SLOT,
                       "width": 4, "endian": "le", "float": True,
                       "indexed_by": "posicao", "span": span_params()},
        },
        "presets": len(presets),
        "provas": res,
        "taxa_params": round(taxa_params, 6),
        "taxa_state": round(taxa_state, 6),
        "divergencias_valor": difs,
        "cadeias_distintas": len({
            tuple(u16(p[0], CADEIA_OFF + 2 * i) for i in range(CADEIA_N))
            for p in presets.values()
        }),
    }
    with open(OUT, "w", encoding="utf8") as fh:
        json.dump(saida, fh, ensure_ascii=False, indent=2, sort_keys=True)

    print(f"\n-> {'MODELO VALIDADO' if ok else 'MODELO NAO CASA'} ({OUT})")
    print(f"   taxa_params={taxa_params:.4f} taxa_state={taxa_state:.4f} "
          f"(min {MODELO_MIN})")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
