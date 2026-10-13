#!/usr/bin/env python3
"""build_proxy.py — generate, build and verify the winmm proxy DLL (Rota 1).

Sources (human-edited):
  analysis/midi_proxy.c    proxy core (hooks + logging), has // __FORWARDERS_INCLUDE__
  analysis/forwarders.def  C source with STUB(...) lazy forwarders (NOT a linker .def)
  analysis/winmm.def       linker .def mapping export -> stub_*/hook_*  (x86 stdcall @n)

Generated:
  analysis/forwarders_impl.c   header + forward decl + STUBs (from forwarders.def)
  analysis/midi_proxy_build.c  midi_proxy.c with #include "forwarders_impl.c"

Usage:
  python analysis/build_proxy.py            # generate + cross-check + build + verify
  python analysis/build_proxy.py --no-build # generate + cross-check only
"""
import re, subprocess, sys, os, shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
A = ROOT / "analysis"
REAL_WINMM = Path(r"C:\Windows\SysWOW64\winmm.dll")  # 32-bit winmm seen by x86 process

# Known export aliases: winmm.def entry -> real export to resolve if primary missing
ALIASES = {"PlaySound": ["PlaySoundW", "PlaySoundA"]}

def parse_linker_def():
    exports = []  # (export_name, internal_name)
    for line in (A / "winmm.def").read_text(encoding="utf8").splitlines():
        line = line.strip()
        if not line or line.startswith(("LIBRARY", "EXPORTS", ";")):
            continue
        m = re.match(r"(\w+)\s*=\s*(\w+)(?:@(\d+))?$", line)
        if m:
            exports.append((m.group(1), m.group(2), m.group(3)))
        else:
            exports.append((line.split()[0], line.split()[0], None))
    return exports

def parse_c_definitions():
    text = (A / "forwarders.def").read_text(encoding="utf8")
    stubs = set(re.findall(r"STUB\(\s*\w+\s*,\s*(\w+)\s*,", text))
    stubs = {f"stub_{n}" for n in stubs}
    proxy = (A / "midi_proxy.c").read_text(encoding="utf8")
    hooks = set(re.findall(r"WINAPI\s+(hook_\w+)\s*\(", proxy))
    return stubs, hooks, proxy, text

def real_winmm_exports():
    import pefile
    pe = pefile.PE(str(REAL_WINMM), fast_load=True)
    pe.parse_data_directories(directories=[pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_EXPORT"]])
    return {e.name.decode() for e in pe.DIRECTORY_ENTRY_EXPORT.symbols if e.name}

def main():
    build = "--no-build" not in sys.argv
    exports = parse_linker_def()
    stubs, hooks, proxy, fwd_text = parse_c_definitions()
    real = real_winmm_exports()
    exe_imports = set()
    try:
        import pefile
        pe = pefile.PE(str(A / "nsis_app" / "GP-100.exe"), fast_load=True)
        pe.parse_data_directories(directories=[pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_IMPORT"]])
        for e in pe.DIRECTORY_ENTRY_IMPORT:
            if e.dll.decode().upper().startswith("WINMM"):
                exe_imports = {i.name.decode() for i in e.imports}
    except Exception as ex:
        print(f"[!] pefile no GP-100.exe: {ex}")

    print(f"winmm.def: {len(exports)} exports | stubs: {len(stubs)} | hooks: {len(hooks)}")
    print(f"winmm real (SysWOW64): {len(real)} exports | exe importa {len(exe_imports)} de winmm")

    problems = []
    for exp, impl, _ in exports:
        if impl not in stubs and impl not in hooks:
            problems.append(f"def '{exp}' -> '{impl}' SEM definicao C")
        base = re.sub(r"^(stub_|hook_)", "", impl)
        ok_real = base in real or any(a in real for a in ALIASES.get(base, []))
        if not ok_real:
            problems.append(f"'{impl}' resolve '{base}' que NAO existe no winmm real")
    for name in exe_imports:
        if name not in {e[0] for e in exports}:
            problems.append(f"exe importa '{name}' e NAO esta no winmm.def")
    missing_hooks = {"hook_midiOutShortMsg", "hook_midiOutLongMsg",
                     "hook_midiInAddBuffer", "hook_midiInOpen"} - hooks
    if missing_hooks:
        problems.append(f"hooks de captura ausentes no C: {sorted(missing_hooks)}")

    if problems:
        print("\n[!] PROBLEMAS:")
        for p in problems:
            print("   -", p)
    else:
        print("[ok] def <-> C <-> winmm real <-> imports do exe: tudo casa")

    # ---- generate forwarders_impl.c ----
    impl = (
        "#define WIN32_LEAN_AND_MEAN\n#include <windows.h>\n#include <mmsystem.h>\n\n"
        "static FARPROC resolve(const char *name);\n\n"
        + fwd_text
    )
    (A / "forwarders_impl.c").write_text(impl, encoding="utf8", newline="\n")

    # ---- generate midi_proxy_build.c ----
    marker = "// __FORWARDERS_INCLUDE__"
    assert marker in proxy, "midi_proxy.c perdeu o marcador __FORWARDERS_INCLUDE__"
    buildc = proxy.replace(marker, '#include "forwarders_impl.c"')
    (A / "midi_proxy_build.c").write_text(buildc, encoding="utf8", newline="\n")
    print("[ok] gerados forwarders_impl.c + midi_proxy_build.c")

    if not build:
        return 1 if problems else 0
    if problems:
        print("\n[!] abortando build por causa dos problemas acima")
        return 1

    # ---- build ----
    zpy = pick_python_with_ziglang()
    if not zpy:
        print("[!] nenhum python com modulo ziglang encontrado (pip install ziglang)")
        return 1
    cmd = [zpy, "-m", "ziglang", "cc", "-shared",
           "-target", "x86-windows-gnu", "-O1", "-fno-lto",
           "-o", str(A / "winmm.dll"), str(A / "midi_proxy_build.c"),
           str(A / "winmm.def"), "-lkernel32", "-Wno-format"]
    print("\n$ " + " ".join(cmd))
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    if r.stdout.strip(): print(r.stdout.strip())
    if r.returncode != 0:
        print(r.stderr.strip()[-4000:])
        print("[!] build FALHOU"); return 1
    print("[ok] build ok")

    # ---- verify exports ----
    import pefile
    pe = pefile.PE(str(A / "winmm.dll"), fast_load=True)
    pe.parse_data_directories(directories=[pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_EXPORT"]])
    got = {e.name.decode() for e in pe.DIRECTORY_ENTRY_EXPORT.symbols if e.name}
    print(f"\n[ok] winmm.dll: {len(got)} exports")
    need = exe_imports | {"midiInOpen", "midiOutOpen"}
    still = need - got
    print("[ok] todos os imports do exe cobertos" if not still else f"[!] faltando: {still}")
    return 0 if not still else 1

def pick_python_with_ziglang():
    """ziglang costuma estar no python global (venv tem so capstone/pefile)."""
    cands = [sys.executable, shutil.which("python"), shutil.which("py")]
    for c in cands:
        if not c:
            continue
        r = subprocess.run([c, "-m", "ziglang", "version"], capture_output=True, text=True)
        if r.returncode == 0:
            return c
    return None

if __name__ == "__main__":
    sys.exit(main())
