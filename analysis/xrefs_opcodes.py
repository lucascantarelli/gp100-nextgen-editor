#!/usr/bin/env python3
"""XREF scan + caller disassembly to extract GP-100 protocol opcodes.
Anchors: bit-writer 0x4b48b0, crc8 0x620920, serializer 0x6209D0."""
import struct, sys, re
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

sys.stdout.reconfigure(errors="replace")
pe = pefile.PE("analysis/nsis_app/GP-100.exe", fast_load=True)
base = pe.OPTIONAL_HEADER.ImageBase
data = pe.__data__
text = [s for s in pe.sections if s.Name.decode().rstrip("\x00") == ".text"][0]
text_va = base + text.VirtualAddress
text_off = pe.get_offset_from_rva(text.VirtualAddress)
text_size = text.SizeOfRawData

def va2off(va):
    return va - text_va + text_off

def build_xrefs(targets):
    """Scan .text for E8/E9 rel32 calls/jumps to target VAs."""
    xr = {t: [] for t in targets}
    i = 0
    blob = data[text_off:text_off + text_size]
    while i < len(blob) - 5:
        if blob[i] in (0xE8, 0xE9):
            rel = struct.unpack_from("<i", blob, i + 1)[0]
            src_va = text_va + i
            tgt = src_va + 5 + rel
            if tgt in xr:
                xr[tgt].append(src_va)
        i += 1
    return xr

def disasm(va, length=0x100, maxn=60):
    out = []
    for ins in md.disasm(data[va2off(va):va2off(va) + length], va):
        out.append(ins)
        if len(out) >= maxn or ins.mnemonic == "ret":
            break
    return out

md = Cs(CS_ARCH_X86, CS_MODE_32)

xr = build_xrefs({0x4B48B0, 0x620920, 0x6209D0})
print("xrefs -> bit-writer 0x4B48B0:", len(xr[0x4B48B0]))
print("xrefs -> crc8       0x620920:", len(xr[0x620920]))
print("xrefs -> serializer 0x6209D0:", [hex(x) for x in xr[0x6209D0]])

# The crc8 caller inside 0x620920's function used call 0x633940 (get buffer).
# Focus: functions that call bit-writer MANY times = packet builders.
from collections import Counter
caller_count = Counter()
# group call sites into functions: walk back to nearest preceding 'int3 padding' heuristic
def fn_start(site_va, maxback=0x800):
    off = va2off(site_va)
    start = off
    # scan back for CC CC or C3 padding boundary
    back = 0
    while back < maxback and start > text_off:
        b = data[start - 1]
        if b in (0xCC, 0xC3) and data[start - 2] in (0xCC, 0xC3, 0x90, 0xC3):
            break
        start -= 1
        back += 1
    return text_va + (start - text_off)

funcs = Counter()
for site in xr[0x4B48B0]:
    funcs[fn_start(site)] += 1

print("\nTop functions by #bit-writer calls:")
for fn, cnt in funcs.most_common(12):
    print(f"  0x{fn:08X}: {cnt} calls")

# Disassemble the top builders and extract immediates of interest
OPS = {0x01, 0x02, 0x11, 0x12, 0x1D, 0x24, 0x40, 0x41, 0x4F, 0x92, 0x9C, 0x13}
def interesting_imms(ins_list):
    found = []
    for ins in ins_list:
        for tok in ins.op_str.replace("[", " ").replace("]", " ").split(","):
            tok = tok.strip()
            if tok.startswith("0x"):
                try:
                    v = int(tok, 16)
                except ValueError:
                    continue
                if v in OPS:
                    found.append((ins.address, ins.mnemonic + " " + ins.op_str, v))
    return found

print("\n=== opcode immediates in top builder functions ===")
for fn, cnt in funcs.most_common(8):
    ins_list = disasm(fn, length=0x400, maxn=400)
    hits = interesting_imms(ins_list)
    print(f"\n--- fn 0x{fn:08X} ({cnt} bw-calls, {len(hits)} interesting imms)")
    for addr, txt, v in hits[:24]:
        print(f"  0x{addr:08X} {txt}")
