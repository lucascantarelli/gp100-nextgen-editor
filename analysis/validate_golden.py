#!/usr/bin/env python3
"""validate_golden.py — valida docs/protocol_golden.json contra as capturas.

Três provas:
  A. ACCOUNTING — toda mensagem das capturas deve ser explicada por um template
     (func/addr + comprimento + segmentos const), byte a byte.
  B. GERAÇÃO/KNOB — regenerar os writes 10xx0002 puramente da SEMÂNTICA
     (slot, effectCode, ctrl, float) sem olhar o fio, e comparar byte-a-byte
     com a captura (S3 = 89 edits; S1 = 3 edits documentados no PROTOCOL.md).
  C. GERAÇÃO/BOOT — regenerar os requests de boot/scan de S1 (t<30s) a partir
     dos templates + regras do §13.10 (inventário de pp vindo da captura,
     todo o resto gerado), e comparar byte a byte por grupo.
"""
import json, os, sys, struct, importlib.util
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("build_golden", os.path.join(HERE, "build_golden.py"))
BG = importlib.util.module_from_spec(spec)
spec.loader.exec_module(BG)

HDR = BG.HDR
CAPS = {f"S{i}": os.path.join(HERE, "captures", f"session{i}.jsonl") for i in (1, 2, 3, 4)}
sys.stdout.reconfigure(errors="replace")

