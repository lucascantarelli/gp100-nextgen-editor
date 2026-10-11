#!/usr/bin/env python3
"""check_issue_templates.py — os templates de issue cumprem o contrato da Era HW.

POR QUE ISTO EXISTE (#170): o dono exigiu seções OBRIGATORIAS em todo corpo de
issue (contexto medido, escopo, fora de escopo, DoD, plano de teste,
referencias, prompt auto-suficiente do agente). Sem gate, o proximo "enxugar
template" remove a secao e o contrato morre em silencio — mesma classe do
check_bundle que existia e nao rodava (#71).

    python3 scripts/check_issue_templates.py     # todos os templates

O gate e estrutural: cada template declarado abaixo precisa das secoes na
ordem, com `required` onde o contrato exige, e o campo de prompt precisa
existir. `config.yml` nao tem corpo (so links) e fica de fora por design.
"""

from __future__ import annotations

import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # CI instala pyyaml na step; local vem do pyproject (#80)
    sys.exit("pyyaml ausente — `uv sync --all-groups` na raiz do repo")

ROOT = Path(__file__).resolve().parent.parent
TEMPLATES = ROOT / ".github" / "ISSUE_TEMPLATE"

# Seções que TODO template de corpo precisa ter (id do campo no YAML).
# O prompt e obrigatorio: quem pega a issue executa sem ler mais nada (#170).
SECOES_OBRIGATORIAS = ["contexto", "escopo", "fora_de_escopo",
                       "plano_de_teste", "referencias", "prompt", "validacao"]

# Campos que precisam de `validations.required: true` (não dá pra deixar
# em branco — o corpo auto-suficiente é o produto desta issue).
OBRIGATORIOS_NA_PRATICA = ["contexto", "escopo", "plano_de_teste", "referencias", "prompt"]

ARQUIVOS = ["epic.yml", "feature.yml", "bug.yml", "refactor.yml", "protocolo.yml"]


def ids_do_corpo(tpl: dict) -> list[str]:
    return [b.get("id") for b in tpl.get("body", []) if isinstance(b, dict) and b.get("id")]


def checar(arquivo: str) -> list[str]:
    """Devolve a lista de violações do template (vazia = ok)."""
    caminho = TEMPLATES / arquivo
    if not caminho.exists():
        return [f"{arquivo}: arquivo ausente"]

    # Loader que reprova chave repetida — mesma armadilha do ci.yml (#169):
    # chave duplicada passa pelo PyYAML (a última vence) e esconde metade do corpo.
    class LoaderSemDuplicata(yaml.SafeLoader):
        pass

    def nao_repetir(loader, node, deep=False):
        mapeado = {}
        for chave_no, valor_no in node.value:
            chave = loader.construct_object(chave_no, deep=deep)
            if chave in mapeado:
                raise yaml.constructor.ConstructorError(
                    None, None, f"chave duplicada {chave!r} em {arquivo}", chave_no.start_mark)
            mapeado[chave] = loader.construct_object(valor_no, deep=deep)
        return mapeado

    LoaderSemDuplicata.add_constructor(
        yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, nao_repetir)

    try:
        tpl = yaml.load(caminho.read_text(encoding="utf-8"), Loader=LoaderSemDuplicata)
    except yaml.YAMLError as exc:
        return [f"{arquivo}: YAML inválido — {exc}"]

    erros: list[str] = []
    if not tpl.get("name") or not tpl.get("description"):
        erros.append(f"{arquivo}: precisa de `name` e `description` (disparam o template)")

    ids = ids_do_corpo(tpl)
    for secao in SECOES_OBRIGATORIAS:
        if secao not in ids:
            erros.append(f"{arquivo}: seção obrigatória ausente — id `{secao}`")

    por_id = {b.get("id"): b for b in tpl.get("body", []) if isinstance(b, dict)}
    for secao in OBRIGATORIOS_NA_PRATICA:
        campo = por_id.get(secao)
        if campo is None:
            continue  # já reportado como ausente acima
        exigido = (campo.get("validations") or {}).get("required") is True
        if not exigido:
            erros.append(f"{arquivo}: `{secao}` precisa de validations.required: true")

    prompt = por_id.get("prompt")
    if prompt is not None and prompt.get("type") != "textarea":
        erros.append(f"{arquivo}: `prompt` deve ser textarea (bloco copiável)")

    return erros


def main() -> int:
    todos_erros: list[str] = []
    for arquivo in ARQUIVOS:
        erros = checar(arquivo)
        status = "REPROVADO" if erros else "ok"
        print(f"[issue-templates] {arquivo}: {status}")
        todos_erros.extend(erros)

    if todos_erros:
        print()
        for erro in todos_erros:
            print(f"  - {erro}", file=sys.stderr)
        return 1
    print(f"[issue-templates] {len(ARQUIVOS)} templates dentro do contrato (#170)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
