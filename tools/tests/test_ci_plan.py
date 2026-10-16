"""Teste de regressão dos ESCOPOS do plano de CI (#246 `scope-ci`, #261 `scope-rust`).

O gate `simulate_release` executa a cadeia rc→promote DE VERDADE num sandbox git
(~2 min) e rodava em TODO PR. Ele passou a rodar por ESCOPO: `schedule`, tag e
dispatch (`CHANGED=ALL`) e PR que toca a maquinaria de release. A regra mora no
`ci_plan.py` — nada de `if:` escrito à mão no YAML (contrato 5 do
`validate_workflows.py`) — e é ELA que este teste trava: um caminho de release
entrando ou saindo da lista muda o custo do CI e a cobertura da cadeia em
silêncio, sem nenhum arquivo de workflow parecer diferente.

O `scope-rust` (#261) entrou aqui depois de a CI ficar VERDE sem os jobs de Rust:
a lista de crates sai dos `members` do manifesto do workspace, o caminho do
manifesto estava um `..` curto e o `except OSError` engolia a falha. Nenhum teste
de produto acusa isso — o que acusa é o `scope-rust` publicado no contrato.
"""
from __future__ import annotations

import importlib.util
import os
import pathlib
import subprocess
import sys

import pytest

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


# ── escopo de RUST (#261): os crates saem do WORKSPACE ─────────────────────
#
# O bug que este bloco trava, medido no PR #260: `crates_rust()` lia
# `.github/Cargo.toml` (um `..` de menos no caminho), o `except OSError` engolia
# a falha e a lista ficava SÓ com os `extra`. PR que mexia apenas em
# `packages/core/`, `packages/cli/` ou `packages/library/` rodava a CI com
# `scope-rust=false` — lint, compile e teste de Rust `skipping` — e ficava
# verde: o pior tipo de bug de CI, o que não dá erro, só não roda.

CRATES_DO_WORKSPACE = ("packages/core", "packages/cli", "packages/library")

# Os `extra` NÃO são membros do workspace e entram de propósito: o shell do
# Tauri mora FORA dele (`exclude` + ADR-7) e `app/ui` acompanha o build do shell
# (o `tauri.conf.json` e o front mudam junto). Manter o `scope-rust` disparando
# onde já disparava é decisão registrada aqui, não acidente.
EXTRAS = ("packages/app/api", "packages/app/ui")

# Caminhos Rust: o `.rs` de cada crate do workspace e os três arquivos que
# governam a toolchain inteira. O `manifest.json` de `packages/core/data/` entra
# porque dado DENTRO do crate é dado EMBUTIDO no binário (`include_str!` em
# `core/src/model.rs`): mudar o dado sem rodar o job de Rust é a mesma classe de
# falha da #261 — mudança que a CI não vê.
CAMINHOS_DE_RUST = (
    "packages/core/src/session.rs",
    "packages/cli/src/main.rs",
    "packages/library/src/diff.rs",
    "packages/core/data/fixtures/manifest.json",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
)

# O resto do repo: mexer só nisso não pode subir a matriz de Rust.
FORA_DO_RUST = (
    "README.md",
    "docs/tecnico/TESTING.md",
    "docs/planejamento/ROADMAP.md",
    ".github/workflows/ci.yml",
    "tools/tests/test_ci_plan.py",
)


def test_manifesto_do_workspace_e_o_da_raiz():
    """O caminho do manifesto é o ponto exato do bug (#261): a RAIZ do repo.

    `__file__` mora em `.github/scripts/`, então o manifesto está TRÊS pais
    acima. Ler `.github/Cargo.toml` não dá erro de git, de YAML nem de sintaxe —
    dá uma lista de crates vazia, que é o que deixou a CI verde sem Rust.
    """
    plan = _carregar_ci_plan()
    assert plan.MANIFESTO_WORKSPACE == RAIZ / "Cargo.toml"
    assert plan.MANIFESTO_WORKSPACE.is_file(), plan.MANIFESTO_WORKSPACE


def test_crates_rust_le_os_membros_do_workspace():
    """Sem os membros a lista é só os `extra` — e o escopo de Rust não existe."""
    plan = _carregar_ci_plan()
    crates = plan.crates_rust()
    for crate in CRATES_DO_WORKSPACE:
        assert crate in crates, (crate, crates)


def test_extras_continuam_na_lista():
    """A correção não podia encolher a lista: `app/api` e `app/ui` ficam."""
    plan = _carregar_ci_plan()
    crates = plan.crates_rust()
    for extra in EXTRAS:
        assert extra in crates, (extra, crates)


def test_scope_rust_liga_nos_crates_do_workspace():
    plan = _carregar_ci_plan()
    for caminho in CAMINHOS_DE_RUST:
        assert plan.scopes(caminho)["rust"], caminho


def test_scope_rust_nao_liga_fora_do_rust():
    plan = _carregar_ci_plan()
    for caminho in FORA_DO_RUST:
        assert not plan.scopes(caminho)["rust"], caminho


def test_scope_rust_liga_no_shell_e_no_front():
    """O shell do Tauri e o front mexem no build do shell: seguem no escopo."""
    plan = _carregar_ci_plan()
    for caminho in ("packages/app/api/src/lib.rs", "packages/app/ui/src/App.tsx"):
        assert plan.scopes(caminho)["rust"], caminho


def test_manifesto_ausente_falha_alto(tmp_path):
    """O silêncio ERA o defeito (#261): sem manifesto, o plano GRITA."""
    plan = _carregar_ci_plan()
    with pytest.raises(RuntimeError, match="manifesto"):
        plan.membros_do_workspace(tmp_path / "Cargo.toml")


def test_manifesto_sem_members_falha_alto(tmp_path):
    """Sem `members` não há o que ler — lista vazia é jobs ausentes, não zero."""
    manifesto = tmp_path / "Cargo.toml"
    manifesto.write_text('[workspace]\nresolver = "2"\n', encoding="utf-8")
    plan = _carregar_ci_plan()
    with pytest.raises(RuntimeError, match="members"):
        plan.membros_do_workspace(manifesto)


def test_plano_publica_scope_rust_no_contrato_de_saida():
    """O `plan` do ci.yml lê `key=value`: escopo que não sai no contrato deixa o
    `if` do job comparando com vazio — e o job fica `skipping` em silêncio."""
    for caminho, esperado in (
        ("packages/core/src/session.rs", True),
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
        esperado_txt = f"scope-rust={'true' if esperado else 'false'}"
        assert esperado_txt in saida.stdout.splitlines(), (caminho, saida.stdout)
