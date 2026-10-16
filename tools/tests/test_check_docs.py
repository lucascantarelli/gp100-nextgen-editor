"""Teste do gate de higiene da documentacao (`check_docs`, #270).

As tres checagens nasceram de uma medicao (29 arquivos sem banner, 27 contadores fora
do §6, **0** link morto) e cada uma tem o mesmo risco: acusar documento inocente. Um
gate que reprova quem esta certo e pior que nenhum — quem o encontra o desliga.

Por isso cada checagem tem teste dos DOIS lados: o caso que reprova e o vizinho que
nao pode ser punido (link de linha `#L42`; arquivo com front-matter, que nao aceita
banner markdown; porcentagem de protocolo, que nao e cobertura; `docs/analises/**` e
`docs/arquivo/**`, que sao evidencia e historico). E o gate roda sobre o repo REAL:
**0 contador fora do §6** e o baseline **vazio** — a divida do primeiro dia (25 contadores
datados) foi paga um a um, para o registro de `docs/analises/` ou para a medicao do §6.
Baseline folgado e deposito de divida invisivel: e por isso que o teto tem motivo e o
vazio e contrato, nao sobra.
"""

from __future__ import annotations

import importlib.util
import pathlib
import subprocess
import sys

_RAIZ = pathlib.Path(__file__).resolve().parents[2]
_SCRIPTS = _RAIZ / ".github" / "scripts"


def _carregar():
    spec = importlib.util.spec_from_file_location("check_docs", _SCRIPTS / "check_docs.py")
    mod = importlib.util.module_from_spec(spec)
    sys.modules["check_docs"] = mod
    spec.loader.exec_module(mod)
    return mod


def _limpar(mod) -> None:
    mod.VIOLACOES.clear()
    mod.CONTADOS.clear()


# ── contrato do repo real ────────────────────────────────────────────────────


def test_docs_do_repo_passa() -> None:
    """O exit code e o contrato: a documentacao atual = 0 violacoes."""
    r = subprocess.run(
        [sys.executable, str(_SCRIPTS / "check_docs.py")],
        cwd=_RAIZ, capture_output=True, text=True, encoding="utf-8",
    )
    assert r.returncode == 0, r.stdout + r.stderr
    assert "violacoes: 0" in r.stdout


def test_baseline_bate_com_a_medicao(tmp_path, monkeypatch) -> None:
    """Entrada folgada esconde divida: o teto tem de ser o numero medido, com motivo."""
    mod = _carregar()
    for arq, (teto, motivo) in mod.BASELINE_CONTADOR.items():
        assert motivo.strip(), f"baseline sem motivo: {arq}"
        assert (_RAIZ / arq).exists(), f"baseline aponta para arquivo inexistente: {arq}"
        texto = (_RAIZ / arq).read_text(encoding="utf-8")
        monkeypatch.setattr(mod, "RAIZ", _RAIZ)
        _limpar(mod)
        mod.checa_contador(_RAIZ / arq, texto)
        assert mod.CONTADOS.get(arq, 0) == teto, (
            f"{arq}: teto {teto} != medido {mod.CONTADOS.get(arq, 0)} — "
            "baixe o teto (divida paga) ou conserte o contador novo"
        )


def test_baseline_nasce_vazio() -> None:
    """Congelar divida nao e conserto (#270): o teto tem de estar vazio, e a entrada nova

    e decisao consciente — quem congelar contador muda ESTE teste junto, com o motivo escrito.
    """
    mod = _carregar()
    assert mod.BASELINE_CONTADOR == {}, (
        f"baseline com entrada: {sorted(mod.BASELINE_CONTADOR)} — contador datado vai para o "
        "registro de docs/analises/ e o doc vivo aponta para la; congelar exige mudar este teste"
    )


def test_repo_nao_tem_contador_fora_do_6() -> None:
    """O contrato do §6: medicao ZERO no repo real — nao "dentro do teto do baseline"."""
    r = subprocess.run(
        [sys.executable, str(_SCRIPTS / "check_docs.py")],
        cwd=_RAIZ, capture_output=True, text=True, encoding="utf-8",
    )
    assert r.returncode == 0, r.stdout + r.stderr
    assert "contadores fora do §6: 0" in r.stdout, r.stdout


