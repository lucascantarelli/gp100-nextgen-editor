#!/usr/bin/env python3
"""manual_decode.py — decodifica o texto do manual GP-100 usando os
ToUnicode CMaps do PDF (CID → Unicode), que o pdf_text.py ignora.

Saída: analysis/manual_decoded.txt (texto corrido) + resumo no stdout.
"""
import re
import sys
import zlib
from pathlib import Path

sys.stdout.reconfigure(errors="replace")

pdfs = sorted(Path("files").glob("*.pdf"))
if not pdfs:
    sys.exit("sem PDF em files/")
pdf = pdfs[0].read_bytes()
print("pdf:", pdfs[0].name, len(pdf), "bytes | encrypted:", b"/Encrypt" in pdf)

# ── 1) CMaps ToUnicode: objeto /ToUnicode com beginbfchar/beginbfrange ──
streams = re.findall(rb"stream\r?\n(.*?)\r?\nendstream", pdf, re.DOTALL)
print("streams:", len(streams))

def try_inflate(s: bytes) -> bytes | None:
    try:
        return zlib.decompress(s)
    except Exception:
        return None

cmaps: dict[str, dict[int, str]] = {}  # fonte → mapa CID→texto
# associa CMap ao nome da fonte via objeto: /FontName + /ToUnicode ref é
# complexo; aproximação: cada CMap decodificado vira um mapa; a escolha do
# mapa certo por página fica pela heurística do maior ganho de texto.
maps: list[dict[int, str]] = []
for s in streams:
    t = try_inflate(s)
    if t is None:
        t = s  # stream sem compressão (cru)
    if b"beginbfchar" not in t and b"beginbfrange" not in t:
        continue
    m: dict[int, str] = {}
    for src, dst in re.findall(rb"beginbfchar\s*<([0-9A-Fa-f]+)>\s*<([0-9A-Fa-f]+)>\s*endbfchar", t, re.DOTALL):
        cid = int(src, 16)
        uni = bytes.fromhex(dst.decode()).decode("utf-16-be", errors="replace")
        m[cid] = uni
    for lo, hi, dst in re.findall(
        rb"beginbfrange\s*<([0-9A-Fa-f]+)>\s*<([0-9A-Fa-f]+)>\s*<([0-9A-Fa-f]+)>\s*endbfrange", t, re.DOTALL
    ):
        lo_i, hi_i = int(lo, 16), int(hi, 16)
        base = int(dst, 16)
        for i in range(lo_i, hi_i + 1):
            m[i] = chr(base + (i - lo_i))
    if m:
        maps.append(m)
print("cmaps:", len(maps), "| tamanhos:", [len(m) for m in maps][:10])

if not maps:
    sys.exit("sem CMaps ToUnicode — manual ilegível sem OCR")

# melhor mapa = maior cobertura
best = max(maps, key=len)
print("usando mapa com", len(best), "entradas")

# ── 2) decodifica TODOS os streams de texto: pares hex <..> em TJ/Tj ──
def decode_hexblob(blob: str) -> str:
    out = []
    for i in range(0, len(blob) - 3, 4):
        cid = int(blob[i : i + 4], 16)
        out.append(best.get(cid, ""))
    return "".join(out)

chunks = []
for s in streams:
    t = try_inflate(s)
    if t is None:
        t = s
    if (b"TJ" not in t and b"Tj" not in t):
        continue
    txt = t.decode("latin-1", errors="replace")
    parts = re.findall(r"<([0-9A-Fa-f]+)>", txt)
    if not parts:
        continue
    line = "".join(decode_hexblob(p) for p in parts)
    if line.strip():
        chunks.append(line)

full = "\n".join(chunks)
Path("analysis/manual_decoded.txt").write_text(full, encoding="utf-8", errors="replace")
print("texto decodificado:", len(full), "chars → analysis/manual_decoded.txt")

# ── 3) seções de interesse ──
flat = re.sub(r"\s+", " ", full)
for kw in ["GLOBAL", "Global EQ", "Footswitch", "USB", "Tap", "Noise", "Tuner",
           "Drum", "Rhythm", "Pattern", "Language", "Bluetooth", "Factory", "Version", "BPM"]:
    idx = flat.find(kw)
    print(f"\n--- {kw}: {len(re.findall(re.escape(kw), flat))} hits")
    if idx >= 0:
        print("  ", flat[max(0, idx - 60) : idx + 500][:560])
