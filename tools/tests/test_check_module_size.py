"""Teste do gate de orcamento de tamanho (`check_module_size`, #248).

O gate nasceu do front (#82) e passou a cobrar os `.rs` de `packages/*/src` na
#248, quando a auditoria de 14/10 mostrou que o Rust nao tinha freio nenhum
(`actor.rs` chegou a 2 702 linhas, F-07). A #262 acrescentou a segunda familia de
coorte — os modulos de `session/`, que o corte da #257 tirou de um arquivo de
1 395 linhas e que, sob o generico de 700, poderiam voltar a ser um deles. Um gate
que ninguem testa e um gate que passa a aprovar por engano — e o modo de falhar
aqui e silencioso: se o `rglob` nao achar os `.rs`, "dentro do orcamento" e a
resposta de uma arvore vazia.

Por isso o teste mede ARVORES SINTETICAS (inflar um arquivo de verdade para
provar que o gate reprova seria um teste que ninguem roda duas vezes) e, alem
disso, confere o orcamento DECLARADO no repo real: todo caminho em `TETOS` /
`TETOS_RUST` tem de existir — senao renomear um arquivo apaga o teto dele sem
que nada acuse.
"""

from __future__ import annotations

import importlib.util
import pathlib
import re
import sys

_RAIZ = pathlib.Path(__file__).resolve().parents[2]
_SCRIPTS = _RAIZ / ".github" / "scripts"


def _carregar():
    """Importa `.github/scripts/check_module_size.py` como modulo (padrao dos specs)."""
    spec = importlib.util.spec_from_file_location(
        "check_module_size", _SCRIPTS / "check_module_size.py"
    )
    mod = importlib.util.module_from_spec(spec)
    sys.modules["check_module_size"] = mod
    spec.loader.exec_module(mod)
    return mod


def _arvore(tmp_path: pathlib.Path, arquivos: dict[str, int]) -> pathlib.Path:
    """Monta a arvore sintetica: `caminho -> n. de linhas`."""
    for rel, linhas in arquivos.items():
        caminho = tmp_path / rel
        caminho.parent.mkdir(parents=True, exist_ok=True)
        caminho.write_text("\n".join(f"// {i}" for i in range(linhas)) + "\n", encoding="utf-8")
    return tmp_path


# ── o orcamento declarado no repo real ──────────────────────────────────────


def test_estado_atual_do_repo_dentro_do_orcamento():
    """A `develop` passa: nenhum excesso e nenhum teto declarado orfao."""
    cm = _carregar()
    excessos, ausentes = cm.avalia_rust(_RAIZ)
    assert ausentes == [], f"teto declarado sem arquivo: {ausentes}"
    assert excessos == [], f"acima do teto: {excessos}"


def test_todo_teto_declarado_do_front_existe():
    """Renomear App.tsx/device.ts nao pode apagar o teto em silencio."""
    cm = _carregar()
    faltando = [rel for rel, _, _ in cm.TETOS if not (cm.UI / rel).is_file()]
    assert faltando == [], f"declarado no orcamento e ausente: {faltando}"


def test_excecao_congelada_cita_a_issue_do_corte():
    """Teto declarado acima do generico e excecao: precisa do corte escrito."""
    cm = _carregar()
    sem_issue = [
        rel
        for rel, _, motivo in cm.TETOS_RUST
        if "congelado" in motivo and not re.search(r"#\d+", motivo)
    ]
    assert sem_issue == [], f"congelado sem issue do corte: {sem_issue}"


# ── teto generico ───────────────────────────────────────────────────────────


def test_arquivo_rust_acima_do_generico_reprova_com_nome_e_contagem(tmp_path):
    arvore = _arvore(tmp_path, {"packages/core/src/gordo.rs": 701})
    cm = _carregar()
    excessos, ausentes = cm.avalia_rust(arvore, tetos=[])
    assert ausentes == []
    assert excessos == [("packages/core/src/gordo.rs", 701, 700)]


def test_arquivo_rust_no_teto_passa(tmp_path):
    arvore = _arvore(tmp_path, {"packages/core/src/no_limite.rs": 700})
    cm = _carregar()
    excessos, _ = cm.avalia_rust(arvore, tetos=[])
    assert excessos == []


def test_coorte_do_actor_tem_teto_300(tmp_path):
    """A coorte (#243/ARCHITECTURE 4.1) e mais estreita que o generico."""
    arvore = _arvore(tmp_path, {"packages/app/api/src/actor/leitura.rs": 301})
    cm = _carregar()
    excessos, _ = cm.avalia_rust(arvore, tetos=[])
    assert excessos == [("packages/app/api/src/actor/leitura.rs", 301, 300)]


# ── coorte do session (#262) ────────────────────────────────────────────


def test_modulo_do_session_acima_da_coorte_reprova(tmp_path):
    """Os modulos do corte da #257 sao UM assunto cada — e o teto e 400."""
    arvore = _arvore(tmp_path, {"packages/core/src/session/escrita.rs": 401})
    cm = _carregar()
    excessos, _ = cm.avalia_rust(arvore, tetos=[])
    assert excessos == [("packages/core/src/session/escrita.rs", 401, 400)]


