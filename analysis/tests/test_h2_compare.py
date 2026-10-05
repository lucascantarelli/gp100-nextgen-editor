#!/usr/bin/env python3
"""test_h2_compare.py — o juiz da FASE C do gate H2 se prova por MUTACAO (#22).

A regra deste arquivo e a mesma do `test_h1_compare.py`: um teste que passa
nao prova nada, um teste que FALHA quando o defeito volta prova. Por isso
quase todo teste aqui quebra de proposito uma coisa — um payload, um
endereco, a ordem, a tabela do §13 — e exige a falha correspondente.

O teste que mais importa e o `test_self_check_pega_o_ack_com_75b`: ele
documenta o erro que ACONTECEU enquanto esta feature era escrita. O ACK do
chunk de IR tem 4B (`[slot][idx u16 BE][01]`), e ele foi escrito com 75B —
que e o tamanho da RESPOSTA DA LISTA dos 20 slots, a mesma mensagem `12/12001002`
com outro conteudo. O self-check so pegou isso porque a `FORMAS_13` e
conferida contra os logs que o `MockDevice` emite, e nao contra a memoria de
quem escreve o juiz. Se alguem voltar a 75B, este teste reprova.
"""

import importlib.util
import io
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
sys.stdout.reconfigure(encoding="utf-8", errors="replace")


def _carregar():
    """Importa `scripts/h2_compare.py` como modulo (mesmo truque do test_h1).

    O registro em `sys.modules` ANTES do `exec_module` nao e opcional: o
    `dataclasses` resolve o modulo da classe por `sys.modules[cls.__module__]`
    e estoura `AttributeError` se ele nao estiver la.
    """
    caminho = ROOT / "scripts" / "h2_compare.py"
    spec = importlib.util.spec_from_file_location("h2_compare", caminho)
    mod = importlib.util.module_from_spec(spec)
    sys.modules["h2_compare"] = mod
    spec.loader.exec_module(mod)
    return mod


h2 = _carregar()


# ---------------------------------------------------------------- helpers
def _log(nome, frames):
    """Grava um log P4 e devolve o caminho."""
    caminho = ROOT / "analysis" / "h2_scratch_test" / nome
    caminho.parent.mkdir(parents=True, exist_ok=True)
    with io.open(caminho, "w", encoding="utf-8", newline="") as fh:
        for dir_, func, addr, data in frames:
            fh.write(
                json.dumps({"s": "H1", "dir": dir_, "func": func, "addr": addr, "data": data})
                + "\n"
            )
    return caminho


def _out(addr, data):
    return ("out", "12", addr, data)


def _in(addr, data):
    return ("in", "12", addr, data)


def _z(n):
    return "00" * n


# F1: o frame do set-param tal como o `MockDevice` o emitiu no ensaio.
F1_OK = [_out("10010002", "060e00000000000700000000000000000c070402")]
# F2: os 9 frames do save, na ordem do §13.12 (op0, op0, op1, op1 em BE).
F2_OK = [
    _out("11000000", _z(4) + "0000" + _z(2) + "4832205445535445" + _z(4)),
    _out("11000004", _z(20)),
    _out("11000005", "00040000"),
    _out("11000007", _z(50)),
    _out("12000002", _z(8)),
    _out("00020000", "0000000000000000"),
    _out("00020000", "0000000000000000"),
    _out("00020000", "0000000000010000"),
    _out("00020000", "0000000000010000"),
]
# F3: BEGIN + 3 chunks (o ultimo duplicado, que fecha o upload) + 3 ACKs.
def _f3(n_chunks=3):
    # O BEGIN e cru e tem 8B: `00 [slot] 00 00 01 00 00 0a` (§13.7).
    # O upload termina pela DUPLICACAO do ultimo chunk (idx 0x226 na captura)
    # — nao ha commit no fio. Por isso o ultimo idx aparece duas vezes, e o
    # juzao tem ACK para cada chunk, inclusive o repetido.
    frames = [_out("10050001", "000000010000000a")]
    for idx in list(range(n_chunks)) + [n_chunks - 1]:
        payload = "00" + f"{idx:04x}" + "00" * 30
        frames.append(_out("12001002", payload))
        frames.append(_in("12001002", "00" + f"{idx:04x}" + "01"))
    return frames


