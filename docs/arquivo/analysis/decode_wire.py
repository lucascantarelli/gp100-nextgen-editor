#!/usr/bin/env python3
"""decode_wire.py — decode the OBSERVED GP-100 MIDI wire format from a capture.

Wire header confirmed in session1:  F0 21 25 7F 47 50 2D 64
  F0 sysex | 21 25 7F = Valeton manufacturer | 47 50 2D = "GP-" | 64 = 100 dec
Then: func byte + address bytes + payload + F7. Raw bytes (proxy logs
midiOutLongMsg / MIM_LONGDATA payloads, no USB-MIDI CIN framing).

Usage: python analysis/decode_wire.py analysis/captures/session1.jsonl
"""
import json, sys
from collections import Counter

HDR = "f021257f47502d64"

def trim_f7(hx):
    """Input buffers often come zero-padded past the real F7 terminator
    (dwBytesRecorded can arrive 0 in MIM_LONGDATA). Keep up to first F7."""
    i = hx.find("f7")
    return hx[:i + 2] if i >= 0 else hx

def load(path):
    evs = []
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.strip()
        if not line:
            continue
        try:
            evs.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return evs

def main(path):
    evs = load(path)
    by_dir = Counter(e.get("dir") for e in evs)
    print(f"events: {len(evs)}  by_dir: {dict(by_dir)}")

    shapes = Counter()      # (dir, func, addr[0], addr[1]) -> count
    sizes = Counter()       # (dir, total bytes) -> count
    shorts_out, shorts_in, bigs = [], [], []
    badhdr = 0
    ts0 = evs[0].get("ts", 0) if evs else 0

    for e in evs:
        d = e.get("dir")
        if d not in ("out_long", "in_long"):
            continue
        hx = trim_f7(e.get("hex", ""))
        if not hx.startswith(HDR):
            if hx:
                badhdr += 1
                if badhdr <= 5:
                    print(f"  [!] header inesperado ({d}): {hx[:48]}")
            continue
        body = hx[len(HDR):-2]           # strip header + F7
        n = len(body) // 2
        if n < 2:
            continue
        func = body[:2]
        a0, a1 = body[2:4], body[4:6]
        shapes[(d, func, a0, a1)] += 1
        sizes[(d, len(hx) // 2)] += 1
        dt = e.get("ts", 0) - ts0
        if d == "out_long" and n <= 16:
            shorts_out.append((dt, n, hx))
        elif d == "in_long" and n <= 16:
            shorts_in.append((dt, n, hx))
        elif d == "in_long":
            bigs.append((dt, n, hx))

    if badhdr:
        print(f"[!] {badhdr} mensagens longas SEM o header GP-100 (investigar)")

    print("\n== formas (dir, func, addr0, addr1) — top 25:")
    for (d, f, a0, a1), c in shapes.most_common(25):
        print(f"  {d:9s} func={f} addr={a0} {a1}  x{c}")

    print("\n== tamanhos (dir, bytes) — top 25:")
    for (d, l), c in sorted(sizes.items(), key=lambda kv: -kv[1])[:25]:
        print(f"  {d:9s} {l:5d}B  x{c}")

    print(f"\n== OUT curtas ({len(shorts_out)}) — pedidos/escritas, todas:")
    for dt, n, hx in shorts_out:
        print(f"  +{dt:6d}ms {n:3d}B  {hx}")

    print(f"\n== IN curtas ({len(shorts_in)}) — acks, primeiras 40:")
    for dt, n, hx in shorts_in[:40]:
        print(f"  +{dt:6d}ms {n:3d}B  {hx}")

    print(f"\n== IN grandes ({len(bigs)}) — dumps:")
    for dt, n, hx in bigs[:12]:
        print(f"  +{dt:6d}ms {n:5d}B  {hx[:96]}...")

    # write candidates: out msgs with func 12 (data after a 4-byte address)
    print("\n== candidatas a WRITE (out, func=12):")
    for dt, n, hx in shorts_out:
        body = hx[len(HDR):-2]
        if body[:2] == "12" and len(body) >= 10:
            print(f"  +{dt:6d}ms  addr={body[2:10]}  data={body[10:]}")

if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "analysis/captures/session1.jsonl")
