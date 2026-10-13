#!/usr/bin/env python3
"""Extract every 8-bit field write (potential command/opcode byte) in the
protocol region: pattern = mov edx, imm8 ; ... ; push 8 ; call 0x4B48B0."""
import struct, sys
from collections import Counter, defaultdict
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

sys.stdout.reconfigure(errors="replace")
pe = pefile.PE("analysis/nsis_app/GP-100.exe")
base = pe.OPTIONAL_HEADER.ImageBase
data = pe.__data__
text = [s for s in pe.sections if s.Name.decode().rstrip("\x00") == ".text"][0]
text_va = base + text.VirtualAddress
text_off = pe.get_offset_from_rva(text.VirtualAddress)
md = Cs(CS_ARCH_X86, CS_MODE_32)
def va2off(va): return va - text_va + text_off

LO, HI = 0x61F000, 0x636000
insns = list(md.disasm(data[va2off(LO):va2off(HI)], LO))
print("insns in protocol region:", len(insns))

BITW = 0x4B48B0
BUF = 0x4B4970
results = []
for i, ins in enumerate(insns):
    if ins.mnemonic == "call":
        tgt = ins.op_str
        if tgt in (f"0x{BITW:x}", f"0x{BUF:x}"):
            # walk back for mov edx, imm within 6 instructions
            val = None
            nb = None
            for c in reversed(insns[max(0, i-6):i]):
                if c.mnemonic == "mov" and c.op_str.startswith("edx,"):
                    tok = c.op_str.split(",", 1)[1].strip()
                    try:
                        val = int(tok, 0)
                    except ValueError:
                        val = None
                    break
                if c.mnemonic == "push":
                    try:
                        nb = int(c.op_str, 0)
                    except ValueError:
                        pass
            if val is not None and val <= 0xFF:
                results.append((ins.address, val, nb, "BUF" if tgt == f"0x{BUF:x}" else "BIT"))

print(f"\n8-bit constant writes in protocol region: {len(results)}")
hist = Counter((v) for _, v, _, _ in results)
print("\nvalue histogram (0x00-0xFF):")
for v, n in sorted(hist.items(), key=lambda kv: -kv[1]):
    print(f"  0x{v:02X}: {n}x")

# group by function-ish windows (0x400 buckets)
print("\nchronological/logical clusters:")
cur_bucket = None
for addr, val, nb, kind in results:
    b = addr & ~0xFF
    print(f"  0x{addr:08X}  {kind} val=0x{val:02X} ({val})")
PYEOF