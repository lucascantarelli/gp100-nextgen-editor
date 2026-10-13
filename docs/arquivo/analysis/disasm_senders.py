#!/usr/bin/env python3
"""Trace virtual calls to MIDI send wrapper (vtable 0x1A4399C slot 3 = 0x4AD1D0)
and dump builder contexts."""
import struct, sys
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

sys.stdout.reconfigure(errors="replace")
pe = pefile.PE("analysis/nsis_app/GP-100.exe")
base = pe.OPTIONAL_HEADER.ImageBase
data = pe.__data__
text = [s for s in pe.sections if s.Name.decode().rstrip("\x00") == ".text"][0]
text_va = base + text.VirtualAddress
text_off = pe.get_offset_from_rva(text.VirtualAddress)
text_size = text.SizeOfRawData
md = Cs(CS_ARCH_X86, CS_MODE_32)
def va2off(va): return va - text_va + text_off

# constructor writes [this]=0x1A4399C at 0x4ACD70; this = ecx (0x4ACD40).
# virtual call pattern: mov eax/ecx/edx,[obj]; call dword ptr [reg+0xC]
insns = list(md.disasm(data[text_off:text_off+text_size], text_va))
print("linear sweep:", len(insns))
sites = []
# track "mov regX, dword ptr [regY]" chains: conservative - look for call [reg+0xC] preceded by
# mov reg,[reg2] and reg2 loaded from [ecx+4] (the obj+4 member holding MidiOutput)
for i, ins in enumerate(insns):
    if ins.mnemonic == "call" and ("[e" in ins.op_str and "+0xc]" in ins.op_str.replace(" ", "")):
        ctx = insns[max(0, i-14):i]
        # look for mov reg,[...] then mov reg2,[reg+4] pattern
        has_obj_load = any(("mov" == c.mnemonic and "[e" in c.op_str and "+0x4]" in c.op_str.replace(" ", "")) for c in ctx)
        if has_obj_load:
            sites.append((ins.address, ins.op_str))
print(f"candidate virtual 'call [reg+0xC]' sites: {len(sites)}")
for a, s in sites[:60]:
    print(f"  0x{a:08X} call {s}")
