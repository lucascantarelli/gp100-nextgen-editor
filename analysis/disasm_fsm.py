#!/usr/bin/env python3
"""Scan the protocol region for midiOut* call sites (Win32 import thunks) and
dump the command-builder contexts around them."""
import struct, sys
from collections import Counter
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

sys.stdout.reconfigure(errors="replace")
pe = pefile.PE("analysis/nsis_app/GP-100.exe")  # full parse for imports
base = pe.OPTIONAL_HEADER.ImageBase
data = pe.__data__
text = [s for s in pe.sections if s.Name.decode().rstrip("\x00") == ".text"][0]
text_va = base + text.VirtualAddress
text_off = pe.get_offset_from_rva(text.VirtualAddress)
text_size = text.SizeOfRawData
md = Cs(CS_ARCH_X86, CS_MODE_32)
def va2off(va): return va - text_va + text_off

# 1) locate import thunks for midiOut* / midiIn*
thunks = {}
for entry in pe.DIRECTORY_ENTRY_IMPORT:
    dll = entry.dll.decode()
    if "winmm" not in dll.lower():
        continue
    for imp in entry.imports:
        if imp.name and imp.name.decode().startswith(("midiOut", "midiIn")):
            thunks[imp.address] = imp.name.decode()
print("winmm IAT entries:")
for va, name in sorted(thunks.items()):
    print(f"  IAT 0x{va:08X} -> {name}")

# 2) scan .text for indirect calls [reg] where reg was just loaded from those IAT slots
# Simpler robust approach: find FF 15 (call dword ptr [imm32]) and FF 10/11/12... patterns plus
# mov reg, [iat]; call reg sequences — we do a full linear sweep and track 'mov r, [imm]' imm in thunks.
IAT = set(thunks.keys())
call_sites = []
cur = {}
insns = list(md.disasm(data[text_off:text_off + text_size], text_va))
print(f"linear sweep: {len(insns)} instructions")
for i, ins in enumerate(insns):
    m = ins.mnemonic
    if m == "mov" and ins.op_str.count("[") == 1 and ins.op_str.startswith(("e", "r")):
        # mov reg, dword ptr [0xXXXXXXXX]
        try:
            addr_part = ins.op_str.split("[", 1)[1].split("]", 1)[0]
            if addr_part.startswith("0x"):
                t = int(addr_part, 16)
                if t in IAT:
                    cur[ins.op_str.split(",")[0]] = (t, i)
        except Exception:
            pass
    if m == "call" and ins.op_str.split()[0] in cur:
        reg = ins.op_str.split()[0]
        t, i0 = cur[reg]
        if i - i0 < 12:
            call_sites.append((ins.address, thunks[t]))
    if m in ("jmp",) and ins.op_str.split()[0] in cur:
        cur.pop(ins.op_str.split()[0], None)

print(f"\nmidi* call sites: {len(call_sites)}")
by_fn = Counter()
for addr, name in call_sites:
    by_fn[name] += 1
print(dict(by_fn))

def fn_start(site_off, maxback=0x1000):
    start = site_off
    back = 0
    while back < maxback and start > 0:
        b = data[start - 1]
        if b in (0xCC, 0xC3) and data[start - 2] in (0xCC, 0xC3, 0x90):
            break
        start -= 1
        back += 1
    return text_va + (start - text_off)

def dump(start, length, mark_at=None):
    for ins in md.disasm(data[va2off(start):va2off(start) + length], start):
        mark = "   <<<<" if mark_at and ins.address == mark_at else ""
        print(f"0x{ins.address:08X}  {ins.mnemonic:<7} {ins.op_str}{mark}")

# 3) dump context around each midiOut call site
seen_starts = set()
for addr, name in call_sites:
    if not name.startswith("midiOut"):
        continue
    off = va2off(addr)
    fs = fn_start(off)
    if fs in seen_starts:
        continue
    seen_starts.add(fs)
    print(f"\n================ {name} call @0x{addr:08X} (fn start ~0x{fs:08X}) ================")
    dump(max(fs, addr - 0x60), 0x90, mark_at=addr)
    if len(seen_starts) >= 8:
        break
