#!/usr/bin/env python3
"""
Plano do CI: classifica a branch e decide quais tipos de job rodam.

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
    stage-test       Testes rodam?
    stage-coverage   Cobertura roda?
    stage-security   Segurança roda?
    stage-dist       Distribuição (artefatos) roda?
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


# Os caminhos que sao "código Rust" vêm do WORKSPACE, não de uma lista escrita
# à mão. A lista manual morava em `ci_plan.py` e em `validate_workflows.py`
# (o debt que a #82 §4 apontou): um crate novo entrava no `Cargo.toml` e o job
# de Rust simplesmente não disparava para ele — o pior tipo de bug de CI, o
# que não dá erro, só não roda. Aqui a fonte é o próprio workspace.
#
# `packages/app/ui` NÃO é crate Rust, mas o `tauri.conf.json` e o front fazem
# parte do build do shell e mudam juntos; ele entra na lista de Paths para o
# escopo `rust` continuar disparando onde já disparava (compatibilidade).
_CRATES_CACHE: list[str] | None = None


def crates_rust() -> list[str]:
    """Diretórios com código Rust, lidos dos membros do workspace."""
    global _CRATES_CACHE
    if _CRATES_CACHE is None:
        manifesto = os.path.join(os.path.dirname(__file__), "..", "Cargo.toml")
        membros: list[str] = []
        try:
            with open(manifesto, encoding="utf-8") as fh:
                texto = fh.read()
            bloco = re.search(r"members\s*=\s*\[(.*?)\]", texto, re.S)
            if bloco:
                membros = re.findall(r'"([^"]+)"', bloco.group(1))
        except OSError:
            membros = []
        # inclui o crate excluido da raiz (o shell do Tauri) e o front
        extra = ["packages/app/api", "packages/app/ui"]
        for e in extra:
            if e not in membros:
                membros.append(e)
        _CRATES_CACHE = membros
    return _CRATES_CACHE


def scope_rust_regex() -> str:
    """Expressão dos caminhos Rust, derivada do workspace (nunca manual)."""
    return "|".join(f"{re.escape(m)}/" for m in crates_rust())


CRATES_RUST = scope_rust_regex()


def scopes(changed: str) -> dict[str, bool]:
    is_all = changed == "ALL"
    # re.M é obrigatório: CHANGED é multi-linha; sem ele o ^ só casa no
    # início da string e NENHUM projeto "muda" (provado na run 36764807538:
    # push de packages/ inteiro → tudo skipped).
    def has(pat: str) -> bool:
        return is_all or re.search(pat, changed, re.I | re.M) is not None

    return {
        "rust": has(r"^(Cargo\.toml$|Cargo\.lock$|rust-toolchain\.toml$|%s)" % CRATES_RUST),
        "front": has(r"^packages/app/ui/"),
        # `scripts/` entrou no escopo da especificacao em #21. A razao e um
        # gate que nao disparava: `analysis/tests/test_h1_compare.py` testa
        # `scripts/h1_compare.py`, e sem esta linha mexer SO no script
        # deixaria o job de pytest como `skipped` — os testes ficariam
        # verdes sem nunca rodarem contra a mudanca. Teste que nao
        # re-executa nao e teste.
        "spec": has(r"^(analysis/|docs/|scripts/|pyproject\.toml$|uv\.lock$)"),
    }


def matrices(sc: dict[str, bool]) -> tuple[str, str]:
    """Matrizes por projeto afetado.

    `ui-rust` (o crate do Tauri) roda em Windows e macOS nesta matriz, e no
    Linux DENTRO do container `ci-linux` (job `Testes · e2e webview`). A
    entrada do macOS voltou em 05/10 e vale a pena registrar O PORQUE, porque
    a decisão anterior era o oposto e igualmente bem pensada:

      Saiu porque: compilar o shell do Tauri no macOS (~96s) não pegava
      classe de bug própria — o vitest 3-OS já cobria a UI.

      Voltou porque: a premissa "não tem nada específico de SO" deixou de
      valer. `--features real-device` liga o `midir`, e no macOS ele fala
      **CoreMIDI** — que só existe ali. Compilar no Windows prova o WinMM e
      no container Linux prova o ALSA; nenhum dos dois diz se o
      `RealDevice`/`impl DeviceBackend for RealDevice`/`abrir_backend`
      fecham no CoreMIDI. Sem esta entrada, o caminho do aparelho real do
      APP em macOS é código que só a máquina de quem tem a pedaleira
      compila — e era esse o ÚNICO motivo técnico que sobrava para o
      `gp100-cli` continuar existindo (`docs/REAL_DEVICE_GAP.md` §6, passo
      6b).

    **E POR QUE O LINUX NÃO ESTÁ NESTA MATRIX.** O `ubuntu-24.04` hosted não
    tem as libs de sistema do Tauri (WebKitGTK + ALSA) — foi justamente por
    isso que o projeto tem a imagem `ci-linux`. Uma entrada `ui-rust` em
    `ubuntu-24.04` aqui não compilaria: ela passaria a exigir `apt-get` de
    GTK em um job que hoje não instala nada. O Linux é coberto pelo
    container, e é lá que o passo `real-device` roda (além do ALSA, prova o
    ALSA + WebKitGTK juntos, combinação que o runner do macOS não reproduz).

    Custo é o preço, não um argumento contra: o cache `ws-api` é
    compartilhado entre jobs (o prefixo automático do rust-cache segue
    separando por triple), então o custo é de um shell do Tauri a mais por
    job, uma vez, e não por push.
    """
    rust: list[dict[str, str]] = []
    front: list[dict[str, str]] = []
    if sc["rust"]:
        # core e cli compartilham o gatilho (moram no mesmo workspace da raiz),
        # então entram juntos — o filtro por arquivo deles é o mesmo.
        for os_name in OSS:
            rust.append({"os": os_name, "project": "core", "workdir": "."})
            rust.append({"os": os_name, "project": "cli", "workdir": "."})
        # o crate do Tauri: WinMM e CoreMIDI aqui, ALSA no container. Nao
        # acrescentar `ubuntu-24.04` — ver a razao no docstring acima.
        for os_name in ("windows-latest", "macos-15-intel"):
            rust.append({"os": os_name, "project": "ui-rust", "workdir": "packages/app/api"})
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

    # ── a regra branch → tipos de job ──────────────────────────────────────
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
