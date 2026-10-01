#!/usr/bin/env python3
"""dump_drum_kit.py — extrai do firmware V2.1 a tabela de RITMOS da bateria
para packages/app/ui/src/artifacts/drumData.ts (GERADO — não editar mão).

Fonte: files/firmware/GP-100 Firmware V2.1.bin — a tabela de estilos fica
entre as divisões de nota (1/8T) e os compassos (2/4…9/8). O firmware NÃO
traz rótulos de gênero; o agrupamento por conteúdo é evidente na ordem
(Electronic → Rock → Pop/Blues → World → Jazz) e os rótulos são INFERIDOS
(documentado em docs/UI_REFERENCE.md §8).

Regenerar: uv run python analysis/dump_drum_kit.py
"""
import re
import sys
from pathlib import Path

FW = next(Path("files/firmware").glob("*.bin"))
OUT = Path("packages/app/ui/src/artifacts/drumData.ts")

sys.stdout.reconfigure(errors="replace")
data = FW.read_bytes()

# âncoras: começa em "Metro " e termina em "Fusion" (seguido de 2/4…)
i_start = data.find(b"Electronic |")
i_start = data.find(b"Electronic")
seg = data[i_start : i_start + 4000]
end = seg.find(b"Fusion")
seg = seg[: end + len(b"Fusion")]
strs = [s.decode("ascii").strip().replace("D& B", "D&B") for s in re.findall(rb"[\x20-\x7e]{3,}", seg)]
assert strs[0] == "Electronic" and strs[-1] == "Fusion", strs[:5] + strs[-5:]
# o firmware grava UM rótulo de gênero ("Electronic") e depois só estilos:
strs = strs[1:]

# grupos:Electronic(12) Rock(33) Pop(14) World(20) Jazz(8) — cortes evidentes:
GROUPS = {
    "Electronic": 12,
    "Rock": 33,
    "Pop": 14,
    "World": 20,
    "Jazz": 8,
}
total = sum(GROUPS.values())
assert len(strs) == total, f"esperava {total} estilos, achei {len(strs)}"

genres: list[tuple[str, list[str]]] = []
idx = 0
for g, n in GROUPS.items():
    genres.append((g, strs[idx : idx + n]))
    idx += n

# compassos reais do firmware (imediatamente após "Fusion")
fusion_abs = i_start + end  # end = offset de "Fusion" dentro de seg
tail = data[fusion_abs : fusion_abs + 220]
beats = [s.decode("ascii").strip() for s in re.findall(rb"[\x20-\x7e]{3,}", tail) if b"/" in s][:8]
assert beats == ["2/4", "3/4", "4/4", "6/4", "7/4", "6/8", "7/8", "9/8"], beats

lines = [
    "/**",
    " * Ritmos da bateria (drum) da GP-100 — GERADO do firmware V2.1",
    " * (tabela de estilos entre as divisões de nota e os compassos).",
    " * NÃO editar mão. Regenerar: uv run python analysis/dump_drum_kit.py",
    " *",
    " * Os RÓTULOS de gênero são inferidos (o firmware ordena os estilos por",
    " * grupo sem gravar os nomes); a LISTA e a ORDEM dos estilos são reais.",
    " */",
    "",
    "export interface DrumGenre {",
    "  genre: string;",
    "  styles: string[];",
    "}",
    "",
    "export const DRUM_GENRES: DrumGenre[] = [",
]
for g, styles in genres:
    lines.append(f'  {{ genre: "{g}", styles: [{", ".join(chr(34) + s + chr(34) for s in styles)}] }},')
lines += [
    "];",
    "",
    "export const DRUM_TOTAL_STYLES = " + str(total) + ";",
    "",
    "/** Compassos do drum (firmware: 2/4…9/8). */",
    "export const DRUM_BEATS = [" + ", ".join(f'"{b}"' for b in beats) + "] as const;",
    "",
    "export type DrumBeat = (typeof DRUM_BEATS)[number];",
    "",
]
OUT.write_text("\n".join(lines), encoding="utf-8")
print(f"OK: {total} estilos em {len(genres)} gêneros → {OUT} ({OUT.stat().st_size} bytes)")
for g, styles in genres:
    print(f"  {g}: {len(styles)} — {styles[:4]}…")
print("beats:", beats)
