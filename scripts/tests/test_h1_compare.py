#!/usr/bin/env python3
"""test_h1_compare.py — o juiz da FASE C do gate H1 se prova por MUTACAO (#21).

A regra deste arquivo e a mesma que vale para os outros gates do repo: um
teste que passa nao prova nada, um teste que FALHA quando o defeito volta
prova. Por isso quase todo teste aqui quebra de proposito uma coisa — um
payload, um endereco, a ordem, a tabela do §13 — e exige a falha
correspondente.

O teste que mais importa e o `test_nivel3_nao_depende_do_mock`: ele prova a
razao de o nivel 3 existir. Se o nivel 3 conferisse as formas contra o
`MockDevice` em vez de conferir contra o §13, um mock que passasse a
responder pagina de 197B geraria uma referencia de 197B e o gate em campo
aprovaria junto com o defeito. Aqui a referencia (que E o mock) e
deliberadamente confrontada com uma `FORMAS_13` errada, e tem de reprovar.
"""

import importlib.util
import io
import json
import os
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
sys.stdout.reconfigure(encoding="utf-8", errors="replace")


def _carregar():
    """Importa `scripts/h1_compare.py` como modulo (mesmo truque do test_protocol).

    O registro em `sys.modules` ANTES do `exec_module` nao e opcional: o
    `dataclasses` resolve o modulo da classe por `sys.modules[cls.__module__]`
    e estoura `AttributeError` se ele nao estiver la. Sem este passo, qualquer
    `@dataclass` num script carregado por importlib quebra na importacao.
    """
    caminho = ROOT / "scripts" / "h1_compare.py"
    spec = importlib.util.spec_from_file_location("h1_compare", caminho)
    mod = importlib.util.module_from_spec(spec)
    sys.modules["h1_compare"] = mod
    spec.loader.exec_module(mod)
    return mod


h1 = _carregar()
REFERENCIA = h1.REFERENCIA


# ---------------------------------------------------------------- helpers
def _frames(nome):
    return h1.ler_log(REFERENCIA / nome)


def _bruto(nome):
    """O log como lista de dicts crus (para mutar antes de reescrever)."""
    with io.open(REFERENCIA / nome, encoding="utf-8") as fh:
        return [json.loads(linha) for linha in fh if linha.strip()]


def _gravar(destino, nome, brutos):
    destino.mkdir(parents=True, exist_ok=True)
    with io.open(destino / nome, "w", encoding="utf-8", newline="") as fh:
        for b in brutos:
            fh.write(
                json.dumps(
                    {
                        "s": "H1",
                        "dir": b["dir"],
                        "func": b["func"],
                        "addr": b["addr"],
                        "data": b["data"],
                    },
                    separators=(",", ":"),
                )
                + "\n"
            )
    return destino


def _resultado(tmp_path, nome_saida, brutos, passo_nome):
    """Escreve a sessao e devolve o `Resultado` do passo correspondente."""
    sessao = _gravar(tmp_path, nome_saida, brutos)
    passo = next(p for p in h1.PASSOS if p.nome == passo_nome)
    return h1.classificar(passo, sessao, ensaio=False)


# ═══════════════════════════════════ as duas invariantes do self-check (CI)

def test_self_check_passa_no_repo():
    """O conjunto de referencia tem de estar de acordo com o §13."""
    assert h1.self_check() == 0, "referencia fora da forma do §13"


def test_self_check_quebra_se_a_tabela_do_13_for_mutada(tmp_path, monkeypatch):
    """MUTACAO: a §13 diz 197B e a referencia tem 196B → tem de reprovar.

    E o teste que separa "gate" de "decoracao": ele prova que o self-check
    discorda do proprio mock. Se um dia o nivel 3 for reescrito para
    comparar contra o mock, este teste e o que avisa.
    """
    monkeypatch.setitem(h1.FORMAS_13, ("in", "12", "13010003"), (197, 32))
    assert h1.self_check() != 0, "self-check aceitou referencia fora do §13"


