"""dump_fx_map.py — mapa REAL dos efeitos da pedaleira para a UI do pedalboard.

Fontes (R1 — nunca adivinhar):
  - analysis/parameters.json : 185 algoritmos / 639 controles (nome, tipo,
    pos, min/max, options, default) — validado 3 vias (Suite XML + 909 slots
    + firmware)
  - files/patches/all.prst   : os 99 presets de fábrica (nomes/effectCode
    reais da cadeia por slot x=0..8)

Saída: analysis/fx_map.json
  - modules: { DST: [ {variant, nibble, index, name, knobs, switches,
    comboxes, control_count, observed_in_patches} ] } — o catálogo completo
    por módulo (o UI usa `variant` para escolher o MODELO do pedal e os
    knobs vêm todos, sem corte).
  - stats: contagens por módulo (quantos SVGs por família e capacidades).

Rodar: uv run python analysis/dump_fx_map.py
"""

import json
import re
import sys
from pathlib import Path

sys.stdout.reconfigure(errors="replace")

ROOT = Path(__file__).resolve().parent.parent
PARAMS = ROOT / "analysis" / "parameters.json"
OUT = ROOT / "analysis" / "fx_map.json"
OUT_TS = ROOT / "packages" / "app" / "ui" / "src" / "artifacts" / "fxData.ts"

TS_HEADER = """/**
 * GERADO por `uv run python analysis/dump_fx_map.py` — NÃO EDITAR NA MÃO.
 * Fonte: analysis/parameters.json (R1 — 185 algoritmos / 639 controles
 * validados 3 vias). O fallback dev/teste do browser usa ESTES dados reais
 * (mesmos nomes/ranges/defaults da pedaleira de fábrica).
 */

export interface FxKnobData {
  name: string;
  pos: number;
  default: string | null;
  min: number | null;
  max: number | null;
}
export interface FxSwitchData {
  name: string;
  pos: number;
  options: string[];
  default: string | null;
}
export interface FxAlgorithm {
  variant: string;
  nibble: number;
  index: number;
  name: string;
  knobs: FxKnobData[];
  switches: FxSwitchData[];
  comboxes: FxSwitchData[];
}
export const FX_MODULES: Record<string, FxAlgorithm[]> = """


def slug(name: str) -> str:
    s = re.sub(r"[^a-zA-Z0-9]+", "-", name.strip()).strip("-").lower()
    return s or "fx"


def main() -> None:
    data = json.loads(PARAMS.read_text(encoding="utf-8"))
    algs = data["algorithms"]

    modules: dict[str, list[dict]] = {}
    for a in algs:
        knobs = [
            {"name": c["name"], "pos": c["pos"], "default": c.get("default"),
             "min": c.get("min"), "max": c.get("max")}
            for c in a["controls"] if c["type"] == "knob"
        ]
        switches = [
            {"name": c["name"], "pos": c["pos"], "options": c.get("options", []),
             "default": c.get("default")}
            for c in a["controls"] if c["type"] == "switch"
        ]
        comboxes = [
            {"name": c["name"], "pos": c["pos"], "options": c.get("options", []),
             "default": c.get("default")}
            for c in a["controls"] if c["type"] == "combox"
        ]
        entry = {
            "variant": slug(a["name"]),
            "nibble": a["nibble"],
            "index": a["index"],
            "name": a["name"],
            "knobs": knobs,
            "switches": switches,
            "comboxes": comboxes,
            "observed_in_patches": a.get("observed_in_patches", 0),
        }
        modules.setdefault(a["module"], []).append(entry)

    for m in modules.values():
        m.sort(key=lambda e: (-e["observed_in_patches"], e["name"]))

    # TS p/ a UI: REMOVE campos não declarados na interface FxAlgorithm
    # (o dumper serializa tudo; o TS é um subconjunto estável).
    ts_modules = {
        m: [
            {k: e[k] for k in ("variant", "nibble", "index", "name", "knobs", "switches", "comboxes")}
            for e in lst
        ]
        for m, lst in modules.items()
    }

    stats = {
        m: {
            "effects": len(lst),
            "max_controls": max(
                len(e["knobs"]) + len(e["switches"]) + len(e["comboxes"]) for e in lst
            ),
            "max_knobs": max(len(e["knobs"]) for e in lst),
            "max_switches": max(len(e["switches"]) for e in lst),
        }
        for m, lst in modules.items()
    }

    out = {
        "meta": {
            "source": "analysis/parameters.json (R1) — gerado por dump_fx_map.py",
            "effects_total": len(algs),
            "modules": {m: len(lst) for m, lst in modules.items()},
        },
        "stats": stats,
        "modules": modules,
    }
    OUT.write_text(json.dumps(out, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")

    # TS p/ o fallback da UI (mesmos dados reais; sem cwd/path hacks).
    OUT_TS.parent.mkdir(parents=True, exist_ok=True)
    OUT_TS.write_text(TS_HEADER + json.dumps(ts_modules, ensure_ascii=False) + " as const;\n", encoding="utf-8")

    print(f"fx_map.json + fxData.ts: {len(algs)} efeitos em {len(modules)} módulos")
    for m, s in stats.items():
        print(f"  {m:4s} {s['effects']:3d} efeitos | knobs≤{s['max_knobs']:2d} "
              f"switch≤{s['max_switches']:2d} | max ctrl {s['max_controls']}")


if __name__ == "__main__":
    main()
