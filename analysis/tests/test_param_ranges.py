"""test_param_ranges.py — a trava de conteudo do knob se prova por MUTACAO (#110).

O que aconteceu em campo (05/10/2026, gate H2, GP-100 V2.1) esta na
docstring de `analysis/param_ranges.py` e vale a repeticao curta: o runbook
mandava um valor inventado, o firmware V2.1 tem o assert
`para <= GetParaMaxVal` (`Drivers/audio/audio.c:1828`), e ao assertar ele
**para de responder a toda transacao** — inclusive as leituras. O device
continua enumerado e `OK` no Windows; a recuperacao e um power-cycle fisico.
Como o `set-param` e fire-and-forget (§13.11, D4), o fio nao avisa nada.

Estes testes sao a versao Python da mesma prova que `packages/core/tests/
value_gate.rs` faz em Rust. Aqui a travessia e a **tabela**; la, e o
**transporte**. Os dois precisam: tabela errada deixa passar valor perigoso,
e tabela certa com a trava no lugar errado ainda deixa o byte sair.
"""
from __future__ import annotations

import importlib.util
import pathlib
import sys

import pytest

RAIZ = pathlib.Path(__file__).resolve().parents[2]


def _carrega():
    """Importa `analysis/param_ranges.py` como modulo (o nome tem caminho)."""
    spec = importlib.util.spec_from_file_location(
        "param_ranges", RAIZ / "analysis" / "param_ranges.py"
    )
    assert spec and spec.loader
    mod = importlib.util.module_from_spec(spec)
    sys.modules["param_ranges"] = mod
    spec.loader.exec_module(mod)
    return mod


pr = _carrega()


@pytest.fixture(scope="module")
def regras():
    return pr.regras()


@pytest.fixture(scope="module")
def amostras():
    return pr.amostras()


# ─── o numero que derrubou o aparelho ──────────────────────────────────────

def test_o_99_5_do_runbook_e_recusado(regras):
    """`Bog RedM` Gain = (slot 3, 0x0700006e, ctrl 0), faixa 0..99.

    O `99.5` do runbook e a razao de a #110 existir. Se este teste falhar,
    a trava em Rust provavelmente tambem.
    """
    regra = regras[(3, 0x0700_006E, 0)]
    assert regra["kind"] == "range"
    assert (regra["min"], regra["max"]) == (0.0, 99.0)
    assert not pr.dentro(regra, 99.5)


def test_o_15_0_do_runbook_corrigido_passa(regras):
    """O valor real da captura (`knobs.jsonl` linha 1) tem que passar."""
    assert pr.dentro(regras[(3, 0x0700_006E, 0)], 15.0)


# ─── a prova empirica: 92 amostras reais, zero contradicao ─────────────────

def test_toda_amostra_real_cabe_na_regra(regras, amostras):
    """A premissa inteira do ADR-10 mora aqui.

    Se uma amostra real caísse fora da regra do dicionario, a politica
    "o dicionario manda" estaria errada e o gate precisa dizer isso —
    em vez de deixar o Rust recusar um valor que o aparelho ja aceitou.
    """
    fora = []
    for (slot, code, ctrl), valores in amostras.items():
        regra = regras.get((slot, code, ctrl))
        if regra is None:
            continue
        for v in valores:
            if not pr.dentro(regra, v):
                fora.append((slot, code, ctrl, v))
    assert fora == [], f"amostras reais fora da regra declarada: {fora}"


def test_trez_e_nove_pareis_tem_regra(regras):
    """Cobertura: a trava vale para TODO controle do dicionario.

    Antes o ADR previa 636; com o slot na chave os 3 pares que colidiam
    (`Boost`/`14 Boost`, mesmo effectCode em PRE e DST) se separam, e a
    cobertura fecha em 639 = 639 controles.
    """
    assert len(regras) == 639


def test_o_slot_entra_na_chave(regras):
    """`Boost` e `14 Boost`: mesmo effectCode, PRE (slot 1) e DST (slot 2).

    Se a chave fosse so `(code, ctrl)`, a regra do PRE contaminaria o DST.
    """
    assert (1, 0x0000_001A, 0) in regras
    assert (2, 0x0000_001A, 0) in regras


# ─── o par sem regra: o backlog honesto do dicionario ───────────────────────

def test_o_knob_do_cab_sem_regra_e_o_unico(amostras, regras):
    """`0x0a00002c`/ctrl 1 — o aparelho varreu 0..99 e o dicionario nao descreve.

    Este e o unico par observado sem regra, e ele existe por um motivo
    concreto: o aparelho provou que o knob funciona, e recusar quebraria
    justamente ele. A politica do ADR-10 aceita por isso — e o gate nomeia o
    par para que o buraco do dicionario fique visivel em vez de sumir.
    """
    sem = [k for k in amostras if k not in regras]
    assert sem == [(5, 0x0A00_002C, 1)], sem
    valores = amostras[(5, 0x0A00_002C, 1)]
    assert min(valores) == 0.0 and max(valores) == 99.0
    assert len(valores) == 8


# ─── a forma da regra: chave discreta e faixa normalizada ───────────────────

def test_chave_discreta_aceita_so_id_declarado(regras):
    """`Boost/Bright` e `[1, 0]`: 0.5 nao e um id, mesmo "dentro" de 0..1."""
    regra = regras[(1, 0x0000_001A, 1)]
    assert regra["kind"] == "discrete"
    assert sorted(regra["ids"]) == [0, 1]
    assert pr.dentro(regra, 1.0)
    assert pr.dentro(regra, 0.0)
    assert not pr.dentro(regra, 0.5)


def test_faixa_bidirecional_e_normalizada(regras):
    """Knobs bidirecionais vem com min > max (`Pitch.L-Pitch` = 0..-24)."""
    violadas = [
        k for k, r in regras.items() if r["kind"] == "range" and r["min"] > r["max"]
    ]
    assert violadas == [], violadas


# ─── o artefato versionado ─────────────────────────────────────────────────

def test_o_artefato_bate_com_os_arquivos():
    """O gate de verdade: o `param_ranges.json` tem que refletir hoje."""
    import json

    artefato = json.loads(
        (RAIZ / "analysis" / "param_ranges.json").read_text(encoding="utf-8")
    )
    erros = pr.checar(artefato, pr.derivar())
    assert erros == [], erros


def test_o_artefato_guarda_o_hash_das_duas_fontes():
    """R2/R3: a proveniencia fica no artefato, nao num comentario."""
    import json

    artefato = json.loads(
        (RAIZ / "analysis" / "param_ranges.json").read_text(encoding="utf-8")
    )
    fontes = artefato["sources"]
    assert fontes["dictionary"]["sha256"] == pr.sha256(pr.DICIONARIO)
    assert fontes["capture"]["sha256"] == pr.sha256(pr.CAPTURA)
    assert artefato["issue"] == "#110"
    assert artefato["adr"] == "ADR-10"
