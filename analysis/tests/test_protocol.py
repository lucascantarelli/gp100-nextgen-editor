#!/usr/bin/env python3
"""test_protocol.py — suíte de regressão do protocolo GP-100 (P3 do ROADMAP).

Gate de qualidade: qualquer mudança em spec (golden), decoders ou capturas
passa por aqui. Critérios = "verde" definidos na skill .agents/skills/protocol-validate:
  - validate_golden: IN 100% / OUT >= 99.5% (accounting); B–E = 100% (geração)
  - validate_knob_map: 13/14 OK (exceção conhecida: CAB/Mic, controle oculto)
"""
import importlib.util
import io
import os
import re
import sys
import contextlib

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
ANALYSIS = os.path.join(ROOT, "analysis")
sys.path.insert(0, ANALYSIS)
sys.stdout.reconfigure(errors="replace")

# ---------------------------------------------------------------- helpers
class _Buf(io.StringIO):
    """StringIO que tolera sys.stdout.reconfigure() dos scripts de analysis/."""
    def reconfigure(self, *a, **k):
        pass

def run_module(filename, *args):
    """executa um script de analysis/ capturando stdout; retorna (exitcode, out)."""
    spec = importlib.util.spec_from_file_location(
        filename[:-3].replace("-", "_"), os.path.join(ANALYSIS, filename))
    mod = importlib.util.module_from_spec(spec)
    buf = _Buf()
    old_argv = sys.argv
    sys.argv = [filename, *args]
    code = 0
    try:
        with contextlib.redirect_stdout(buf):
            spec.loader.exec_module(mod)
            main = getattr(mod, "main", None)
            if main is not None:
                code = main() or 0
    except SystemExit as e:
        code = e.code if isinstance(e.code, int) else 0
    finally:
        sys.argv = old_argv
    return code, buf.getvalue()

# ---------------------------------------------------------------- A. golden
def _load_golden_summary():
    code, out = run_module("validate_golden.py")
    assert code == 0, f"validate_golden saiu com código {code}\n{out[-2000:]}"
    m = re.search(r"== RESUMO ==\n(.*)$", out, re.S)
    assert m, "RESUMO não encontrado na saída do validate_golden"
    summary = m.group(1)
    nums = {}
    for key, pat in (("A_out", r"A accounting: OUT ([\d.]+)% \| IN ([\d.]+)%"),
                     ("A_in", r"A accounting: OUT ([\d.]+)% \| IN ([\d.]+)%"),
                     ("C", r"C boot gen:\s+(\d+)/(\d+)"),
                     ("D", r"D save gen:\s+(\d+)/(\d+)"),
                     ("E", r"E IR upload:\s+(\d+)/(\d+)")):
        mm = re.search(pat, summary)
        assert mm, f"padrão {key} não achado no RESUMO:\n{summary}"
        nums[key] = mm
    # knobs: linha "B knob gen:   S3 89/89 | S1 3/3"
    mb = re.search(r"B knob gen:\s+S3 (\d+)/(\d+) \| S1 (\d+)/(\d+)", summary)
    assert mb, f"linha B não achada:\n{summary}"
    nums["B"] = mb
    return summary, nums

def test_golden_accounting_in_100():
    _, nums = _load_golden_summary()
    assert float(nums["A_in"].group(2)) == 100.0, "accounting IN < 100%"

def test_golden_accounting_out_min_99_5():
    _, nums = _load_golden_summary()
    assert float(nums["A_out"].group(1)) >= 99.5, "accounting OUT < 99.5%"

def test_golden_knobs_100():
    _, nums = _load_golden_summary()
    b = nums["B"]
    assert (b.group(1), b.group(2)) == ("89", "89"), "knobs S3 != 89/89"
    assert (b.group(3), b.group(4)) == ("3", "3"), "knobs S1 != 3/3"

def test_golden_boot_100():
    _, nums = _load_golden_summary()
    assert nums["C"].group(1) == nums["C"].group(2) == "2299", "boot gen != 2299/2299"

def test_golden_save_100():
    _, nums = _load_golden_summary()
    assert nums["D"].group(1) == nums["D"].group(2) == "77", "save gen != 77/77"

def test_golden_ir_100():
    _, nums = _load_golden_summary()
    assert nums["E"].group(1) == nums["E"].group(2) == "1186", "IR gen != 1186/1186"

def test_golden_baseline_hash_unchanged():
    """P1: baseline congelada — golden não pode mudar sem spec-baseline."""
    import hashlib
    golden = os.path.join(ROOT, "docs", "protocol_golden.json")
    h = hashlib.sha256(open(golden, "rb").read()).hexdigest()
    assert h == "0426d6a8843c57d16ab917170ed5f50eb95026281f1f460f4c2520aaeefef859", (
        "protocol_golden.json mudou sem bump de baseline (regra R2/R3)!\n"
        "Fluxo: captura -> build_golden -> validate 100% -> novo hash no §13.")

# ---------------------------------------------------------------- fixtures (P4)
def test_fixtures_parity():
    """P4: fixtures regeneradas batem com o golden (paridade do manifest)."""
    code, out = run_module("make_fixtures.py")
    assert code == 0, f"make_fixtures falhou\n{out[-2000:]}"
    assert "[DIF" not in out, "fixtures fora de paridade com o golden"
    assert "Paridade 100%" in out, out[-500:]

# ---------------------------------------------------------------- B. knob_map
def test_knob_map_13_of_14():
    code, out = run_module("validate_knob_map.py")
    assert code == 0, f"validate_knob_map saiu com código {code}\n{out[-2000:]}"
    groups = re.findall(
        r"^  (\w+)\s+ctrl(\d+)\s+alg=(\S+|\?)\s+.*?(OK|FORA|ALG-\?|CTRL-\?)",
        out, re.M)
    assert len(groups) == 14, f"esperava 14 grupos, achei {len(groups)}"
    bad = [(m, c) for m, c, _a, st in groups if st not in ("OK", "CTRL-?")]
    assert not bad, f"grupos fora do range do dicionário: {bad}"
    # a única exceção permitida é CAB ctrl1 (Mic, controle oculto)
    ctrl_q = [(m, c) for m, c, _a, st in groups if st == "CTRL-?"]
    assert ctrl_q == [("CAB", "1")], f"exceção inesperada: {ctrl_q}"

def test_knob_map_preset_matched():
    code, out = run_module("validate_knob_map.py")
    assert "PRESET CASADO" in out or "PRESET:" in out, "preset editado não foi casado com .prst"
