#!/usr/bin/env python3
"""dump_edit_writes.py — extrai writes OUT func 12 addr 10 XX 00 02 (fase de edição)."""
import json, os, sys
from collections import Counter, defaultdict

HDR = "f021257f47502d64"
CAP = sys.argv[1] if len(sys.argv) > 1 else os.path.join("analysis", "captures", "session3.jsonl")

def trim(hx):
    i = hx.find("f7")
    return hx[: i + 2] if i >= 0 else hx

def load(path):
    evs = []
    for line in open(path, encoding="utf8", errors="replace"):
        s = line.strip()
        if '"hex":"' in s and s.endswith("}") and not s.endswith('"}'):
            s = s[:-1] + '"'
        if s.endswith('}"') and '"hex":"' in s:
            try:
                json.loads(s)
            except json.JSONDecodeError:
                s = s[:-2] + '"}'
        if not s.startswith("{"):
            continue
        try:
            e = json.loads(s)
        except json.JSONDecodeError:
            continue
        hx = e.get("hex", "")
        if e.get("dir") in ("out_long", "in_long") and hx:
            e["hex"] = trim(hx)
            evs.append(e)
    return evs

def body(e):
    hx = e["hex"]
    if not hx.startswith(HDR) or len(hx) < 30 or len(hx) % 2:
        return None
    b = hx[16:-2]
    if len(b) % 2:
        b = b[:-1]
    try:
        return b[:2], b[2:10], bytes.fromhex(b[10:])
    except ValueError:
        return None

def main():
    sys.stdout.reconfigure(errors="replace")
    evs = load(CAP)
    ts0 = evs[0]["ts"]
    rows = []
    for e in evs:
        b = body(e)
        if b:
            rows.append((e["ts"] - ts0, e["dir"], b[0], b[1], b[2]))

    # writes para enderecos 10 XX 00 02 (fora do bloco 13xx)
    hits = [(t, a, d) for t, dr, f, a, d in rows
            if dr == "out_long" and f == "12" and a.startswith("10") and a.endswith("0002") and t > 30000]
    print(f"writes OUT 10xx0002 (t>30s): {len(hits)}")
    byaddr = defaultdict(list)
    for t, a, d in hits:
        byaddr[a].append((t, d))
    for a in sorted(byaddr):
        ds = byaddr[a]
        forms = Counter(d.hex() for _, d in ds)
        print(f"\n== {a}: {len(ds)} writes, {len(forms)} datas distintas ==")
        for hx, c in forms.most_common(12):
            print(f"   {hx}  x{c}")
        print(f"   t: {ds[0][0]:.0f}ms .. {ds[-1][0]:.0f}ms")

    # sequencia temporal completa (apertada)
    print("\n== sequencia temporal (addr, data) ==")
    for t, a, d in hits:
        print(f"  +{t:8.0f}ms  {a}  {d.hex(' ')}")

if __name__ == "__main__":
    main()