# ── 1. LINK ──────────────────────────────────────────────────────────────────


def test_link_morto_reprova(tmp_path, monkeypatch) -> None:
    mod = _carregar()
    _limpar(mod)
    monkeypatch.setattr(mod, "RAIZ", tmp_path)
    p = tmp_path / "doc.md"
    p.write_text("veja [isto](nao-existe-mesmo.md).\n", encoding="utf-8")
    mod.checa_links({p: p.read_text(encoding="utf-8")})
    assert any("nao existe" in v for v in mod.VIOLACOES), mod.VIOLACOES


def test_link_e_ancora_validos_passam(tmp_path, monkeypatch) -> None:
    mod = _carregar()
    _limpar(mod)
    monkeypatch.setattr(mod, "RAIZ", tmp_path)
    alvo = tmp_path / "alvo.md"
    alvo.write_text("# Minha Seção\n", encoding="utf-8")
    p = tmp_path / "doc.md"
    p.write_text("veja [o alvo](alvo.md#minha-seção) e [também](alvo.md#L42).\n", encoding="utf-8")
    mod.checa_links({p: p.read_text(encoding="utf-8")})
    assert not mod.VIOLACOES, mod.VIOLACOES


def test_ancora_inexistente_reprova(tmp_path, monkeypatch) -> None:
    mod = _carregar()
    _limpar(mod)
    monkeypatch.setattr(mod, "RAIZ", tmp_path)
    alvo = tmp_path / "alvo.md"
    alvo.write_text("# Outra Coisa\n", encoding="utf-8")
    p = tmp_path / "doc.md"
    p.write_text("veja [o alvo](alvo.md#seção-que-nao-existe).\n", encoding="utf-8")
    mod.checa_links({p: p.read_text(encoding="utf-8")})
    assert any("ancora nao existe" in v for v in mod.VIOLACOES), mod.VIOLACOES


# ── 2. BANNER ────────────────────────────────────────────────────────────────


def test_banner_ausente_reprova(tmp_path, monkeypatch) -> None:
    mod = _carregar()
    _limpar(mod)
    monkeypatch.setattr(mod, "RAIZ", tmp_path)
    p = tmp_path / "doc.md"
    p.write_text("# Titulo\n\nTexto sem banner.\n", encoding="utf-8")
    mod.checa_banner({p: p.read_text(encoding="utf-8")})
    assert len(mod.VIOLACOES) == 2, mod.VIOLACOES  # sem marcador E sem data


def test_banner_completo_passa(tmp_path, monkeypatch) -> None:
    mod = _carregar()
    _limpar(mod)
    monkeypatch.setattr(mod, "RAIZ", tmp_path)
    p = tmp_path / "doc.md"
    p.write_text("> ✅ **atual** · última revisão 2026-10-15.\n\n# Titulo\n", encoding="utf-8")
    mod.checa_banner({p: p.read_text(encoding="utf-8")})
    assert not mod.VIOLACOES, mod.VIOLACOES


def test_frontmatter_nao_e_punido(tmp_path) -> None:
    """Skill arquivada: banner markdown ANTES do `---` invalida o bloco YAML."""
    mod = _carregar()
    _limpar(mod)
    p = tmp_path / "SKILL.md"
    p.write_text("---\nname: x\ndescription: y\n---\n\n# Skill\n", encoding="utf-8")
    mod.checa_banner({p: p.read_text(encoding="utf-8")})
    assert not mod.VIOLACOES, mod.VIOLACOES


