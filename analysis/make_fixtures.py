#!/usr/bin/env python3
"""make_fixtures.py — fatia as 4 capturas em fixtures de replay (ROADMAP P4).

Gera analysis/fixtures/ a partir dos loaders de build_golden.py (repair de aspa,
trim no 1º F7, body()), SEM tocar os .jsonl originais. Formato: JSONL compacto,
1 mensagem por linha, na ordem cronológica do log:

  {"s":"S2","t":80012.3,"dir":"out","func":"12","addr":"12001002","data":"00..."}

Fases (janelas IDÊNTICAS às provas de validate_golden.py — paridade por contrato):
  boot  (S1)  t<=30s, TODAS as mensagens com body          -> OUT 2299 (prova C)
  knobs       S3 t>30s writes 10xx0002 (89) + S1 30<t<50s
              addr 10010002 (3, §13.4)                     -> 92   (prova B)
  save        S4 OUT 1660-1670s + resync IN 11000008 t>1670s
              + status IN 12000001 t>1670s;
              S2 OUT 145-155s + resync IN 12001002 155-165s -> 77  (prova D)
  ir    (S2)  2x BEGIN 10050001 (±0.5s) + chunks 33B [t0,t1)
              + ACKs 4B [t0,t1+0.5s)                       -> 1186 (prova E)

Mensagens SEM-HDR (ring buffer do proxy truncado) são EXCLUÍDAS e contadas.
O manifest.json grava contagens, paridade vs golden e o sha256 do golden usado:
se o golden mudar (fluxo R2/R3), as fixtures TÊM de ser regeneradas.

Nota: cada captura guarda o próprio save — janelas S4 em session4.jsonl e S2 em
session2.jsonl (idem prova D). session4.jsonl contém a S3 inteira (append-only):
separar sessões por gaps >30s.
"""
import hashlib
import importlib.util
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location("build_golden", os.path.join(HERE, "build_golden.py"))
BG = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(BG)

CAPS = {f"S{i}": os.path.join(HERE, "captures", f"session{i}.jsonl") for i in (1, 2, 3, 4)}
FIXTURES = os.path.join(HERE, "fixtures")
GOLDEN = os.path.join(HERE, os.pardir, "docs", "protocol_golden.json")
# O hash da baseline NÃO mora aqui como literal (#23): vem de
# `analysis/baseline.json`, que é o registro com versão, data, motivo e
# histórico. Um literal aqui obrigava a colar o hash novo em DOIS lugares
# (aqui e no teste) e nenhum dos dois exigia justificativa.
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)),
                       "baseline.json"), encoding="utf-8") as _fh:
    GOLDEN_BASELINE_SHA = json.load(_fh)["hash"]
KNOB_MAP = os.path.join(HERE, "knob_map.json")

# Alvos de paridade (DoD P4 — números provados pelo validate_golden.py / ROADMAP)
EXPECT_BOOT_OUT = 2299   # prova C
EXPECT_KNOBS_S3 = 89     # prova B (S3)
EXPECT_KNOBS_S1 = 3      # prova B (S1, §13.4)
# Prova D: era 77. **63 dessas mensagens eram fabricaradas** (#23).
# O lado OUT (9 frames do S4 + 5 do S2, derivados do .prst) continua byte-a-byte
# e e o que o alvo mede. O lado IN (resync 11000008 x31, resync 12001002 x32,
# status 12000001) veio de linhas de captura CORROPIDAS: o proxy morreu no flush
# e deixou 100 linhas de 256 bytes preenchidas com ff, sem F7 final. O trim
# antigo (`find("f7")`) cortava DENTRO do buffer e fabricava um payload de 14
# bytes com cara de registro de usuário — que passava na regra e entrava na
# fixture. Nao lowering do alvo: o alvo antigo media dados que nao existem.
# O que falta esta em PROTOCOL.md §13.14 e no runbook de campo.
EXPECT_SAVE = 14         # prova D: só o lado OUT, que é o que foi capturado
EXPECT_IR = 1186         # prova E = 2 BEGIN + 592 chunks + 592 ACKs


def _t(e, ts0):
    return e["ts"] - ts0


def _rec(sess, e, ts0, b):
    return {"s": sess, "t": round(_t(e, ts0), 1),
            "dir": "out" if e["dir"] == "out_long" else "in",
            "func": b[0], "addr": b[1], "data": b[2].hex()}


def _write_jsonl(path, recs):
    with open(path, "w", encoding="utf8", newline="\n") as fh:
        for r in recs:
            fh.write(json.dumps(r, ensure_ascii=False, separators=(",", ":")) + "\n")


