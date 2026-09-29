#!/usr/bin/env python3
"""find_817_imm.py — find x86 instructions in GP-100.exe that use the immediate
0x817 (push/mov/cmp), with capstone disassembly context. Locates the code that
produces/consumes the type-5 magic, revealing the owning feature.

Usage: python analysis/find_817_imm.py
"""
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

EXE = "analysis/nsis_app/GP-100.exe"
TARGET = 0x817

def main():
    sys.stdout.reconfigure(errors="replace")
    pe = pefile.PE(EXE)
    image = pe.get_memory_mapped_image(ImageBase=pe.OPTIONAL_HEADER.ImageBase)
    base = pe.OPTIONAL_HEADER.ImageBase

    md = Cs(CS_ARCH_X86, CS_MODE_32)
    md.detail = False

    # find all little-endian occurrences of the 16-bit immediate in .text
    text = None
    for s in pe.sections:
        if b".text" in s.Name:
            text = s
            break
    data = image[text.VirtualAddress:text.VirtualAddress + text.Misc_VirtualSize]
    hits = []
    i = 0
    while True:
        i = data.find(TARGET.to_bytes(2, "little"), i)
        if i < 0:
            break
        hits.append(i)
        i += 1
    print(f"ocorrencias do padrao bytes 17 81 no .text: {len(hits)}")

    shown = 0
    for off in hits:
        # disassemble backwards heuristically: start up to 24 bytes before
        start = max(0, off - 24)
        code = data[start:start + 48]
        va = base + text.VirtualAddress + start
        prev_instr_end = None
        instrs = list(md.disasm(code, va))
        # keep instructions whose immediate equals 0x817
        for ins in instrs:
            if "0x817" in ins.op_str:
                # only show a few, and require plausible mnemonics
                if ins.mnemonic in ("push", "mov", "cmp", "movzx", "or", "add") and shown < 40:
                    ctx_off = ins.address - (base + text.VirtualAddress)
                    ctx = data[ctx_off - 20: ctx_off + 20]
                    print(f"\nVA 0x{ins.address:08x}: {ins.mnemonic} {ins.op_str}")
                    print(f"   bytes ctx: {ctx.hex(' ')}")
                    shown += 1
                break
    print(f"\ntotal exibido: {shown}")

if __name__ == "__main__":
    import sys
    main()