def test_self_check_quebra_se_o_framing_dos_dumps_divergir(tmp_path, monkeypatch):
    """MUTACAO: um `mock_dump_*` com framing diferente do outro reprova.

    Invariante: o dump-preset fala com o device por pp, e o framing nao
    pode depender do pp — so o conteudo muda entre os tres dumps.
    """
    monkeypatch.setattr(h1, "REFERENCIA", tmp_path)
    _gravar(tmp_path, "mock_dump_0x0000.jsonl", _bruto("mock_dump_0x0000.jsonl"))
    trocado = _bruto("mock_dump_0x0031.jsonl")
    trocado[3]["addr"] = "13010005"  # endereco que existe no §13, outro frame
    _gravar(tmp_path, "mock_dump_0x0031.jsonl", trocado)
    assert h1.self_check() != 0, "self-check aceitou dumps com framing diferente"


def test_formas_13_cobre_todos_os_endpoints_da_referencia():
    """Todo endpoint que o roteiro produz tem forma declarada no §13.

    Sem isto, um endpoint novo cairia no ramo "fora do vocabulario" e o
    veredito seria `bloqueia-H1` — certo por acaso, e nao por prova.
    """
    for nome in ("mock_list_irs.jsonl", "mock_dump_0x0000.jsonl"):
        for f in _frames(nome):
            assert f.endpoint in h1.FORMAS_13, (
                f"{nome}: endpoint {f.endpoint} sem forma declarada no §13"
            )


# ══════════════════════════════════════════════ nivel 3: forma (o §13)

def test_payload_fora_da_forma_e_nivel3_bloqueia(tmp_path):
    """MUTACAO: página de 197B onde o §13 diz 196B."""
    brutos = _bruto("mock_dump_0x0000.jsonl")
    brutos[3]["data"] += "ab"
    r = _resultado(tmp_path, "b3_dump.jsonl", brutos, "B3 dump-preset 0x0000")
    assert r.estado == "bloqueia", r.detalhe
    assert [a.nivel for a in r.achados] == [3]
    assert r.achados[0].severidade == "bloqueia-H1"
    assert "196/32B" in r.achados[0].esperado


def test_pagina_truncada_de_32b_e_legal(tmp_path):
    """O §13.10 diz que a última página do banco vem truncada em 32B.

    Se `FORMAS_13` aceitasse só 196B, uma sessão de campo LEGÍTIMA seria
    reprovada como falha de protocolo — e o operador, com a pedaleira na
    mão, receberia um "PARAR" sem motivo.
    """
    brutos = _bruto("mock_dump_0x0000.jsonl")
    for i, b in enumerate(brutos):
        if b["addr"] == "13010003" and i >= 3:
            b["data"] = b["data"][: 32 * 2]
    r = _resultado(tmp_path, "b3_dump.jsonl", brutos, "B3 dump-preset 0x0000")
    assert r.estado != "bloqueia", [a.esperado for a in r.achados]


def test_achados_de_forma_sao_agregados_por_endpoint(tmp_path):
    """8 páginas fora da forma = 1 linha na tabela do §5, com a contagem.

    Oito linhas iguais escondem o que importa — que TODAS as 8 estão fora.
    """
    brutos = _bruto("mock_dump_0x0000.jsonl")
    for i, b in enumerate(brutos):
        if b["addr"] == "13010003":
            b["data"] += "ab"
    r = _resultado(tmp_path, "b3_dump.jsonl", brutos, "B3 dump-preset 0x0000")
    assert len(r.achados) == 1, [a.endpoint for a in r.achados]
    assert "8 frame(s)" in r.achados[0].obtido


# ══════════════════════════════════════════ nivel 1: framing (o vocabulario)

