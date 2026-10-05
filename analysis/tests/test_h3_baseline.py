#!/usr/bin/env python3
"""test_h3_baseline.py — suite do gate H3 (#23): normalizador, terminador
SysEx, baseline versionada e juiz da captura do core.

Todos os testes aqui sao **mutation-provados**: cada um quebra o valor que
protege e exige a falha. A razao e o bug que este PR corrige: o golden v1.0
carregava um template (`push 13000000`) e 63 mensagens da prova de save que
NAO VINHAM do fio — o trim cortava linhas de captura corrompidas e produzia
bytes plausiveis. Nenhum gate apanhava, porque os dados eram fabricados e os
padroes comparados com dados fabricados. Um gate que so verifica
"o arquivo nao mudou" nao pega isso; um gate que verifica "esse byte e esse
valor" pega.
"""
import importlib.util
import io
import json
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
ANALYSIS = os.path.join(ROOT, "analysis")
sys.path.insert(0, ANALYSIS)


class _Buf(io.StringIO):
    """StringIO que tolera o sys.stdout.reconfigure dos scripts de analysis/."""

    def reconfigure(self, *a, **k):
        pass


def run_module(filename, *args):
    """Executa um script de analysis/ capturando stdout; devolve (exit, out)."""
    spec = importlib.util.spec_from_file_location(
        filename[:-3].replace("-", "_"), os.path.join(ANALYSIS, filename))
    mod = importlib.util.module_from_spec(spec)
    buf = _Buf()
    old_argv, old_stdout = sys.argv, sys.stdout
    sys.argv = [filename, *args]
    code = 0
    try:
        with _quiet_stdout(buf):
            spec.loader.exec_module(mod)
            main = getattr(mod, "main", None)
            if main is not None:
                code = main()
    except SystemExit as e:
        code = e.code if isinstance(e.code, int) else 1
    finally:
        sys.argv, sys.stdout = old_argv, old_stdout
    return code, buf.getvalue()


def _quiet_stdout(buf):
    import contextlib

    @contextlib.contextmanager
    def cm():
        with contextlib.redirect_stdout(buf):
            yield
    return cm()


import contextlib  # noqa: E402
import wirelog  # noqa: E402

HDR = wirelog.HDR


def frame(func: str, addr: str, data: str) -> str:
    """Monta um frame SysEx completo (envelope + terminador)."""
    return f"{HDR}{func}{addr}{data}f7"


# ════════════════════════════════════ TERMINADOR SysEx
# O bug que motivou o PR inteiro. Nomes de preset em ASCII quase sempre tem um
# par de nibbles "f7" em algum lugar ("World" = 576f726c64 contem "f7" em
# 6f72; 'o' seguido de 'p'..'z' e o caso comum). Cortar no PRIMEIRO "f7" corta
# o frame no meio.

def test_trim_nao_corta_frame_completo():
    """Um frame que TERMINA em f7 esta inteiro — ponto."""
    f = frame("12", "11000008", "00010000" + "576f726c64" + "0000000000")
    assert wirelog.trim(f) == f, "frame completo nao pode ser cortado"


def test_trim_preserva_nome_com_f7_no_meio():
    """"World", "Acoustic" e "Country" sao os nomes que dispararam o bug."""
    casos = [
        ("00010000" + b"World".hex() + "0000000000"),
        ("000a0000" + b"Acoustic".hex() + "0000"),
        ("00030000" + b"Country".hex() + "000000"),
    ]
    for payload in casos:
        f = frame("12", "11000008", payload)
        got = wirelog.parse_hex_frame(f)
        assert got is not None, f"payload com 'f7' no meio nao parseou: {payload}"
        assert got[2] == payload, (
            f"payload truncado: esperado {payload}, veio {got[2]}")


def test_trim_preserva_byte_f7_que_e_dado():
    """Um byte 0xF7 no payload e DADO, nao terminador."""
    payload = "010e0000" + "00" * 9 + "f7" + "00" * 6
    f = frame("12", "11000008", payload)
    got = wirelog.parse_hex_frame(f)
    assert got is not None and got[2] == payload, "byte f7 de dados foi tratado como fim"


def test_linha_sem_f7_final_e_rejeitada():
    """O outro meio do bug: as capturas tem linhas cortadas pelo proxy.

    Rebuilding com rfind('f7') produz payloads de 196 bytes que nunca
    existiram no fio. Linha sem F7 final nao e frame — e saida truncada.
    """
    cortada = HDR + "12" + "13000000" + "00" * 30  # sem f7
    assert wirelog.trim(cortada) is None, "linha sem F7 final foi aceita"
    assert wirelog.parse_hex_frame(cortada) is None, "frame fabricado a partir de linha cortada"


