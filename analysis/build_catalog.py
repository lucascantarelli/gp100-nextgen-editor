#!/usr/bin/env python3
"""Build the effect catalog from all .prst XML patches and correlate
effectCode values with firmware string offsets where possible."""
import glob, csv, re
from collections import defaultdict
import xml.etree.ElementTree as ET

rows = []
for path in glob.glob("files/patches/*.prst"):
    try:
        root = ET.parse(path).getroot()
    except Exception as e:
        print(f"[!] {path}: {e}")
        continue
    for preset in root.iter("presets"):
        pname = preset.get("ppName")
        for eff in preset.findall("Effect"):
            rows.append({
                "file": path.split("/")[-1],
                "preset": pname,
                "slot_x": eff.get("x"),
                "module": eff.get("effectModuleName"),
                "name": (eff.get("effectName") or "").strip(),
                "code": int(eff.get("effectCode")),
                "state": eff.get("effectState"),
                "param0": eff.get("params_0"),
            })

# Decode effectCode semantics
for r in rows:
    c = r["code"]
    r["mod_nib"] = (c >> 24) & 0xFF
    r["mod_hex"] = f"0x{(c >> 24) & 0xFF:02X}"
    r["idx"] = c & 0xFFFFFF

by_mod = defaultdict(set)
for r in rows:
    by_mod[(r["module"], r["mod_hex"])].add((r["name"], r["idx"]))

with open("analysis/effect_catalog.csv", "w", newline="", encoding="utf-8") as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
    w.writeheader()
    w.writerows(rows)

print(f"total effect slots parsed: {len(rows)}")
print(f"distinct effects: {len(set((r['module'], r['name'], r['code']) for r in rows))}")
print()
for (mod, mh), names in sorted(by_mod.items()):
    print(f"module {mod:4s} (nibble {mh}): {len(names):3d} distinct effects")

# distinct modules and slot order
mods = sorted(set(r["module"] for r in rows))
print("\nmodules present:", mods)
xs = defaultdict(list)
for r in rows:
    xs[r["module"]].append(r["slot_x"])
for m in mods:
    print(f"  {m}: slots x={sorted(set(xs[m]))}")
