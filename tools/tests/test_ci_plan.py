"""Teste de regressão do `scope-ci` (#246, achado F-13 da auditoria de 14/10).

O gate `simulate_release` executa a cadeia rc→promote DE VERDADE num sandbox git
(~2 min) e rodava em TODO PR. Ele passou a rodar por ESCOPO: `schedule`, tag e
dispatch (`CHANGED=ALL`) e PR que toca a maquinaria de release. A regra mora no
`ci_plan.py` — nada de `if:` escrito à mão no YAML (contrato 5 do
`validate_workflows.py`) — e é ELA que este teste trava: um caminho de release
entrando ou saindo da lista muda o custo do CI e a cobertura da cadeia em
silêncio, sem nenhum arquivo de workflow parecer diferente.
"""
from __future__ import annotations

import importlib.util
import os
import pathlib
import subprocess
import sys

_SCRIPTS = pathlib.Path(__file__).resolve().parents[2] / ".github" / "scripts"
RAIZ = pathlib.Path(__file__).resolve().parents[2]


def _carregar_ci_plan():
    """Importa `.github/scripts/ci_plan.py` como módulo (padrão dos specs)."""
    spec = importlib.util.spec_from_file_location("ci_plan", _SCRIPTS / "ci_plan.py")
    mod = importlib.util.module_from_spec(spec)
    sys.modules["ci_plan"] = mod
    spec.loader.exec_module(mod)
    return mod


# Os arquivos que GOVERNAM a cadeia de release: o pipeline, o bump de versão e
# os scripts que a cadeia executa. Tocar em qualquer um deles pode quebrar a
# release sem tocar em nada que a suíte veja (#235 nasceu assim).
CAMINHOS_DA_CADEIA = (
    ".github/workflows/ci.yml",
    ".github/workflows/release-rc.yml",
    ".github/workflows/release-promote.yml",
    ".github/workflows/release-play.yml",
    ".github/workflows/dist-cli.yml",
    ".github/actions/version-bump/action.yml",
    ".github/scripts/simulate_release.py",
    ".github/scripts/ci_plan.py",
    ".github/scripts/sync_version.py",
)

# O resto do repo: aqui o gate de 2 min é custo, não prova.
FORA_DA_CADEIA = (
    "README.md",
    "docs/tecnico/TESTING.md",
    ".github/ISSUE_TEMPLATE/bug.yml",
    "packages/core/src/lib.rs",
    "packages/app/ui/src/App.tsx",
    "tools/tests/test_ci_plan.py",
)


def test_scope_ci_liga_nos_caminhos_que_governam_a_release():
    plan = _carregar_ci_plan()
    for caminho in CAMINHOS_DA_CADEIA:
        assert plan.scopes(caminho)["ci"], caminho


def test_scope_ci_nao_liga_fora_da_cadeia():
    plan = _carregar_ci_plan()
    for caminho in FORA_DA_CADEIA:
        assert not plan.scopes(caminho)["ci"], caminho


def test_changed_all_liga_o_escopo():
    """Tag/schedule/dispatch chegam com `CHANGED=ALL` (#228) e provam a cadeia."""
    plan = _carregar_ci_plan()
    assert plan.scopes("ALL")["ci"]


def test_diff_multi_linha_liga_o_escopo_uma_vez():
    """`CHANGED` é multi-linha: um PR de CI no meio de outros arquivos conta."""
    plan = _carregar_ci_plan()
    diff = "docs/README.md\n.github/workflows/release-rc.yml\npackages/core/src/lib.rs"
    assert plan.scopes(diff)["ci"]


def test_plano_publica_scope_ci_no_contrato_de_saida():
    """O `plan` do ci.yml lê a saída como `key=value` em `$GITHUB_OUTPUT`.

    Um escopo que existe no código mas não sai no contrato deixa o `if` do passo
    comparando com vazio — o gate fica `skipped` em silêncio, que é a falha mais
    cara das duas (nunca roda e ninguém vê).
    """
    for caminho, esperado in (
        (".github/workflows/ci.yml", True),
        ("docs/tecnico/TESTING.md", False),
    ):
        saida = subprocess.run(
            [sys.executable, str(_SCRIPTS / "ci_plan.py")],
            cwd=RAIZ,
            capture_output=True,
            text=True,
            env={**os.environ, "CHANGED": caminho, "EVENT": "pull_request", "BASE_REF": "develop"},
        )
        assert saida.returncode == 0, saida.stderr
        esperado_txt = f"scope-ci={'true' if esperado else 'false'}"
        assert esperado_txt in saida.stdout.splitlines(), (caminho, saida.stdout)
