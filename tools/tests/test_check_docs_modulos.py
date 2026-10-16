"""Teste do gate de obsolescencia doc x modulo (`check_docs_modulos`, #275).

O gate mede TEMPO por ancestralidade de commit e CONTEUDO por superficie. Cada uma
das duas medidas tem um falso positivo classico, e um gate que reprova documento
inocente e pior que nenhum — quem o encontra o desliga. Por isso todo caso nocivo
tem o vizinho que NAO pode ser punido, medido ao lado:

  * o doc e o modulo no MESMO commit (e o que o P7 pede: acompanham juntos);
  * o modulo que mexeu so o CORPO desde a revisao do doc (refatoracao interna nao
    obriga documento — foi o que os 4 pares "ok (corpo)" do repo real mostraram);
  * o doc que veio DEPOIS do modulo (mais novo, nao mais velho);
  * `docs/arquivo/**` e `docs/analises/**`, que sao historico e evidencia datada:
    envelhecer e o trabalho deles;
  * citacao sem caminho (`session.rs` solto) e caminho de dado/config, que o gate
    nao adivinha nem possui.

E o lado que REPROVA precisa de historico de verdade — medir contra o repo real so
prova o dia de hoje. Por isso os casos de obsolescencia rodam num repositorio git
SINTETICO, montado aqui: commits de verdade, ordem de verdade.
"""

from __future__ import annotations

import importlib.util
import os
import pathlib
import subprocess
import sys

_RAIZ = pathlib.Path(__file__).resolve().parents[2]
_SCRIPTS = _RAIZ / ".github" / "scripts"

# O modulo e o doc do repo sintetico: `tools/modulo.py` e citado com caminho (e o
# que o gate exige) e mora em `docs/tecnico/` (genero vivo).
MODULO = "tools/modulo.py"
DOC = "docs/tecnico/guia.md"


def _carregar():
    spec = importlib.util.spec_from_file_location(
        "check_docs_modulos", _SCRIPTS / "check_docs_modulos.py"
    )
    mod = importlib.util.module_from_spec(spec)
    sys.modules["check_docs_modulos"] = mod
    spec.loader.exec_module(mod)
    return mod


def _limpar(mod) -> None:
    mod.VIOLACOES.clear()
    mod._CACHE.clear()


# ── repositorio sintetico: commits de verdade ────────────────────────────────

# Identidade PROPRIA no commit do repo sintetico: sem isto o teste depende do
# `user.name` da maquina (e em runner limpo ele nao existe).
_ENV = {
    **os.environ,
    "GIT_AUTHOR_NAME": "teste",
    "GIT_AUTHOR_EMAIL": "teste@local",
    "GIT_COMMITTER_NAME": "teste",
    "GIT_COMMITTER_EMAIL": "teste@local",
}


def _git(repo: pathlib.Path, *args: str) -> None:
    r = subprocess.run(
        ["git", *args],
        cwd=repo,
        capture_output=True,
        text=True,
        encoding="utf-8",
        env=_ENV,
    )
    assert r.returncode == 0, (args, r.stdout, r.stderr)


def _commit(repo: pathlib.Path, mensagem: str, arquivos: dict[str, str]) -> None:
    for caminho, texto in arquivos.items():
        p = repo / caminho
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(texto, encoding="utf-8")
    _git(repo, "add", "-A")
    _git(repo, "-c", "commit.gpgsign=false", "commit", "-q", "-m", mensagem)


def _repo(tmp_path: pathlib.Path) -> pathlib.Path:
    _git(tmp_path, "init", "-q", "-b", "main")
    return tmp_path


def _sha(repo: pathlib.Path) -> str:
    r = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=repo,
        capture_output=True,
        text=True,
        encoding="utf-8",
        env=_ENV,
    )
    assert r.returncode == 0, r.stderr
    return r.stdout.strip()


def _git_branch(repo: pathlib.Path, *args: str) -> None:
    _git(repo, "checkout", *args)


