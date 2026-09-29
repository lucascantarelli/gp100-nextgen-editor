#!/usr/bin/env python3
"""map_state_pages.py — build the preset state-page map by crossing the
session1 scan (99 presets x 9 pages of 192B via addr 13 01 00 03) with the
99 XML presets of all.prst.

Per preset we test which page holds each effect's params (u8 and u16-LE runs),
then verify offset consistency across presets (same x/param -> same page+offset).

Usage: python analysis/map_state_pages.py
"""
import json, os, sys
import xml.etree.ElementTree as ET
from collections import defaultdict, Counter

HDR = "f021257f47502d64"
CAP1 = os.path.join("analysis", "captures", "session1.jsonl")
ALLP = r"D:\GP-100 app\files\patches\all.prst"

def trim(hx):
    i = hx.find("f7")
    return hx[:i + 2] if i >= 0 else hx

def load_rows(path):
    evs = []
    for line in open(path, encoding="utf8", errors="replace"):
        s = line.strip()
        if not s:
            continue
        try:
            evs.append(json.loads(s))
        except Exception:
            pass
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
            effs[(eff.get("effectModuleName"), int(eff.get("x", -1)))] = ps
        out.append({"name": presets.get("ppName"), "irnum": presets.get("ppIRNum"),
                    "bank": presets.get("ppBank"), "effects": effs})
    return out

def main():
    sys.stdout.reconfigure(errors="replace")
    rows = load_rows(CAP1)
    # collect page dumps: in_long 13010003 with data[0]==preset? Structure:
    # OUT 13010000 <- [pp]  -> IN 13010001 (6B meta)
    # OUT 13010002 <- [pp] 01 -> IN 13010003 page0..8 (196B; d[0]=pp, d[1]=?, d[2]=?, d[3]=page)
    presets = defaultdict(dict)   # pp -> {page: bytes}
    cur_pp = None
    last_write02 = None
    for t, dr, f, a, d in rows:
        if dr == "out_long" and a == "13010000" and len(d) >= 1:
            cur_pp = d[0] if len(d) == 1 else int.from_bytes(d[:2], "big")
        if dr == "in_long" and a == "13010003" and len(d) >= 100:
            pp = d[0]
            page = d[3] if len(d) > 3 else 0
            presets[pp][page] = d[4:] if d[3] == 0 else d[4:]
    print(f"presets capturados: {len(presets)} | paginas por preset: "
          f"{Counter(len(v) for v in presets.values()).most_common(5)}")
    if not presets:
        return

    xmlp = xml_presets()
    print(f"presets XML: {len(xmlp)}")

    # index: (x, param_index, value) -> Counter of (page, offset, width)
    votes = defaultdict(Counter)
    for pp, pages in list(presets.items())[:40]:
        if pp >= len(xmlp):
            continue
        xp = xmlp[pp]
        for page, blob in pages.items():
            for (mod, x), ps in xp["effects"].items():
                for pi, v in enumerate(ps):
                    if v is None or not (0 <= v <= 0xFFFF):
                        continue
                    b8 = bytes([v & 0xFF, (v >> 8) & 0xFF])
                    # u8 at any offset
                    for off in (i for i, c in enumerate(blob) if c == (v & 0xFF)):
                        votes[(x, pi)][("u8", page, off)] += 0  # placeholder
                    # u16 LE exact
                    start = blob.find(b8)
                    while start >= 0:
                        votes[(x, pi)][("u16", page, start)] += 1
                        start = blob.find(b8, start + 1)
    print("\n== top candidatos por (slot x, param):")
    for key in sorted(votes):
        c = votes[key]
        if not c:
            continue
        top = c.most_common(3)
        total = sum(v for _, v in top)
        pretty = ", ".join(f"{w} pg{p}@{o} ({n})" for (w, p, o), n in top)
        print(f"  x={key[0]:2d} param_{key[1]:<2d}: {pretty}")

if __name__ == "__main__":
    main()
