"""Teste do gate da auditoria de movimento (`check_move_only`, #263).

O gate existe para uma promessa que nenhum outro gate cobre: "este commit so
moveu texto". A suite de testes prova COMPORTAMENTO — nao acusa comentario
perdido, doc trocado nem um `if` invertido que ainda passa —, e o revisor nao
confere 1 300 linhas linha a linha. O modo de falhar daqui e o pior dos dois:
silencioso (um movimento que perdeu uma linha passa como movimento) ou barulhento
demais (o andaime legitimo de um corte — `mod`, `use`, `impl`, fechamento,
visibilidade `pub(super)` — reprovando). Por isso os dois lados tem teste, em
repos git SINTETICOS: inflar movimento de verdade para provar que o gate reprova
seria teste que ninguem roda duas vezes.

O fixture imita o corte REAL (#257/PR #260): os itens saem de dentro de um `impl`
e chegam no modulo novo DENTRO de outro `impl`, com a MESMA indentacao — so a
assinatura ganha o `pub(super)`. Indentacao faz parte do texto: um movimento que
re-indenta nao e movimento puro, e o gate diz isso.

Os testes chamam as funcoes do gate direto (auditoria/parse) e, nos casos de
contrato, o proprio script pela linha de comando — porque o que a CI cobra e o
exit code.
"""

from __future__ import annotations

import importlib.util
import os
import pathlib
import subprocess
import sys

import pytest

_RAIZ = pathlib.Path(__file__).resolve().parents[2]
_SCRIPTS = _RAIZ / ".github" / "scripts"

# Git deterministico e sem depender da config de quem roda.
AMBIENTE = {
    **os.environ,
    "GIT_AUTHOR_NAME": "teste",
    "GIT_AUTHOR_EMAIL": "teste@teste",
    "GIT_COMMITTER_NAME": "teste",
    "GIT_COMMITTER_EMAIL": "teste@teste",
    "GIT_AUTHOR_DATE": "2026-10-14T12:00:00-03:00",
    "GIT_COMMITTER_DATE": "2026-10-14T12:00:00-03:00",
}


def _carregar():
    """Importa o gate como modulo (padrao dos specs)."""
    spec = importlib.util.spec_from_file_location(
        "check_move_only", _SCRIPTS / "check_move_only.py"
    )
    mod = importlib.util.module_from_spec(spec)
    sys.modules["check_move_only"] = mod
    spec.loader.exec_module(mod)
    return mod


def _git(raiz: pathlib.Path, *args: str) -> str:
    r = subprocess.run(
        ["git", *args], cwd=raiz, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=AMBIENTE
    )
    assert r.returncode == 0, f"git {' '.join(args)}: {r.stderr.decode('utf-8', 'replace')}"
    return r.stdout.decode("utf-8", "replace")


def _repo(tmp_path: pathlib.Path) -> pathlib.Path:
    raiz = tmp_path / "repo"
    raiz.mkdir()
    _git(raiz, "init", "-q", "-b", "main")
    return raiz


def _commit(raiz: pathlib.Path, mensagem: str, arquivos: dict[str, str]) -> str:
    """Escreve os arquivos, commita e devolve o sha."""
    for rel, texto in arquivos.items():
        caminho = raiz / rel
        caminho.parent.mkdir(parents=True, exist_ok=True)
        caminho.write_text(texto, encoding="utf-8", newline="\n")
    _git(raiz, "add", "-A")
    _git(raiz, "commit", "-q", "-m", mensagem, "--no-verify")
    return _git(raiz, "rev-parse", "HEAD").strip()


# ── o fixture: o corte de um arquivo em modulo (como o do session.rs) ───────────

DOC_ORIGEM = "//! session — o arquivo que vai ser fatiado."
USE_ORIGEM = "use crate::ProtocolError;"
ABRE_IMPL = "impl<T: DeviceTransport> Session<T> {"

CORPO = """\
    /// Le a pagina e devolve o payload.
    fn le_pagina(pp: u16) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(pp as u8);
        out
    }

    /// Grava o pp corrente (fire-and-forget).
    fn grava_pp(&mut self, pp: u16) {
        self.current_pp = pp;
    }
"""

# O mesmo corpo com a visibilidade que um corte de verdade precisa acrescentar.
CORPO_PUB = CORPO.replace("    fn ", "    pub(super) fn ")

ORIGEM = f"{DOC_ORIGEM}\n{USE_ORIGEM}\n\n{ABRE_IMPL}\n{CORPO}}}\n"

# A fachada: o doc da origem continua (o corte nao reescreve o cabecalho) e o
# `mod`/`pub use` sao o andaime que liga o modulo novo.
FACADE = f"{DOC_ORIGEM}\n{USE_ORIGEM}\n\nmod transacao;\n\npub use transacao::{{grava_pp, le_pagina}};\n"


