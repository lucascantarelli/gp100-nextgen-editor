#!/usr/bin/env python3
"""dump_preset_list.py — extrai a biblioteca de fábrica do all.prst (XML)
para packages/app/ui/src/artifacts/presetData.ts (const FACTORY_PRESETS).

Fonte: files/patches/all.prst (schema §13.9 do PROTOCOL.md). O artefato é
GERADO — não editar mão. Regenerar: uv run python analysis/dump_preset_list.py
"""
import re
import sys
from pathlib import Path

SRC = Path("files/patches/all.prst")
OUT = Path("packages/app/ui/src/artifacts/presetData.ts")

sys.stdout.reconfigure(errors="replace")

data = SRC.read_text(encoding="utf-8")
tags = re.findall(r'<presets\b([^>]*)/>|<presets\b([^>]*)>', data)
rows = []
for a, b in tags:
    attrs = a or b
    def attr(name: str):
        m = re.search(rf'{name}="([^"]*)"', attrs)
        return m.group(1) if m else ""
    rows.append(
        (int(attr("ppID") or "0"), attr("ppName"), attr("ppTypeName"), int(attr("ppType") or "0"))
    )

rows.sort()
assert len(rows) == 99, f"esperava 99 presets, achei {len(rows)}"
ids = [r[0] for r in rows]
assert ids == list(range(99)), f"ppIDs não são 0..98: {ids[:5]}…{ids[-5:]}"

lines = [
    "/**",
    " * Biblioteca de FÁBRICA da GP-100 — GERADO de files/patches/all.prst",
    " * (schema §13.9 do PROTOCOL.md). NÃO editar mão.",
    " * Regenerar: uv run python analysis/dump_preset_list.py",
    " */",
    "",
    "export interface FactoryPreset {",
    "  pp: number;",
    "  name: string;",
    "  ppTypeName: string;",
    "  ppType: number;",
    "}",
    "",
    "export const FACTORY_PRESETS: FactoryPreset[] = [",
]
for pp, name, tname, ttype in rows:
    n = name.replace("\\", "\\\\").replace('"', '\\"')
    lines.append(f'  {{ pp: {pp}, name: "{n}", ppTypeName: "{tname}", ppType: {ttype} }},')
lines.append("];")
lines.append("")

OUT.write_text("\n".join(lines), encoding="utf-8")
print(f"OK: {len(rows)} presets → {OUT} ({OUT.stat().st_size} bytes)")
print("amostra:", rows[:3], "…", rows[-2:])
