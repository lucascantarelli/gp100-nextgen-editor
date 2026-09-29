#!/usr/bin/env python3
"""validate_knob_map.py — validação final do mapa knob→fio da sessão 3.

Cruza os writes 10[mod]0002 (payload nibble-expandido, 10 bytes reais)
com analysis/parameters.json (nomes/ranges de controls[pos]) e com os
.prst (match do preset por módulo→alg). Emite analysis/knob_map.json.

Layout do payload real (10 bytes):
  [0..3] effectCode BE  (= nibble<<24 | index, igual ao .prst)
  [4]    índice do controle (pos do parameters.json)
  [5]    00 (constante nas 89 amostras)
  [6..9] float32 LE do valor (unidade física = min/max do dicionário)
"""
import json, os, sys, glob, struct
import xml.etree.ElementTree as ET
from collections import defaultdict

HDR = "f021257f47502d64"
# Caminhos relativos à RAIZ do repo (independem do cwd e de máquina — CI incluído)
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
CAP = os.path.join(ROOT, "analysis", "captures", "session3.jsonl")
PARAMS = os.path.join(ROOT, "analysis", "parameters.json")
PATCHES = os.path.join(ROOT, "files", "patches")
SLOTS = {1: "PRE", 2: "DST", 3: "AMP", 4: "NR", 5: "CAB",
         6: "EQ", 7: "MOD", 8: "DLY", 9: "RVB"}

def trim(hx):
    i = hx.find("f7")
    return hx[: i + 2] if i >= 0 else hx

def load(path):
    evs = []
    for line in open(path, encoding="utf8", errors="replace"):
        s = line.strip()
        if '"hex":"' in s and s.endswith("}") and not s.endswith('"}'):
            s = s[:-1] + '"'
        if s.endswith('}"') and '"hex":"' in s:
            try:
                json.loads(s)
            except json.JSONDecodeError:
                s = s[:-2] + '"}'
        if not s.startswith("{"):
            continue
        try:
            e = json.loads(s)
        except json.JSONDecodeError:
            continue
        hx = e.get("hex", "")
        if e.get("dir") in ("out_long", "in_long") and hx:
            e["hex"] = trim(hx)
            evs.append(e)
    return evs

def body(e):
    hx = e["hex"]
    if not hx.startswith(HDR) or len(hx) < 30 or len(hx) % 2:
        return None
    b = hx[16:-2]
    if len(b) % 2:
        b = b[:-1]
    try:
        return b[:2], b[2:10], bytes.fromhex(b[10:])
    except ValueError:
        return None

def unpack(payload):
    return bytes((payload[i] << 4) | payload[i + 1] for i in range(0, len(payload), 2))

