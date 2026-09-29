#!/usr/bin/env python3
"""MSVC RTTI vtable locator + capstone disassembler for GP-100.exe protocol classes.

x86 (32-bit) layouts:
  TypeDescriptor: [vfptr(4)][spare(4)][name...]  -> name at TD+8
  CompleteObjectLocator: [sig=0][offset][cdOffset][pTypeDescriptor VA][pClassDescriptor VA]
  vftable preceded by pointer to COL at (vftable - 4)
"""
import struct, sys
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

sys.stdout.reconfigure(errors="replace")

EXE = "analysis/nsis_app/GP-100.exe"
pe = pefile.PE(EXE, fast_load=True)
image_base = pe.OPTIONAL_HEADER.ImageBase
data = pe.__data__

def rva2off(rva):
    return pe.get_offset_from_rva(rva)

def va2off(va):
    return pe.get_offset_from_rva(va - image_base)

def cstr_at(off, maxn=120):
    end = data.find(b"\x00", off, off + maxn)
    return data[off:end].decode("utf-8", "replace") if end != -1 else ""

# ---- find TypeDescriptors by class name substring ----
tds = {}  # name -> (td_va, td_off)
sec_data = [(s, s.get_data()) for s in pe.sections if s.Name.decode().rstrip("\x00") == ".data"]
# scan .data and .rdata for "..?AV" names
for s in pe.sections:
    sname = s.Name.decode().rstrip("\x00")
    if sname not in (".data", ".rdata"):
        continue
    blob = s.get_data()
    base_rva = s.VirtualAddress
    start = 0
    while True:
        i = blob.find(b".?AV", start)
        if i < 0:
            break
        end = blob.find(b"\x00", i, i + 160)
        if end == -1:
            start = i + 1
            continue
        name = blob[i:end].decode("utf-8", "replace")
        # TypeDescriptor starts at name_off - 8
        td_off = i - 8
        if td_off >= 0:
            td_va = image_base + base_rva + td_off
            tds[name] = (td_va, td_off)
        start = end
    # end while

def find_td(sub):
    return {k: v for k, v in tds.items() if sub in k}

# ---- locate vtables via CompleteObjectLocator ----
def vtables_for_td(td_va, td_off):
    """Find COLs pointing to this TD, then vftables pointing to those COLs."""
    results = []
    pat = struct.pack("<I", td_va)
    for s in pe.sections:
        sname = s.Name.decode().rstrip("\x00")
        if sname not in (".data", ".rdata"):
            continue
        blob = s.get_data()
        p = 0
        while True:
            i = blob.find(pat, p)
            if i < 0:
                break
            col_off = i - 12
            if col_off >= 0:
                sig, off_, cd, tdptr, cdptr = struct.unpack_from("<IIIII", blob, col_off)
                if sig == 0 and tdptr == td_va:
                    col_va = image_base + s.VirtualAddress + col_off
                    # find vftable: pointer to col_va somewhere (usually rdata, at vtable-4)
                    colpat = struct.pack("<I", col_va)
                    for s2 in pe.sections:
                        blob2 = s2.get_data()
                        j = blob2.find(colpat)
                        while j != -1:
                            vft_rva = s2.VirtualAddress + j + 4
                            results.append((col_va, image_base + vft_rva, vft_rva))
                            j = blob2.find(colpat, j + 1)
            p = i + 1
    return results

def read_vtable(vft_rva, max_entries=40):
    off = rva2off(vft_rva)
    entries = []
    for k in range(max_entries):
        fn = struct.unpack_from("<I", data, off + 4 * k)[0]
        if fn < image_base or fn > image_base + 0x2000000:
            break
        entries.append(fn)
    return entries

def disasm_fn(va, max_bytes=400):
    off = va2off(va)
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    md.detail = False
    out = []
    for ins in md.disasm(data[off:off + max_bytes], va):
        out.append(f"0x{ins.address:08X}  {ins.mnemonic:<8} {ins.op_str}")
        if ins.mnemonic == "ret":
            break
        if len(out) > 120:
            break
    return out

if __name__ == "__main__":
    print(f"image base 0x{image_base:08X}")
    print(f"TypeDescriptors found: {len(tds)}")
    targets = ["MIDIUSBFsm", "MIDIMSGProcess", "MIDIManager", "RequestPreset",
               "RequstPresetPerPackage", "UploadOnePreset", "UpdateAllPreset"]
    for t in targets:
        found = find_td(t)
        for name, (td_va, td_off) in found.items():
            vts = vtables_for_td(td_va, td_off)
            print(f"\n=== {name} TD@0x{td_va:08X} -> {len(vts)} vtable(s)")
            for col_va, vft_va, vft_rva in vts[:4]:
                entries = read_vtable(vft_rva, 30)
                print(f"  vftable RVA 0x{vft_rva:06X} VA 0x{vft_va:08X}: {len(entries)} entries")
                for e in entries[:30]:
                    print(f"    0x{e:08X}")
