#!/usr/bin/env python3
"""Static strings extractor for Valeton GP-100 firmware V2.1.
Extracts ASCII and UTF-16LE strings with file offsets for later correlation
with patch XML opcodes (effectCode values) and preset field names.
"""
import re
import sys
from collections import Counter

FW = "files/firmware/GP-100 Firmware V2.1.bin"
MIN_LEN = 5

data = open(FW, "rb").read()
print(f"[i] firmware size: {len(data):,} bytes")

ascii_re = re.compile(rb"[\x20-\x7e]{%d,}" % MIN_LEN)
out = []

# Heuristic entropy scan to find non-code (resource/filesystem) regions
window = 4096
step = 4096
high_ent_regions = []
import math
for off in range(0, len(data) - window, step):
    chunk = data[off:off + window]
    counts = Counter(chunk)
    ent = -sum((c / window) * math.log2(c / window) for c in counts.values())
    high_ent_regions.append((off, ent))

# Report regions that look like FAT/FS data vs executable code
zeros = data.count(0)
ffs = data.count(0xFF)
print(f"[i] 0x00 bytes: {zeros:,} ({zeros/len(data)*100:.1f}%) | 0xFF bytes: {ffs:,} ({ffs/len(data)*100:.1f}%)")

for m in ascii_re.finditer(data):
    s = m.group().decode("ascii", "replace")
    out.append((m.start(), "A", s))

utf16_re = re.compile(rb"(?:[\x20-\x7e]\x00){%d,}" % MIN_LEN)
for m in utf16_re.finditer(data):
    s = m.group().decode("utf-16-le", "replace")
    out.append((m.start(), "W", s))

out.sort(key=lambda t: t[0])
with open("analysis/fw_strings.txt", "w", encoding="utf-8") as f:
    for off, kind, s in out:
        f.write(f"0x{off:08X} [{kind}] {s}\n")

print(f"[i] total strings: {len(out):,} -> analysis/fw_strings.txt")

# Quick category census of interesting tokens
interesting = Counter()
kw = ["Effect", "effect", "Amp", "AMP", "CAB", "IR", "DLY", "RVB", "MOD", "DST",
      "PRE", "EQ", "NR ", "GATE", "Chorus", "Delay", "Reverb", "Tremolo", "Phaser",
      "Flanger", "Wah", "Comp", "Looper", "Drum", "USB", "HID", "MIDI", "Patch",
      "preset", "Preset", "Bank", "FAT", "boot", "BOOT"]
for _, _, s in out:
    for k in kw:
        if k.lower() in s.lower():
            interesting[k] += 1
print("[i] keyword census:", dict(interesting.most_common(30)))