def test_endpoint_desconhecido_e_nivel1_bloqueia(tmp_path):
    """MUTACAO: o device responde num endereço que o §13 não conhece."""
    brutos = _bruto("mock_dump_0x0000.jsonl")
    brutos[3]["addr"] = "13010006"
    r = _resultado(tmp_path, "b3_dump.jsonl", brutos, "B3 dump-preset 0x0000")
    assert r.estado == "bloqueia"
    assert 1 in [a.nivel for a in r.achados]
    assert any("fora do §13" in a.hipotese for a in r.achados)


def test_ordem_diferente_e_nivel1_bloqueia(tmp_path):
    """MUTACAO: os mesmos frames, fora de ordem.

    Aqui os endereços são TODOS válidos e todos do tamanho certo — por isso
    a prova de que o nível 1 existe separada do nível 3. Se a ordem fosse
    comparada só como conjunto, isto passaria.

    A troca é entre o `in 13010003` (página) e o `out 13010004` (avanço)
    que o deveria seguir: dois frames de endereços DIFERENTES, senão a
    sequência nem muda.
    """
    brutos = _bruto("mock_dump_0x0000.jsonl")
    assert brutos[3]["addr"] == "13010003" and brutos[4]["addr"] == "13010004", (
        "a referencia mudou de forma; a mutacao precisa de frames vizinhos "
        "com enderecos diferentes"
    )
    brutos[3], brutos[4] = brutos[4], brutos[3]
    r = _resultado(tmp_path, "b3_dump.jsonl", brutos, "B3 dump-preset 0x0000")
    assert r.estado == "bloqueia"
    assert any("ordem/endpoint" in a.hipotese for a in r.achados)


def test_trocar_frames_de_endereco_igual_nao_e_divergencia(tmp_path):
    """O contraprova: duas páginas trocadas de posição NÃO são divergência.

    As 8 páginas do dump têm o mesmo endpoint e o mesmo tamanho; a ordem
    entre elas é indistinguível no fio. Se este caso bloqueasse, um
    `diff` de sessão real que demorasse um frame a responder reprovaria o
    H1 sem motivo.
    """
    brutos = _bruto("mock_dump_0x0000.jsonl")
    brutos[3], brutos[5] = brutos[5], brutos[3]
    r = _resultado(tmp_path, "b3_dump.jsonl", brutos, "B3 dump-preset 0x0000")
    assert r.estado != "bloqueia", [a.hipotese for a in r.achados]


def test_frame_a_menos_e_nivel1(tmp_path):
    """MUTACAO: o device cala no meio (Timeout no meio do dump)."""
    brutos = _bruto("mock_dump_0x0000.jsonl")[:-1]
    r = _resultado(tmp_path, "b3_dump.jsonl", brutos, "B3 dump-preset 0x0000")
    assert r.estado == "bloqueia"
    assert any("fim do log real" in a.obtido for a in r.achados)


# ══════════════════════════════════════════ nivel 2: conteudo (o esperado)

def test_so_conteudo_diverge_e_nivel2_nao_bloqueia(tmp_path):
    """MUTACAO: mesmos endereços, mesmos tamanhos, bytes diferentes.

    E o caso ESPERADO (§4 nível 2): o device real não é o `all.prst`. Se
    isto bloqueasse, o H1 seria reprovado em toda sessão legítima.
    """
    brutos = _bruto("mock_dump_0x0000.jsonl")
    brutos[9]["data"] = "ff" + brutos[9]["data"][2:]
    r = _resultado(tmp_path, "b3_dump.jsonl", brutos, "B3 dump-preset 0x0000")
    assert r.estado == "estado", r.estado
    assert [a.nivel for a in r.achados] == [2]
    assert r.achados[0].severidade == "estado"


def test_pp_diferente_no_select_e_nivel2(tmp_path):
    """O `select` leva o pp no payload — pp ≠ pp é ESTADO, não protocolo.

    O §13.10 avisa que o inventário do device real pode começar em 0x0100
    enquanto o default do core é 0..198. Esse caso tem de ser nivel 2, senao
    o primeiro `bloqueia-H1` do H1 seria essa diferenca legitima.
    """
    brutos = _bruto("mock_dump_0x0000.jsonl")
    brutos[0]["data"] = "0100"
    r = _resultado(tmp_path, "b3_dump.jsonl", brutos, "B3 dump-preset 0x0000")
    assert r.estado == "estado", r.estado
    assert any(a.nivel == 2 and "out 11/13010000" == a.endpoint for a in r.achados)


