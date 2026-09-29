#!/usr/bin/env python3
"""Parse official algorithm.xml -> full parameter dictionary (JSON + CSV),
cross-check effectCode semantics against the patch-derived catalog."""
import json, csv, xml.etree.ElementTree as ET
from collections import defaultdict

root = ET.parse("analysis/algorithm.xml").getroot()
algs = []
for alg in root.iter("Alg"):
    code_hex = alg.get("Code").replace(" ", "")
    code = int(code_hex, 16)
    entry = {
        "name": alg.get("Name").strip(),
        "module": alg.get("Module"),
        "code": code,
        "mod_nib": f"0x{(code >> 24) & 0xFF:02X}",
        "idx": code & 0xFFFFFF,
        "xml_index": int(alg.get("Index")),
        "controls": [],
    }
    for ctrl in alg:
        tag = ctrl.tag  # Knob / Switch / Combox
        c = {"type": tag, "name": ctrl.get("Name"), "idx": ctrl.get("idx"),
             "default": ctrl.get("default")}
        if ctrl.get("Dmin") is not None:
            c["min"] = ctrl.get("Dmin")
            c["max"] = ctrl.get("Dmax")
        if ctrl.get("Step") is not None:
            c["step"] = ctrl.get("Step")
        if ctrl.get("bind") is not None:
            c["bind"] = ctrl.get("bind")
        menus = [m.get("Name") for m in ctrl.findall("Menu")]
        if menus:
            c["options"] = menus
        entry["controls"].append(c)
    algs.append(entry)

json.dump(algs, open("analysis/algorithm_dict.json", "w", encoding="utf-8"),
          ensure_ascii=False, indent=1)

with open("analysis/algorithm_dict.csv", "w", newline="", encoding="utf-8") as f:
    w = csv.writer(f)
    w.writerow(["module", "nib", "code_idx", "alg_name", "param_idx", "param_name",
                "type", "min", "max", "default", "options"])
    for a in algs:
        for c in a["controls"]:
            w.writerow([a["module"], a["mod_nib"], a["idx"], a["name"], c.get("idx"),
                        c.get("name"), c["type"], c.get("min", ""), c.get("max", ""),
                        c.get("default", ""), "|".join(c.get("options", []))])

print(f"algorithms: {len(algs)}")
mods = defaultdict(int)
for a in algs:
    mods[a["mod_nib"]] += 1
print("per nibble:", dict(sorted(mods.items())))

# cross-check with patch catalog
patch_names = set()
for r in csv.DictReader(open("analysis/effect_catalog.csv", encoding="utf-8")):
    patch_names.add((r["module"], r["name"]))
xml_names = set((a["module"], a["name"]) for a in algs)
print(f"\nin patches but not in algorithm.xml: {sorted(patch_names - xml_names)}")
print(f"in algorithm.xml but not in patches: {len(xml_names - patch_names)} (expected: newer/unused effects)")

# controls type census
tc = defaultdict(int)
for a in algs:
    for c in a["controls"]:
        tc[c["type"]] += 1
print("\ncontrol types:", dict(tc))

# sample: SnapTone & Shimmer & UK 900 entries
for a in algs:
    if a["name"] in ("SnapTone", "Shimmer", "UK 900", "Hammy", "Dual Echo"):
        print("\n", a["module"], a["name"], hex(a["code"]))
        for c in a["controls"]:
            print("   ", c)