def test_linha_cortada_nao_gera_payload_fabricado():
    """O modo de falha concreto: como o golden v1.0 produziu um template falso.

    A linha real que o proxy deixou (session4, `in 11000008`) e um buffer de
    256 bytes que comeca em `010e` + zeros, tem um byte `f7` no MEIO (que e
    dado) e acaba em `ff` sem F7 final. O `find("f7")` cortava ali e o
    `body()` devolvia 14 bytes — `010e000000000000000000000000` — que e
    exatamente a forma de um registro de usuario, e passou na regra do
    `validate_golden`. Este teste usa essa linha como esta.
    """
    payload = "010e" + "00" * 12 + "f7" + "00" * 4 + "ff" * 40  # sem F7 final
    cortada = HDR + "12" + "11000008" + payload
    assert not cortada.endswith("f7")

    # 1) a CORRETA recusa a linha
    assert wirelog.parse_hex_frame(cortada) is None, "aceitou linha cortada"

    # 2) e o que o codigo antigo fazia, para o teste documentar o estrago real
    i = cortada.find("f7")
    antigo = cortada[: i + 2]
    antigo_payload = antigo[26:-2] if antigo.endswith("f7") else antigo[26:]
    if len(antigo_payload) % 2:  # body() cortava o byte ímpar sobrando
        antigo_payload = antigo_payload[:-1]
    assert antigo_payload == "010e000000000000000000000000", (
        f"premissa do teste mudou: o antigo produzia {antigo_payload}")
    assert len(antigo_payload) // 2 == 14, "14B = o formato do registro de usuário"


# ════════════════════════════════════ NORMALIZADOR (Suite + P4)

def test_le_captura_do_suite_e_log_do_core():
    """O objetivo do modulo: os dois formatos viram a MESMA forma."""
    evs_suite, _ = wirelog.load(os.path.join(ANALYSIS, "captures", "session1.jsonl"))
    evs_p4, _ = wirelog.load(os.path.join(ANALYSIS, "fixtures", "boot.jsonl"))
    assert evs_suite, "captura do Suite nao carregou"
    assert evs_p4, "log P4 nao carregou"
    for evs in (evs_suite, evs_p4):
        e = evs[0]
        assert set(e) == {"t", "dir", "func", "addr", "data"}, e
        assert e["dir"] in ("out", "in")
        assert len(e["func"]) == 2 and len(e["addr"]) == 8


def test_log_do_core_carrega_eventos():
    """Regressão do H3: o build_golden via 0 eventos num log do core.

    Feito antes da #23, o `load` do build_golden aceitava so `out_long`/
    `in_long` + `hex`, e um log do gp100-core (P4: `out` + func/addr/data)
    carregava ZERO eventos — sem erro, sem aviso, so um golden degenerado.
    """
    evs, diag = wirelog.load(os.path.join(ANALYSIS, "fixtures", "knobs.jsonl"))
    assert len(evs) > 0, "log P4 do core carregou 0 eventos (o bug do H3)"
    assert diag["formato"] == "p4"
    assert diag["nao_eventos"] == 0, diag


def test_rebuild_devolve_o_envelope_original():
    """ida e volta sem perda — o que permite alimentar o pipeline de spec."""
    f = frame("12", "11000008", "00010000" + b"World".hex() + "0000000000")
    func, addr, data = wirelog.parse_hex_frame(f)
    assert wirelog.rebuild(func, addr, data) == f, "rebuild nao devolveu o frame original"


def test_relogio_ausente_e_reportado():
    """Sem `t` nao da para segmentar nem casar OUT->IN. E preciso dizer."""
    evs, diag = wirelog.load(os.path.join(ANALYSIS, "captures", "session1.jsonl"))
    assert diag["sem_relogio"] == 0, "captura do Suite tem relogio"
    assert wirelog.segmentos(evs), "captura com relogio deveria segmentar"
    # e um log sem relogio NAO pode fingir que sabe segmentar:
    sem_t = [{**e, "t": None} for e in evs[:50]]
    assert wirelog.segmentos(sem_t) == [], "inventou fases sem relogio"


# ════════════════════════════════════ BASELINE VERSIONADA

def test_baseline_check_passa_no_estado_do_repo():
    code, out = run_module("baseline.py", "--check")
    assert code == 0, f"baseline reprovada:\n{out}"


