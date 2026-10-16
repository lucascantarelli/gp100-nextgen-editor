"""Teste do contrato 16 do `validate_workflows.py`: a ligação gate → job (#273).

O contrato 16 existe por causa de uma classe que já custou caro duas vezes: o
gate **escrito e NÃO LIGADO**. No #71 o `check_bundle.py` foi documentado como
gate do CI e rodava em lugar nenhum; medido em 15/10 (#273), a mesma classe
ainda tinha três casos vivos — `byte_order`, `param_ranges` e `state_pages`
estavam na lista do `gates.py` e em nenhum passo do `ci.yml` (o primeiro sem
cobertura nenhuma, nem pelo pytest).

Um gate de ligação que ninguém testa aprova por engano no dia seguinte: a
tabela `GATES_LIGADOS` é uma lista escrita à mão, e o modo de falhar dela é
silencioso — se o nome do job ou o trecho do `run:` não casar com a realidade,
a leitura mais confortável ("então está tudo ligado") é justamente a errada.

Por isso este arquivo mede três coisas, em ordem de força:
  * a ÁRVORE REAL — todo gate do `gates.py` tem o passo no job declarado (é o
    teste que teria reprovado os três gates desligados);
  * as MUTAÇÕES sobre o `ci.yml` real (apagar a linha do `run:`, trocar o job,
    deixar só o comentário) — cada uma tem de reprovar, senão o contrato é
    decorativo;
  * o `main()` de ponta a ponta numa árvore sintética — prova que o contrato é
    CHAMADO pelo gate (uma função que só o teste chama não protege o CI).
"""

from __future__ import annotations

import copy
import importlib.util
import pathlib
import sys

_RAIZ = pathlib.Path(__file__).resolve().parents[2]
_SCRIPTS = _RAIZ / ".github" / "scripts"


def _carregar():
    """Importa `.github/scripts/validate_workflows.py` como módulo (padrão dos specs)."""
    spec = importlib.util.spec_from_file_location(
        "validate_workflows", _SCRIPTS / "validate_workflows.py"
    )
    mod = importlib.util.module_from_spec(spec)
    sys.modules["validate_workflows"] = mod
    spec.loader.exec_module(mod)
    return mod


def _ci_real(mod) -> dict:
    """O `ci.yml` parseado — é o insumo de todo teste de mutação."""
    doc = mod.load(".github/workflows/ci.yml")
    assert doc, "o ci.yml do repo não carregou"
    return doc


def _sem_a_linha(ci: dict, job: str, trecho: str) -> dict:
    """O `ci.yml` sem as linhas que contêm `trecho` nos `run:` de `job`.

    É a mutação do "enxugar": a linha do passo sai, o resto do arquivo fica.
    """
    copia = copy.deepcopy(ci)
    for passo in copia["jobs"][job]["steps"]:
        if isinstance(passo.get("run"), str):
            passo["run"] = "\n".join(
                linha for linha in passo["run"].splitlines() if trecho not in linha
            )
    return copia


# ── a árvore real: o que o contrato protege hoje ─────────────────────────────


def test_todo_gate_do_gates_py_tem_o_passo_no_job_declarado():
    """SEM mutação: nenhum gate do `gates.py` pode ficar desligado.

    Este é o teste que teria pegado os três gates mortos (#273) — e o que pega
    o próximo: um gate novo entra no `gates.py` e no `ci.yml` no mesmo commit, e
    esquecer o segundo é o modo de falhar natural (a lista local fica verde).
    """
    mod = _carregar()
    assert mod.gates_desligados(_ci_real(mod)) == []


def test_a_tabela_declara_job_e_trecho_para_cada_gate_de_hoje():
    """A tabela cobre TODOS os gates de hoje e nenhum a mais (linha órfã já é falha).

    Cobre também a assinatura derivada: o `pytest` não nomeia um `.py`, então o
    trecho dele é o comando sem flags e sem o interpretador (`sys.executable` é
    caminho absoluto e muda por runner).
    """
    mod = _carregar()
    declarados = mod.gates_declarados()
    assert sorted(declarados) == sorted(mod.GATES_LIGADOS), (
        "a tabela GATES_LIGADOS e o gates.py divergiram — a ligação é UMA tabela"
    )
    assert declarados["pytest"] == ["uv run pytest"]
    assert declarados["check_bundle"] == [".github/scripts/check_bundle.py"]
    assert declarados["simulate_release"] == [".github/scripts/simulate_release.py"]


