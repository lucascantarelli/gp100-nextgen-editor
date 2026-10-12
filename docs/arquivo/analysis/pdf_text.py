#!/usr/bin/env python3
"""Crude PDF text extraction via zlib stream decode + Tj/TJ string scraping."""
import re, zlib, glob, sys
sys.stdout.reconfigure(errors="replace")

pdfs = glob.glob("files/*.pdf")
d = open(pdfs[0], "rb").read()
print("pdf:", pdfs[0], "size:", len(d), "| encrypted:", b"/Encrypt" in d)

streams = re.findall(rb"stream\r?\n(.*?)endstream", d, re.DOTALL)
print("streams:", len(streams))

chunks = []
for s in streams:
    try:
        t = zlib.decompress(s)
    except Exception:
        continue
    if b"Tj" in t or b"TJ" in t:
        chunks.append(t)
print("text streams:", len(chunks))
full = b"\n".join(chunks)
open("analysis/manual_streams.txt", "wb").write(full)

strings = re.findall(rb"\((.*?)(?<!\\)\)\s*Tj", full, re.DOTALL)
sample = b" ".join(strings[:300])
print(sample[:1500].decode("latin-1"))