F3_OK = _f3()


def _sessao(tmp_path, frames_por_nome):
    """Monta um diretorio de sessao com os logs pedidos."""
    sessao = tmp_path / "sessao"
    sessao.mkdir(exist_ok=True)
    for nome_log, frames in frames_por_nome.items():
        caminho = sessao / nome_log
        with io.open(caminho, "w", encoding="utf-8", newline="") as fh:
            for dir_, func, addr, data in frames:
                fh.write(
                    json.dumps(
                        {"s": "H1", "dir": dir_, "func": func, "addr": addr, "data": data}
                    )
                    + "\n"
                )
    return sessao


def _por_fluxo(estado_por_fluxo):
    return {f.nome: estado_por_fluxo[f.nome] for f in h2.FLUXOS}


# ============================================================ self-check

def test_self_check_passa_no_repo():
    """A transcricao do §13 bate com a referencia versionada."""
    assert h2.self_check() == 0


def test_self_check_quebra_se_a_tabela_do_13_for_mutada(monkeypatch, capsys):
    """ACK de 75B = o erro que aconteceu. A referencia tem que reprovar."""
    formas = dict(h2.FORMAS_13)
    formas[("in", "12", "12001002")] = (75,)
    monkeypatch.setattr(h2, "FORMAS_13", formas)
    assert h2.self_check() == 1
    assert "12001002" in capsys.readouterr().err


def test_self_check_quebra_se_o_endpoint_do_ack_sumir(monkeypatch):
    """Endereco que sai da tabela envelhece: reprova em vez de virar 'ok'."""
    formas = {k: v for k, v in h2.FORMAS_13.items() if k != ("in", "12", "12001002")}
    monkeypatch.setattr(h2, "FORMAS_13", formas)
    assert h2.self_check() == 1


def test_self_check_aceita_o_slot_9_e_recusa_o_10(monkeypatch):
    """O endereco do set-param carrega o slot: 1..=9 (§13.11)."""
    assert h2.RE_SET_PARAM.match("10090002")
    assert not h2.RE_SET_PARAM.match("100a0002")
    assert not h2.RE_SET_PARAM.match("10000002")


# =================================================== nivel 3: as formas

def test_payload_fora_da_forma_bloqueia(tmp_path, monkeypatch):
    sessao = _sessao(tmp_path, {"f1_setparam.jsonl": [_out("10010002", _z(4))]})
    r = h2.conferir_fluxo(h2.FLUXOS[0], sessao / "f1_setparam.jsonl")
    assert r.estado == "bloqueia"
    assert any(a.invariant == "forma do payload" for a in r.achados)


def test_endpoint_desconhecido_bloqueia(tmp_path):
    sessao = _sessao(tmp_path, {"f1_setparam.jsonl": [_out("13010000", _z(2))]})
    r = h2.conferir_fluxo(h2.FLUXOS[0], sessao / "f1_setparam.jsonl")
    assert r.estado == "bloqueia"
    assert any(a.invariant == "vocabulario" for a in r.achados)


# ============================================ nivel 1: os 3 fluxos

def test_f1_com_um_frame_e_ok(tmp_path):
    sessao = _sessao(tmp_path, {"f1_setparam.jsonl": F1_OK})
    r = h2.conferir_fluxo(h2.FLUXOS[0], sessao / "f1_setparam.jsonl")
    assert r.estado == "ok"
    # O display tem que dizer o valor que o operador digitou (99.5), e nao o
    # nibble: o payload vem expandido e o f32 so aparece depois de
    # desnibelar.
    assert "99.5" in r.display


def test_f1_com_dois_frames_bloqueia(tmp_path):
    """Um knob e UM frame. Dois = dois efeitos escritos sem pedir."""
    sessao = _sessao(tmp_path, {"f1_setparam.jsonl": F1_OK + F1_OK})
    r = h2.conferir_fluxo(h2.FLUXOS[0], sessao / "f1_setparam.jsonl")
    assert r.estado == "bloqueia"
    assert any(a.invariant == "contagem de writes" for a in r.achados)


