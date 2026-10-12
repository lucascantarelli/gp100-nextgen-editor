#!/usr/bin/env python3
"""manual_tuner.py — extrai do manual (manual_streams.txt) os trechos legíveis
sobre TUNER/afinação, com contexto, para o inventário do afinador da UI.

O PDF mistura strings literais (parênteses) com CIDs (<hex>); juntamos as
literais em ordem de stream (a leitura por palavra-chave é fragmentada, mas
suficiente p/ achar os trechos) e mostramos janelas de contexto.
"""
import re
import sys

sys.stdout.reconfigure(errors="replace")
data = open("analysis/manual_streams.txt", "rb").read()

lits = [(m.start(), m.group(1).decode("latin-1", errors="replace"))
        for m in re.finditer(rb"\((.*?)(?<!\\)\)", data, re.DOTALL)]
text = " ".join(lit for _, lit in lits)
text = re.sub(r"\s+", " ", text)
print(f"literais: {len(lits)} | texto: {len(text)} chars")

KEYWORDS = [
    "Tuner", "tuner", "TUNER", "Tune", "Cent", "cent", "CALIB", "Calib",
    "A4", "440", "435", "445", "Reference", "Flat", "Sharp", "In Tune",
    "Mute", "Bypass", "Chromatic", "Guitar", "Bass", "Pitch", "Hz",
]

for kw in KEYWORDS:
    hits = [m.start() for m in re.finditer(re.escape(kw), text)]
    if not hits:
        continue
    print(f"\n=== {kw}: {len(hits)} ocorrências")
    # agrupa janelas de 320 chars, sem repetir sobreposição
    shown = -10**9
    for i in hits:
        if i - shown < 320:
            continue
        print(f"  …{text[max(0, i-160):i+160]}…")
        shown = i
