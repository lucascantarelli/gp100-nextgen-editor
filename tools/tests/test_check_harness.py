"""Teste do gate de higiene do harness (`check_harness`, #249/15-10).

O gate nasceu de três sondas de rascunho (`.re_tmp/check_{agent_paths,
harness_frontmatter,harness_dedup}.py`) que fecharam a #249 por leitura manual —
e a própria #249 mediu duas vezes "está tudo certo" quando não estava. Gate que
ninguém testa aprova por engano, e o modo de falhar aqui é o pior: se a extração
de code spans não achar nada, "zero caminho morto" é a resposta de uma varredura
vazia.

Por isso os testes usam ÁRVORES SINTÉTICAS (cada checagem tem o caso que
REPROVA e o caso que o vizinho não pode punir) e, além disso, conferem o
contrato no repo real: o harness de hoje passa, e todo caminho na allowlist de
exceção existe de fato *não* existindo — allowlist que aponta para caminho vivo é
exceção esquecida.

Os testes chamam as funções direto (a checagem) e o script pela linha de comando
(o que a CI cobra é o exit code).
"""

from __future__ import annotations

import importlib.util
import pathlib
import subprocess
import sys

_RAIZ = pathlib.Path(__file__).resolve().parents[2]
_SCRIPTS = _RAIZ / ".github" / "scripts"


def _carregar():
    """Importa `.github/scripts/check_harness.py` como módulo (padrão dos specs)."""
    spec = importlib.util.spec_from_file_location("check_harness", _SCRIPTS / "check_harness.py")
    mod = importlib.util.module_from_spec(spec)
    sys.modules["check_harness"] = mod
    spec.loader.exec_module(mod)
    return mod


def _limpar(mod) -> None:
    mod.VIOLACOES.clear()


# ── contrato do repo real ────────────────────────────────────────────────────


