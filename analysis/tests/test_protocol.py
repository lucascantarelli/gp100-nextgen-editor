#!/usr/bin/env python3
"""test_protocol.py — suíte de regressão do protocolo GP-100 (P3 do ROADMAP).

Gate de qualidade: qualquer mudança em spec (golden), decoders ou capturas
passa por aqui. Critérios = "verde" definidos na skill .agents/skills/protocol-validate:
  - validate_golden: IN 100% / OUT >= 99.5% (accounting); B–E = 100% (geração)
  - validate_knob_map: 13/14 OK (exceção conhecida: CAB/Mic, controle oculto)
"""
import importlib.util
import io
import json
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
    """Prova D: o save.

    Era 77/77. **63 dessas 77 mensagens eram fabricaradas** (#23): o lado IN
    (resync `11000008` x31, resync `12001002` x32, status `12000001`) veio de
    linhas de captura corrompidas — o proxy morreu no flush e deixou 100
    linhas de 256 bytes preenchidas com `ff`, sem `F7` final. O trim antigo
    (`find("f7")`) cortava DENTRO do buffer e produzia um payload de 14 bytes
    com cara de registro de usuário, que passava na regra e entrava na fixture.

    Entao o alvo e 14: o lado OUT (9 frames do S4 + 5 do S2, derivados do
    `.prst`), que e byte-a-byte e continua provado. O lado IN e um GAP de
    captura — `docs/PROTOCOL.md` §13.14 e o runbook de campo. Este numero NAO
    volta a 77 por ajuste de codigo: volta com captura nova (R2).
    """
    _, nums = _load_golden_summary()
    assert nums["D"].group(1) == nums["D"].group(2) == "14", (
        "save gen != 14/14 — o lado OUT do save (9 do S4 + 5 do S2) e o que "
        "foi capturado; o lado IN virou gap em #23 e so volta com captura nova"
    )

def test_golden_ir_100():
    _, nums = _load_golden_summary()
    assert nums["E"].group(1) == nums["E"].group(2) == "1186", "IR gen != 1186/1186"

def test_golden_baseline_hash_unchanged():
    """P1: baseline congelada — golden nao pode mudar sem spec-baseline.

    O hash NAO mora mais aqui como literal (#23): mora em
    `analysis/baseline.json`, que alem do hash carrega versao, data, motivo e
    historico. Um literal aqui verificava que o arquivo nao mudou, mas nao
    que alguem explicou POR QUE — e era so colar o hash novo para passar.
    """
    code, out = run_module("baseline.py", "--check")
    assert code == 0, (
        f"baseline reprovada (regra R2/R3):\n{out}")


def test_baseline_tem_motivo_em_cada_versao():
    """O registro que a #23 criou nao pode virar um hash com historia vazia.

    Nao e teste de `baseline.py --check` (esse roda acima): e o teste de que
    o ARQUIVO tem conteudo. Um `--check` que so valida o hash passaria com
    um historico de motivacoes em branco.
    """
    reg = os.path.join(ROOT, "analysis", "baseline.json")
    data = json.load(open(reg, encoding="utf-8"))
    assert data.get("versao"), "registro sem versao"
    assert data.get("motivo", "").strip(), "versao atual sem motivo"
    assert data.get("data", "").strip(), "versao atual sem data"
    hist = data.get("historico") or []
    assert hist, "historico vazio: a primeira baseline precisa entrar nele"
    for h in hist:
        assert h.get("motivo", "").strip(), f"historico {h.get('versao')} sem motivo"
        assert h.get("data", "").strip(), f"historico {h.get('versao')} sem data"
        assert len(str(h.get("hash", ""))) == 64, f"historico {h.get('versao')} sem hash"

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
