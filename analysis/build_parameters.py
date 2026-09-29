#!/usr/bin/env python3
"""Build the canonical parameters.json for GP-100 NextGen.

Sources:
  1. analysis/algorithm.xml      — official app dictionary (185 algs, ranges/defaults)
  2. analysis/effect_catalog.csv — 909 real effect slots from user patches (observed codes/values)
  3. analysis/fw_strings.txt     — firmware string table (name presence check)

Output:
  analysis/parameters.json       — canonical dictionary, validated & annotated
"""
import json, csv, re, sys
from collections import defaultdict
import xml.etree.ElementTree as ET

sys.stdout.reconfigure(errors="replace")

# ---------- 1. official dictionary ----------
root = ET.parse("analysis/algorithm.xml").getroot()
algs = []
for alg in root.iter("Alg"):
    code = int(alg.get("Code").replace(" ", ""), 16)
    a = {
        "name": alg.get("Name").strip(),
        "module": alg.get("Module"),
        "code": code,
        "nibble": (code >> 24) & 0xFF,
        "index": code & 0xFFFFFF,
        "controls": [],
    }
    for c in alg:
        entry = {"type": c.tag.lower(), "name": c.get("Name"),
                 "pos": int(c.get("idx")), "default": c.get("default")}
        if c.get("Dmin") is not None:
            entry["min"] = float(c.get("Dmin"))
            entry["max"] = float(c.get("Dmax"))
        if c.get("Step") is not None:
            entry["step"] = float(c.get("Step"))
        if c.get("bind") is not None:
            entry["bind"] = c.get("bind")
        menus = [m.get("Name") for m in c.findall("Menu")]
        if menus:
            entry["options"] = menus
            entry["option_ids"] = [int(m.get("ID")) for m in c.findall("Menu")]
        a["controls"].append(entry)
    a["controls"].sort(key=lambda c: c["pos"])
    algs.append(a)

by_code = {a["code"]: a for a in algs}

# ---------- 2. patch observations ----------
obs = defaultdict(lambda: {"count": 0, "param_samples": defaultdict(set), "slots": set()})
for r in csv.DictReader(open("analysis/effect_catalog.csv", encoding="utf-8")):
    key = int(r["code"])
    o = obs[key]
    o["count"] += 1
    o["slots"].add(r["slot_x"])
    for i in range(15):
        v = r.get(f"param{i}")
        if v not in (None, ""):
            try:
                o["param_samples"][i].add(float(v))
            except ValueError:
                pass

# ---------- 3. firmware strings ----------
fw_lines = open("analysis/fw_strings.txt", encoding="utf-8").read().splitlines()
fw_ascii = set()
for ln in fw_lines:
    if " [A] " in ln:
        fw_ascii.add(ln.split(" [A] ", 1)[1].strip())

def fw_has(name):
    n = name.strip()
    return n in fw_ascii

# ---------- merge & validate ----------
report = {"total_algs": len(algs), "codes_dup": [], "observed_codes": len(obs),
          "patch_codes_missing": [], "fw_name_hits": 0, "fw_name_miss": [],
          "param_conflicts": [], "range_notes": []}
seen = set()
for a in algs:
    if a["code"] in seen:
        report["codes_dup"].append(f"{a['name']} 0x{a['code']:08X}")
    seen.add(a["code"])
    # firmware name presence (skip names that are generic/short)
    if len(a["name"]) >= 4:
        if fw_has(a["name"]):
            report["fw_name_hits"] += 1
            a["fw_name_verified"] = True
        else:
            report["fw_name_miss"].append(a["name"])
    o = obs.get(a["code"])
    if o:
        a["observed_in_patches"] = o["count"]
        a["observed_slots"] = sorted(o["slots"])
        # range validation: observed param values must fall within declared min/max
        for c in a["controls"]:
            samples = o["param_samples"].get(c["pos"], set())
            if samples and "min" in c and "max" in c:
                out_of_range = [v for v in samples if v < c["min"] or v > c["max"]]
                if out_of_range:
                    report["param_conflicts"].append({
                        "alg": a["name"], "param": c["name"], "pos": c["pos"],
                        "declared": [c["min"], c["max"]],
                        "out_of_range": sorted(out_of_range)[:6]})

# patches referencing codes not in the dictionary
for code in obs:
    if code not in by_code:
        report["patch_codes_missing"].append(f"0x{code:08X} ({obs[code]['count']}x)")

# resolve duplicate codes: PRE/DST share Boost-family codes (module nibble 0x00
# is shared between NR and PRE and DST default). Keep both but annotate the alias.
code_groups = defaultdict(list)
for a in algs:
    code_groups[a["code"]].append(a)
for code, group in code_groups.items():
    if len(group) > 1:
        for a in group:
            a["code_shared_with"] = [f"{g['module']}:{g['name']}" for g in group if g is not a]

meta = {
    "product": "GP-100",
    "firmware_target": "2.1",
    "source_of_truth": "official Valeton Suite algorithm.xml (V1.5.1 payload), cross-validated",
    "validation": {
        "patch_slots_checked": sum(o["count"] for o in obs.values()),
        "distinct_codes_observed": report["observed_codes"],
        "patch_codes_missing": report["patch_codes_missing"],
        "param_range_conflicts": report["param_conflicts"],
        "fw_name_verified": report["fw_name_hits"],
        "fw_name_unverified": report["fw_name_miss"],
    },
    "modules": {},
}
mods = defaultdict(list)
for a in algs:
    mods[a["module"]].append(a["name"])
meta["modules"] = {m: sorted(v) for m, v in sorted(mods.items())}

json.dump({"meta": meta, "algorithms": algs},
          open("analysis/parameters.json", "w", encoding="utf-8"),
          ensure_ascii=False, indent=1)

print(json.dumps(report, ensure_ascii=False, indent=1)[:3000])
print("\nwrote analysis/parameters.json:", len(algs), "algorithms")