def test_nivel2_avisa_quando_o_tamanho_dos_bytes_difere(tmp_path):
    """Mesmo endereco e framing, mas um payload mais curto que o do mock.

    As DUAS formas (196 e 32) são legais no §13, então isto nao é nivel 3 —
    mas é um sinal de que o device e um firmware diferente, e a linha do §5
    precisa dizer isso em vez de so "estado".
    """
    brutos = _bruto("mock_dump_0x0000.jsonl")
    brutos[9]["data"] = brutos[9]["data"][: 32 * 2]
    r = _resultado(tmp_path, "b3_dump.jsonl", brutos, "B3 dump-preset 0x0000")
    assert r.estado == "estado", r.estado
    assert any("TAMANHO tambem difere" in a.hipotese for a in r.achados)


def test_framing_quebrado_nao_gera_ruido_de_nivel2(tmp_path):
    """Com o framing quebrado, o nivel 2 NAO fala.

    Um `diff` de conteudo alinhado frame-a-frame sobre logs de tamanhos
    diferentes produziria um emaranhado de "divergencias de estado" que
    esconderiam o primeiro achado de protocolo — que é o que importa.
    """
    brutos = _bruto("mock_dump_0x0000.jsonl")[:-1]
    for b in brutos:
        if b["addr"] == "13010003":
            b["data"] = "ff" + b["data"][2:]
    r = _resultado(tmp_path, "b3_dump.jsonl", brutos, "B3 dump-preset 0x0000")
    assert r.estado == "bloqueia"
    assert not any(a.nivel == 2 for a in r.achados)


# ═══════════════════════════════════════ passos sem evidencia (o B1 honesto)

def test_log_vazio_e_sem_evidencia_e_nao_ok(tmp_path):
    """Um arquivo vazio NÃO pode virar "verde".

    O `info --real` não emite tráfego de fio, então o `b1_info.jsonl` de
    campo é vazio POR CONSTRUÇÃO. Se isso contasse como passou, o B1 do
    roteiro estaria verde sem ter provado nada.
    """
    sessao = tmp_path / "vazio"
    sessao.mkdir()
    io.open(sessao / "b1_info.jsonl", "w", encoding="utf-8", newline="").close()
    passo = next(p for p in h1.PASSOS if p.nome == "B1 info")
    r = h1.classificar(passo, sessao, ensaio=False)
    assert r.estado == "sem-evidencia", r.estado
    assert "info --real" in r.detalhe


def test_log_inexistente_e_sem_evidencia(tmp_path):
    passo = next(p for p in h1.PASSOS if p.nome == "B2 list-user-irs")
    r = h1.classificar(passo, tmp_path, ensaio=False)
    assert r.estado == "sem-evidencia"


def test_nenhum_passo_sem_evidencia_conta_como_ok(tmp_path):
    """A regra geral: 'sem evidência' é um terceiro estado, nunca um verde.

    Com a sessao vazia, o veredito é 0 (o roteiro nao foi rodado), mas
    NENHUM passo pode ter saído 'ok' — se um tivesse, o relatório affirmaria
    uma prova que nao existe.
    """
    sessao = tmp_path / "nada"
    sessao.mkdir()
    estados = {h1.classificar(p, sessao, ensaio=False).estado for p in h1.PASSOS}
    assert "ok" not in estados, estados


# ════════════════════════════════════════════════ a referencia como sessao

def test_referencia_como_sessao_da_ok(tmp_path):
    """O conjunto de referencia é uma sessão válida do juiz (auto-teste)."""
    estados = {
        h1.classificar(p, REFERENCIA, ensaio=False).estado for p in h1.PASSOS
    }
    assert estados <= {"ok", "sem-evidencia"}, estados
    assert "ok" in estados, "nenhum passo casou com a propria referencia"


