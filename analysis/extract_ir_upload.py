#!/usr/bin/env python3
"""extract_ir_upload.py — reassemble GP-100 user-IR uploads from a proxy capture.

Wire (confirmed in session2, docs/PROTOCOL.md §13.7):
  F0 21 25 7F 47 50 2D 64 | 12 12001002 | slot u8 | idx u16 BE | 30 nibbles | F7
  - each payload BYTE carries one nibble (pairs of nibbles = real bytes)
  - 30 nibbles per message = 15 real bytes; u16 index counts CHUNKS
  - chunk indices come in pages of 128 (0-127, 256-383, 512-...) — gaps are
    page boundaries, NOT loss. Last chunk (all 0F) is an end marker, sent twice.

Usage:
  python analysis/extract_ir_upload.py analysis/captures/session2.jsonl
  python analysis/extract_ir_upload.py <trace> --burst 84-100 126-142  (janelas em s)
"""
import json, os, sys

HDR = "f021257f47502d64"

def trim_f7(hx):
    i = hx.find("f7")
    return hx[:i + 2] if i >= 0 else hx

def load(path):
    evs = []
    for line in open(path, encoding="utf-8", errors="replace"):
        s = line.strip()
        if '"hex":"' in s and s.endswith("}") and not s.endswith('"}'):
            s = s[:-1] + '"'          # old proxy bug repair
        if s.endswith('}"') and '"hex":"' in s:
            try:
                json.loads(s)
            except json.JSONDecodeError:
                s = s[:-2] + '"}'
        if not s.startswith("{"):
            continue
        try:
            evs.append(json.loads(s))
        except json.JSONDecodeError:
            continue
    return evs

def collect(evs):
    """-> {(slot, idx): 15 real bytes, ...}, ts0"""
    chunks = {}
    ts0 = evs[0]["ts"]
    for e in evs:
        if e.get("dir") != "out_long":
            continue
        hx = trim_f7(e.get("hex", ""))
        if not hx.startswith(HDR):
            continue
        body = hx[16:-2]
        if body[:10] != "1212001002":
            continue
        data = body[10:]
        if len(data) < 10:
            continue
        slot = int(data[0:2], 16)
        idx = int(data[2:6], 16)
        nibs = list(bytes.fromhex(data[6:]))
        if any(n > 15 for n in nibs):
            continue
        real = bytes((nibs[i] << 4) | nibs[i + 1] for i in range(0, len(nibs) - 1, 2))
        chunks.setdefault((slot, idx), []).append((e["ts"] - ts0, real))
    # keep the most common value per (slot, idx)
    out = {}
    from collections import Counter
    for k, lst in chunks.items():
        c = Counter(r for _, r in lst)
        out[k] = c.most_common(1)[0][0]
    return out, ts0

def group_by_slot(chunks, ta=0, tb=None, ts0=0):
    """-> {slot: [bytes...]} using only records whose first-seen ts is in window."""
    slots = {}
    idxs = sorted({i for (s, i) in chunks})
    for slot in sorted({s for (s, _) in chunks}):
        have = sorted(i for (s, i) in chunks if s == slot)
        blob = b"".join(chunks[(slot, i)] for i in have)
        slots[slot] = {"blob": blob, "idxs": have,
                       "missing": (have[-1] + 1) - len(have)}
    return slots

def main():
    sys.stdout.reconfigure(errors="replace")
    path = sys.argv[1] if len(sys.argv) > 1 else os.path.expandvars(
        r"%TEMP%\midi_trace.jsonl")
    evs = load(path)
    if not evs:
        print("sem eventos"); return 1
    chunks, ts0 = collect(evs)
    print(f"eventos: {len(evs)} | chunks IR: {len(chunks)}")
    slots = group_by_slot(chunks)
    for slot, info in sorted(slots.items()):
        blob = info["blob"]
        fn = os.path.join("analysis", "captures", f"ir_slot{slot}.bin")
        open(fn, "wb").write(blob)
        name = blob[:16].split(b"\x00")[0].decode("ascii", "replace")
        print(f"slot{slot}: {len(info['idxs'])} chunks (idx 0..{info['idxs'][-1]}, "
              f"brechas de pagina: {info['missing']}), {len(blob)}B -> {fn}")
        print(f"   nome no header: '{name}'")
        print(f"   head: {blob[:32].hex(' ')}")
    return 0

if __name__ == "__main__":
    sys.exit(main())
