"""Compara dois dump-preset (frames IN) e mostra ONDE o estado mudou.

A pagina 0 do preset guarda os knobs da cadeia; a pagina 7 (32B) guarda a
cadeia de IRs. Nibble-expand por byte: b -> [b>>4, b & 0x0F].
"""
import json
import struct
import sys
from pathlib import Path

SESS = Path(__file__).resolve().parent


def load_in(path: Path) -> dict[str, str]:
    """devolve {rotulo: data_hex} dos frames IN de pagina.

    O dump responde 13010001 (select), 13010003 (paginas) e 13010005 (open).
    So as PAGINAS sao as respostas 13010003, e o contador de 4 bytes no
    inicio do payload e 1-based: a pagina 0 do dump traz `00000001`.
    """
    pages: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        r = json.loads(line)
        if r.get("dir") != "in" or r.get("addr") != "13010003":
            continue
        real = collapse(r["data"])
        # contador de 2B no inicio do payload real, 1-based e BIG-endian:
        # o fio `00000001` colapsa para [00, 01] = pagina 0 do dump.
        counter = int.from_bytes(real[:2], "big")
        pages[counter - 1] = r["data"]
    return pages


def collapse(wire_hex: str) -> bytes:
    w = bytes.fromhex(wire_hex)
    return bytes((w[i] << 4) | w[i + 1] for i in range(0, len(w), 2))


def knobs_de_pagina0(real: bytes) -> list[tuple[int, int, float]]:
    """Registros de 8B [ctrl, 0, 0, 0, 0, 0, 0, 0, 0, 0, f32LE] -> knob.

    Heuristica: o preset guarda o valor do knob como f32 dentro do registro;
    procuramos os-aligned floats que batem com um valor plausivel de pedal.
    """
    out = []
    for i in range(0, len(real) - 9):
        ctrl = real[i]
        if ctrl > 9:
            continue
        val = struct.unpack("<f", real[i + 2 : i + 6])[0]
        if -100.0 <= val <= 100.0 and val != 0.0:
            out.append((i, ctrl, val))
    return out


antes = load_in(SESS / "antes.jsonl")
depois = load_in(SESS / "rb_dump_final2.jsonl")

print(f"paginas antes={len(antes)} depois={len(depois)}\n")
for idx in sorted(set(antes) | set(depois)):
    a, d = antes.get(idx), depois.get(idx)
    if a == d:
        continue
    ra, rd = collapse(a), collapse(d)
    diffs = [k for k in range(min(len(ra), len(rd))) if ra[k] != rd[k]]
    print(f"--- pagina {idx} ({len(ra)}B) mudou em {len(diffs)} posicao(oes) ---")
    for k in diffs[:24]:
        print(f"    byte[{k:>3}]  {ra[k]:02x} -> {rd[k]:02x}")
    # f32 LE em cada posicao mudada, para ler o valor em vez do hex cru
    for k in diffs:
        if k + 4 <= len(ra):
            va = struct.unpack("<f", ra[k : k + 4])[0]
            vd = struct.unpack("<f", rd[k : k + 4])[0]
            if 1e-3 < abs(va) < 1e6 and 1e-3 < abs(vd) < 1e6:
                print(f"      -> f32 LE no byte[{k}]: {va:g} -> {vd:g}")
    print()