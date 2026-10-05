#!/usr/bin/env python3
"""param_ranges.py — a tabela de faixas do knob, com versao e prova (#110).

O QUE ISTO E
------------
Em 05/10/2026 o gate H2 derrubou o aparelho: o runbook mandava
`set-param 3 0x0700006e 0 99.5`, o `99.5` era um numero inventado, e o
firmware V2.1 tem, em `Drivers/audio/audio.c:1828`, o assert

    CODE:para <= GetParaMaxVal(

Ao assertar ele PARA de responder a toda transacao, inclusive as leituras.
O device continua enumerado e `OK` no Windows, e a unica recuperacao e um
power-cycle fisico. Como o `set-param` e fire-and-forget (§13.11, D4), o fio
nao da nenhum aviso: o operador so descobre que quebrou o pedal quando a
leitura seguinte toma timeout.

A trava que evita isso vive no core (`packages/core/src/param_range.rs`) e
consulta o **dicionario** (`analysis/parameters.json`), que ja traz `min`/
`max` dos knobs e `option_ids` das chaves. Este script nao reimplementa essa
tabela — ele produz o **registro versionado** que responde as perguntas que
o codigo nao responde:

1. **Qual arquivo a trava le?** O sha256 do dicionario. Se alguem editar o
   dicionario sem rodar isto, o gate falha e a mudanca aparece como diff de
   artefato — nao como um comportamento que mudou em silencio.
2. **A tabela foi alguma vez testada contra o aparelho?** As 92 amostras
   reais de `analysis/fixtures/knobs.jsonl` sao o unico fio que o firmware
   ja aceitou. O gate compara cada uma com a regra declarada.
3. **Onde a tabela NAO tem regra?** Um par sem regra e aceito por decisao do
   owner (ADR-10) — entao essa lista e o backlog do dicionario, e precisa
   ser visivel, nao enterrada.

    python analysis/param_ranges.py            # regera o artefato
    python analysis/param_ranges.py --check    # gate (CI)
    python analysis/param_ranges.py show       # o relatório, legível
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
DICIONARIO = os.path.join(ROOT, "analysis", "parameters.json")
CAPTURA = os.path.join(ROOT, "analysis", "fixtures", "knobs.jsonl")
ARTEFATO = os.path.join(ROOT, "analysis", "param_ranges.json")

# slot do fio (§13.11) -> modulo do dicionario. O nibble do effectCode NAO e
# o modulo (0x05 = C-Wah no PRE, 0x03 = Green OD no DST); o slot do endereco
# e. E por isso que `Boost` e `14 Boost` — mesmo effectCode, PRE e DST — sao
# entradas distintas.
MODULE_OF_SLOT = {
    1: "PRE", 2: "DST", 3: "AMP", 4: "NR",
    5: "CAB", 6: "EQ", 7: "MOD", 8: "DLY", 9: "RVB",
}


def sha256(path: str) -> str:
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def collapse(wire: bytes) -> bytes:
    """Nibble-expandido -> bytes reais (§13.1)."""
    if len(wire) % 2:
        raise ValueError("numero impar de nibbles")
    return bytes((wire[i] << 4) | wire[i + 1] for i in range(0, len(wire), 2))


def regras() -> dict[tuple[int, int, int], dict]:
    """(slot, code, ctrl) -> regra, derivada do dicionario."""
    with open(DICIONARIO, encoding="utf-8") as f:
        d = json.load(f)
    out: dict[tuple[int, int, int], dict] = {}
    for slot, module in MODULE_OF_SLOT.items():
        for alg in d["algorithms"]:
            if alg["module"] != module:
                continue
            for c in alg["controls"]:
                chave = (slot, alg["code"], c["pos"])
                if c["type"] == "knob":
                    if "min" not in c or "max" not in c:
                        continue
                    lo, hi = c["min"], c["max"]
                    lo, hi = (lo, hi) if lo <= hi else (hi, lo)
                    out[chave] = {
                        "kind": "range",
                        "min": lo,
                        "max": hi,
                        "alg": alg["name"],
                        "ctrl_name": c["name"],
                    }
                else:
                    ids = c.get("option_ids") or []
                    if not ids:
                        continue
                    out[chave] = {
                        "kind": "discrete",
                        "ids": ids,
                        "alg": alg["name"],
                        "ctrl_name": c["name"],
                    }
    return out


def amostras() -> dict[tuple[int, int, int], list[float]]:
    """(slot, code, ctrl) -> valores, das 92 capturas reais de knob."""
    import struct

    obs: dict[tuple[int, int, int], list[float]] = {}
    with open(CAPTURA, encoding="utf-8") as f:
        for line in f:
            if not line.strip():
                continue
            r = json.loads(line)
            if r.get("dir") != "out":
                continue
            addr = r["addr"]
            if not (addr.startswith("10") and addr.endswith("0002")):
                continue
            real = collapse(bytes.fromhex(r["data"]))
            if len(real) != 10 or real[5] != 0x00:
                continue
            slot = int(addr[2:4], 16)
            code = struct.unpack("<I", real[0:4])[0]
            ctrl = real[4]
            obs.setdefault((slot, code, ctrl), []).append(
                struct.unpack("<f", real[6:10])[0]
            )
    return obs


def dentro(regra: dict, valor: float) -> bool:
    if regra["kind"] == "range":
        return regra["min"] <= valor <= regra["max"]
    return valor == int(valor) and int(valor) in regra["ids"]


def derivar() -> dict:
    regras_tab = regras()
    obs = amostras()

    pares = {}
    for (slot, code, ctrl), regra in sorted(regras_tab.items()):
        valores = obs.get((slot, code, ctrl), [])
        pares[f"{slot}/{code:08x}/{ctrl}"] = {
            **regra,
            "samples": len(valores),
            **(
                {"observed_min": min(valores), "observed_max": max(valores)}
                if valores
                else {}
            ),
            **(
                {"observed_ids": sorted(set(int(v) for v in valores))}
                if valores and regra["kind"] == "discrete"
                else {}
            ),
        }

    # Pares observados SEM regra: sao o backlog do dicionario, e o motivo de
    # a politica do ADR-10 aceitar o que nao tem regra.
    sem_regra = {}
    for (slot, code, ctrl), valores in sorted(obs.items()):
        if (slot, code, ctrl) in regras_tab:
            continue
        sem_regra[f"{slot}/{code:08x}/{ctrl}"] = {
            "slot": slot,
            "code": f"0x{code:08x}",
            "ctrl": ctrl,
            "samples": len(valores),
            "observed_min": min(valores),
            "observed_max": max(valores),
        }

    # Corroboracao: cada amostra real dentro da regra do dicionario?
    for chave, regra in regras_tab.items():
        for v in obs.get(chave, []):
            if not dentro(regra, v):
                raise SystemExit(
                    f"[!] amostra real {v} fora da regra de "
                    f"{chave[0]}/{chave[1]:08x}/{chave[2]} — o dicionario "
                    f"mentiu, ou o aparelho aceitou o que a Suite nao declara"
                )

    n_knob = sum(1 for r in regras_tab.values() if r["kind"] == "range")
    n_disc = len(regras_tab) - n_knob
    dentro_n = sum(
        1
        for chave, regra in regras_tab.items()
        if obs.get(chave)
    )
    total_amostras = sum(len(v) for v in obs.values())

    return {
        "version": 1,
        "motivo": (
            "trava de conteudo do set-param: o firmware V2.1 asserta em "
            "para <= GetParaMaxVal (audio.c:1828) e derruba o aparelho sem "
            "aviso no fio"
        ),
        "issue": "#110",
        "adr": "ADR-10",
        "policy": (
            "regra do dicionario manda; par sem regra aceita; NaN/inf sempre "
            "recusado"
        ),
        "sources": {
            "dictionary": {
                "path": "analysis/parameters.json",
                "sha256": sha256(DICIONARIO),
                "provenance": "algorithm.xml oficial da Suite (V1.5.1)",
            },
            "capture": {
                "path": "analysis/fixtures/knobs.jsonl",
                "sha256": sha256(CAPTURA),
                "provenance": "captura real da Suite no GP-100 (sessao S3)",
            },
        },
        "counts": {
            "rules": len(regras_tab),
            "rules_knob": n_knob,
            "rules_discrete": n_disc,
            "observed_pairs": len(obs),
            "observed_pairs_with_rule": dentro_n,
            "observed_pairs_without_rule": len(sem_regra),
            "observed_samples": total_amostras,
        },
        "pairs": pares,
        "observed_without_rule": sem_regra,
    }


def checar(artefato: dict, atual: dict) -> list[str]:
    """Diferencas entre o artefato versionado e o que os arquivos dizem hoje."""
    erros: list[str] = []

    for chave in ("sources", "counts", "pairs", "observed_without_rule"):
        if artefato.get(chave) != atual[chave]:
            if chave == "sources":
                erros.append(
                    "os arquivos de origem mudaram (sha256) — regere com "
                    "`python analysis/param_ranges.py` e revise o diff"
                )
            else:
                erros.append(
                    f"a secao '{chave}' do artefato diverge do que os "
                    f"arquivos dizem hoje — regere com "
                    f"`python analysis/param_ranges.py`"
                )
    return erros


def relatorio(d: dict) -> None:
    c = d["counts"]
    print(f"param_ranges v{d['version']} — {d['policy']}")
    print(f"  dicionario {d['sources']['dictionary']['sha256'][:16]}…")
    print(f"  captura    {d['sources']['capture']['sha256'][:16]}…")
    print()
    print(f"  {c['rules']} pares com regra "
          f"({c['rules_knob']} knob faixa + {c['rules_discrete']} discretos)")
    print(f"  {c['observed_pairs']} pares observados no aparelho, "
          f"{c['observed_samples']} amostras — TODAS dentro da regra")
    print(f"  {c['observed_pairs_without_rule']} par(es) observado(s) SEM regra "
          f"(aceito por decisao; backlog do dicionario):")
    for chave, v in d["observed_without_rule"].items():
        print(f"      slot {v['slot']} {v['code']}/{v['ctrl']} "
              f"({v['samples']} amostras, {v['observed_min']}..{v['observed_max']})")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--check", action="store_true",
                    help="gate: falha se o artefato divergir dos arquivos")
    ap.add_argument("show", nargs="?", const="show",
                    help="imprime o relatorio")
    args = ap.parse_args()

    atual = derivar()

    if args.show:
        relatorio(atual)
        return 0

    if args.check:
        if not os.path.exists(ARTEFATO):
            print(f"[!] {ARTEFATO} nao existe — rode `python analysis/param_ranges.py`",
                  file=sys.stderr)
            return 1
        with open(ARTEFATO, encoding="utf-8") as f:
            artefato = json.load(f)
        erros = checar(artefato, atual)
        if erros:
            for e in erros:
                print(f"[!] {e}", file=sys.stderr)
            return 1
        c = atual["counts"]
        print(f"[ok] tabela de faixas consistente: {c['rules']} regras, "
              f"{c['observed_samples']} amostras reais dentro delas")
        return 0

    with open(ARTEFATO, "w", encoding="utf-8", newline="\n") as f:
        json.dump(atual, f, ensure_ascii=False, indent=2, sort_keys=True)
        f.write("\n")
    relatorio(atual)
    return 0


if __name__ == "__main__":
    sys.exit(main())