# ------------------------------------------------------------------ fases
def build_boot():
    """Boot/scan: S1 inteira até t<=30s — TODAS as mensagens com body."""
    evs = BG.load(CAPS["S1"])
    ts0 = evs[0]["ts"]
    recs = []
    st = {"session": "S1", "window": "t <= 30s", "out": 0, "in": 0, "semi_hdr_skipped": 0}
    for e in evs:
        t = _t(e, ts0)
        if t > 30000:
            continue
        b = BG.body(e)
        if not b:
            st["semi_hdr_skipped"] += 1
            continue
        recs.append(_rec("S1", e, ts0, b))
        st["out" if e["dir"] == "out_long" else "in"] += 1
    return recs, st


def build_knobs():
    """Edits: writes 10xx0002 — S3 t>30s (89 do knob_map) + S1 30<t<50s (3 do §13.4)."""
    recs = []
    st = {"s3_writes": 0, "s1_doc_writes": 0}
    evs3 = BG.load(CAPS["S3"])
    ts03 = evs3[0]["ts"]
    for e in evs3:
        if e["dir"] != "out_long" or _t(e, ts03) <= 30000:
            continue
        b = BG.body(e)
        if b and b[0] == "12" and b[1].startswith("10") and b[1].endswith("0002"):
            recs.append(_rec("S3", e, ts03, b))
            st["s3_writes"] += 1
    evs1 = BG.load(CAPS["S1"])
    ts01 = evs1[0]["ts"]
    for e in evs1:
        t = _t(e, ts01)
        if e["dir"] != "out_long" or not (30000 < t < 50000):
            continue
        b = BG.body(e)
        if b and b[0] == "12" and b[1] == "10010002":
            recs.append(_rec("S1", e, ts01, b))
            st["s1_doc_writes"] += 1
    return recs, st


def build_save():
    """Save: S4 OUT 1660-1670s + resync IN 11000008 (t>1670s) + status IN 12000001;
    S2 OUT 145-155s + resync IN 12001002 (155-165s) — janelas da prova D.
    Cada captura tem o seu próprio save: janelas S4 em session4.jsonl, S2 em
    session2.jsonl (mesma técnica da prova D de validate_golden.py)."""
    evs4 = BG.load(CAPS["S4"])
    ts04 = evs4[0]["ts"]
    evs2 = BG.load(CAPS["S2"])
    ts02 = evs2[0]["ts"]
    recs = []
    st = {"s4_out": 0, "s4_resync_in_11000008": 0, "s4_status_in_12000001": 0,
          "s2_out": 0, "s2_resync_in_12001002": 0, "excluded_other": 0}
    for e in evs4:
        t = _t(e, ts04)
        b = BG.body(e)
        if e["dir"] == "out_long" and 1660000 <= t <= 1670000 and b:
            recs.append(_rec("S4", e, ts04, b)); st["s4_out"] += 1; continue
        if e["dir"] == "in_long" and t > 1670000 and b:
            if b[1] == "11000008":
                recs.append(_rec("S4", e, ts04, b)); st["s4_resync_in_11000008"] += 1; continue
            if b[1] == "12000001":
                recs.append(_rec("S4", e, ts04, b)); st["s4_status_in_12000001"] += 1; continue
        if b and 1660000 <= t <= 1680000:
            st["excluded_other"] += 1
    for e in evs2:
        t = _t(e, ts02)
        b = BG.body(e)
        if e["dir"] == "out_long" and 145000 <= t <= 155000 and b:
            recs.append(_rec("S2", e, ts02, b)); st["s2_out"] += 1; continue
        if e["dir"] == "in_long" and 155000 <= t <= 165000 and b and b[1] == "12001002":
            recs.append(_rec("S2", e, ts02, b)); st["s2_resync_in_12001002"] += 1; continue
        if b and 145000 <= t <= 165000:
            st["excluded_other"] += 1
    return recs, st


def build_ir():
    """Upload IR: S2 — BEGIN ±0.5s do instante; chunks OUT 33B [t0,t1);
    ACKs IN 4B [t0,t1+0.5s) — janelas da prova E."""
    evs = BG.load(CAPS["S2"])
    ts0 = evs[0]["ts"]
    begins = sorted(_t(e, ts0) for e in evs
                    if e["dir"] == "out_long" and (b := BG.body(e))
                    and b[0] == "12" and b[1] == "10050001")
    wins = [(t0, end) for t0, end in zip(begins, (100_000, 145_000))]
    recs = []
    per = [{"t0": t0, "begin": 0, "chunks": 0, "acks": 0} for t0, _ in wins]
    st = {"slots": per, "excluded_other": 0}
    for e in evs:
        t = _t(e, ts0)
        b = BG.body(e)
        if not b:
            continue
        for slot, (t0, t1) in enumerate(wins):
            if e["dir"] == "out_long" and b[0] == "12" and b[1] == "10050001" and abs(t - t0) <= 500:
                recs.append(_rec("S2", e, ts0, b)); per[slot]["begin"] += 1; break
            if e["dir"] == "out_long" and b[0] == "12" and b[1] == "12001002" \
                    and len(b[2]) == 33 and t0 <= t < t1:
                recs.append(_rec("S2", e, ts0, b)); per[slot]["chunks"] += 1; break
            if e["dir"] == "in_long" and b[0] == "12" and b[1] == "12001002" \
                    and len(b[2]) == 4 and t0 <= t < t1 + 500:
                recs.append(_rec("S2", e, ts0, b)); per[slot]["acks"] += 1; break
        else:
            if any(t0 - 1000 <= t < t1 + 1000 for t0, t1 in wins):
                st["excluded_other"] += 1
    st["begins"] = sum(p["begin"] for p in per)
    st["chunks"] = sum(p["chunks"] for p in per)
    st["acks"] = sum(p["acks"] for p in per)
    return recs, st


