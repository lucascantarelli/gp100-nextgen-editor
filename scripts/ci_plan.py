#!/usr/bin/env python3
"""
Plano do CI: classifica a branch e decide quais estágios rodam.

POR QUE UM SCRIPT E NÃO `if:` ESPALHADO NO YAML: a regra "build roda em toda
branch, dist só na tag, testes não rodam em release/**" apareceia em cinco
`if:` diferentes — e quando uma delas divergia, ninguém via. Aqui a regra é
uma tabela, roda local em segundos e é testável sem GitHub.

Também monta as MATRIZES por projeto afetado (mudança em `packages/core/`
não sobe o front; mudança só em docs não sobe nada). O filtro de caminhos é
o que mantém um PR de documentação barato.

Contrato de saída (lido pelo `plan` do .github/workflows/ci.yml):
    branch-class     develop | main | dev | release | hotfix | tag | schedule
    scope-rust       gp100-core/cli/ui mudaram?
    scope-front      packages/app/ui mudou?
    scope-spec       analysis|docs|pyproject mudaram?
    stage-test       estágio 3 roda?
    stage-coverage   estágio 4 roda?
    stage-security   estágio 5 roda?
    stage-dist       estágio 7 (artefatos de distribuição) roda?
    stage-release    versionamento está autorizado (dispatch com action)?
    rust-matrix      {include:[{os,project,workdir}]}
    front-matrix     {include:[{os}]}

Uso: python3 scripts/ci_plan.py   (env: REF, BASE_REF, EVENT, ACTION, CHANGED)
"""
from __future__ import annotations

import json
import os
import re

# O macOS fica em `macos-15-intel` (x86_64) de propósito: todo runner macos-*
# arm64 carrega a anotação "capacity constraints … longer queue times" do
# GitHub — ruído de infra que não controlamos — e o label Intel casa com a
# arquitetura de Windows/Linux (suporte até ~08/2027).
OSS = ["windows-latest", "macos-15-intel", "ubuntu-24.04"]

DEV_PREFIXES = (
    "feature/",
    "bugfix/",
    "fix/",
    "chore/",
    "docs/",
    "refactor/",
    "hotfix-release/",
)


def classify(ref: str, base_ref: str, event: str) -> str:
    """A branch que manda na regra é a BASE no PR (refs/pull/N/merge não diz
    nada sobre o tipo de branch) e a própria ref nos demais eventos."""
    name = (base_ref if event == "pull_request" else ref) or ""
    if name.startswith("refs/tags/") or event == "release":
        return "tag"
    if event == "schedule":
        return "schedule"
    if event == "workflow_dispatch":
        return "dispatch"
    if name == "refs/heads/main":
        return "main"
    if name == "refs/heads/develop":
        return "develop"
    if name.startswith("refs/heads/release/"):
        return "release"
    if name.startswith("refs/heads/hotfix/"):
        return "hotfix"
    if name.startswith(tuple(f"refs/heads/{p}" for p in DEV_PREFIXES)):
        return "dev"
    return "dev"


def scopes(changed: str) -> dict[str, bool]:
    is_all = changed == "ALL"
    # re.M é obrigatório: CHANGED é multi-linha; sem ele o ^ só casa no
    # início da string e NENHUM projeto "muda" (provado na run 36764807538:
    # push de packages/ inteiro → tudo skipped).
    def has(pat: str) -> bool:
        return is_all or re.search(pat, changed, re.I | re.M) is not None

    return {
        "rust": has(r"^(Cargo\.toml$|Cargo\.lock$|rust-toolchain\.toml$|packages/core/|packages/cli/|packages/app/api/|packages/app/ui/)"),
        "front": has(r"^packages/app/ui/"),
        "spec": has(r"^(analysis/|docs/|pyproject\.toml$|uv\.lock$)"),
    }


def matrices(sc: dict[str, bool]) -> tuple[str, str]:
    """Matrizes por projeto afetado.

    `ui-rust` fica em Windows (hosted) + Linux (container ci-linux, no job
    `3 test · e2e webview`); o macOS saiu porque compilar o shell Tauri lá
    (~96s) não pegava classe de bug própria — o vitest 3-OS já cobre a UI.
    """
    rust: list[dict[str, str]] = []
    front: list[dict[str, str]] = []
    if sc["rust"]:
        # core e cli compartilham o gatilho (moram no mesmo workspace da raiz),
        # então entram juntos — o filtro por arquivo deles é o mesmo.
        for os_name in OSS:
            rust.append({"os": os_name, "project": "core", "workdir": "."})
            rust.append({"os": os_name, "project": "cli", "workdir": "."})
        rust.append({"os": "windows-latest", "project": "ui-rust", "workdir": "packages/app/api"})
    if sc["front"]:
        front = [{"os": o} for o in OSS]
    return _compact({"include": rust}), _compact({"include": front})


def _compact(obj: object) -> str:
    return json.dumps(obj, separators=(",", ":"))


def main() -> int:
    ref = os.environ.get("REF", "")
    base_ref = os.environ.get("BASE_REF", "")
    event = os.environ.get("EVENT", "")
    action = os.environ.get("ACTION", "")
    changed = os.environ.get("CHANGED", "ALL")

    klass = classify(ref, base_ref, event)
    sc = scopes(changed)

    # ── a regra branch → estágios ──────────────────────────────────────────
    # release/** é um FREEZE da develop (que já foi testada no merge): lint +
    # build verificam que o freeze não quebrou nada, e o gate de verdade da
    # release é o push da tag v*, que roda a suíte inteira. hotfix/** é o
    # contrário — nasce da main, é correção urgente e tem teste.
    run_tests = klass in ("dev", "develop", "main", "hotfix", "tag", "schedule", "dispatch")
    run_coverage = klass in ("dev", "develop", "main", "hotfix", "tag")
    # security: sempre, menos no schedule em push-noturno (que é o único
    # lugar onde ele roda). dispatch manual roda para re-auditar sob demanda.
    run_security = klass != "release"
    # DIST é distribuição: só quando existe versão para publicar.
    run_dist = klass == "tag"
    # Versionamento NUNCA em push ou PR — só na mão do dono.
    run_release = klass == "dispatch" and action in ("release", "promote", "play")

    rust_matrix, front_matrix = matrices(sc)

    out = {
        "branch-class": klass,
        "scope-rust": "true" if sc["rust"] else "false",
        "scope-front": "true" if sc["front"] else "false",
        "scope-spec": "true" if sc["spec"] else "false",
        "stage-test": "true" if run_tests else "false",
        "stage-coverage": "true" if run_coverage else "false",
        "stage-security": "true" if run_security else "false",
        "stage-dist": "true" if run_dist else "false",
        "stage-release": "true" if run_release else "false",
        "rust-matrix": rust_matrix,
        "front-matrix": front_matrix,
    }
    for key, value in out.items():
        print(f"{key}={value}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())