def test_modulo_do_session_no_teto_passa(tmp_path):
    arvore = _arvore(tmp_path, {"packages/core/src/session/boot.rs": 400})
    cm = _carregar()
    excessos, _ = cm.avalia_rust(arvore, tetos=[])
    assert excessos == []


def test_as_tres_reguas_de_rust_nao_vazam_para_o_lado(tmp_path):
    """session (400), actor (300) e generico (700) valem cada um no seu lugar."""
    arvore = _arvore(
        tmp_path,
        {
            "packages/core/src/session/x.rs": 401,  # coorte do session: reprova
            "packages/core/src/outro.rs": 401,  # generico: passa
            "packages/app/api/src/actor/x.rs": 301,  # coorte do actor: reprova
        },
    )
    cm = _carregar()
    excessos, _ = cm.avalia_rust(arvore, tetos=[])
    assert excessos == [
        ("packages/app/api/src/actor/x.rs", 301, 300),
        ("packages/core/src/session/x.rs", 401, 400),
    ]


def test_teto_declarado_vence_a_coorte_do_diretorio(tmp_path):
    """Decisao de UM arquivo vence a regra do diretorio (precedencia do `teto_rust`).

    Hoje nenhum teto declarado mora DENTRO de um diretorio de coorte (os handles
    ficam ao lado: `src/actor.rs`, `session.rs`) — o teste trava a precedencia
    para o dia em que um morar, e para que a ordem das checagens nao seja
    invertida numa refatoracao futura.
    """
    tetos = [("packages/core/src/session/escrita.rs", 450, "#262: IR/SnapTone saem depois")]
    arvore = _arvore(tmp_path, {"packages/core/src/session/escrita.rs": 420})
    cm = _carregar()
    excessos, ausentes = cm.avalia_rust(arvore, tetos=tetos)
    assert (excessos, ausentes) == ([], [])


def test_arquivo_de_teste_fica_fora_do_orcamento(tmp_path):
    """Teste cresce com a cobertura, nao com funcionalidade (#243: 961 linhas)."""
    arvore = _arvore(
        tmp_path,
        {
            "packages/core/src/tests.rs": 4000,
            "packages/core/src/tests/suporte.rs": 4000,
            "packages/app/api/src/actor/tests.rs": 4000,
        },
    )
    cm = _carregar()
    excessos, _ = cm.avalia_rust(arvore, tetos=[])
    assert excessos == []


def test_crate_novo_entra_no_orcamento_sem_mudar_o_gate(tmp_path):
    """O gate varre `packages/*/src` por glob: crate novo ja nasce cobrado."""
    arvore = _arvore(tmp_path, {"packages/novo/src/gordo.rs": 701})
    cm = _carregar()
    excessos, _ = cm.avalia_rust(arvore, tetos=[])
    assert excessos == [("packages/novo/src/gordo.rs", 701, 700)]


# ── teto declarado ──────────────────────────────────────────────────────────


def test_teto_declarado_vale_no_lugar_do_generico(tmp_path):
    tetos = [("packages/cli/src/main.rs", 1300, "congelado: corte na #257")]
    arvore = _arvore(tmp_path, {"packages/cli/src/main.rs": 1250})
    cm = _carregar()
    excessos, ausentes = cm.avalia_rust(arvore, tetos=tetos)
    assert (excessos, ausentes) == ([], [])


def test_teto_declarado_ainda_reprova_quem_cresce(tmp_path):
    tetos = [("packages/cli/src/main.rs", 1300, "congelado: corte na #257")]
    arvore = _arvore(tmp_path, {"packages/cli/src/main.rs": 1301})
    cm = _carregar()
    excessos, _ = cm.avalia_rust(arvore, tetos=tetos)
    assert excessos == [("packages/cli/src/main.rs", 1301, 1300)]


def test_teto_declarado_sem_arquivo_e_erro(tmp_path):
    """Apagar o arquivo e a forma mais barata de aprovar — o gate recusa."""
    tetos = [("packages/core/src/session.rs", 1395, "congelado: corte na #257")]
    cm = _carregar()
    excessos, ausentes = cm.avalia_rust(tmp_path, tetos=tetos)
    assert excessos == []
    assert ausentes == ["packages/core/src/session.rs"]


# ── excecao (--allow) ───────────────────────────────────────────────────────


def test_allow_sobe_o_teto_do_arquivo(tmp_path):
    arvore = _arvore(tmp_path, {"packages/core/src/gordo.rs": 820})
    cm = _carregar()
    excessos, _ = cm.avalia_rust(
        arvore, tetos=[], excecoes={"packages/core/src/gordo.rs": 850}
    )
    assert excessos == []


def test_allow_sem_a_chave_continua_reprovando(tmp_path):
    arvore = _arvore(tmp_path, {"packages/core/src/gordo.rs": 820})
    cm = _carregar()
    excessos, _ = cm.avalia_rust(
        arvore, tetos=[], excecoes={"packages/core/src/outro.rs": 850}
    )
    assert excessos == [("packages/core/src/gordo.rs", 820, 700)]


def test_allow_malformado_e_recusado():
    cm = _carregar()
    assert cm._parse_allow(["semdoispontos"]) is None
    assert cm._parse_allow(["packages/core/src/x.rs:abc"]) is None
    assert cm._parse_allow(["packages/core/src/x.rs:120"]) == {
        "packages/core/src/x.rs": 120
    }