def test_banner_historico_sem_ponteiro_reprova(tmp_path, monkeypatch) -> None:
    """A §4.4 do INDEX: o arquivado aponta para onde o assunto vive agora."""
    mod = _carregar()
    _limpar(mod)
    monkeypatch.setattr(mod, "RAIZ", tmp_path)
    p = tmp_path / "doc.md"
    p.write_text(
        "> 📜 **histórico** — arquivado na #168 · última revisão 2026-10-13.\n",
        encoding="utf-8",
    )
    mod.checa_banner({p: p.read_text(encoding="utf-8")})
    assert len(mod.VIOLACOES) == 1 and "sem ponteiro" in mod.VIOLACOES[0], mod.VIOLACOES


def test_banner_historico_com_ponteiro_passa(tmp_path, monkeypatch) -> None:
    mod = _carregar()
    _limpar(mod)
    monkeypatch.setattr(mod, "RAIZ", tmp_path)
    p = tmp_path / "doc.md"
    p.write_text(
        "> 📜 **histórico** — arquivado na #168 · última revisão 2026-10-13"
        " · substituto: `docs/INDEX.md` §3.\n",
        encoding="utf-8",
    )
    mod.checa_banner({p: p.read_text(encoding="utf-8")})
    assert not mod.VIOLACOES, mod.VIOLACOES


def test_doc_vivo_nao_precisa_de_ponteiro(tmp_path, monkeypatch) -> None:
    """A regra é do ARQUIVADO: o doc vivo não aponta para substituto nenhum."""
    mod = _carregar()
    _limpar(mod)
    monkeypatch.setattr(mod, "RAIZ", tmp_path)
    p = tmp_path / "doc.md"
    p.write_text("> ✅ **atual** · última revisão 2026-10-15.\n", encoding="utf-8")
    mod.checa_banner({p: p.read_text(encoding="utf-8")})
    assert not mod.VIOLACOES, mod.VIOLACOES


# ── 3. CONTADOR ──────────────────────────────────────────────────────────────


def _conta(mod, monkeypatch, tmp_path, rel: str, texto: str) -> int:
    monkeypatch.setattr(mod, "RAIZ", tmp_path)
    p = tmp_path / rel
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(texto, encoding="utf-8")
    _limpar(mod)
    mod.checa_contador(p, texto)
    return mod.CONTADOS.get(rel, 0)


def test_contador_fora_do_index_reprova(tmp_path, monkeypatch) -> None:
    mod = _carregar()
    assert _conta(mod, monkeypatch, tmp_path, "docs/dev/x.md", "rodamos 42 testes aqui\n") > 0
    assert any("CONTADOR" in v for v in mod.VIOLACOES), mod.VIOLACOES


def test_contador_no_index_ou_em_evidencia_nao_conta(tmp_path, monkeypatch) -> None:
    mod = _carregar()
    for rel in ("docs/INDEX.md", "docs/analises/a.md", "docs/arquivo/b.md"):
        assert _conta(mod, monkeypatch, tmp_path, rel, "42 testes e 13 gates\n") == 0, rel
    assert not mod.VIOLACOES, mod.VIOLACOES


def test_porcentagem_de_protocolo_nao_e_cobertura(tmp_path, monkeypatch) -> None:
    """`99,57%` do PROTOCOL.md e metrica de byte-stream, nao contador de repositorio."""
    mod = _carregar()
    assert _conta(mod, monkeypatch, tmp_path, "docs/tecnico/p.md", "assertividade 99,57% no fio\n") == 0
    assert _conta(mod, monkeypatch, tmp_path, "docs/tecnico/c.md", "cobertura 87,62% stmts\n") > 0


def test_teto_congelado_bloqueia_o_novo_e_libera_o_existente(tmp_path, monkeypatch) -> None:
    mod = _carregar()
    monkeypatch.setitem(mod.BASELINE_CONTADOR, "docs/dev/base.md", (2, "motivo do teto"))
    assert _conta(mod, monkeypatch, tmp_path, "docs/dev/base.md", "12 gates e 13 testes\n") == 2
    assert not mod.VIOLACOES, mod.VIOLACOES
    _conta(mod, monkeypatch, tmp_path, "docs/dev/base.md", "12 gates, 13 testes e 14 baselines\n")
    assert any("baseline congelado" in v for v in mod.VIOLACOES), mod.VIOLACOES
