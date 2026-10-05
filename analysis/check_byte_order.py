#!/usr/bin/env python3
"""check_byte_order.py — decide LE vs BE do `effectCode` com o FIO REAL.

Não é opinião de documentação. Pega os writes `12 | 10[slot]0002` da captura
da sessão 3, colapsa o nibble, lê os 4 primeiros bytes reais das DUAS formas e
compara com o `effectCode` que a documentação diz que o `.prst` usa
(`nibble << 24 | index`). A forma cuja faixa cai no catálogo de efeitos ganha.

Formato da captura: cada linha é o SysEx COMPLETO.

    f0 21 25 7f 47 50 2d 64 | <func> | <addr 4B> | <payload> | f7
     0  1  2  3  4  5  6  7     8        9..12      13..        -1

Uso:  python analysis/check_byte_order.py
"""
import json
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CAP = ROOT / "analysis" / "captures" / "session3.jsonl"
HDR = bytes.fromhex("f021257f47502d64")  # 8 bytes de cabeçalho do SysEx


def collapse(wire: bytes) -> bytes:
    """O fio manda cada byte real como dois nibbles: b -> [b>>4, b & 0x0F]."""
    return bytes((wire[i] << 4) | wire[i + 1] for i in range(0, len(wire), 2))


def knob_payloads():
    """(slot, bytes_reais_do_payload) de cada write de knob da captura."""
    for linha in CAP.read_text(encoding="utf-8", errors="replace").splitlines():
        linha = linha.strip()
        if not linha.startswith("{"):
            continue
        try:
            ev = json.loads(linha)
        except json.JSONDecodeError:
            continue
        if ev.get("dir") != "out_long":
            continue
        mensagem = bytes.fromhex(ev.get("hex", ""))
        if not mensagem.startswith(HDR) or len(mensagem) < 14:
            continue
        func = mensagem[8]
        addr = mensagem[9:13]
        # endereço do knob: 10 <slot 1..9> 00 02
        if func != 0x12 or addr[0] != 0x10 or addr[2] != 0x00 or addr[3] != 0x02:
            continue
        try:
            cru = collapse(mensagem[13:-1])
        except ValueError:
            continue
        if len(cru) == 10:
            yield addr[1], cru


payloads = list(knob_payloads())
print(f"writes de knob lidos da captura: {len(payloads)}\n")

if not payloads:
    sys.exit("nenhum payload de knob encontrado — a captura mudou de formato?")

for slot, cru in payloads[:3]:
    le = struct.unpack("<I", cru[0:4])[0]
    be = struct.unpack(">I", cru[0:4])[0]
    print(f"  slot {slot}: bytes reais [{cru[0:4].hex(' ')}]  ->  LE {le:#010x}  |  BE {be:#010x}")

le_vals = [struct.unpack("<I", c[0:4])[0] for _, c in payloads]
be_vals = [struct.unpack(">I", c[0:4])[0] for _, c in payloads]

print()
print("o catálogo usa `nibble << 24 | index` (PROTOCOL.md §12). Então o efeito tem")
print("de ter o nibble do módulo em 1..15 no byte mais alto, e index pequeno:\n")
print(f"  LE: min={min(le_vals):#010x}  max={max(le_vals):#010x}  "
      f"nibbles={sorted({v >> 24 for v in le_vals})}")
print(f"  BE: min={min(be_vals):#010x}  max={max(be_vals):#010x}  "
      f"nibbles={sorted({v >> 24 for v in be_vals})}")

def plausivel(valores):
    """O efeito tem de ser `nibble<<24 | index`: nibble <= 0x0F e index pequeno.

    O nibble 0 aparece na captura (um parâmetro que não é de módulo), então o
    teste é `<= 0x0F`, não `1..=15`. O que ele descarta é o BE, onde o byte
    alto vira 0x6e = 110 — impossível para um módulo da cadeia.
    """
    return all((v >> 24) <= 0x0F and (v & 0xFFFFFF) < (1 << 16) for v in valores)

ok_le, ok_be = plausivel(le_vals), plausivel(be_vals)
print()
print(f"  todos plausíveis como LE? {ok_le}")
print(f"  todos plausíveis como BE? {ok_be}")

print()
print("testemunha independente — o par do F1 do H2, confirmado em CAMPO em 05/10/2026:")
print("  bytes reais [6e 00 00 07] -> LE 0x0700006e = Bog RedM (o efeito que o display mostrou)")
print("                            -> BE 0x6e000007 = não existe no catálogo")

if ok_le and not ok_be:
    print("\nVEREDITO: o effectCode vai como u32 **LE**. Quem diz BE está errado.")
    sys.exit(0)
if ok_be and not ok_le:
    print("\nVEREDITO: o effectCode vai como u32 **BE** (a documentação está certa).")
    sys.exit(0)
print("\nINCONCLUSIVO — as duas leituras passam o filtro; decidir com o .prst.")
sys.exit(2)