def test_baseline_recusa_hash_divergente(tmp_path=None):
    """Gate novo é inútil sem provar que ele falha."""
    reg_path = os.path.join(ANALYSIS, "baseline.json")
    original = open(reg_path, encoding="utf-8").read()
    try:
        reg = json.loads(original)
        reg["hash"] = "0" * 64
        with open(reg_path, "w", encoding="utf-8", newline="\n") as f:
            json.dump(reg, f, ensure_ascii=False, indent=2)
        code, out = run_module("baseline.py", "--check")
        assert code == 1, f"hash divergente passou (exit {code}):\n{out}"
        assert "sem bump" in out, out
    finally:
        with open(reg_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(original)


def test_baseline_recusa_motivo_vazio():
    """O motivo é o que separa 'corrigi um bug' de 'rodei o script'."""
    reg_path = os.path.join(ANALYSIS, "baseline.json")
    original = open(reg_path, encoding="utf-8").read()
    try:
        reg = json.loads(original)
        reg["motivo"] = "   "
        with open(reg_path, "w", encoding="utf-8", newline="\n") as f:
            json.dump(reg, f, ensure_ascii=False, indent=2)
        code, out = run_module("baseline.py", "--check")
        assert code == 1, f"motivo vazio passou:\n{out}"
        assert "motivo" in out, out
    finally:
        with open(reg_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(original)


def test_bump_sem_motivo_e_recusado():
    code, out = run_module("baseline.py", "bump")
    assert code == 2, f"bump sem --motivo passou (exit {code}):\n{out}"
    assert "motivo" in out, out


def test_registro_tem_historico_com_motivo_e_data():
    reg = json.load(open(os.path.join(ANALYSIS, "baseline.json"), encoding="utf-8"))
    assert reg["versao"], "sem versao"
    assert reg["motivo"].strip(), "versao atual sem motivo"
    hist = reg["historico"]
    assert hist, "historico vazio"
    for h in hist:
        assert h["motivo"].strip(), f"historico {h['versao']} sem motivo"
        assert h["data"].strip(), f"historico {h['versao']} sem data"
        assert len(h["hash"]) == 64, f"historico {h['versao']} sem hash"


# ════════════════════════════════════ JUIZ DA CAPTURA DO CORE

def _log_do_core_temporario(tmp, alterar=None):
    """Fabrica um log P4 minimo e realista (metas do core)."""
    linhas = [
        {"s": "H3", "t": 7.1, "dir": "out", "func": "11", "addr": "13010000", "data": "0000"},
        {"s": "H3", "t": 7.6, "dir": "in", "func": "12", "addr": "13010001", "data": "00000c1c0140"},
    ]
    if alterar:
        alterar(linhas)
    with open(tmp, "w", encoding="utf-8") as f:
        for e in linhas:
            f.write(json.dumps(e, ensure_ascii=False) + "\n")
    return tmp


def test_juiz_apova_captura_em_conformidade(tmp_path):
    p = str(tmp_path / "ok.jsonl")
    _log_do_core_temporario(p)
    code, out = run_module("validate_core_capture.py", p)
    assert code == 0, f"captura conforme foi reprovada:\n{out}"
    assert "2/2" in out, out


def test_juiz_pega_payload_divergente(tmp_path):
    """Um byte trocado num segmento const tem de aparecer."""
    def muda(ls):
        ls[1]["data"] = "00000c1c0141"  # ultimo byte 40 -> 41
    p = str(tmp_path / "bad_payload.jsonl")
    _log_do_core_temporario(p, muda)
    code, out = run_module("validate_core_capture.py", p)
    assert code == 1, f"payload divergente nao foi pego (exit {code}):\n{out}"
    assert "DIVERGEM" in out, out


def test_juiz_pega_endereco_fora_da_spec(tmp_path):
    """Endereco que o golden nao descreve é captura nova, nao bug de codigo."""
    def injeta(ls):
        ls.append({"s": "H3", "t": 9.9, "dir": "out", "func": "12",
                   "addr": "deadbeef", "data": "00"})
    p = str(tmp_path / "fora.jsonl")
    _log_do_core_temporario(p, injeta)
    code, out = run_module("validate_core_capture.py", p)
    assert code == 2, f"endereco fora da spec nao foi pego (exit {code}):\n{out}"
    assert "FORA DA SPEC" in out, out


def test_juiz_nao_aceita_formato_ilegivel(tmp_path):
    """0 eventos é erro de ENTRADA (exit 3), nao 'spec cobriu tudo'."""
    p = str(tmp_path / "vazio.jsonl")
    open(p, "w", encoding="utf-8").write("isto nao e jsonl\n")
    code, out = run_module("validate_core_capture.py", p)
    assert code == 3, f"arquivo ilegivel deu exit {code}, esperava 3:\n{out}"
    assert "0 EVENTOS" in out, out


def test_build_golden_nao_fabrica_frames():
    """O golden v1.0 nao pode mais conter template so de dado corrompido.

    `push 13000000` saia com um const de 242 bytes que era o prefixo de uma
    captura cortada. Este teste e o guarda: se alguem reintroduzir a
    reconstrucao por rfind, o golden volta a ter template sem evidencia.
    """
    g = json.load(open(os.path.join(ROOT, "docs", "protocol_golden.json"), encoding="utf-8"))
    addrs = {(t.get("out") or t.get("in") or {}).get("addr") for t in g["transactions"]}
    assert "13000000" not in addrs, (
        "push 13000000 voltou ao golden sem captura completa: a resposta real "
        "a esse endereco nunca foi capturada (o proxy corta em 256 bytes). "
        "Ver docs/PROTOCOL.md 13.14."
    )
    excl = g["_meta"].get("excluded", {})
    assert "linhas_cortadas" in excl, (
        "golden nao declara quantas linhas cortadas descartou — sem isso, "
        "alguem reintroduz a fabricacao sem ninguem ver"
    )