def _doc_do_modulo(*, extra: str = "") -> str:
    return f"# Guia\n\nO modulo `{MODULO}` faz o trabalho. {extra}\n"


def _assinatura(versao: str = "x") -> str:
    return f"def fazer(x, {versao}):\n    return x\n"


def _corpo(valor: int) -> str:
    return f"def fazer(x):\n    return x + {valor}\n"


def _rodar(mod, repo: pathlib.Path, monkeypatch) -> list[str]:
    """Roda o gate com a raiz apontada para o repo sintetico (e devolve as violacoes)."""
    monkeypatch.setattr(mod, "RAIZ", repo)
    _limpar(mod)
    mod.main([])
    return list(mod.VIOLACOES)


# ── o contrato do repo real ──────────────────────────────────────────────────


def test_documentacao_do_repo_real_passa() -> None:
    """O exit code e o contrato: a arvore de hoje = 0 violacoes.

    Este e o teste que teria pegado os 2 documentos velhos medidos em 15/10
    (`PACKAGING.md` e `RELEASE_PLAN.md`): o gate nasce do conserto deles.
    """
    r = subprocess.run(
        [sys.executable, str(_SCRIPTS / "check_docs_modulos.py")],
        cwd=_RAIZ,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    assert r.returncode == 0, r.stdout + r.stderr
    assert "violacoes: 0" in r.stdout


def test_repo_real_nao_tem_par_nenhum_com_pendencia() -> None:
    """Medido no processo: `checa_pares` nao acha violacao na arvore real."""
    mod = _carregar()
    _limpar(mod)
    mod.main([])
    assert mod.VIOLACOES == [], mod.VIOLACOES


def test_repo_real_mede_doc_e_modulo_com_caminho() -> None:
    """Sanidade da materia-prima: existem pares e eles sao (doc.md, modulo)."""
    mod = _carregar()
    medidos = mod.pares()
    assert len(medidos) > 10, medidos
    assert all(d.endswith(".md") and "/" in m for d, m in medidos), medidos[:5]


# ── o caso que REPROVA: o doc parado e o modulo andando ──────────────────────


def test_doc_velho_reprova_e_aponta_o_documento(tmp_path, monkeypatch) -> None:
    """A regra do #275: o gate APONTA O DOCUMENTO, com modulo e shas."""
    repo = _repo(tmp_path)
    _commit(repo, "modulo", {MODULO: _corpo(1)})
    _commit(repo, "doc", {DOC: _doc_do_modulo()})
    _commit(repo, "modulo ganha assinatura nova", {MODULO: _assinatura()})
    mod = _carregar()
    violacoes = _rodar(mod, repo, monkeypatch)
    assert len(violacoes) == 1, violacoes
    assert DOC in violacoes[0] and MODULO in violacoes[0], violacoes[0]
    assert "superficie" in violacoes[0], violacoes[0]


def test_doc_e_modulo_no_mesmo_commit_passam(tmp_path, monkeypatch) -> None:
    """O P7: acompanhar NA MESMA PR e o caminho certo — nao pode reprovar."""
    repo = _repo(tmp_path)
    _commit(repo, "tudo junto", {MODULO: _corpo(1), DOC: _doc_do_modulo()})
    _commit(repo, "modulo e doc juntos", {MODULO: _assinatura(), DOC: _doc_do_modulo(extra="Revisto.")})
    mod = _carregar()
    assert _rodar(mod, repo, monkeypatch) == []


def test_mudanca_so_de_corpo_nao_reprova(tmp_path, monkeypatch) -> None:
    """Refatoracao interna nao obriga documento (o falso positivo mais caro)."""
    repo = _repo(tmp_path)
    _commit(repo, "modulo", {MODULO: _corpo(1)})
    _commit(repo, "doc", {DOC: _doc_do_modulo()})
    _commit(repo, "so o corpo mudou", {MODULO: _corpo(2)})
    mod = _carregar()
    assert _rodar(mod, repo, monkeypatch) == []


def test_doc_depois_do_modulo_nao_reprova(tmp_path, monkeypatch) -> None:
    """Ordem invertida: o doc esta MAIS NOVO que o modulo (nada a cobrar)."""
    repo = _repo(tmp_path)
    _commit(repo, "modulo", {MODULO: _corpo(1)})
    _commit(repo, "modulo muda a assinatura", {MODULO: _assinatura()})
    _commit(repo, "doc escrito depois", {DOC: _doc_do_modulo(extra="Ja descreve a assinatura nova.")})
    mod = _carregar()
    assert _rodar(mod, repo, monkeypatch) == []


# ── isencoes: excecao declarada, nunca deposito ──────────────────────────────


def test_isencao_com_motivo_passa(tmp_path, monkeypatch) -> None:
    """O par que e CITACAO, nao descricao, se declara — com o motivo no diff."""
    repo = _repo(tmp_path)
    _commit(repo, "modulo", {MODULO: _corpo(1)})
    _commit(repo, "doc", {DOC: _doc_do_modulo()})
    _commit(repo, "modulo ganha assinatura nova", {MODULO: _assinatura()})
    mod = _carregar()
    monkeypatch.setitem(mod.ISENCOES, (DOC, MODULO), "o doc cita o modulo de passagem; nao descreve a API")
    assert _rodar(mod, repo, monkeypatch) == []


def test_isencao_sem_motivo_reprova(tmp_path, monkeypatch) -> None:
    repo = _repo(tmp_path)
    _commit(repo, "modulo", {MODULO: _corpo(1)})
    _commit(repo, "doc", {DOC: _doc_do_modulo()})
    mod = _carregar()
    monkeypatch.setitem(mod.ISENCOES, (DOC, MODULO), "   ")
    violacoes = _rodar(mod, repo, monkeypatch)
    assert any("sem motivo" in v for v in violacoes), violacoes


def test_isencao_orfa_reprova(tmp_path, monkeypatch) -> None:
    """Isencao para par que o repo nao tem = linha que ninguem confere mais."""
    repo = _repo(tmp_path)
    _commit(repo, "modulo", {MODULO: _corpo(1)})
    _commit(repo, "doc", {DOC: _doc_do_modulo()})
    mod = _carregar()
    monkeypatch.setitem(mod.ISENCOES, ("docs/tecnico/fantasma.md", MODULO), "motivo")
    violacoes = _rodar(mod, repo, monkeypatch)
    assert any("orfa" in v for v in violacoes), violacoes


# ── a previsao do squash (#282) ──────────────────────────────────────────────


def _rodar_com_base(mod, repo: pathlib.Path, monkeypatch, base: str) -> list[str]:
    """Roda o gate no regime do PR (o range decide) com a raiz apontada ao repo."""
    monkeypatch.setattr(mod, "RAIZ", repo)
    _limpar(mod)
    mod.main(["--base", base])
    return list(mod.VIOLACOES)


def test_previsao_cobra_o_par_que_o_range_toca_sem_o_doc(tmp_path, monkeypatch) -> None:
    """O PR mexe na superficie do modulo e nao toca o documento: cobra NO PR."""
    repo = _repo(tmp_path)
    _commit(repo, "modulo", {MODULO: _corpo(1)})
    _commit(repo, "doc", {DOC: _doc_do_modulo()})
    base = _sha(repo)
    _commit(repo, "modulo ganha assinatura nova", {MODULO: _assinatura()})
    mod = _carregar()
    violacoes = _rodar_com_base(mod, repo, monkeypatch, base)
    assert any("PREVISTO" in v and MODULO in v for v in violacoes), violacoes


def test_previsao_libera_o_par_que_o_doc_acompanhou_no_range(tmp_path, monkeypatch) -> None:
    """O P7 no mesmo PR: o doc acompanha o modulo DEPOIS dele — nao cobra nada."""
    repo = _repo(tmp_path)
    _commit(repo, "modulo", {MODULO: _corpo(1)})
    _commit(repo, "doc", {DOC: _doc_do_modulo()})
    base = _sha(repo)
    _commit(repo, "modulo ganha assinatura nova", {MODULO: _assinatura()})
    _commit(repo, "doc acompanha o modulo (P7)", {DOC: _doc_do_modulo(extra="Já com `x2`.")})
    mod = _carregar()
    assert _rodar_com_base(mod, repo, monkeypatch, base) == []


def test_previsao_pega_o_verde_falso_das_linhas_paralelas(tmp_path, monkeypatch) -> None:
    """O caso MEDIDO no PR #260: o corte nasceu na branch, a revisao do doc chegou
    depois pelo merge da base. Sem o regime do PR o gate fica VERDE (a ancestralidade
    por arquivo nao tem resposta entre linhas paralelas) e a `develop` fica vermelha
    depois do squash.
    """
    repo = _repo(tmp_path)
    _commit(repo, "modulo", {MODULO: _corpo(1)})
    _git_branch(repo, "-b", "corte")
    _commit(repo, "corte: modulo ganha assinatura nova", {MODULO: _assinatura()})
    _git(repo, "checkout", "-q", "main")
    _commit(repo, "base: doc do modulo", {DOC: _doc_do_modulo()})
    base = _sha(repo)  # a base do PR: a revisao do doc veio junto com ela
    _git(repo, "merge", "-q", "--no-ff", "-m", "merge do PR (arvore sintetica)", "corte")

    mod = _carregar()
    assert _rodar(mod, repo, monkeypatch) == []  # o verde falso, medido
    violacoes = _rodar_com_base(mod, repo, monkeypatch, base)
    assert any("PREVISTO" in v and DOC in v for v in violacoes), violacoes


def test_sem_base_o_regime_e_o_da_develop(tmp_path, monkeypatch) -> None:
    """A previsao e do modo PR: sem `PR_BASE`/`--base` o gate nao muda de regime."""
    repo = _repo(tmp_path)
    _commit(repo, "modulo", {MODULO: _corpo(1)})
    _commit(repo, "doc", {DOC: _doc_do_modulo()})
    _commit(repo, "modulo ganha assinatura nova", {MODULO: _assinatura()})
    mod = _carregar()
    violacoes = _rodar(mod, repo, monkeypatch)
    assert any(v.startswith("VELHO") for v in violacoes), violacoes
    assert not [v for v in violacoes if "PREVISTO" in v], violacoes


# ── genero e materia-prima: o que o gate NAO pode cobrar ─────────────────────


def test_historico_e_evidencia_datada_ficam_fora(tmp_path, monkeypatch) -> None:
    """`docs/arquivo/**` e `docs/analises/**` envelhecem de proposito."""
    repo = _repo(tmp_path)
    _commit(repo, "modulo", {MODULO: _corpo(1)})
    _commit(
        repo,
        "docs de genero historico",
        {
            "docs/arquivo/velho.md": f"`{MODULO}` como era.\n",
            "docs/analises/auditoria.md": f"medido em 01/01: `{MODULO}` fazia X.\n",
        },
    )
    _commit(repo, "modulo ganha assinatura nova", {MODULO: _assinatura()})
    mod = _carregar()
    _rodar(mod, repo, monkeypatch)
    assert not [v for v in mod.VIOLACOES if "VELHO" in v], mod.VIOLACOES


def test_citacao_sem_caminho_e_dado_nao_sao_modulo(tmp_path, monkeypatch) -> None:
    """`session.rs` solto o gate nao adivinha; `.json`/`.yml` mudam por dado."""
    repo = _repo(tmp_path)
    _commit(repo, "modulo", {MODULO: _corpo(1)})
    _commit(
        repo,
        "doc de genero vivo",
        {
            DOC: "Veja `session.rs`, `modulo.py` (sem caminho) e `tools/dados.json`.\n",
            "tools/dados.json": "{}\n",
        },
    )
    _commit(repo, "modulo ganha assinatura nova", {MODULO: _assinatura()})
    mod = _carregar()
    _rodar(mod, repo, monkeypatch)
    medidos = mod.pares()
    assert medidos == [], medidos