def test_o_job_da_tabela_existe_no_ci_e_roda_o_gate():
    """O job declarado tem de existir E ter o passo — a ligação é por job."""
    mod = _carregar()
    ci = _ci_real(mod)
    jobs = ci["jobs"]
    for nome, (jobs_do_gate, trecho) in mod.GATES_LIGADOS.items():
        for job_id in jobs_do_gate:
            assert job_id in jobs, (nome, job_id)
            assert mod.passos_que_rodam(jobs.get(job_id), trecho), (nome, job_id, trecho)


def test_os_gates_de_dados_rodam_no_job_da_spec():
    """Os três que estavam mortos (#273) apontam para `test-spec` — e rodam lá."""
    mod = _carregar()
    ci = _ci_real(mod)
    for nome, trecho in (
        ("byte_order", "tools/check_byte_order.py"),
        ("param_ranges", "tools/param_ranges.py --check"),
        ("state_pages", "tools/validate_state_pages.py"),
    ):
        assert mod.GATES_LIGADOS[nome][0] == ("test-spec",), nome
        assert mod.passos_que_rodam(ci["jobs"]["test-spec"], trecho), (nome, trecho)


# ── mutações: o contrato tem de REPROVAR ─────────────────────────────────────


def test_apagar_a_linha_do_passo_reprova_com_o_job_na_mensagem():
    """A mutação do #71: o `run:` do gate sai e o contrato acusa o job."""
    mod = _carregar()
    ci = _sem_a_linha(_ci_real(mod), "lint-workflows", ".github/scripts/check_bundle.py")
    faltas = [f for f in mod.gates_desligados(ci) if "check_bundle" in f]
    assert len(faltas) == 1, faltas
    assert "lint-workflows" in faltas[0]


def test_mover_o_passo_para_outro_job_reprova():
    """Rodar em QUALQUER job não vale: a tabela diz em qual (é o pin)."""
    mod = _carregar()
    ci = _ci_real(mod)
    # O passo sai do job declarado e reaparece em outro — o CI continua com um
    # passo que roda o gate, e mesmo assim tem de reprovar.
    ci = _sem_a_linha(ci, "lint-workflows", ".github/scripts/check_bundle.py")
    ci["jobs"]["lint-ui"]["steps"].append({"run": "python3 .github/scripts/check_bundle.py"})
    faltas = [f for f in mod.gates_desligados(ci) if "check_bundle" in f]
    assert len(faltas) == 1, faltas
    assert "lint-workflows" in faltas[0]


def test_gate_sem_linha_na_tabela_ainda_e_cobrado(monkeypatch):
    """O caso do PR irmão: gate no `gates.py` + passo, sem linha na tabela.

    A ausência de linha NÃO é falha (senão o `develop` vermelha quando outro PR
    acrescenta gate + passo), mas o gate continua cobrado pelo comando: sem o
    passo em lugar nenhum, ele reprova igual.
    """
    mod = _carregar()
    monkeypatch.delitem(mod.GATES_LIGADOS, "check_bundle")
    ci = _ci_real(mod)
    assert [f for f in mod.gates_desligados(ci) if "check_bundle" in f] == []
    sem_passo = _sem_a_linha(ci, "lint-workflows", ".github/scripts/check_bundle.py")
    faltas = [f for f in mod.gates_desligados(sem_passo) if "check_bundle" in f]
    assert len(faltas) == 1, faltas
    assert "NÃO LIGADO" in faltas[0]


def test_linha_orfa_reprova(monkeypatch):
    """Linha para gate que não existe mais = ligação deixou de ser conferida."""
    mod = _carregar()
    monkeypatch.setitem(mod.GATES_LIGADOS, "check_inexistente", (("lint-workflows",), "x.py"))
    faltas = mod.gates_desligados(_ci_real(mod))
    assert any("check_inexistente" in f and "órfã" in f for f in faltas), faltas