def test_referencia_como_sessao_nao_tem_achado():
    for p in h1.PASSOS:
        r = h1.classificar(p, REFERENCIA, ensaio=False)
        assert not r.achados, (p.nome, [a.endpoint for a in r.achados])


# ══════════════════════════════════════════════════════════ a tabela do §5

def test_tabela_tem_cabecalho_e_uma_linha_por_achado(tmp_path):
    brutos = _bruto("mock_dump_0x0000.jsonl")
    brutos[9]["data"] = "ff" + brutos[9]["data"][2:]
    r = _resultado(tmp_path, "b3_dump.jsonl", brutos, "B3 dump-preset 0x0000")
    linhas = h1.tabela([r]).splitlines()
    assert linhas[0].startswith("| # | Passo | Endpoint")
    assert len(linhas) == 2 + len(r.achados)
    assert "documentar" in linhas[-1], "severidade 'estado' pede 'documentar'"


def test_tabela_de_achado_bloqueante_manda_para_o_fluxo_r3(tmp_path):
    brutos = _bruto("mock_dump_0x0000.jsonl")
    brutos[3]["data"] += "ab"
    r = _resultado(tmp_path, "b3_dump.jsonl", brutos, "B3 dump-preset 0x0000")
    assert "fluxo R3" in h1.tabela([r])


# ══════════════════════════════════════════════════════════════ leitura

def test_linha_sem_campos_do_schema_p4_e_erro(tmp_path):
    """Lixo no log tem de ser erro, não um frame com addr vazio."""
    sessao = tmp_path / "lixo"
    sessao.mkdir()
    with io.open(sessao / "b2_list.jsonl", "w", encoding="utf-8", newline="") as fh:
        fh.write('{"s":"H1","dir":"in","func":"12"}\n')
    passo = next(p for p in h1.PASSOS if p.nome == "B2 list-user-irs")
    try:
        h1.classificar(passo, sessao, ensaio=False)
    except ValueError as exc:
        assert "schema P4" in str(exc)
    else:
        raise AssertionError("linha fora do schema P4 passou sem erro")


def test_log_nao_json_e_erro(tmp_path):
    sessao = tmp_path / "naojson"
    sessao.mkdir()
    io.open(sessao / "b2_list.jsonl", "w", encoding="utf-8", newline="").write("F0 21\n")
    passo = next(p for p in h1.PASSOS if p.nome == "B2 list-user-irs")
    try:
        h1.classificar(passo, sessao, ensaio=False)
    except ValueError as exc:
        assert "nao-JSON" in str(exc)
    else:
        raise AssertionError("log nao-JSON passou sem erro")


# ═════════════════════════════════════════════════════════════ o runbook

def test_runbook_chama_o_juiz_na_fase_c():
    """`h1_field.sh` tem de chamar `h1_compare.py` nas DUAS fases.

    Sem isto, o juiz existe mas o operador nunca o ve — e a Fase C volta a
    ser o lembrete em texto que a #21 veio substituir.
    """
    with io.open(ROOT / "scripts" / "h1_field.sh", encoding="utf-8") as fh:
        runbook = fh.read()
    assert runbook.count("h1_compare.py") >= 2, "a Fase C nao esta automatizada"
    assert "framing_of()" not in runbook, (
        "o runbook ainda tem a comparacao de framing propria — duas fontes "
        "de verdade para a mesma regra"
    )


def test_kit_de_campo_embala_o_juiz():
    """O zip do CLI de campo tem de incluir `h1_compare.py`.

    O kit sem o juiz obriga o operador a fazer a Fase C na mao — que e
    exatamente o que a issue veio tirar.
    """
    with io.open(ROOT / ".github" / "workflows" / "ci.yml", encoding="utf-8") as fh:
        ci = fh.read()
    assert "scripts/h1_compare.py" in ci, "o kit de campo nao embarca o juiz"