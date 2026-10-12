#!/usr/bin/env python3
"""flow_dump.py — dumps do fluxo de records 11xx/12xx/0002xx de uma sessão."""
import json, os, sys

HDR = "f021257f47502d64"
CAP = sys.argv[1] if len(sys.argv) > 1 else os.path.join("analysis", "captures", "session1.jsonl")
TMAX = float(sys.argv[2]) if len(sys.argv) > 2 else 200000.0

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
        if t > TMAX:
            break
        b = body(e)
        if not b:
            continue
        dr, f, a, d = e["dir"], b[0], b[1], b[2]
        if (a[:4] in ("1100", "1200") or a[:6] == "000200") and d is not None:
            txt = "".join(chr(c) if 32 <= c < 127 else "." for c in d)
            print(f"+{t/1000:9.2f}s  {dr[:3]} f={f} a={a}  {d.hex(' ')}")
            print(f"{'':22s}txt='{txt}'")

if __name__ == "__main__":
    main()
