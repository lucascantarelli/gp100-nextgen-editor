#!/usr/bin/env python3
"""manual_settings.py — extrai do manual (manual_streams.txt) os trechos
legíveis sobre SETTINGS/parâmetros globais, para o inventário da UI.

O PDF mistura strings literais (parênteses) com CIDs (<hex>); aqui juntamos
SÓ as literais e mostramos o contexto ao redor de cada keyword de settings.
"""
import re
import sys

sys.stdout.reconfigure(errors="replace")
data = open("analysis/manual_streams.txt", "rb").read()

# 1) todas as strings literais ( ... ) Tj/TJ, em ordem, com offsets
lits = [(m.start(), m.group(1).decode("latin-1", errors="replace")) for m in re.finditer(rb"\((.*?)(?<!\\)\)", data, re.DOTALL)]
text = " ".join(lit for _, lit in lits)
text = re.sub(r"\s+", " ", text)
print(f"literais: {len(lits)} | texto: {len(text)} chars")

KEYWORDS = [
    "Global EQ", "Footswitch Mode", "USB Audio", "L- CUT", "L-CUT", "H- CUT", "H-CUT",
    "Noise Gate", "Noise Mode", "Tap Tempo", "Input Level", "Normal Level",
    "Hint Mode", "APP Language", "Language", "Drum", "Tuner", "Bluetooth",
    "Master VOL", "Stomp Mode", "Factory Reset", "Version", "About",
    "Release Note", "Info Frame", "Phrase Loop", "Sample Rate", "Backup",
    "Restore", "Screen", "Contrast", "Battery", "Firmware", "Update",
]

seen = set()
for kw in KEYWORDS:
    hits = [m.start() for m in re.finditer(re.escape(kw), text)]
    if not hits:
        print(f"\n=== {kw}: (0 ocorrências)")
        continue
    # agrupa ocorrências próximas (janela de 400 chars)
    windows = []
    for i in hits:
        if windows and i - windows[-1][0] < 400:
            windows[-1] = (i, windows[-1][1] + 400)
        else:
            windows.append((i, i + 400))
    print(f"\n=== {kw}: {len(hits)} hits, {len(windows)} janelas")
    for start, end in windows[:4]:
        chunk = text[start:end].strip()
        key = chunk[:80]
        if key in seen:
            continue
        seen.add(key)
        print("  …", chunk[:380])
