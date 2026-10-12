#!/usr/bin/env python3
"""derive_save_ops.py — re-deriva a OPERAÇÃO de save do log cru (D3 do ADR-6).

Perguntas que responde (rev.2 do ADR-6, regra D3):
  1. Qual a sequência REAL de writes/ops em torno de cada save (S2 × S4)?
  2. O que o device EMPURRA (IN) depois do save — e há padrão silencioso (S2)?
  3. Intervalos (pacing) entre ops e entre pushes (janela de silêncio?).

Uso: uv run python analysis/derive_save_ops.py
Saída:stdout legível (transações comprimidas + resumo por save).
"""
import json
import re
import sys
from pathlib import Path

sys.stdout.reconfigure(errors="replace")  # console cp1252 (knowledge.md)

ROOT = Path(__file__).resolve().parent.parent
HDR = "f021257f47502d64"
SAVE_MARK = "11000000"  # 1º write do meta_block (nome do preset)


def load(path: Path):
    """Carrega o log cru; repara o bug da aspa dos logs velhos (loaders)."""
    msgs = []
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            rec = json.loads(line)
        except json.JSONDecodeError:
            fixed = re.sub(r'("hex":"[0-9a-f]*)"\}', r'\1"}', line) or line
            try:
                rec = json.loads(fixed)
            except json.JSONDecodeError:
                continue
        if rec.get("dir") not in ("out_long", "in_long") or "hex" not in rec:
            continue
        hexs = rec["hex"].lower()
        cut = hexs.find("f7")  # trim no 1º F7 (cauda stale do ring buffer)
        if cut != -1:
            hexs = hexs[: cut + 2]
        if not hexs.startswith(HDR) or len(hexs) < 28:
            continue  # SEM-HDR ou curto demais (dado conhecido)
        msgs.append(
            {
                "ts": rec["ts"],
                "dir": "OUT" if rec["dir"] == "out_long" else "IN",
                "func": hexs[16:18],
                "addr": hexs[18:26],
                "data": hexs[26:-2],
            }
        )
    return msgs


def compress(events):
    """Colapsa repetições consecutivas (mesmo dir/addr/data) em contagens."""
    out = []
    for e in events:
        key = (e["dir"], e["addr"], e["data"])
        if out and out[-1]["key"] == key:
            out[-1]["n"] += 1
            out[-1]["t_end"] = e["ts"]
        else:
            out.append({"key": key, "n": 1, "t": e["ts"], "t_end": e["ts"], "e": e})
    return out


def show(events, t0, title):
    print(f"\n=== {title} ===")
    for c in compress(events):
        d, addr, data = c["key"]
        dt = c["t"] - t0
        ddata = data if len(data) <= 24 else data[:24] + f"…(+{len(data)//2 - 12}B)"
        suffix = f" ×{c['n']}" if c["n"] > 1 else ""
        gap = ""
        if c["n"] > 1:
            gap = f" [span {c['t_end'] - c['t']}ms]"
        print(f"  +{dt:7d}ms {d:3} {addr} {ddata}{suffix}{gap}")


def save_windows(msgs, ctx_before_ms=4000, ctx_after_ms=16000):
    """Janelas centradas em cada write `11000000` OUT (marcador do save)."""
    marks = [i for i, m in enumerate(msgs) if m["dir"] == "OUT" and m["addr"] == SAVE_MARK]
    windows = []
    for idx in marks:
        t0 = msgs[idx]["ts"]
        lo = idx
        while lo > 0 and msgs[lo - 1]["ts"] >= t0 - ctx_before_ms:
            lo -= 1
        hi = idx
        while hi < len(msgs) - 1 and msgs[hi + 1]["ts"] <= t0 + ctx_after_ms:
            hi += 1
        windows.append((t0, msgs[lo : hi + 1]))
    return windows


def ops_anywhere(msgs):
    """Todas as ocorrências de ops `00020000` no log inteiro (contexto)."""
    return [(m["ts"], m["dir"], m["data"]) for m in msgs if m["addr"] == "00020000"]


def main():
    for name in ("session2.jsonl", "session4.jsonl"):
        msgs = load(ROOT / "analysis" / "captures" / name)
        print(f"\n################ {name}: {len(msgs)} msgs com envelope ################")
        ops = ops_anywhere(msgs)
        print(f"ops 00020000 no log INTEIRO: {len(ops)} ocorrências")
        for ts, d, data in ops[:12]:
            print(f"  ts={ts} {d} data={data}")
        wins = save_windows(msgs)
        print(f"\n{len(wins)} save(s) detectado(s) (marcador {SAVE_MARK} OUT)")
        for i, (t0, evs) in enumerate(wins):
            show(evs, t0, f"save #{i + 1} de {name} (t0 = meta write)")
            # resumo IN pós-save: primeiras/últimas msgs IN da janela
            ins = [e for e in evs if e["dir"] == "IN"]
            if ins:
                span = ins[-1]["ts"] - ins[0]["ts"]
                print(f"  → IN na janela: {len(ins)} msgs, span {span}ms")
            else:
                print("  → IN na janela: NENHUM (save silencioso)")


if __name__ == "__main__":
    main()