def test_f1_com_in_nao_falha_mas_documenta(tmp_path):
    """D4: o set-param nao tem resposta. O IN e registrado, nao bloqueio.

    Se isso virar bloqueio, o operador tem um problema maior (o device
    empurra burst espontaneo no fim da sessao, §7 do checklist) mas o
    veredito do fluxo nao muda por causa disso.
    """
    frames = F1_OK + [_in("12001002", _z(4))]
    sessao = _sessao(tmp_path, {"f1_setparam.jsonl": frames})
    r = h2.conferir_fluxo(h2.FLUXOS[0], sessao / "f1_setparam.jsonl")
    assert r.estado == "ok"
    assert any(a.invariant == "fire-and-forget" for a in r.achados)


def test_f2_com_9_frames_na_ordem_e_ok(tmp_path):
    sessao = _sessao(tmp_path, {"f2_save.jsonl": F2_OK})
    r = h2.conferir_fluxo(h2.FLUXOS[1], sessao / "f2_save.jsonl")
    assert r.estado == "ok"
    # O nome vem do proprio log: e o que o display DEVERIA mostrar.
    assert "H2 TESTE" in r.display


def test_f2_com_8_frames_bloqueia(tmp_path):
    """Faltou frame = o preset nao gravou tudo."""
    sessao = _sessao(tmp_path, {"f2_save.jsonl": F2_OK[:-1]})
    r = h2.conferir_fluxo(h2.FLUXOS[1], sessao / "f2_save.jsonl")
    assert r.estado == "bloqueia"
    assert any(a.invariant == "contagem de writes" for a in r.achados)


def test_f2_fora_de_ordem_bloqueia(tmp_path):
    """A ordem do §12 e o invariante: os mesmos 9 frames fora de ordem
    gravam outra coisa."""
    fora = list(F2_OK)
    fora[1], fora[2] = fora[2], fora[1]
    sessao = _sessao(tmp_path, {"f2_save.jsonl": fora})
    r = h2.conferir_fluxo(h2.FLUXOS[1], sessao / "f2_save.jsonl")
    assert r.estado == "bloqueia"
    assert any(a.invariant == "ordem dos writes" for a in r.achados)


def test_f2_com_op_errada_bloqueia(tmp_path):
    """A S4 capturou 0,0,1,1. op errada grava um preset diferente."""
    errado = list(F2_OK)
    errado[7] = _out("00020000", "0000000000020000")
    sessao = _sessao(tmp_path, {"f2_save.jsonl": errado})
    r = h2.conferir_fluxo(h2.FLUXOS[1], sessao / "f2_save.jsonl")
    assert r.estado == "bloqueia"
    assert any(a.invariant == "valor da op" for a in r.achados)


def test_f3_com_ack_por_chunk_e_ok(tmp_path):
    sessao = _sessao(tmp_path, {"f3_upload.jsonl": F3_OK})
    r = h2.conferir_fluxo(h2.FLUXOS[2], sessao / "f3_upload.jsonl")
    assert r.estado == "ok"
    assert "slot 0" in r.display


def test_f3_sem_ack_bloqueia(tmp_path):
    """Chunk sem ACK = buraco no meio do IR, e o aparelho nao avisa."""
    # remove SO o ACK do primeiro chunk (o chunk fica, que e o caso perigoso:
    # o operador mandou o pedaco e o aparelho nunca respondeu)
    sem_ack = F3_OK[:2] + F3_OK[3:]
    sessao = _sessao(tmp_path, {"f3_upload.jsonl": sem_ack})
    r = h2.conferir_fluxo(h2.FLUXOS[2], sessao / "f3_upload.jsonl")
    assert r.estado == "bloqueia"
    assert any(a.invariant == "ACK por chunk" for a in r.achados)


