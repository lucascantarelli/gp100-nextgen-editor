#!/usr/bin/env python3
"""scan_prst.py — scan .prst patch files for the object-file envelope (§10-11)
and the type-5 magic 0x817, byte-aligned AND bit-shifted (the envelope is
bit-packed: class(3b)+small(1b)+varint+type(4b), so 0x817 may span bytes).

Usage: python analysis/scan_prst.py <file.prst> [more.prst ...]
"""
import sys, glob, os

def bitfind(data, pattern, plen):
    """find `pattern` (int, plen bits, MSB-first) in a bitstream; return byte offsets."""
    hits = []
    total = len(data) * 8
    for start in range(0, total - plen):
        v = 0
        for k in range(plen):
            bitpos = start + k
            b = data[bitpos >> 3]
            bit = (b >> (7 - (bitpos & 7))) & 1
            v = (v << 1) | bit
        if v == pattern:
            hits.append(start)
    return hits

def varint_at(data, bitpos):
    """decode the envelope varint (MIDI-style) at bit position; return (value, bits_used)."""
    def byte_at(p):
        v = 0
        for k in range(8):
            b = data[(p + k) >> 3]
            v = (v << 1) | ((b >> (7 - ((p + k) & 7))) & 1)
        return v
    first = byte_at(bitpos)
    if not (first & 0x80):
        return first, 8
    prefix = first >> 4
    if prefix == 0xC:
        return ((first & 0x3F) << 7) | (byte_at(bitpos + 8) & 0x7F), 16
    if prefix == 0xE:
        mid = byte_at(bitpos + 8) & 0x7F
        lo = byte_at(bitpos + 16) & 0x7F
        return ((first & 0x1F) << 14) | (mid << 7) | lo, 24
    v, used, p = first & 0x7F, 8, bitpos + 8
    while True:
        b = byte_at(p)
        v = (v << 7) | (b & 0x7F)
        used += 8
        p += 8
        if not (b & 0x80):
            return v, used

def scan(path):
    data = open(path, "rb").read()
    name = os.path.basename(path)
    print(f"=== {name}: {len(data)}B")
    print("  head[0:48]:", data[:48].hex(" "))
    print("  ascii      :", "".join(chr(c) if 32 <= c < 127 else "." for c in data[:48]))

    # byte-aligned occurrences
    hits817 = [i for i in range(len(data) - 1) if data[i] == 0x81 and data[i + 1] == 0x7F]
    hits0817 = [i for i in range(len(data) - 1) if data[i] == 0x08 and data[i + 1] == 0x17]
    print(f"  byte-aligned: 81 7F x{len(hits817)} | 08 17 x{len(hits0817)}")
    for i in hits817[:8]:
        print(f"    817F @0x{i:x}: ...{data[max(0,i-10):i+14].hex(' ')}...")

    # bit-shifted scan (0x817 as 12 bits) — only if byte-aligned search came up empty
    if not hits817:
        print("  varrendo bit-stream por 0x817 em qualquer shift (pode levar alguns s)...")
        hits = bitfind(data, 0x817, 12)
        byshift = {}
        for s in hits:
            byshift[s % 8] = byshift.get(s % 8, 0) + 1
        print(f"  bit-shifted: {len(hits)} hits | por shift: {byshift}")
        for s in hits[:6]:
            byteoff, shift = s // 8, s % 8
            lo = max(0, byteoff - 6)
            chunk = data[lo:byteoff + 8]
            print(f"    @0x{byteoff:x} shift{shift}: ...{chunk.hex(' ')}...")

    # envelope probes: try decoding class/small/varint/type at plausible start offsets
    print("  sondas de envelope (offsets 0,4,8,...,64):")
    for off in range(0, 65, 4):
        bitpos = off * 8
        try:
            # class(3) + small(1) come before varint
            cls = 0
            for k in range(3):
                b = data[(bitpos + k) >> 3]
                cls = (cls << 1) | ((b >> (7 - ((bitpos + k) & 7))) & 1)
            sm = (data[(bitpos + 3) >> 3] >> (7 - ((bitpos + 3) & 7))) & 1
            v, used = varint_at(data, bitpos + 4)
            tb = 0
            tp = bitpos + 4 + used
            for k in range(4):
                b = data[(tp + k) >> 3]
                tb = (tb << 1) | ((b >> (7 - ((tp + k) & 7))) & 1)
            if tb <= 6:
                print(f"    @{off:3d}: class={cls} small={sm} value={v} type={tb}")
        except Exception:
            pass
    print()

if __name__ == "__main__":
    sys.stdout.reconfigure(errors="replace")
    args = sys.argv[1:] or sorted(glob.glob(r"D:\GP-100 app\files\patches\*.prst"))
    for a in args:
        scan(a)
