#!/usr/bin/env python3
"""Deep disassembly pass: find code referencing the CRC-8/0x07 table and the
FSM vtable methods; extract protocol opcode constants from the disassembly."""
import struct, sys
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

sys.stdout.reconfigure(errors="replace")
EXE = "analysis/nsis_app/GP-100.exe"
pe = pefile.PE(EXE, fast_load=True)
base = pe.OPTIONAL_HEADER.ImageBase
data = pe.__data__

def va2off(va):
    return pe.get_offset_from_rva(va - base)

def off2va(off):
    # find section containing file offset
    for s in pe.sections:
        fo = pe.get_offset_from_rva(s.VirtualAddress)
        size = s.SizeOfRawData
        if fo <= off < fo + size:
            return base + s.VirtualAddress + (off - fo)
    return None

text = [s for s in pe.sections if s.Name.decode().rstrip("\x00") == ".text"][0]
text_va = base + text.VirtualAddress
text_off = pe.get_offset_from_rva(text.VirtualAddress)
text_size = text.SizeOfRawData

# ---- CRC-8 table VA (found earlier at file 0x1620b40) ----
crc_off = 0x1620B40
crc_va = off2va(crc_off)
print(f"CRC table: file 0x{crc_off:X} -> VA 0x{crc_va:08X}")

# find references: scan .text for the VA as imm32
pat = struct.pack("<I", crc_va)
refs = []
p = text_off
while True:
    i = data.find(pat, p, text_off + text_size)
    if i < 0:
        break
    refs.append(i)
    p = i + 1
print(f".text references to CRC table: {len(refs)}")
for r in refs:
    print(f"  .text file 0x{r:X} VA 0x{text_va + (r - text_off):08X}")

md = Cs(CS_ARCH_X86, CS_MODE_32)

def disasm_fn_containing(off, before=0x60, length=0x220, label=""):
    """Walk back to probable function start (int3 padding / prologue) and disassemble."""
    start = off - before
    # align to C3/CC boundary for cleanliness (best effort)
    va = text_va + (start - text_off)
    out = []
    n_bytes = 0
    for ins in md.disasm(data[start:start + length], va):
        out.append(f"  0x{ins.address:08X}  {ins.mnemonic:<7} {ins.op_str}")
        n_bytes += ins.size
        if n_bytes > length - 16:
            break
        if len(out) > 160:
            break
    return out

def scan_consts(out_lines):
    """Extract interesting immediates: opcodes/ranges."""
    hits = []
    import re
    for ln in out_lines:
        m = re.findall(r",\s*(0x[0-9a-f]{1,2}|1[0-9]{1,2})\b", ln)
        for x in m:
            v = int(x, 16) if x.startswith("0x") else int(x)
            if v in (0x11, 0x12, 0x1D, 0x24, 0x40, 0x41, 0x4F, 0x92, 0x9C, 0x02, 0x01, 0x13):
                hits.append((ln.strip(), v))
    return hits

print("\n=== Disassembly around CRC-table references ===")
for r in refs[:4]:
    va = text_va + (r - text_off)
    print(f"\n--- ref VA 0x{va:08X}")
    lines = disasm_fn_containing(r, before=0x40, length=0x180)
    for ln in lines:
        print(ln)

print("\n=== MIDIUSBFsm vtable unique .text methods ===")
vft_rva = 0x163B290
voff = pe.get_offset_from_rva(vft_rva)
seen = set()
for k in range(30):
    fn = struct.unpack_from("<I", data, voff + 4 * k)[0]
    if not (text_va <= fn < text_va + text_size):
        continue
    if fn in seen:
        continue
    seen.add(fn)
    fo = va2off(fn)
    print(f"\n--- vtable[{k}] VA 0x{fn:08X}")
    for ins in list(md.disasm(data[fo:fo + 0x80], fn))[:24]:
        print(f"  0x{ins.address:08X}  {ins.mnemonic:<7} {ins.op_str}")
