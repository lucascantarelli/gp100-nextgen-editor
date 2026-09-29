#!/usr/bin/env python3
"""recon_session3.py — recon da captura: timeline por janelas de 5s + formas."""
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
    if not evs:
        print("nada carregado"); return
    ts0 = evs[0]["ts"]
    rows = []
    other = Counter()
    for e in evs:
        b = body(e)
        if b:
            rows.append((e["ts"] - ts0, e["dir"], b[0], b[1], b[2]))
        else:
            other[(e["dir"], e["hex"][:16])] += 1
    print(f"eventos headerados: {len(rows)} | fora de header: {sum(other.values())}")
    for k, v in other.most_common(5):
        print(f"  {k}: {v}")

    # janelas de 5s
    win = defaultdict(Counter)
    for t, d, f, a, dat in rows:
        win[int(t // 5000) * 5][f"{d[:3]}/{f}/{a}"] += 1
    print("\n== timeline (janelas de 5s, top formas) ==")
    for w in sorted(win):
        top = "  ".join(f"{k}:{v}" for k, v in win[w].most_common(6))
        print(f"  {w:4d}s  {top}")

    # formas de write 13010004 (out) e 13010003 (in)
    w04 = Counter()
    for t, d, f, a, dat in rows:
        if d == "out_long" and f == "12" and a == "13010004":
            w04[(len(dat),)] += 1
    print("\n== writes OUT 13010004 por tamanho de data:", dict(w04))
    w03 = Counter()
    for t, d, f, a, dat in rows:
        if d == "in_long" and f == "12" and a == "13010003":
            w03[len(dat)] += 1
    print("== pushes IN 13010003 por tamanho:", dict(w03))

    # amostra de writes 5B fora da fase de scan (t > 30s)
    print("\n== amostra OUT 13010004 com data 5B, t>30s ==")
    shown = 0
    for t, d, f, a, dat in rows:
        if d == "out_long" and f == "12" and a == "13010004" and len(dat) == 5 and t > 30000 and shown < 15:
            print(f"  +{t:8.0f}ms  {dat.hex(' ')}")
            shown += 1

if __name__ == "__main__":
    main()
