#!/usr/bin/env python3
"""decode_types_table.py — fecha 2 lacunas de documentação:

1. Tabela de TIPOS (12001002): respostas de 75B no boot (20 páginas, lidas 2×).
   Hipóteses a testar: [page][nome 32B 0xff][00][trailer 4B] com trailer=CRC32?
   Cross-ref: nomes de tipo em ASCII na 11000008 (banco 0) + ppType/ppTypeName
   do all.prst (99 presets).
2. Schema do 11000007: write de 50B no bloco de metadados (S1/S2/S4) —
   verificar: sempre zeros? correlação com ppAuthor/ppNotes/ppVolume?
"""
import json, os, sys, struct, zlib
import xml.etree.ElementTree as ET
from collections import Counter, defaultdict
import importlib.util

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("bg", os.path.join(HERE, "build_golden.py"))
BG = importlib.util.module_from_spec(spec)
spec.loader.exec_module(BG)
sys.stdout.reconfigure(errors="replace")

def page_name_bytes(d):
    """payload cru de 75B -> (page, 32B do campo nome, trailer 4B)"""
    return d[0], d[1:33], d[33:37], d[37:]

def unpack_nibbles(p):
    return bytes((p[i] << 4) | p[i + 1] for i in range(0, len(p), 2))

def main():
    # ---------- 1) TIPOS: coletar todas as respostas 12001002 de 75B ----------
    print("== 1. Tabela de TIPOS (12001002, respostas 75B) ==")
    pages = {}
    for i in (1, 2, 3, 4):
        evs = BG.load(os.path.join(HERE, "captures", f"session{i}.jsonl"))
        ts0 = evs[0]["ts"]
        for e in evs:
            b = BG.body(e)
            if b and e["dir"] == "in_long" and b[1] == "12001002" and len(b[2]) == 75:
                pg = b[2][0]
                if pg not in pages:
                    pages[pg] = b[2]
    print(f"páginas distintas coletadas: {len(pages)} -> {sorted(pages)}")

    # nomes de tipo na tabela de nomes 11000008 (S1, chaves 0x0000..)
    evs1 = BG.load(os.path.join(HERE, "captures", "session1.jsonl"))
    name_table = {}
    for e in evs1:
        b = BG.body(e)
        if b and e["dir"] == "in_long" and b[1] == "11000008" and len(b[2]) == 14:
            key = int.from_bytes(b[2][0:2], "big")
            nm = b[2][4:].split(b"\0")[0].decode("ascii", "replace")
            if nm:
                name_table[key] = nm
    print(f"nomes ASCII na 11000008: {name_table}")

    # ppType -> ppTypeName do all.prst
    root = ET.parse(r"D:\GP-100 app\files\patches\all.prst").getroot()
    tmap = {}
    for p in root.iter("presets"):
        t, tn = p.get("ppType"), p.get("ppTypeName")
        if t and tn:
            tmap.setdefault(int(t), tn)
    print(f"ppType->ppTypeName (all.prst): {tmap}")

    print("\npágina | nome decodificado (32B c/ 0xFF...) | trailer[33:37] | resto[37:]")
    for pg in sorted(pages):
        d = pages[pg]
        raw = d[1:33]
        txt = raw.split(b"\xff")[0].decode("ascii", "replace")
        trailer = d[33:37]
        rest = d[37:]
        print(f"  {pg:2d} | '{txt}' raw={raw.hex()[:32]}… | {trailer.hex(' ')} | {rest.hex(' ')}")

    # checar se trailer muda por página e se parece CRC32
    trs = {pg: pages[pg][33:37] for pg in pages}
    uniq = {pg: t.hex() for pg, t in trs.items()}
    print(f"\ntrailers únicos: {len(set(uniq.values()))} de {len(uniq)} páginas")
    d0 = pages[0]
    body0 = d0[1:33]
    print(f"crc32(nome pág0)={zlib.crc32(body0):08x} | trailer pág0={trs[0].hex()}")

    # páginas de tipos vs ppTypeName: há 9 módulos? 20 páginas lidas (0..0x13)
    # hipótese: página = entrada da tabela de tipos de preset (Rock/Pop/…)
    # verificar igualdade de páginas entre sessões (device state muda?)
    print("\n-- verificação: página repetida em outra sessão é idêntica?")
    seen = defaultdict(set)
    for i in (1, 3, 4):
        evs = BG.load(os.path.join(HERE, "captures", f"session{i}.jsonl"))
        for e in evs:
            b = BG.body(e)
            if b and e["dir"] == "in_long" and b[1] == "12001002" and len(b[2]) == 75:
                seen[b[2][0]].add(b[2].hex())
    for pg in sorted(seen):
        if len(seen[pg]) > 1:
            print(f"  página {pg}: {len(seen[pg])} variantes distintas")

    # ---------- 2) Schema do 11000007 ----------
    print("\n== 2. Schema do 11000007 (write 50B nos saves) ==")
    for i, label, wins in ((1, "S1@54.7s", [(54000, 56000)]),
                           (2, "S2@149.6s", [(148000, 152000)]),
                           (4, "S4@1663s", [(1660000, 1670000)])):
        evs = BG.load(os.path.join(HERE, "captures", f"session{i}.jsonl"))
        ts0 = evs[0]["ts"]
        for e in evs:
            t = e["ts"] - ts0
            if not any(a <= t <= b for a, b in wins):
                continue
            b = BG.body(e)
            if b and e["dir"] == "out_long" and b[1] == "11000007":
                print(f"  {label}: {b[2].hex(' ')} | nonzero: {[j for j,x in enumerate(b[2]) if x]}")

    # ---------- 3) TIPOS: requisição de 2 bytes = page? confirmar paridade OUT/IN ----------
    print("\n== 3. Paridade de requisições 12001002 (OUT 1B) vs respostas ==")
    outc = Counter()
    for e in evs1:
        b = BG.body(e)
        if b and e["dir"] == "out_long" and b[1] == "12001002" and len(b[2]) == 1:
            outc[b[2][0]] += 1
    print(f"  pages pedidas: {sorted(outc)} | com resposta: {sorted(pages)}")

if __name__ == "__main__":
    main()
