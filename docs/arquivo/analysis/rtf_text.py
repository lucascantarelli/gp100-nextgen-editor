#!/usr/bin/env python3
"""Extract plain text from the GP-100 firmware release note RTF.
Uses balanced-brace group skipping to remove embedded pictures/shapes."""
import sys

src = open("files/firmware/GP-100 Firmware Release Note.rtf", encoding="utf-8", errors="replace").read()

def find_group_end(s, start):
    """start = index of '{'; return index after matching '}'."""
    depth = 0
    i = start
    n = len(s)
    while i < n:
        c = s[i]
        if c == "\\":
            i += 2
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return n

# Remove picture/shape groups by scanning top-level-ish group starts
markers = ("{\\pict", "{\\*\\shppict", "{\\nonshppict", "{\\object", "{\\*\\datastore")
out = src
while True:
    low = out
    hit = None
    for m in markers:
        idx = low.find(m)
        if idx != -1 and (hit is None or idx < hit[1]):
            hit = (m, idx)
    if hit is None:
        break
    _, idx = hit
    end = find_group_end(out, idx)
    out = out[:idx] + " " + out[end:]

# Decode escapes
out = out.replace("\\'8d", "-")  # fixup later if needed
out = out.replace("\\'9d", "-")
import re
out = re.sub(r"\\u(-?\d+)\s?\??", lambda m: chr(int(m.group(1)) % 65536), out)
out = re.sub(r"\\'([0-9a-fA-F]{2})", lambda m: chr(int(m.group(1), 16)), out)
out = re.sub(r"\\par[d]?\b", "\n", out)
out = re.sub(r"\\line\b", "\n", out)
out = re.sub(r"\\tab\b", "\t", out)
out = re.sub(r"\\[a-zA-Z]+-?\d*\s?", "", out)
out = out.replace("{", "").replace("}", "")

lines = [l.strip() for l in out.splitlines()]
lines = [l for l in lines if l]
text = "\n".join(lines)
open("analysis/release_note.txt", "w", encoding="utf-8").write(text)
sys.stdout.reconfigure(errors="replace")
print(text[:8000])
