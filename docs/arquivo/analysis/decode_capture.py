#!/usr/bin/env python3
"""Decode a MIDI capture (JSONL from midi_proxy or converted DebugView log)
against the documented GP-100 protocol envelope.

Usage: python decode_capture.py %TEMP%/midi_trace.jsonl
Output: decoded events + list of unknown field values (the calibration targets:
0x817, session flags, subtags 0x6E/0x18).
"""
import json, sys
from collections import Counter

sys.stdout.reconfigure(errors="replace")

# ---------- protocol primitives (from FINDINGS_*) ----------

def crc8_07(buf):
    c = 0
    for b in buf:
        c ^= b
        for _ in range(8):
            c = ((c << 1) ^ 0x07) & 0xFF if c & 0x80 else (c << 1) & 0xFF
    return c

class BitReader:
    def __init__(self, data):
        self.data = data
        self.pos = 0  # bit position
    def read(self, n):
        v = 0
        for _ in range(n):
            byte = self.data[self.pos >> 3]
            bit = (byte >> (7 - (self.pos & 7))) & 1
            v = (v << 1) | bit
            self.pos += 1
        return v
    def remaining_bits(self):
        return len(self.data) * 8 - self.pos

def read_varint(r):
    """MIDI-style varint with continuation bytes 0x80|xxxxxx (from 0x6331D0/0x633520)."""
    first = r.read(8)
    if not (first & 0x80):
        return first
    prefix = first >> 4
    if prefix == 0xC:   # 2-byte
        lo = r.read(8) & 0x7F
        return ((first & 0x3F) << 7) | lo
    if prefix == 0xE:   # 3-byte
        mid = r.read(8) & 0x7F
        lo = r.read(8) & 0x7F
        return ((first & 0x1F) << 14) | (mid << 7) | lo
    # generic continuation chain
    v = first & 0x7F
    while True:
        b = r.read(8)
        v = (v << 7) | (b & 0x7F)
        if not (b & 0x80):
            return v

OBJ_TYPES = {0: "RAW_BLOB", 1: "U32_LENGTH", 2: "HEADERED_BLOB",
             3: "PATTERN_TABLE", 4: "STRING_TABLE", 5: "IR_HW_PARAMS",
             6: "DUAL_STRING_U64"}

def decode_object(data, depth=0):
    r = BitReader(data)
    cls = r.read(3)
    small = r.read(1)
    value = read_varint(r)
    tfield = r.read(4)
    out = {"class": cls, "small": small, "value": value, "type": tfield,
           "typename": OBJ_TYPES.get(tfield, f"?{tfield}"),
           "payload_bits": r.remaining_bits()}
    # type-5 magic / subtags we are calibrating:
    unknown = []
    if tfield == 5:
        raw = bytes(data)
        unknown = [hex(off) for off in range(len(raw) - 1)
                   if raw[off] == 0x81 and raw[off + 1] == 0x7F]
        out["magic_0x817_hits"] = len(unknown)
    return out, unknown

# ---------- main ----------

def main(path):
    events = []
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.strip()
        if not line:
            continue
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            continue
        # normalize: proxy logs "hex"; converted DebugView logs may carry "bytes"
        if "bytes" in ev and "hex" not in ev:
            try:
                ev["hex"] = bytes(ev["bytes"]).hex()
            except (ValueError, TypeError):
                continue
        events.append(ev)
    print(f"events: {len(events)}")
    dirs = Counter(e.get("dir", "?") for e in events)
    print("directions:", dict(dirs))

    unknown_fields = Counter()
    decoded = 0
    for e in events:
        try:
            b = bytes.fromhex(e.get("hex", ""))
        except ValueError:
            continue
        if not b:
            continue
        # short channel/realtime msgs carry no object envelope — only sysex does
        if e.get("dir") in ("out_short", "in_short") and b[0] < 0xF0:
            continue
        # long msgs may arrive USB-MIDI framed (32-byte packets, CIN byte first) or raw
        payload = b
        if len(payload) > 3 and payload[0] in (0x04, 0x07, 0x08, 0x09, 0x0A,
                                               0x0B, 0x0C, 0x0D, 0x0E, 0x0F):
            # naive USB-MIDI de-framing: keep low 3 bytes of each group of 4
            de = bytearray()
            for i in range(0, len(payload) - 3, 4):
                de += payload[i + 1:i + 4]
            payload = bytes(de)
        if not payload:
            continue
        try:
            obj, unk = decode_object(payload)
        except Exception as ex:
            print(f"  [!] decode fail at ts={e.get('ts')}: {ex}")
            continue
        decoded += 1
        print(f"ts={e.get('ts')} {e.get('dir'):9s} {obj['typename']:16s} class={obj['class']} "
              f"small={obj['small']} value={obj['value']} payload={obj['payload_bits']}b")
        if unk:
            for off in unk:
                unknown_fields[off] += 1
    print(f"\ndecoded objects: {decoded}")
    print("0x817 offsets seen:", dict(unknown_fields) or "none")
    print("\n[calibration targets]")
    print("  - magic 0x817: ocorrencias acima (offsets dentro do payload)")
    print("  - confirmar flags (2 bits do tipo 5) contra acoes manuais de knob/IR")
    print("  - subtags 0x6E/0x18: inspecionar payloads type-5 gravados abaixo")

if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "analysis/captures/session1.jsonl")
