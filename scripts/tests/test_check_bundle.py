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

_SCRIPTS = pathlib.Path(__file__).resolve().parents[1]


def _carregar_check_bundle():
    """Importa `scripts/check_bundle.py` como módulo (padrão do specs)."""
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