def test_harness_do_repo_passa() -> None:
    """O exit code é o contrato: harness atual = 0 violações."""
    r = subprocess.run(
        [sys.executable, str(_SCRIPTS / "check_harness.py")],
        cwd=_RAIZ,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    assert r.returncode == 0, r.stdout + r.stderr
    assert "violacoes: 0" in r.stdout


def test_allowlist_aponta_para_caminho_que_nao_existe() -> None:
    """Exceção só vale para caminho ausente; senão vira caminho vivo escondido."""
    mod = _carregar()
    for caminho, motivo in mod.CAMINHOS_DE_PROPOSITO.items():
        assert motivo.strip(), f"allowlist sem motivo: {caminho}"
        assert not (_RAIZ / caminho).exists(), f"allowlist desnecessária (existe): {caminho}"


# ── 1. CAMINHO ───────────────────────────────────────────────────────────────


def test_caminho_morto_reprova() -> None:
    mod = _carregar()
    _limpar(mod)
    mod.checa_caminhos({"AGENTS.md": "veja `docs/nao-existe-mesmo-assim.md`."})
    assert len(mod.VIOLACOES) == 1
    assert "docs/nao-existe-mesmo-assim.md" in mod.VIOLACOES[0]


def test_caminho_curto_passa_quando_o_arquivo_declara_a_base() -> None:
    """`actor/fila.rs` ao lado da rota-cheia é navegação, não caminho morto."""
    mod = _carregar()
    _limpar(mod)
    mod.checa_caminhos(
        {".agents/skills/app-dev/SKILL.md": "## O actor (`packages/app/api/src/actor/`)\n\n| `actor/fila.rs` | a fila |"}
    )
    assert not mod.VIOLACOES


def test_caminho_curto_sem_base_no_arquivo_reprova() -> None:
    """Sem a rota-cheia em lugar nenhum, o curto não é resolvível — é ref solta."""
    mod = _carregar()
    _limpar(mod)
    mod.checa_caminhos({".agents/skills/app-dev/SKILL.md": "veja `actor/fila.rs`."})
    assert len(mod.VIOLACOES) == 1


def test_nome_curto_sem_barra_nao_e_caminho() -> None:
    mod = _carregar()
    _limpar(mod)
    mod.checa_caminhos({"knowledge.md": "o `ci.yml` e o `actor.rs` são citados por nome."})
    assert not mod.VIOLACOES


# ── 4. FONTE ÚNICA ───────────────────────────────────────────────────────────


def _violacoes_da_regra(mod, trecho: str) -> list[str]:
    """Só as violações da regra sob teste.

    As fontes sintéticas trazem UMA dona; as outras cinco regras acusam "dona
    fora do escopo" (correto para um recorte, ruído para a asserção).
    """
    return [v for v in mod.VIOLACOES if trecho.lower() in v.lower()]


def _fontes_fonte_unica(extra_skill: str) -> dict[str, str]:
    return {
        ".agents/skills/hardware-dev/SKILL.md": "## Regra\n1. O aparelho é a fonte: **medir, não deduzir**.",
        ".agents/skills/outra/SKILL.md": extra_skill,
        "knowledge.md": "Premissa do trabalho: o aparelho real é a fonte.",
    }


def test_re_enunciado_em_skill_reprova() -> None:
    mod = _carregar()
    _limpar(mod)
    mod.checa_fonte_unica(
        _fontes_fonte_unica("## Como ler valor\nSempre medir, não deduzir: leia o valor do aparelho.")
    )
    assert any("re-enunciada" in v for v in _violacoes_da_regra(mod, "aparelho")), mod.VIOLACOES


def test_ponteiro_para_a_dona_nao_reprova() -> None:
    mod = _carregar()
    _limpar(mod)
    mod.checa_fonte_unica(
        _fontes_fonte_unica("A regra do aparelho vive na skill `hardware-dev` (medir, não deduzir).")
    )
    assert not _violacoes_da_regra(mod, "aparelho"), mod.VIOLACOES


def test_dona_que_nao_enuncia_reprova() -> None:
    """Regra órfã (a dona perdeu o texto) é a outra ponta do mesmo defeito."""
    mod = _carregar()
    _limpar(mod)
    fontes = _fontes_fonte_unica("nada aqui")
    fontes[".agents/skills/hardware-dev/SKILL.md"] = "skill sem a regra"
    mod.checa_fonte_unica(fontes)
    assert any("nenhuma dona" in v for v in _violacoes_da_regra(mod, "aparelho")), mod.VIOLACOES


def test_prompt_citando_a_premissa_nao_e_re_enunciado() -> None:
    """Prompt e knowledge são ENTRADA do trabalho, não skill — o teto é por skill."""
    mod = _carregar()
    _limpar(mod)
    fontes = _fontes_fonte_unica("nada")
    fontes[".agents/prompts/implementar-issue.md"] = "Antes de codar: medir, não deduzir."
    mod.checa_fonte_unica(fontes)
    assert not _violacoes_da_regra(mod, "aparelho"), mod.VIOLACOES


# ── 2/3. FRONT-MATTER e PAPEL ────────────────────────────────────────────────


def test_frontmatter_fora_do_padrao_reprova(tmp_path, monkeypatch) -> None:
    mod = _carregar()
    _limpar(mod)
    sk = tmp_path / ".agents" / "skills" / "minha" / "SKILL.md"
    sk.parent.mkdir(parents=True)
    sk.write_text("---\nname: outro\n---\n", encoding="utf-8")
    monkeypatch.setattr(mod, "AGENTS", tmp_path / ".agents")
    monkeypatch.setattr(mod, "RAIZ", tmp_path)
    mod.checa_frontmatter()
    assert any("!= pasta" in v for v in mod.VIOLACOES), mod.VIOLACOES


def test_papel_orfao_e_fantasma_reprovam(tmp_path, monkeypatch) -> None:
    mod = _carregar()
    _limpar(mod)
    agentes = tmp_path / ".agents" / "agents"
    agentes.mkdir(parents=True)
    (agentes / "revisor.md").write_text("# Papel\n", encoding="utf-8")       # órfão
    (agentes / "README.md").write_text(
        "| [`verificador.md`](.agents/agents/verificador.md) |\n", encoding="utf-8"
    )  # fantasma + revisor não citado
    monkeypatch.setattr(mod, "AGENTS", tmp_path / ".agents")
    monkeypatch.setattr(mod, "RAIZ", tmp_path)
    mod.checa_papeis()
    texto = " ".join(mod.VIOLACOES)
    assert "revisor.md existe mas nao e citado" in texto
    assert "citado e ausente" in texto or "que nao existe" in texto
# ── 5. TABELA (saída do gate, não texto de doc) ──────────────────────────────
#
# A tabela de fontes únicas do `.agents/README.md` era escrita à mão e listava 9
# regras enquanto o REGRAS do gate cobria 6 — duas colunas que ninguém comparava.
# Agora a tabela é EMITIDA do REGRAS: quem edita à mão reprova, e a lista de linhas
# é o próprio dict. Os testes fixam as duas pontas: o README real bate com o gate, e
# o que diverge dele é reprovado.


def _readme(bloco: str) -> str:
    return (
        "# `.agents/` — o harness do agente\n\n"
        "## Fontes únicas por regra (não duplicar)\n\n"
        f"{bloco}\n\n"
        "## Modo ausente do dono (#202)\n"
    )


def test_tabela_do_readme_bate_com_o_gate() -> None:
    """O contrato do repo: a tabela publicada é exatamente a que o gate emite."""
    mod = _carregar()
    texto = (_RAIZ / mod.TABELA_DOC).read_text(encoding="utf-8")
    assert mod.bloco_no_readme(texto) == mod.bloco_gerado()


def test_tabela_editada_a_mao_reprova(tmp_path, monkeypatch) -> None:
    mod = _carregar()
    _limpar(mod)
    monkeypatch.setattr(mod, "RAIZ", tmp_path)
    destino = tmp_path / mod.TABELA_DOC
    destino.parent.mkdir(parents=True)
    tabela = mod.bloco_gerado()
    destino.write_text(
        _readme(tabela.replace("| apontam |", "| apontam (a mao) |", 1)), encoding="utf-8"
    )
    mod.checa_tabela()
    assert any("divergiu" in v for v in mod.VIOLACOES), mod.VIOLACOES


def test_tabela_sem_marcadores_reprova(tmp_path, monkeypatch) -> None:
    mod = _carregar()
    _limpar(mod)
    monkeypatch.setattr(mod, "RAIZ", tmp_path)
    destino = tmp_path / mod.TABELA_DOC
    destino.parent.mkdir(parents=True)
    destino.write_text(_readme("| Assunto | Casa (dono) | Os demais |"), encoding="utf-8")
    mod.checa_tabela()
    assert any("marcadores" in v for v in mod.VIOLACOES), mod.VIOLACOES


def test_gera_tabela_escreve_lf_e_e_idempotente(tmp_path, monkeypatch) -> None:
    """Gerador que suja a árvore a cada rodada é dívida (docs-sync 10)."""
    mod = _carregar()
    _limpar(mod)
    monkeypatch.setattr(mod, "RAIZ", tmp_path)
    destino = tmp_path / mod.TABELA_DOC
    destino.parent.mkdir(parents=True)
    destino.write_text(
        _readme(f"{mod.TABELA_INICIO} -->\n{mod.TABELA_FIM}"), encoding="utf-8"
    )

    assert mod.gera_tabela() == 0
    primeira = destino.read_bytes()
    assert b"\r" not in primeira, "o gerador escreveu CR: o .gitattributes é eol=lf"
    assert mod.bloco_no_readme(primeira.decode("utf-8")) == mod.bloco_gerado()

    assert mod.gera_tabela() == 0
    assert destino.read_bytes() == primeira, "gerar de novo mudou o arquivo"

    _limpar(mod)
    mod.checa_tabela()
    assert not mod.VIOLACOES, mod.VIOLACOES


def test_gera_tabela_sem_marcadores_falha_alto(tmp_path, monkeypatch) -> None:
    """Sem marcador o gerador não adivinha: exit 1 em vez de reescrever o arquivo."""
    mod = _carregar()
    _limpar(mod)
    monkeypatch.setattr(mod, "RAIZ", tmp_path)
    destino = tmp_path / mod.TABELA_DOC
    destino.parent.mkdir(parents=True)
    original = _readme("| tabela à mão | x | y |")
    destino.write_text(original, encoding="utf-8")

    assert mod.gera_tabela() == 1
    assert destino.read_text(encoding="utf-8") == original


def test_toda_regra_tem_linha_e_dona_no_disco() -> None:
    """A tabela é o REGRAS: nenhuma regra sem linha, nenhuma dona fantasma."""
    mod = _carregar()
    tabela = "\n".join(mod.linhas_da_tabela())
    for regra, (donas, _) in mod.REGRAS.items():
        assert f"| {regra} |" in tabela, regra
        for dona in donas:
            assert (_RAIZ / dona).exists(), f"regra {regra}: dona inexistente {dona}"
    linhas = [l for l in mod.linhas_da_tabela() if l.startswith("| ") and "---" not in l]
    assert len(linhas) == len(mod.REGRAS) + 1, "linha na tabela sem regra no gate"


def test_dona_fora_da_varredura_e_lida_do_disco() -> None:
    """A casa pode ser doc de área (`docs/dev/ARCHITECTURE.md`): o gate lê do disco."""
    mod = _carregar()
    _limpar(mod)
    mod.checa_fonte_unica({})
    assert not mod.VIOLACOES, mod.VIOLACOES