def main():
    sys.stdout.reconfigure(errors="replace")
    dic = json.load(open(PARAMS, encoding="utf8"))["algorithms"]
    by_code = {(a["nibble"], a["index"]): a for a in dic}

    evs = load(CAP)
    ts0 = evs[0]["ts"]
    edits, bad_nib, non_ack = [], 0, 0
    for e in evs:
        b = body(e)
        if not b:
            continue
        t, dr, f, a, d = e["ts"] - ts0, e["dir"], b[0], b[1], b[2]
        if dr == "in_long" and a.startswith("10") and a.endswith("0002"):
            non_ack += 1
        if dr != "out_long" or f != "12" or not a.startswith("10") or not a.endswith("0002"):
            continue
        if t <= 30000 or len(d) != 20:
            continue
        if any(x > 0x0F for x in d):
            bad_nib += 1
        real = unpack(d)
        mod = int(a[2:4], 16)
        code = int.from_bytes(real[0:4], "little")   # u32 LE: b0=index LSB, b3=nibble
        ctrl, b5 = real[4], real[5]
        val = struct.unpack("<f", real[6:10])[0]
        edits.append({"t": round(t / 1000, 2), "slot": mod, "module": SLOTS.get(mod),
                      "nibble": real[3], "index": real[0], "code": code,
                      "ctrl": ctrl, "b5": b5, "value": round(val, 4)})
    print(f"writes de edição: {len(edits)} | bytes >0x0f no payload: {bad_nib} | ACKs IN 10xx0002: {non_ack}")

    # ---- valida nibble/índice/range contra o dicionário ----
    print("\n== validação por (slot, ctrl) ==")
    groups, errs = defaultdict(list), []
    for ed in edits:
        groups[(ed["slot"], ed["ctrl"])].append(ed)
    for (slot, ctrl) in sorted(groups, key=lambda k: groups[k][0]["t"]):
        rows = groups[(slot, ctrl)]
        ed = rows[0]
        alg = by_code.get((ed["nibble"], ed["index"]))
        name = rng = "?"
        ok = "?"
        if alg is None:
            ok = "ALG-?"
            errs.append(f"{ed['module']} ctrl{ctrl}: alg não está no dicionário")
        else:
            cs = alg["controls"]
            if ctrl < len(cs):
                c = cs[ctrl]
                name = c["name"]
                lo, hi = c["min"], c["max"]
                vs = [r["value"] for r in rows]
                inr = all(lo - 1e-4 <= v <= hi + 1e-4 for v in vs)
                ok = "OK " if inr else "FORA"
                if not inr:
                    errs.append(f"{alg['name']}.{name}: valores {vs} fora de [{lo},{hi}]")
                rng = f"[{lo},{hi}]"
            else:
                ok = "CTRL-?"
                errs.append(f"{alg['name']}: ctrl {ctrl} >= len(controls)={len(cs)}")
        vals = " -> ".join(f"{r['value']}" for r in rows)
        print(f"  {ed['module']:4s} ctrl{ctrl} alg={alg['name'] if alg else '?':12s} "
              f"{name:12s} {rng:16s} {ok} | {vals}")

    # ---- match do preset editado ----
    per_mod = {ed["module"]: (ed["nibble"], ed["index"]) for ed in edits}
    print("\n== match do preset editado ==")
    for pf in glob.glob(os.path.join(PATCHES, "*.prst")):
        root = ET.parse(pf).getroot()
        for presets in root.iter("presets"):
            effs = {}
            for eff in presets.iter("Effect"):
                code = int(eff.get("effectCode", "0"))
                effs[eff.get("effectModuleName")] = ((code >> 24) & 0xFF, code & 0xFFFFFF)
            if all(effs.get(m) == per_mod[m] for m in per_mod):
                print(f"  PRESET: {os.path.basename(pf)} -> '{presets.get('ppName')}' "
                      f"(ppID={presets.get('ppID')}, bank={presets.get('ppBank')})")
                for (slot, ctrl) in sorted(groups, key=lambda k: groups[k][0]["t"]):
                    ed = groups[(slot, ctrl)][0]
                    eff = next(e for e in presets.iter("Effect") if e.get("effectModuleName") == ed["module"])
                    pv = eff.get(f"params_{ctrl}")
                    alg = by_code.get((ed["nibble"], ed["index"]))
                    cname = alg["controls"][ctrl]["name"] if alg and ctrl < len(alg["controls"]) else "?"
                    print(f"    {ed['module']:4s} ctrl{ctrl} {cname:12s} prst_params_{ctrl}={pv}  "
                          f"wire={ed['value']}")
                break

    # ---- emite knob_map.json ----
    out = {"source": "session3", "wire_write": {
        "addr": "10 [slot 1..9] 00 02", "func": "0x12",
        "payload": "nibble-expandido 20B -> 10B reais",
        "layout": "[effectCode BE 4B][ctrl u8][00][float32 LE valor físico]",
        "ack": "nenhum IN 10xx0002 observado",
    }, "slots": SLOTS, "edits": edits,
       "validation_errors": errs}
    with open(os.path.join("analysis", "knob_map.json"), "w", encoding="utf8") as fh:
        json.dump(out, fh, ensure_ascii=False, indent=2)
    print(f"\nknob_map.json salvo ({len(edits)} edits). erros: {len(errs)}")
    for e in errs:
        print("  !", e)

if __name__ == "__main__":
    main()