def destino(corpo: str = CORPO_PUB, imports: str = "use super::Session;\n") -> str:
    """O modulo novo: doc, imports, `impl` e o corpo MOVIDO (com `pub(super)`)."""
    return (
        "//! transacao — o pedido e a resposta.\n"
        "\n"
        f"{USE_ORIGEM}\n"
        "\n"
        f"{imports}"
        "\n"
        f"{ABRE_IMPL}\n"
        f"{corpo}}}\n"
    )


def _movimento(tmp_path: pathlib.Path, destino_txt: str = "", facade: str = ""):
    """Repo com o corte: commit 1 traz a origem, commit 2 declara o movimento."""
    raiz = _repo(tmp_path)
    base = _commit(raiz, "chore: base", {"src/a.rs": ORIGEM})
    topo = _commit(
        raiz,
        "refactor(core): fatia a.rs\n\nMove-only: src/a.rs",
        {"src/a.rs": facade or FACADE, "src/transacao.rs": destino_txt or destino()},
    )
    return raiz, base, topo


# ── o caso bom: movimento nao perde nem reescreve ──────────────────────────────


def test_movimento_puro_passa(tmp_path):
    raiz, base, topo = _movimento(tmp_path)
    cm = _carregar()
    r = cm.audita(raiz, "src/a.rs", base, topo, "teste")
    assert r.ok, (r.faltando, r.fora)
    assert r.linhas_item > 0
    assert r.andaime > 0  # cabecalho/`use`/`impl`/fechamento do modulo novo


def test_trailer_no_commit_e_lido(tmp_path):
    raiz, _, topo = _movimento(tmp_path)
    cm = _carregar()
    assert cm.declara_movimento(raiz, topo) == ["src/a.rs"]


def test_visibilidade_pub_super_nao_e_conteudo_novo(tmp_path):
    raiz, base, topo = _movimento(tmp_path)
    cm = _carregar()
    r = cm.audita(raiz, "src/a.rs", base, topo, "teste")
    assert r.faltando == []
    assert r.fora == []  # nenhuma assinatura virou "conteudo novo"


def test_use_de_varias_linhas_continua_sendo_andaime(tmp_path):
    """O rustfmt quebra a lista de import: as linhas de dentro tambem sao andaime."""
    raiz, base, topo = _movimento(
        tmp_path,
        destino_txt=destino(imports="use super::{\n    Session, transacao,\n};\n"),
    )
    cm = _carregar()
    r = cm.audita(raiz, "src/a.rs", base, topo, "teste")
    assert r.ok, (r.faltando, r.fora)


def test_doc_alterado_no_mesmo_range_nao_interfere(tmp_path):
    """A clausula do andaime vale para arquivos do MESMO tipo: doc nao entra."""
    raiz = _repo(tmp_path)
    base = _commit(raiz, "chore: base", {"src/a.rs": ORIGEM})
    topo = _commit(
        raiz,
        "refactor(core): fatia a.rs\n\nMove-only: src/a.rs",
        {
            "src/a.rs": FACADE,
            "src/transacao.rs": destino(),
            "docs/nota.md": "# nota\n\nTexto novo, que um movimento nao explica.\n",
        },
    )
    cm = _carregar()
    r = cm.audita(raiz, "src/a.rs", base, topo, "teste")
    assert r.ok, (r.faltando, r.fora)


# ── o caso ruim: perdeu, reescreveu, inventou ─────────────────────────────────


def test_linha_perdida_reprova_nomeando_a_linha(tmp_path):
    raiz, base, topo = _movimento(
        tmp_path, destino_txt=destino(CORPO_PUB.replace("        out.push(pp as u8);\n", ""))
    )
    cm = _carregar()
    r = cm.audita(raiz, "src/a.rs", base, topo, "teste")
    assert not r.ok
    assert any("out.push(pp as u8);" in linha for linha in r.faltando)


def test_linha_reescrita_reprova_como_conteudo_novo(tmp_path):
    raiz, base, topo = _movimento(
        tmp_path, destino_txt=destino(CORPO_PUB.replace("out.push(pp as u8);", "out.push(pp as u16);"))
    )
    cm = _carregar()
    r = cm.audita(raiz, "src/a.rs", base, topo, "teste")
    assert any("out.push(pp as u16);" in linha for _, linha in r.fora)
    assert any("out.push(pp as u8);" in linha for linha in r.faltando)


def test_codigo_inventado_reprova(tmp_path):
    """Linha nova que nao e andaime: um corte nao inventa logica."""
    raiz, base, topo = _movimento(
        tmp_path, destino_txt=destino(CORPO_PUB + "    fn inventada(&self) {}\n")
    )
    cm = _carregar()
    r = cm.audita(raiz, "src/a.rs", base, topo, "teste")
    assert not r.ok
    assert ("src/transacao.rs", "    fn inventada(&self) {}") in r.fora


