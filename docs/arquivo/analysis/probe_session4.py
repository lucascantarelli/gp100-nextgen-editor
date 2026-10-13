#!/usr/bin/env python3
"""probe_session4.py — decifra endereços 11xx/12xx/0002xxxx da sessão 4."""
import json, os, sys
from collections import Counter, defaultdict

HDR = "f021257f47502d64"
CAP = os.path.join("analysis", "captures", "session4.jsonl")

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
        if e.get("dir") in ("out_long", "in_long", "out_short", "in_short") and hx:
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
    dirs = Counter()
    for e in evs:
        dirs[e["dir"]] += 1
        b = body(e)
        if b:
            rows.append((e["ts"] - ts0, e["dir"], b[0], b[1], b[2]))
        else:
            rows.append((e["ts"] - ts0, e["dir"], None, None, None))
    print("dirs:", dict(dirs))

    # 1) janela da sessao 4: TODOS os eventos (incl. short/sem header)
    print("\n== janela 1650-1690s: TODOS os eventos ==")
    for t, dr, f, a, d in rows:
        if 1650000 <= t <= 1690000:
            if f:
                print(f"+{t/1000:9.2f}s  {dr[:3]} f={f} a={a}  {d.hex(' ')[:90]}")
            else:
                print(f"+{t/1000:9.2f}s  {dr[:3]} SEM-HDR  raw={e_hex if (e_hex:=None) else '?'}")

    # 2) boot: trafego nos enderecos 11xx/12xx/0002xxxx (t<30s)
    print("\n== boot (t<30s): amostra por (dir,f,addr) ==")
    boot = Counter()
    examples = defaultdict(list)
    for t, dr, f, a, d in rows:
        if t > 30000 or f is None:
            continue
        if a and (a.startswith("11") or a.startswith("12") or a.startswith("0002")):
            boot[(dr[:3], f, a, len(d) if d else 0)] += 1
            if len(examples[(dr[:3], f, a, len(d) if d else 0)]) < 3:
                examples[(dr[:3], f, a, len(d) if d else 0)].append(d.hex(" ") if d else "")
    for k, c in boot.most_common(30):
        print(f"  {k}  x{c}")
        for ex in examples[k]:
            print(f"      ex: {ex}")

    # 3) reads OUT 11 11000008 no boot: payload = requisicao?
    print("\n== OUT 11 11000008 no boot (payloads distintos) ==")
    reqs = Counter()
    for t, dr, f, a, d in rows:
        if t <= 30000 and f == "11" and a == "11000008" and d:
            reqs[d.hex(" ")] += 1
    for hx, c in list(reqs.most_common())[:40]:
        print(f"  {hx}  x{c}")

    # 4) compares: writes 00020000 em todo o log
    print("\n== writes OUT 00020000 (todo o log) ==")
    for t, dr, f, a, d in rows:
        if f == "12" and a == "00020000" and d:
            print(f"  +{t/1000:9.2f}s  {d.hex(' ')}")

if __name__ == "__main__":
    main()
