#!/usr/bin/env python3
"""dump_preset_chains.py — extrai a CADEIA (9 slots + params) de cada preset
do all.prst para packages/app/ui/src/artifacts/presetChains.ts.

Fonte: files/patches/all.prst (schema §13.9 do PROTOCOL.md). O artefato é
GERADO — não editar à mão.

Por que existe: o core Rust já monta o board por preset
(`pedalboard::board_view_for`: `slot = @x`, `family = effectModuleName`,
`code = (nibble << 24) | index`, valor do knob = `params_N`), mas o MOCK WEB
do front (`src/ipc/device.ts`) tinha a cadeia FIXA — trocar de preset só
mudava o nome do LED e os 9 pedais continuavam iguais. Este artefato dá ao
mock a MESMA fonte de verdade do core, extraída do mesmo arquivo.

Achado do dump: em 20 dos 99 presets a cadeia está TROCADA (ex.: ppID 5 tem
DST antes de PRE; ppID 11 tem CAB antes de NR) — `@x` é a posição real e o
GP-100 deixa reordenar os pedais. Por isso `slot` vem de `@x` e `family` do
módulo, sem supor que sejam iguais.

Regenerar: uv run python analysis/dump_preset_chains.py
"""
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

SRC = Path("files/patches/all.prst")
OUT = Path("packages/app/ui/src/artifacts/presetChains.ts")

PARAMS = 15

sys.stdout.reconfigure(errors="replace")

root = ET.parse(SRC).getroot()
presets = root.findall("presets")
assert len(presets) == 99, f"esperava 99 presets, achei {len(presets)}"

chains = []
for p in presets:
    pp = int(p.get("ppID") or "0")
    effects = p.findall("Effect")
    assert len(effects) == 9, f"preset {pp}: esperava 9 <Effect>, achei {len(effects)}"

    slots = []
    for e in effects:
        slot = int(e.get("x"))
        params = [e.get(f"params_{i}") for i in range(PARAMS)]
        slots.append(
            {
                "slot": slot,
                "module": e.get("effectModuleName") or "",
                "name": (e.get("effectName") or "").strip(),
                "state": e.get("effectState") == "1",
                "code": int(e.get("effectCode") or "0"),
                "params": params,
            }
        )

    slots.sort(key=lambda s: s["slot"])
    # @x é uma PERMUTAÇÃO de 0..8 — não precisa seguir a ordem canônica das
    # famílias (20 presets têm a cadeia trocada; ver cabeçalho).
    assert [s["slot"] for s in slots] == list(range(9)), (
        f"preset {pp}: @x não é 0..8: {[s['slot'] for s in slots]}"
    )

    chains.append({"pp": pp, "slots": slots})

chains.sort(key=lambda c: c["pp"])
assert [c["pp"] for c in chains] == list(range(99)), "ppIDs não são 0..98"

lines = [
    "/**",
    " * CADEIA dos 99 presets de fábrica — GERADO de files/patches/all.prst",
    " * (schema §13.9 do PROTOCOL.md). NÃO editar à mão.",
    " * Regenerar: uv run python analysis/dump_preset_chains.py",
    " *",
    " * É a MESMA leitura que o core Rust faz em `pedalboard::board_view_for`:",
    " * `slot` = `@x` (posição na cadeia), `family` = `effectModuleName`,",
    " * `code` = `effectCode` ((nibble << 24) | index), knob = `params_N`.",
    " * `params` traz o valor CRU: quem converte em valor de knob é o front",
    " * (`src/ipc/device.ts`) com o dicionário (fxData) e a validação de range",
    " * — o sentinel 0xFFFF (65535 = “não configurado”) cai no default.",
    " *",
    " * 20 dos 99 presets têm a cadeia TROCADA (ex.: ppID 5 = DST antes de PRE)",
    " * — `slot` e `family` por isso são campos independentes.",
    " */",
    "",
    "export interface PresetSlot {",
    "  slot: number;",
    '  family: "PRE" | "DST" | "AMP" | "NR" | "CAB" | "EQ" | "MOD" | "DLY" | "RVB";',
    "  name: string;",
    "  state: boolean;",
    "  code: number;",
    "  params: (string | null)[];",
    "}",
    "",
    "export interface PresetChain {",
    "  pp: number;",
    "  slots: PresetSlot[];",
    "}",
    "",
    "export const PRESET_CHAINS: PresetChain[] = [",
]


def q(s: str) -> str:
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


for chain in chains:
    body = ",\n      ".join(
        "{ slot: %d, family: %s, name: %s, state: %s, code: %d, params: [%s] }"
        % (
            s["slot"],
            q(s["module"]),
            q(s["name"]),
            "true" if s["state"] else "false",
            s["code"],
            ", ".join(q(p) if p is not None else "null" for p in s["params"]),
        )
        for s in chain["slots"]
    )
    lines.append(f"  {{ pp: {chain['pp']}, slots: [\n      {body},\n    ] }},")
lines.append("];")
lines.append("")

OUT.write_text("\n".join(lines), encoding="utf-8")

trocas = sum(
    1
    for c in chains
    if [s["module"] for s in c["slots"]] != ["PRE", "DST", "AMP", "NR", "CAB", "EQ", "MOD", "DLY", "RVB"]
)
print(f"OK: {len(chains)} presets × 9 slots → {OUT} ({OUT.stat().st_size} bytes)")
print(f"presets com cadeia trocada: {trocas}/99")
print("amostra pp0:", [(s["slot"], s["module"], s["name"]) for s in chains[0]["slots"][:3]])
print("amostra pp5:", [(s["slot"], s["module"], s["name"]) for s in chains[5]["slots"][:3]])