# ------------------------------------------------------------------ main
def main():
    sys.stdout.reconfigure(errors="replace")
    os.makedirs(FIXTURES, exist_ok=True)

    sha = hashlib.sha256(open(GOLDEN, "rb").read()).hexdigest()
    builders = [("boot", build_boot, "boot.jsonl"),
                ("knobs", build_knobs, "knobs.jsonl"),
                ("save", build_save, "save.jsonl"),
                ("ir", build_ir, "ir.jsonl")]

    print(f"== P4 fixtures -> {os.path.relpath(FIXTURES, os.path.join(HERE, '..'))} ==")
    files = {}
    for name, fn, fname in builders:
        recs, st = fn()
        _write_jsonl(os.path.join(FIXTURES, fname), recs)
        st["file"] = fname
        st["events"] = len(recs)
        files[name] = st
        extra = ""
        if name == "ir":
            extra = "  " + " | ".join(
                f"slot{s}: begin {p['begin']} chunks {p['chunks']} acks {p['acks']}"
                for s, p in enumerate(st["slots"]))
        print(f"  {name:6s} -> {fname:12s} {len(recs):5d} msgs  {st}{extra}")

    save = files["save"]
    save_total = (save["s4_out"] + save["s4_resync_in_11000008"]
                  + save["s2_out"] + save["s2_resync_in_12001002"])
    ir = files["ir"]
    ir_total = ir["begins"] + ir["chunks"] + ir["acks"]
    n_knob_edits = len(json.load(open(KNOB_MAP, encoding="utf8"))["edits"])

    parity = {
        "boot_out": [files["boot"]["out"], EXPECT_BOOT_OUT],
        "knobs_s3": [files["knobs"]["s3_writes"], EXPECT_KNOBS_S3],
        "knobs_s1_doc": [files["knobs"]["s1_doc_writes"], EXPECT_KNOBS_S1],
        "save_proof": [save_total, EXPECT_SAVE],
        "ir_total": [ir_total, EXPECT_IR],
        "ir_begins": [ir["begins"], 2],
        "golden_sha256": [sha, GOLDEN_BASELINE_SHA],
        "knob_map_edits": [n_knob_edits, EXPECT_KNOBS_S3],
    }
    bad = {k: v for k, v in parity.items() if v[0] != v[1]}

    print("\n== Paridade vs golden ==")
    for k, (got, want) in parity.items():
        mark = "OK " if got == want else "DIF"
        print(f"  [{mark}] {k}: {got} (esperado {want})")

    manifest = {
        "_meta": {
            "title": "Fixtures de replay do protocolo GP-100 (ROADMAP P4)",
            "generated_by": "analysis/make_fixtures.py",
            "method": "loaders de build_golden.py (repair de aspa, trim ate o F7 FINAL via "
                      "analysis/wirelog.py, body()); "
                      "janelas = provas de validate_golden.py",
            "format": "JSONL compacto, 1 mensagem por linha na ordem do log; "
                      "campos: s (sessao), t (ms desde o inicio do arquivo), "
                      "dir (out/in), func (11/12), addr (8 hex), data (payload hex)",
            "semi_hdr": "mensagens truncadas pelo ring buffer do proxy (SEM-HDR) "
                        "sao excluidas e contadas por fase",
            "golden_sha256": sha,
            "golden_baseline": GOLDEN_BASELINE_SHA,
            "reference": "docs/PROTOCOL.md §13.1-13.12; docs/ROADMAP.md P4; "
                         "docs/protocol_golden.json",
            "note": "se o golden mudar (fluxo R2/R3), REGENERAR as fixtures",
        },
        "fixtures": files,
        "parity": parity,
        "ok": not bad,
    }
    mpath = os.path.join(FIXTURES, "manifest.json")
    with open(mpath, "w", encoding="utf8", newline="\n") as fh:
        json.dump(manifest, fh, ensure_ascii=False, indent=2)
    print(f"\n  manifest: {os.path.relpath(mpath, os.path.join(HERE, '..'))}")
    if bad:
        print("  PARIDADE FALHOU - capturas/golden mudaram? Rodar build_golden + validate (R2/R3).")
        return 1
    print("  Paridade 100% - fixtures prontas para o replay do M0.6.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
