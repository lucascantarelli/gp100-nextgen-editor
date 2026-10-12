#!/usr/bin/env python3
"""check_session2.py — analyzer for capture session 2 (IR/NAM import).

Run after the user performs the IR-import session:
  python analysis/check_session2.py [%TEMP%/midi_trace.jsonl]

What it does:
  1. Parses the proxy trace (defensive: repairs the old missing-quote bug too).
  2. Histogram of func/addr shapes, printing ONLY shapes not seen in session1.
  3. Greps the wire for IR signatures:
       - RIFF header bytes 52 49 46 46        (raw WAV upload)
       - ASCII "GP100"  = 47 50 31 30 30
       - int24-LE signature: 47 00 00 50 00 00 31 00 00 30 00 00 30 00 00
  4. Scans every message for the file-format magic "817" with context.
  5. Lists files under %APPDATA% modified in the last 2h and greps them
     for the same signatures (the Suite may cache imported IRs on disk).
"""
import json, os, sys, time, glob
from collections import Counter

HDR = "f021257f47502d64"
KNOWN_SHAPES = {  # (dir, func, addr0, addr1) observed in session1
    ("in_long", "12", "13", "01"), ("out_long", "12", "13", "01"),
    ("out_long", "11", "13", "01"), ("in_long", "12", "13", "00"),
    ("out_long", "11", "13", "00"), ("in_long", "12", "11", "00"),
    ("out_long", "11", "11", "00"), ("in_long", "12", "12", "00"),
    ("out_long", "11", "12", "00"), ("in_long", "12", "13", "02"),
    ("out_long", "12", "13", "02"), ("out_long", "12", "00", "02"),
    ("out_long", "12", "11", "00"), ("out_long", "11", "13", "02"),
    ("out_long", "12", "10", "01"), ("out_long", "11", "13", "00"),
    ("out_long", "12", "12", "00"),
}
SIGS = {
    "RIFF(raw wav)": "52494646",
    "ASCII GP100": "4750313030",
    "int24-LE sig": "470000500000310000300000300000",
}

def repair(line):
    s = line.rstrip()
    if '"hex":"' in s and s.endswith('}') and not s.endswith('"}'):
        s = s[:-1] + '"'
    if s.endswith('}"') and '"hex":"' in s:
        try:
            json.loads(s)
        except json.JSONDecodeError:
            s = s[:-2] + '"}'
    return s

def load(path):
    evs = []
    for line in open(path, encoding="utf-8", errors="replace"):
        line = repair(line.strip())
        if not line.startswith("{"):
            continue
        try:
            evs.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return evs

def trim_f7(hx):
    i = hx.find("f7")
    return hx[:i + 2] if i >= 0 else hx

def main(path):
    evs = load(path)
    longs = [e for e in evs if e.get("dir") in ("out_long", "in_long")]
    print(f"events: {len(evs)}  longs: {len(longs)}  "
          f"by_dir: {dict(Counter(e.get('dir') for e in evs))}")

    # --- new shapes vs session1 ---
    shapes = Counter()
    for e in longs:
        hx = trim_f7(e["hex"])
        if hx.startswith(HDR) and len(hx) >= len(HDR) + 12:
            b = hx[len(HDR):-2]
            shapes[(e["dir"], b[:2], b[2:4], b[4:6])] += 1
    new = {k: v for k, v in shapes.items() if k not in KNOWN_SHAPES}
    print("\n== FORMAS NOVAS vs sessao 1 (dir, func, a0, a1):")
    for k, v in sorted(new.items(), key=lambda kv: -kv[1]):
        print(f"  {k[0]:8s} func={k[1]} addr={k[2]} {k[3]}  x{v}")
    if not new:
        print("  (nenhuma — sessao nao exercitou caminhos novos?)")

    # --- signature scan on the wire ---
    print("\n== ASSINATURAS no fio:")
    hits = Counter()
    for name, sig in SIGS.items():
        for e in longs:
            if sig in e["hex"]:
                hits[name] += 1
    for name, sig in SIGS.items():
        print(f"  {name:14s} {hits.get(name, 0)} mensagens"
              + ("  <<< IR ENCONTRADO NO FIO!" if hits.get(name) else ""))

    # --- 0x817 scan ---
    print("\n== magic '817' (contexto, primeiras 12):")
    n = 0
    for e in longs:
        hx = e["hex"]
        i = hx.find("817")
        if i >= 0 and i % 2 == 0:
            n += 1
            if n <= 12:
                lo = max(0, i - 24)
                print(f"  {e['dir']:8s} ...{hx[lo:i+34]}...")
    print(f"  total: {n} mensagens com 817")

    # --- files in %APPDATA% ---
    print("\n== arquivos em %APPDATA% (2h) e assinaturas:")
    roots = [os.path.expandvars(r"%APPDATA%"), os.path.expandvars(r"%LOCALAPPDATA%")]
    cand = []
    now = time.time()
    for root in roots:
        for pat in ("GP-100*", "Valeton*"):
            for p in glob.glob(os.path.join(root, pat, "**", "*"), recursive=True):
                try:
                    if os.path.isfile(p) and now - os.path.getmtime(p) < 2 * 3600:
                        cand.append(p)
                except OSError:
                    pass
    for p in sorted(set(cand))[:30]:
        try:
            data = open(p, "rb").read()
            marks = [name for name, sig in SIGS.items()
                     if bytes.fromhex(sig) in data]
            m817 = b"\x81\x7f" in data or b"\x08\x17" in data
            print(f"  {os.path.getsize(p):9d}B  {p}"
                  + (f"  <<< {marks}" if marks else "")
                  + ("  [817?]" if m817 else ""))
        except OSError:
            pass
    if not cand:
        print("  (nenhum arquivo modificado recentemente)")

if __name__ == "__main__":
    default = os.path.expandvars(r"%TEMP%\midi_trace.jsonl")
    main(sys.argv[1] if len(sys.argv) > 1 else default)
