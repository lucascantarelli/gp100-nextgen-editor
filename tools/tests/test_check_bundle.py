"""Teste de regressão do `check_bundle.check_dev_command` (#192).

O bug: `build.beforeDevCommand` vazio no `tauri.conf.json` deixava o `tauri dev`
(debug do VS Code e terminal) preso para sempre em "Waiting for your frontend
dev server" — o Tauri não sobe o vite, e nenhum job do CI roda `tauri dev`, então
o gate é a Única garantia. Mesma classe do check #71: um contrato que apodreceria
em silêncio.

O schema do Tauri v2 aceita `beforeDevCommand` como string OU objeto
`{script, cwd}`; o gate precisa aceitar as duas formas vazias como falha.
"""
import importlib.util
import pathlib
import sys

_SCRIPTS = pathlib.Path(__file__).resolve().parents[2] / ".github" / "scripts"


def _carregar_check_bundle():
    """Importa `.github/.github/scripts/check_bundle.py` como módulo (padrão do specs)."""
    spec = importlib.util.spec_from_file_location(
        "check_bundle", _SCRIPTS / "check_bundle.py"
    )
    mod = importlib.util.module_from_spec(spec)
    sys.modules["check_bundle"] = mod
    spec.loader.exec_module(mod)
    return mod


def _rodar_check(build: dict) -> list[str]:
    """Roda `check_dev_command` num conf sintético e devolve as falhas."""
    cb = _carregar_check_bundle()
    cb.FAILURES.clear()
    cb.check_dev_command({"build": build})
    return list(cb.FAILURES)


def test_before_dev_command_string_vazia_reprova():
    assert _rodar_check({"beforeDevCommand": ""})


def test_before_dev_command_ausente_reprova():
    assert _rodar_check({})


def test_before_dev_command_objeto_sem_script_reprova():
    assert _rodar_check({"beforeDevCommand": {"cwd": "../ui"}})


def test_before_dev_command_objeto_com_script_vazio_reprova():
    assert _rodar_check({"beforeDevCommand": {"script": "  ", "cwd": "../ui"}})


def test_before_dev_command_string_preenchida_passa():
    assert not _rodar_check({"beforeDevCommand": "pnpm dev"})


def test_before_dev_command_objeto_com_script_passa():
    bdc = {"script": "pnpm dev", "cwd": "../ui"}
    assert not _rodar_check({"beforeDevCommand": bdc})


# ── #230: o tarball de fonte tem de embutir TODA dependencia de CAMINHO ──────
#
# O bug: `make_sdist.py` levava `packages/core` (fechado em #72) e nao
# `packages/library` — a SEGUNDA dependencia de caminho do crate do Tauri
# (`gp100-library = { path = "../../library" }`). O tarball publicado morreu no
# primeiro run real da distribuicao (run 38553264817): `cargo metadata --locked`
# dentro dele → "failed to load manifest for dependency gp100-library". O gate
# so lia a dependencia que ja tinha doido; agora a exigencia e DERIVADA do
# manifesto, entao uma terceira dependencia de caminho nao passa em silencio.

RAIZ = pathlib.Path(__file__).resolve().parents[2]

SD_SO_COM_CORE = 'INCLUDE_DIRETORIOS = [\n    "packages/core",\n]\n'
SD_COMPLETO = 'INCLUDE_DIRETORIOS = [\n    "packages/core",\n    "packages/library",\n]\n'


def _rodar_sdist_deps(sd_text: str, deps: list[str]) -> list[str]:
    cb = _carregar_check_bundle()
    cb.FAILURES.clear()
    cb.check_sdist_deps(sd_text, deps)
    return list(cb.FAILURES)


def test_sdist_sem_dependencia_de_caminho_reprova():
    assert _rodar_sdist_deps(SD_SO_COM_CORE, ["packages/core", "packages/library"])


def test_sdist_com_todas_as_dependencias_passa():
    assert not _rodar_sdist_deps(SD_COMPLETO, ["packages/core", "packages/library"])


def test_sdist_sem_dependencia_declarada_passa():
    assert not _rodar_sdist_deps("INCLUDE_DIRETORIOS = []\n", [])


def test_dependencias_de_caminho_saem_do_manifesto_do_shell():
    """A exigencia vem do manifesto: core e library sao as duas de hoje."""
    deps = _carregar_check_bundle().deps_caminho_do_shell()
    assert "packages/core" in deps
    assert "packages/library" in deps


def test_make_sdist_embute_as_dependencias_de_caminho_do_shell():
    """Regressão #230 — o tarball publicado tem de resolver o manifesto."""
    cb = _carregar_check_bundle()
    sd = (RAIZ / "packaging" / "make_sdist.py").read_text(encoding="utf-8")
    cb.FAILURES.clear()
    cb.check_sdist_deps(sd, cb.deps_caminho_do_shell())
    assert not cb.FAILURES, cb.FAILURES


def test_o_proprio_gate_de_bundle_roda_limpo_no_repo(monkeypatch):
    """O contrato vale para a arvore REAL: `check_arch` com o conf do repo.

    `monkeypatch.chdir` (e nao `os.chdir`) porque `check_arch` le caminhos
    relativos e o cwd do processo tem de voltar ao lugar no fim do teste.
    """
    import json

    cb = _carregar_check_bundle()
    monkeypatch.chdir(RAIZ)
    conf = json.loads(
        (RAIZ / "packages" / "app" / "api" / "tauri.conf.json").read_text(encoding="utf-8")
    )
    cb.FAILURES.clear()
    cb.check_arch(conf, RAIZ)
    assert not cb.FAILURES, cb.FAILURES