# ---------------------------------------------------------------- patterns
def match_pattern(pat, data):
    """True se data satisfaz integralmente o padrão (len + segmentos const)."""
    k = pat.get("kind")
    if k == "empty":
        return len(data) == 0
    if k == "by-len":
        sub = pat["by_len"].get(str(len(data)))
        return sub is not None and match_pattern(sub, data)
    if k == "variable-len":
        return False
    if len(data) != pat.get("len"):
        return False
    pos = 0
    for seg in pat.get("segments", []):
        n = (len(seg["hex"]) // 2) if seg["kind"] == "const" else seg["count"]
        if seg["kind"] == "const" and data[pos:pos + n] != bytes.fromhex(seg["hex"]):
            return False
        pos += n
    return pos == len(data)

def load_golden():
    doc = json.load(open(os.path.join(HERE, "..", "docs", "protocol_golden.json"), encoding="utf8"))
    out_idx, in_idx = defaultdict(list), defaultdict(list)
    for t in doc["transactions"]:
        if t["type"] in ("write", "req"):
            out_idx[(t["out"]["func"], t["out"]["addr"])].append(t)
        if t["type"] in ("push", "req"):
            in_idx[(t["in"]["func"], t["in"]["addr"])].append(t)
    return doc, out_idx, in_idx

# ---------------------------------------------------------------- A. accounting
def accounting(name, evs, out_idx, in_idx, tmax=None):
    ts0 = evs[0]["ts"]
    ok_out = ok_in = bad_out = bad_in = 0
    leftover = Counter()
    for e in evs:
        t = e["ts"] - ts0
        if tmax is not None and t > tmax:
            continue
        b = BG.body(e)
        if not b:
            bad_out += 1
            leftover[("SEM-HDR", e["dir"], e["hex"][:24])] += 1
            continue
        dr, f, a, d = e["dir"], b[0], b[1], b[2]
        if dr == "out_long":
            cands = out_idx.get((f, a), [])
            if cands and any(match_pattern(t["request_payload"], d) for t in cands):
                ok_out += 1
            else:
                bad_out += 1
                leftover[("OUT", f, a, len(d), d.hex()[:32])] += 1
        else:
            cands = in_idx.get((f, a), [])
            pats = [t["response_payload"] for t in cands]
            if cands and any(match_pattern(p, d) for p in pats):
                ok_in += 1
            else:
                bad_in += 1
                leftover[("IN", f, a, len(d), d.hex()[:32])] += 1
    tot = ok_out + bad_out
    toti = ok_in + bad_in
    print(f"  [{name}] OUT {ok_out}/{tot} ({100*ok_out/tot:.2f}%)  "
          f"IN {ok_in}/{toti} ({100*ok_in/toti:.2f}%)")
    return ok_out, bad_out, ok_in, bad_in, leftover

# ---------------------------------------------------------------- B. knobs
def nibble_expand(real):
    out = bytearray()
    for b in real:
        out.append((b >> 4) & 0x0F)
        out.append(b & 0x0F)
    return bytes(out)

def gen_knob_sysex(slot, nibble, index, ctrl, value):
    code_le = [(index >> k) & 0xFF for k in (0, 8, 16)] + [nibble]
    real = bytes(code_le + [ctrl, 0]) + struct.pack("<f", value)
    payload = nibble_expand(real)
    sysex = HDR + "12" + f"10{slot:02x}0002" + payload.hex() + "f7"
    return sysex

def knob_flow():
    print("\n== B. GERAÇÃO de writes de knob (semântica -> fio) ==")
    # --- S3: 89 edits do knob_map.json ---
    km = json.load(open(os.path.join(HERE, "knob_map.json"), encoding="utf8"))["edits"]
    evs = BG.load(CAPS["S3"])
    ts0 = evs[0]["ts"]
    wire = []
    for e in evs:
        b = BG.body(e)
        if b and e["dir"] == "out_long" and b[0] == "12" and b[1].startswith("10") and \
           b[1].endswith("0002") and (e["ts"] - ts0) > 30000:
            wire.append(HDR + "12" + b[1] + b[2].hex() + "f7")
    gen = [gen_knob_sysex(ed["slot"], ed["nibble"], ed["index"], ed["ctrl"], ed["value"]) for ed in km]
    exact = sum(1 for g, w in zip(gen, wire) if g == w)
    print(f"  S3 (knob_map): {exact}/{len(wire)} byte-a-byte  "
          f"(gerados {len(gen)}, capturados {len(wire)})")
    for i, (g, w) in enumerate(zip(gen, wire)):
        if g != w:
            print(f"    DIF i={i} ed={km[i]}")
            print(f"      gen: {g}")
            print(f"      cap: {w}")
            break
    # --- S1: 3 edits documentados no §13.4/13.11 (C-Wah Range 3.0/27.0/47.0) ---
    evs1 = BG.load(CAPS["S1"])
    ts01 = evs1[0]["ts"]
    wire1 = []
    for e in evs1:
        b = BG.body(e)
        if b and e["dir"] == "out_long" and b[0] == "12" and b[1] == "10010002" and \
           30000 < (e["ts"] - ts01) < 50000:
            wire1.append(HDR + "12" + b[1] + b[2].hex() + "f7")
    gen1 = [gen_knob_sysex(1, 0x05, 0x08, 0, v) for v in (3.0, 27.0, 47.0)]
    exact1 = sum(1 for g, w in zip(gen1, wire1) if g == w)
    print(f"  S1 (doc §13.4: C-Wah Range 3/27/47): {exact1}/{len(wire1)} byte-a-byte")
    for g, w in zip(gen1, wire1):
        if g != w:
            print(f"      gen: {g}\n      cap: {w}")
            break
    return exact, len(wire), exact1, len(wire1)

# ---------------------------------------------------------------- D. save
def _prst_meta(path, pp_id=None):
    import xml.etree.ElementTree as ET
    root = ET.parse(path).getroot()
    p = next(root.iter("presets"))
    name = p.get("ppName", "")
    pp = int(p.get("ppID", "0")) if pp_id is None else pp_id
    ptype = int(p.get("ppType", "0"))
    return pp, ptype, name

def _meta_block(pp, ptype, name):
    """5 writes do bloco de metadados (regras do §13.12 + achados pp/ppType)."""
    nb = name.encode("ascii")[:12]
    m = {
        ("12", "11000000"): [bytes(4) + pp.to_bytes(2, "big") + bytes(2) + nb.ljust(12, b"\0")],
        ("12", "11000004"): [bytes(20)],
        ("12", "11000005"): [ptype.to_bytes(2, "big") + bytes(2)],
        ("12", "11000007"): [bytes(50)],
        ("12", "12000002"): [bytes(4) + pp.to_bytes(2, "big") + bytes(2)],
    }
    return m

def _ops(ops):
    return {
        ("12", "00020000"): [bytes(4) + op.to_bytes(2, "big") + bytes(2) for op in ops],
    }

def _collect_out(evs, ts0, t0, t1):
    cap = defaultdict(list)
    for e in evs:
        t = e["ts"] - ts0
        if not (t0 <= t <= t1):
            continue
        b = BG.body(e)
        if b and e["dir"] == "out_long":
            cap[(b[0], b[1])].append(b[2])
    return cap

def _cmp_groups(title, gen, cap):
    ok = tot = 0
    print(f"  -- {title}")
    for k in sorted(set(gen) | set(cap)):
        g, c = gen.get(k, []), cap.get(k, [])
        inter = sum((Counter(g) & Counter(c)).values())
        ok += inter
        tot += len(c)
        mark = "" if len(g) == len(c) == inter else f"   ← cap {len(c)} vs gen {len(g)}, casam {inter}"
        print(f"     {k[0]}/{k[1]}: {inter}/{len(c)} byte-a-byte{mark}")
        if inter != len(c) or len(g) != len(c):
            miss_c = [x.hex(" ") for x in list((Counter(c) - Counter(g)).elements())[:2]]
            miss_g = [x.hex(" ") for x in list((Counter(g) - Counter(c)).elements())[:2]]
            if miss_c: print(f"        captura sem geração: {miss_c}")
            if miss_g: print(f"        geração sem captura: {miss_g}")
    return ok, tot

def save_flow():
    print("\n== D. GERAÇÃO do fluxo de save (metadados a partir do .prst + regras) ==")
    ok = tot = 0
    # --- S4: 'It's GP100' (ppID 0, ppType 4) via all.prst; janela 1660-1680s ---
    evs4 = BG.load(CAPS["S4"])
    ts04 = evs4[0]["ts"]
    pp, ptype, name = _prst_meta(os.path.join(HERE, "..", "files", "patches", "all.prst"), pp_id=0)
    gen = defaultdict(list)
    for k, v in _meta_block(pp, ptype, name).items():
        gen[k] += v
    for k, v in _ops([0, 0, 1, 1]).items():   # op0 ×2 (sai) + op1 ×2 (reentra)
        gen[k] += v
    cap = _collect_out(evs4, ts04, 1660000, 1670000)
    a, b = _cmp_groups(f"S4 save @1663s (pp={pp}, type={ptype}, '{name}')", gen, cap)
    ok += a; tot += b
    # resync IN: 31 pushes 11000008 (região usuário: 010E,010F,02xx,0300-030C) + status
    push_keys = [0x010E, 0x010F] + [0x0200 + i for i in range(16)] + [0x0300 + i for i in range(13)]
    cap_in = [BG.body(e)[2] for e in evs4
              if (e["ts"] - ts04) / 1000 > 1670 and BG.body(e) and BG.body(e)[1] == "11000008"]
    gen_resync = [k.to_bytes(2, "big") + bytes(2) + bytes(10) for k in push_keys]
    resync_ok = sum((Counter(gen_resync) & Counter(cap_in)).values())
    print(f"     resync 11000008 (IN): {resync_ok}/{len(cap_in)} (regra: região usuário 31 regs) + "
          f"12000001: {'ok' if any(BG.body(e) and BG.body(e)[1]=='12000001' and (e['ts']-ts04)/1000>1670 for e in evs4) else 'FALTA'}")
    ok += resync_ok; tot += len(cap_in)

    # --- S2: 'Blink OD' (ppID 1, ppType 6) via arquivo próprio; janela 145-155s ---
    evs2 = BG.load(CAPS["S2"])
    ts02 = evs2[0]["ts"]
    pp2, ptype2, name2 = _prst_meta(os.path.join(HERE, "..", "files", "patches", "Blink OD.prst"))
    gen2 = defaultdict(list)
    for k, v in _meta_block(pp2, ptype2, name2).items():
        gen2[k] += v
    cap2 = _collect_out(evs2, ts02, 145000, 155000)
    a, b = _cmp_groups(f"S2 save @149.6s (pp={pp2}, type={ptype2}, '{name2}')", gen2, cap2)
    ok += a; tot += b
    # resync IN S2: 32 pushes 12001002 4B [01][02][idx][01], idx 0x08..0x26, último dup
    cap_in2 = [BG.body(e)[2] for e in evs2
               if 155000 <= (e["ts"] - ts02) <= 165000 and BG.body(e) and BG.body(e)[1] == "12001002"]
    idxs = [0x08 + i for i in range(0x26 - 0x08 + 1)]
    gen_resync2 = [bytes([1, 2, i, 1]) for i in idxs] + [bytes([1, 2, idxs[-1], 1])]
    r2 = sum((Counter(gen_resync2) & Counter(cap_in2)).values())
    print(f"     resync 12001002 (IN): {r2}/{len(cap_in2)} (regra: [01][02][idx 0x08..0x26][01], último dup)")
    ok += r2; tot += len(cap_in2)
    print(f"  TOTAL save: {ok}/{tot} ({100*ok/tot:.2f}%)")
    return ok, tot

# ---------------------------------------------------------------- E. upload IR
def _idx_rule(k):
    """posição 0-based -> idx do chunk (páginas de 128 com salto: 0-127, 256-383, 512-...)"""
    return k if k < 128 else (k + 128 if k < 256 else k + 256)

def ir_flow():
    print("\n== E. GERAÇÃO do upload de IR (S2): framing por regras, dados = inventário ==")
    evs = BG.load(CAPS["S2"])
    ts0 = evs[0]["ts"]
    rows = []
    for e in evs:
        b = BG.body(e)
        if b:
            rows.append(((e["ts"] - ts0) / 1000, e["dir"], b[0], b[1], b[2]))
    begins = sorted(t for t, dr, f, a, d in rows if dr == "out_long" and f == "12" and a == "10050001")
    # ordem real (v2): BEGIN em 57.5s -> chunks 80-90s; BEGIN em 121.1s -> chunks 130-140s
    bounds = [(begins[0], 100.0), (begins[1], 145.0)]
    ok = tot = 0
    for slot, (t0, t1) in enumerate(bounds):
        chunks = [d for t, dr, f, a, d in rows
                  if t0 <= t < t1 and dr == "out_long" and f == "12" and a == "12001002" and len(d) == 33]
        acks = [d for t, dr, f, a, d in rows
                if t0 <= t < t1 + 0.5 and dr == "in_long" and f == "12" and a == "12001002" and len(d) == 4]
        # geração: header por regra (slot + idx com salto de página; último idx duplicado)
        gen_chunks, gen_acks = [], []
        n = len(chunks)
        for k, d in enumerate(chunks):
            idx = _idx_rule(k) if k < n - 1 else _idx_rule(n - 2)
            hdr = bytes([slot]) + idx.to_bytes(2, "big")
            gen_chunks.append(hdr + d[3:])          # dados = inventário (conteúdo do IR)
            gen_acks.append(hdr + b"\x01")
        begin = bytes([0, slot, 0, 0, 1, 0, 0, 0x0A])
        cc = sum((Counter(gen_chunks) & Counter(chunks)).values())
        ca = sum((Counter(gen_acks) & Counter(acks)).values())
        print(f"  slot {slot}: chunks {cc}/{n} | ACKs {ca}/{len(acks)} | "
              f"idx final 0x{_idx_rule(n-2):X} dup ({n} msgs = {n-1} únicos + fim 2×)")
        begin_cap = [d for t, dr, f, a, d in rows if t0 - 0.5 <= t <= t0 + 0.5 and dr == "out_long" and a == "10050001"]
        b_ok = 1 if begin_cap and begin_cap[0] == begin else 0
        print(f"      BEGIN gerado {begin.hex(' ')} == capturado: "
              f"{begin_cap[0].hex(' ') if begin_cap else '?'} ({'ok' if b_ok else 'DIF'})")
        ok += cc + ca + b_ok
        tot += n + len(acks) + 1
    print(f"  TOTAL upload IR: {ok}/{tot} ({100*ok/tot:.2f}%) — dados dos chunks (30 nibbles/msg) = inventário")
    return ok, tot
def boot_flow():
    print("\n== C. GERAÇÃO do boot/scan de S1 (t<30s) ==")
    evs = BG.load(CAPS["S1"])
    ts0 = evs[0]["ts"]
    captured = defaultdict(list)   # (dir,f,addr) -> payloads em ordem
    for e in evs:
        t = e["ts"] - ts0
        if t > 30000:
            continue
        b = BG.body(e)
        if not b or e["dir"] != "out_long":
            continue
        captured[(b[0], b[1])].append(b[2].hex())

    gen = defaultdict(list)
    # 1) tabela de tipos: 20 paginas, cada uma lida 2x (comportamento do Suite)
    for p in range(0x14):
        gen[("11", "12001002")] += [f"{p:02x}"] * 2
    # 2) 12001012: 5 entradas
    gen[("11", "12001012")] += [f"{i:02x}" for i in range(5)]
    # 3) tabela de nomes 11000008: chave = [banco u8][índice u8];
    #    bancos 0x00-0x02 completos (16) + banco 0x03 com 13 = 61 leituras
    for bank in range(3):
        for idx in range(16):
            gen[("11", "11000008")].append(f"{bank:02x}{idx:02x}0000")
    for idx in range(13):
        gen[("11", "11000008")].append(f"03{idx:02x}0000")
    # 4) keepalive
    gen[("12", "00020001")] += ["00000000"] * 2
    # 5) scan de presets (§13.10): pp vem da captura (inventário do device);
    #    regras geradas: select [pp] (u16 BE) + open [pp] 01 + 9 paginas 5B.
    #    Quirk de boot: o preset ATUAL (0x0100, §13.4) ganha select+open duplicado.
    pps = []
    for hx in captured.get(("11", "13010000"), []):
        if hx not in pps:
            pps.append(hx)
    for pp in pps:
        sel = [pp]
        opn = [pp + "01"]
        if pp == "0100":            # preset atual: re-leitura no boot
            sel.append(pp)
            opn.append(pp + "01")
        gen[("11", "13010000")] += sel
        gen[("12", "13010002")] += opn
        for pg in range(9):
            gen[("12", "13010004")].append(pp + f"{pg:04x}01")   # pp u16BE + PG u16BE + 01
    # 6) sonda do banco 2 (§13.3/13.10): pp 0000 do banco 02, 9 páginas
    gen[("11", "13020000")].append("0000")
    gen[("12", "13020002")].append("000001")
    for pg in range(9):
        gen[("12", "13020004")].append("0000" + f"{pg:04x}01")

    tot_ok = tot_cap = tot_gen = 0
    keys = sorted(set(captured) | set(gen))
    for k in keys:
        c, g = captured.get(k, []), gen.get(k, [])
        cc, gc = Counter(c), Counter(g)
        inter = sum((cc & gc).values())
        tot_ok += inter
        tot_cap += len(c)
        tot_gen += len(g)
        mark = "" if len(c) == len(g) == inter else f"   ← cap {len(c)} vs gen {len(g)}, casam {inter}"
        print(f"  {k[0]}/{k[1]}: {inter}/{len(c)} byte-a-byte{mark}")
        if inter != len(c) or len(g) != len(c):
            miss_c = list((cc - gc).elements())[:3]
            miss_g = list((gc - cc).elements())[:3]
            if miss_c: print(f"      na captura e não gerado: {miss_c}")
            if miss_g: print(f"      gerado e não na captura: {miss_g}")
    print(f"  TOTAL boot: {tot_ok}/{tot_cap} gerados corretos ({100*tot_ok/tot_cap:.2f}%), "
          f"gerados {tot_gen} (excedente {tot_gen-tot_ok})")
    return tot_ok, tot_cap

def main():
    doc, out_idx, in_idx = load_golden()
    print("== A. ACCOUNTING: toda mensagem explicada por algum template ==")
    total = [0, 0, 0, 0]
    leftovers = Counter()
    for name, cap in CAPS.items():
        evs = BG.load(cap)
        a = accounting(name, evs, out_idx, in_idx)
        for i in range(4):
            total[i] += a[i]
        leftovers.update(a[4])
    print(f"  TOTAL: OUT {total[0]}/{total[0]+total[1]} ({100*total[0]/(total[0]+total[1]):.2f}%)  "
          f"IN {total[2]}/{total[2]+total[3]} ({100*total[2]/(total[2]+total[3]):.2f}%)")
    if leftovers:
        print("  sobras (top 12):")
        for k, c in leftovers.most_common(12):
            print(f"    x{c:5d} {k}")
    kb = knob_flow()
    bb = boot_flow()
    sf = save_flow()
    ir = ir_flow()
    print("\n== RESUMO ==")
    print(f"  A accounting: OUT {100*total[0]/(total[0]+total[1]):.2f}% | IN {100*total[2]/(total[2]+total[3]):.2f}%")
    print(f"  B knob gen:   S3 {kb[0]}/{kb[1]} | S1 {kb[2]}/{kb[3]}")
    print(f"  C boot gen:   {bb[0]}/{bb[1]} ({100*bb[0]/bb[1]:.2f}%)")
    print(f"  D save gen:   {sf[0]}/{sf[1]} ({100*sf[0]/sf[1]:.2f}%)")
    print(f"  E IR upload:  {ir[0]}/{ir[1]} ({100*ir[0]/ir[1]:.2f}%) — dados dos chunks = inventário")

if __name__ == "__main__":
    main()
