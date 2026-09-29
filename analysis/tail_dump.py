#!/usr/bin/env python3
"""tail_dump.py — despeja todos os eventos a partir de t0 ms, com hex truncado."""
import json, os, sys

HDR = "f021257f47502d64"
CAP = sys.argv[1] if len(sys.argv) > 1 else os.path.join("analysis", "captures", "session4.jsonl")
T0 = float(sys.argv[2]) if len(sys.argv) > 2 else 1600000.0
MAXHEX = int(sys.argv[3]) if len(sys.argv) > 3 else 48

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
    n = 0
    for e in evs:
        t = e["ts"] - ts0
        if t < T0:
            continue
        b = body(e)
        if b:
            f, a, d = b
            hx = d.hex(" ")
            more = f" (+{len(d)}B)" if len(d) > MAXHEX // 3 else ""
            print(f"+{t/1000:9.2f}s  {e['dir'][:3]} f={f} a={a}  {hx[:MAXHEX]}{more}")
        else:
            print(f"+{t/1000:9.2f}s  {e['dir'][:3]} SEM-HDR  {e['hex'][:MAXHEX]}")
        n += 1
    print(f"-- total: {n}")

if __name__ == "__main__":
    main()