def test_f3_sem_duplicacao_do_ultimo_bloqueia(tmp_path):
    """O §13.7 fecha o upload na DUPLICACAO do ultimo chunk (idx 0x226)."""
    # 3 chunks SEM repetir o ultimo: o fim nao aconteceu.
    frames = [
        _out("10050001", "000000010000000a"),
        _out("12001002", "00" + "0000" + "00" * 30),
        _in("12001002", "00" + "0000" + "01"),
        _out("12001002", "00" + "0001" + "00" * 30),
        _in("12001002", "00" + "0001" + "01"),
    ]
    sessao = _sessao(tmp_path, {"f3_upload.jsonl": frames})
    r = h2.conferir_fluxo(h2.FLUXOS[2], sessao / "f3_upload.jsonl")
    assert r.estado == "bloqueia"
    assert any(a.invariant == "fecho por duplicacao" for a in r.achados)


def test_f3_sem_begin_bloqueia(tmp_path):
    frames = [f for f in F3_OK if f[2] != "10050001"]
    sessao = _sessao(tmp_path, {"f3_upload.jsonl": frames})
    r = h2.conferir_fluxo(h2.FLUXOS[2], sessao / "f3_upload.jsonl")
    assert r.estado == "bloqueia"
    assert any(a.invariant == "contagem de BEGIN" for a in r.achados)


# ============================================== nivel 2: o que o display ve

def test_o_juiz_imprime_o_valor_que_o_display_deve_mostrar(tmp_path):
    """E o que troca 'pareceu que' por comparacao (regra 2.5)."""
    sessao = _sessao(tmp_path, {"f1_setparam.jsonl": F1_OK})
    r = h2.conferir_fluxo(h2.FLUXOS[0], sessao / "f1_setparam.jsonl")
    assert "99.5" in r.display


def test_o_nome_do_save_vem_do_log(tmp_path):
    sessao = _sessao(tmp_path, {"f2_save.jsonl": F2_OK})
    r = h2.conferir_fluxo(h2.FLUXOS[1], sessao / "f2_save.jsonl")
    assert "H2 TESTE" in r.display
    assert "persist" in r.display.lower() or "voltar" in r.display


# ================================================== sem evidencia (nunca ok)

def test_log_ausente_e_sem_evidencia(tmp_path):
    r = h2.conferir_fluxo(h2.FLUXOS[0], None)
    assert r.estado == "sem-evidencia"


def test_log_vazio_e_sem_evidencia_e_nao_ok(tmp_path):
    """Log vazio e teto que nunca dispara — tem que ser visivel como falta."""
    caminho = _log("vazio.jsonl", [])
    r = h2.conferir_fluxo(h2.FLUXOS[0], caminho)
    assert r.estado == "sem-evidencia"


def test_log_corrompido_bloqueia(tmp_path):
    caminho = tmp_path / "ruim.jsonl"
    caminho.write_text("{nao json}\n", encoding="utf-8")
    r = h2.conferir_fluxo(h2.FLUXOS[0], caminho)
    assert r.estado == "bloqueia"


# ========================================================= a sessao inteira

def test_sessao_completa_da_zero(tmp_path, capsys):
    sessao = _sessao(
        tmp_path,
        {
            "f1_setparam.jsonl": F1_OK,
            "f2_save.jsonl": F2_OK,
            "f3_upload.jsonl": F3_OK,
        },
    )
    assert h2.main([str(sessao)]) == 0
    saida = capsys.readouterr().out
    assert "3 fluxos obedeceram" in saida
    assert "display" in saida


def test_sessao_sem_um_dos_logs_nao_da_zero(tmp_path, capsys):
    """Falta um log = um fluxo sem verificacao, e o gate nao passa."""
    sessao = _sessao(
        tmp_path,
        {"f1_setparam.jsonl": F1_OK, "f2_save.jsonl": F2_OK},
    )
    assert h2.main([str(sessao)]) == 1
    assert "sem evidencia" in capsys.readouterr().out


def test_a_tabela_do_markdown_tem_uma_linha_por_fluxo(tmp_path, capsys):
    sessao = _sessao(
        tmp_path,
        {
            "f1_setparam.jsonl": F1_OK,
            "f2_save.jsonl": F2_OK,
            "f3_upload.jsonl": F3_OK,
        },
    )
    h2.main([str(sessao), "--markdown"])
    saida = capsys.readouterr().out
    # Uma linha por fluxo — e o que o §6 pede para colar no relatório.
    for fluxo in h2.FLUXOS:
        assert f"| {fluxo.nome} |" in saida
