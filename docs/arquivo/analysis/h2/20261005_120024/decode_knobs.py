"""Decodifica os 92 frames REAIS de knob da captura S3 (analysis/fixtures/knobs.jsonl).

Cada real byte b vira dois bytes no fio: [b>>4, b & 0x0F].
O payload de set-param sao 10 bytes reais:
    [code u32 LE][ctrl u8][0x00][value f32 LE]
"""
import json
import struct
import sys
from collections import defaultdict
from pathlib import Path

FIX = Path(__file__).resolve().parents[2] / "fixtures" / "knobs.jsonl"


def collapse(wire: bytes) -> bytes:
    if len(wire) % 2:
        raise ValueError("numero impar de nibbles")
    return bytes((wire[i] << 4) | wire[i + 1] for i in range(0, len(wire), 2))


rows = []
for line in FIX.read_text(encoding="utf-8").splitlines():
    if not line.strip():
        continue
    r = json.loads(line)
    if r.get("dir") != "out" or not r["addr"].endswith("0002"):
        continue
    if not r["addr"].startswith("10"):
        continue
    real = collapse(bytes.fromhex(r["data"]))
    if len(real) != 10:
        continue
    code = struct.unpack("<I", real[0:4])[0]
    ctrl, sep = real[4], real[5]
    value = struct.unpack("<f", real[6:10])[0]
    chain = int(r["addr"][2:4], 16)
    rows.append((chain, code, ctrl, sep, value))

print(f"{len(rows)} frames de knob decodificados da captura real\n")
by_code = defaultdict(list)
for chain, code, ctrl, sep, value in rows:
    by_code[(code, ctrl)].append(value)

print(f"{'effectCode':>12} {'ctrl':>4} {'n':>4}  {'min':>10} {'max':>10}")
print("-" * 50)
for (code, ctrl), vals in sorted(by_code.items(), key=lambda kv: -len(kv[1])):
    print(
        f"0x{code:08x} {ctrl:>4} {len(vals):>4}  "
        f"{min(vals):>10.4f} {max(vals):>10.4f}"
    )

allv = [v for *_, v in rows]
print(f"\nFAIXA GLOBAL das amostras reais: {min(allv):.4f} .. {max(allv):.4f}")
print(f"99.5 (o valor que o runbook inventou) esta dentro? "
      f"{'SIM' if min(allv) <= 99.5 <= max(allv) else 'NAO — acima do teto observado'}")
seps = {r[3] for r in rows}
print(f"separadores observados: {sorted(seps)} (o codec exige 0x00)")