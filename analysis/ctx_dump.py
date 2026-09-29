#!/usr/bin/env python3
"""ctx_dump.py — TODOS os eventos (incl. sem-header) numa janela, com hex completo."""
import json, os, sys

HDR = "f021257f47502d64"
CAP = sys.argv[1]
T0 = float(sys.argv[2])
T1 = float(sys.argv[3])

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
    for e in evs:
        t = e["ts"] - ts0
        if not (T0 <= t <= T1):
            continue
        b = body(e)
        if b:
            f, a, d = b
            print(f"+{t/1000:9.2f}s  {e['dir'][:3]} f={f} a={a}  {d.hex(' ')[:100]}")
        else:
            print(f"+{t/1000:9.2f}s  {e['dir'][:3]} RAW  {e['hex']}")

if __name__ == "__main__":
    main()