def test_reindentar_reprova(tmp_path):
    """Indentacao faz parte do texto: mover re-indentando nao e movimento puro."""
    raiz, base, topo = _movimento(
        tmp_path, destino_txt=destino(CORPO_PUB.replace("    ", "        "))
    )
    cm = _carregar()
    r = cm.audita(raiz, "src/a.rs", base, topo, "teste")
    assert not r.ok


def test_origem_apagada_por_inteiro_ainda_audita(tmp_path):
    """Corte total: a origem sai do repo e o texto dela tem de estar no destino.

    A fachada guarda o cabecalho (doc + `use`) e o modulo novo, o corpo: e o que
    o corte do `session.rs` fez. Se o cabecalho sumisse, o gate reprova — e
    reprovar ali e o comportamento correto (a linha foi perdida).
    """
    raiz = _repo(tmp_path)
    base = _commit(raiz, "chore: base", {"src/a.rs": ORIGEM})
    (raiz / "src" / "a.rs").unlink()
    topo = _commit(
        raiz,
        "refactor(core): fatia a.rs\n\nMove-only: src/a.rs",
        {
            "src/fachada.rs": f"{DOC_ORIGEM}\n{USE_ORIGEM}\n",
            "src/transacao.rs": destino(),
        },
    )
    cm = _carregar()
    r = cm.audita(raiz, "src/a.rs", base, topo, "teste")
    assert r.ok, (r.faltando, r.fora)


def test_origem_com_caminho_errado_e_erro_de_uso(tmp_path):
    """Trailer apontando para arquivo que nao existe no pai nao pode passar."""
    raiz, base, _ = _movimento(tmp_path)
    cm = _carregar()
    with pytest.raises(cm.UsoErro):
        cm.audita(raiz, "src/nao_existe.rs", base, base, "teste")


# ── o contrato do CLI (o que a CI cobra: o exit code) ─────────────────────────


def _cli(raiz: pathlib.Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(_SCRIPTS / "check_move_only.py"), "--raiz", str(raiz), *args],
        capture_output=True,
        text=True,
        env={k: v for k, v in AMBIENTE.items() if k not in ("PR_BASE", "BEFORE")},
    )


def test_cli_sem_trailer_nao_audita_e_sai_zero(tmp_path):
    raiz = _repo(tmp_path)
    _commit(raiz, "chore: base", {"src/a.rs": ORIGEM})
    _commit(raiz, "feat: acrescenta codigo", {"src/b.rs": "fn nova() {}\n"})
    saida = _cli(raiz)
    assert saida.returncode == 0, saida.stderr
    assert "nada a auditar" in saida.stdout


def test_cli_movimento_puro_sai_zero(tmp_path):
    raiz, _, _ = _movimento(tmp_path)
    saida = _cli(raiz)
    assert saida.returncode == 0, saida.stderr
    assert "perdida(s)" in saida.stdout and "Move-only: src/a.rs" in saida.stdout


def test_cli_movimento_com_perda_sai_um(tmp_path):
    raiz, _, _ = _movimento(
        tmp_path, destino_txt=destino(CORPO_PUB.replace("        out.push(pp as u8);\n", ""))
    )
    saida = _cli(raiz)
    assert saida.returncode == 1
    assert "PERDIDAS" in saida.stderr


def test_cli_base_irresolvivel_sai_dois(tmp_path):
    raiz, _, _ = _movimento(tmp_path)
    saida = _cli(raiz, "--from", "nao-existe")
    assert saida.returncode == 2
    assert "base irresolvivel" in saida.stderr


def test_cli_declared_audita_range_sem_trailer(tmp_path):
    """Modo retroativo: audita squash/merge em que o commit nao existe mais."""
    raiz = _repo(tmp_path)
    base = _commit(raiz, "chore: base", {"src/a.rs": ORIGEM})
    _commit(
        raiz,
        "refactor(core): fatia a.rs (sem trailer)",
        {"src/a.rs": FACADE, "src/transacao.rs": destino()},
    )
    saida = _cli(raiz, "--declared", "src/a.rs", "--from", base, "--to", "HEAD")
    assert saida.returncode == 0, saida.stderr
    assert "declarado: src/a.rs" in saida.stdout


def test_cli_declared_com_perda_sai_um(tmp_path):
    raiz = _repo(tmp_path)
    base = _commit(raiz, "chore: base", {"src/a.rs": ORIGEM})
    _commit(
        raiz,
        "refactor(core): fatia a.rs (sem trailer)",
        {
            "src/a.rs": FACADE,
            "src/transacao.rs": destino(CORPO_PUB.replace("        out.push(pp as u8);\n", "")),
        },
    )
    saida = _cli(raiz, "--declared", "src/a.rs", "--from", base, "--to", "HEAD")
    assert saida.returncode == 1
    assert "PERDIDAS" in saida.stderr