def test_linha_apontando_para_job_inexistente_reprova(monkeypatch):
    """Job renomeado: o passo passa a não existir em job nenhum que o contrato veja."""
    mod = _carregar()
    monkeypatch.setitem(
        mod.GATES_LIGADOS, "check_bundle", (("lint-nao-existe",), ".github/scripts/check_bundle.py")
    )
    faltas = [f for f in mod.gates_desligados(_ci_real(mod)) if "check_bundle" in f]
    assert len(faltas) == 1, faltas
    assert "lint-nao-existe" in faltas[0]


def test_mencao_em_comentario_nao_liga_o_gate():
    """Comentário NÃO é instrução (a regra da casa): só `run:` de verdade liga."""
    mod = _carregar()
    ci = _ci_real(mod)
    for passo in ci["jobs"]["lint-workflows"]["steps"]:
        texto = passo.get("run")
        if isinstance(texto, str) and ".github/scripts/check_bundle.py" in texto:
            passo["run"] = "# veja o check_bundle.py\n" + "\n".join(
                linha
                for linha in texto.splitlines()
                if ".github/scripts/check_bundle.py" not in linha
            )
    faltas = [f for f in mod.gates_desligados(ci) if "check_bundle" in f]
    assert len(faltas) == 1, faltas


def test_gates_py_ilegivel_reprova(tmp_path, monkeypatch):
    """Sem a lista não há contrato — e falhar é o que não deixa o gate morrer calado."""
    mod = _carregar()
    monkeypatch.setattr(mod, "REPO", str(tmp_path))  # sem gates.py e sem ci.yml
    mod.FAILURES.clear()
    try:
        assert mod.gates_desligados({}) == []
        assert any("gates.py" in f for f in mod.FAILURES), mod.FAILURES
    finally:
        mod.FAILURES.clear()


# ── o `main()` de ponta a ponta: o contrato é CHAMADO pelo gate ──────────────


def _arvore(tmp_path, *, ligado: bool) -> pathlib.Path:
    """Árvore sintética mínima: um gate no `gates.py` e um passo (ou não) no `ci.yml`."""
    (tmp_path / ".github" / "scripts").mkdir(parents=True)
    (tmp_path / ".github" / "workflows").mkdir(parents=True)
    (tmp_path / ".github" / "scripts" / "fake_gate.py").write_text(
        "print('ok')\n", encoding="utf-8"
    )
    (tmp_path / ".github" / "scripts" / "gates.py").write_text(
        "import sys\n\n"
        "GATES = [(\"fake_gate\", [sys.executable, \".github/scripts/fake_gate.py\"],"
        " \"gate de teste\")]\n",
        encoding="utf-8",
    )
    passo = "python3 .github/scripts/fake_gate.py" if ligado else "echo nada"
    (tmp_path / ".github" / "workflows" / "ci.yml").write_text(
        "name: ci\n"
        "on:\n"
        "  pull_request:\n"
        "jobs:\n"
        "  lint-workflows:\n"
        "    runs-on: ubuntu-24.04\n"
        "    steps:\n"
        f"      - run: {passo}\n",
        encoding="utf-8",
    )
    return tmp_path


def test_main_reprova_o_gate_desligado(tmp_path, monkeypatch, capsys):
    """O `main()` acusa o gate desligado — não é uma função que só o teste chama.

    A árvore é irrelevante de propósito: os OUTROS contratos reprovam nela (não
    há os três workflows nem os 25 jobs), então o que se mede aqui é a MENSAGEM
    deste contrato, e não o exit code.
    """
    mod = _carregar()
    monkeypatch.setattr(mod, "REPO", str(_arvore(tmp_path, ligado=False)))
    mod.FAILURES.clear()
    try:
        mod.main()
        saida = capsys.readouterr().out
    finally:
        mod.FAILURES.clear()
    assert "fake_gate" in saida, saida
    assert "NÃO LIGADO" in saida, saida


def test_main_aceita_o_gate_ligado(tmp_path, monkeypatch, capsys):
    """Controle positivo: com o passo no `ci.yml`, este contrato fica calado."""
    mod = _carregar()
    monkeypatch.setattr(mod, "REPO", str(_arvore(tmp_path, ligado=True)))
    mod.FAILURES.clear()
    try:
        mod.main()
        saida = capsys.readouterr().out
    finally:
        mod.FAILURES.clear()
    assert "fake_gate" not in saida, saida
