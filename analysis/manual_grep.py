#!/usr/bin/env python3
"""manual_grep.py — extrai strings Tj do manual (manual_streams.txt) e
procura keywords da UI oficial; útil como referência de features."""
import re, sys

sys.stdout.reconfigure(errors="replace")
data = open("analysis/manual_streams.txt", "rb").read()
strings = re.findall(rb"\((.*?)(?<!\\)\)\s*Tj", data, re.DOTALL)
text = " ".join(s.decode("latin-1", errors="replace") for s in strings)
text = re.sub(r"\s+", " ", text)
print("texto extraído:", len(text), "chars")

keywords = [
    "Stomp Mode", "Tap Tempo", "Global EQ", "EXP Setting", "Patch BPM",
    "Effects List", "Factory Patch", "User Patch", "Release Note",
    "Info Frame", "Noise Gate", "Normal Level", "Tap Tempo Mode",
    "APP Language", "Input Level", "USB Audio", "Drum",
]
for kw in keywords:
    idxs = [m.start() for m in re.finditer(re.escape(kw), text)][:2]
    for i in idxs:
        snippet = text[max(0, i - 80) : i + 300].strip()
        print(f"--- {kw}: {snippet[:380]}")
