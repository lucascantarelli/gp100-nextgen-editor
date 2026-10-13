#!/usr/bin/env python3
"""map_params_wire.py v3 — decodifica writes de edição 10 [mod] 00 02 da sessão 3.

Modelo (sessão 3):
  - Edição de knob pela UI => OUT func 12, addr `10 [mod 01..09] 00 02`,
    payload de 20 bytes em expansão NIBBLE-POR-BYTE (como upload de IR).
  - Bytes reais (pairing even/odd): [0]=índice do alg no .prst (BE nibbles),
    [3]=id do módulo no firmware, [4]=índice do CONTROLE dentro do módulo.
  - Últimos 2 bytes reais variam => valor + flags/checksum a calibrar.

Este script extrai os writes, agrupa por (módulo, controle) e imprime a
evolução temporal; além de identificar o preset editado pelos alg indices.
"""
import json, os, sys, glob
import xml.etree.ElementTree as ET
from collections import defaultdict, Counter

HDR = "f021257f47502d64"
CAP = os.path.join("analysis", "captures", "session3.jsonl")
PATCHES = r"D:\GP-100 app\files\patches"

SLOTS = {1: "PRE", 2: "DST", 3: "AMP", 4: "NR", 5: "CAB", 6: "EQ",
         7: "MOD", 8: "DLY", 9: "RVB"}

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

def unpack_nibbles(payload):
    """20 bytes expandidos -> 10 bytes reais (hi nibble = byte par)."""
    assert len(payload) % 2 == 0
    return bytes((payload[i] << 4) | payload[i + 1] for i in range(0, len(payload), 2))

def parse_prst(path):
    """retorna lista de presets: dict com slot -> (alg_idx, [params])"""
    root = ET.parse(path).getroot()
    out = []
    for presets in root.iter("presets"):
        info = {"name": presets.get("ppName", "?"), "ppID": presets.get("ppID"),
                "bank": presets.get("ppBank"), "slots": {}}
        for eff in presets.iter("Effect"):
            x = int(eff.get("x", "0"))
            code = int(eff.get("effectCode", "0"))
            idx = code & 0xFFFFFF
            ps = []
            i = 0
            while eff.get(f"params_{i}") is not None:
                try:
                    ps.append(int(eff.get(f"params_{i}")))
                except ValueError:
                    ps.append(None)
                i += 1
            info["slots"][x] = {"alg": idx, "name": eff.get("effectName"),
                                "module": eff.get("effectModuleName"), "params": ps}
        out.append(info)
    return out

def main():
    sys.stdout.reconfigure(errors="replace")
    evs = load(CAP)
    ts0 = evs[0]["ts"]
    edits = []
    for e in evs:
        b = body(e)
        if not b:
            continue
        t, dr, f, a, d = e["ts"] - ts0, e["dir"], b[0], b[1], b[2]
        if dr == "out_long" and f == "12" and a.startswith("10") and a.endswith("0002") and t > 30000 and len(d) == 20:
            mod = int(a[2:4], 16)
            real = unpack_nibbles(d)
            edits.append((t, mod, real, d))

    print(f"writes de edição: {len(edits)}")
    groups = defaultdict(list)
    for t, mod, real, d in edits:
        groups[(mod, real[4])].append((t, real))

    print("\n== grupos (módulo, controle) em ordem temporal ==")
    for (mod, ctrl) in sorted(groups, key=lambda k: groups[k][0][0]):
        rows = groups[(mod, ctrl)]
        alg = rows[0][1][0]
        t0, t1 = rows[0][0], rows[-1][0]
        print(f"\n-- mod {mod} ({SLOTS.get(mod,'?')}), ctrl {ctrl}, alg 0x{alg:02x} ({alg}), "
              f"{len(rows)} writes, {t0/1000:.1f}s..{t1/1000:.1f}s")
        for t, real in rows:
            print(f"   +{t/1000:7.2f}s  real={real.hex(' ')}  "
                  f"[b5..9]={real[5]:02x} {real[6]:02x} {real[7]:02x} {real[8]:02x} {real[9]:02x}")

    # ---- identificar o preset editado pelos índices de alg ----
    per_mod = {}
    for (mod, ctrl), rows in groups.items():
        per_mod.setdefault(mod, rows[0][1][0])
    print("\n== alg indices observados ==")
    for mod in sorted(per_mod):
        print(f"  {SLOTS.get(mod)}: alg {per_mod[mod]} (0x{per_mod[mod]:02x})")

    for pf in glob.glob(os.path.join(PATCHES, "*.prst")):
        presets = parse_prst(pf)
        for info in presets:
            ok, miss = True, []
            for mod, alg in per_mod.items():
                s = info["slots"].get(mod)
                if s is None or s["alg"] != alg:
                    ok = False
                    break
            if ok:
                print(f"\n== PRESET CASADO: {pf} | {info['name']} (ppID={info['ppID']} bank={info['bank']}) ==")
                for mod in sorted(per_mod):
                    s = info["slots"][mod]
                    ctrl_rows = [(c, rows) for (m, c), rows in groups.items() if m == mod]
                    for c, rows in ctrl_rows:
                        init = s["params"][c] if c < len(s["params"]) else None
                        first8 = rows[0][1][8]
                        print(f"   {SLOTS[mod]:4s} ctrl {c} ({s['name'][:14]:14s}) "
                              f"prst_params[{c}]={init}  first_b8=0x{first8:02x} ({first8}) "
                              f"b8/2={first8/2:.1f} b8hi={first8>>4} b8lo={first8&0xf}")
                return

if __name__ == "__main__":
    main()
