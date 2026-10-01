#!/usr/bin/env python3
"""
Validação sintática + de CONTRATO dos workflows/templates do GitHub (gate do CI).

Os testes de INTEGRAÇÃO real acontecem onde o workflow é a própria prova:
o close-linked fecha issue no merge em develop; a cadeia rc->promote roda no
`scripts/simulate_release.py`. Aqui travamos os CONTRATOS que guardam o fluxo:

  1. TODO workflow/template parseia como YAML;
  2. ci.yml: gatilhos por FUNÇÃO de branch (push só em integração — feature/*
     NÃO dispara), close-linked integrado (types closed + develop + guards) e
     o job `validate` chamando o reusable _validate.yml;
  3. _validate.yml: workflow_call + o conjunto de jobs de validação;
  4. _publish.yml: workflow_call com inputs tag/prerelease + jobs cli/installer;
  5. release.yml: inputs action (version|rc|promote), jobs rc/promote e os
     publishes do MESMO run (lição: tag do bot não dispara workflow);
  6. security.yml: schedule + issue ACHADOS;
  7. os workflows legados (pipeline.yml, close-issues.yml) NÃO existem mais.

Uso: python3 scripts/validate_workflows.py  (exit 1 na primeira violação de
contrato; YAML inválido acumula falhas e lista todas).
"""
from __future__ import annotations

import glob
import sys

import yaml

FAILURES: list[str] = []


def load(path: str) -> dict | None:
    try:
        with open(path, encoding="utf-8") as fh:
            data = yaml.safe_load(fh)
        return data if isinstance(data, dict) else {}
    except Exception as exc:  # noqa: BLE001 — reportar qualquer YAML quebrado
        FAILURES.append(f"{path}: YAML inválido ({exc})")
        return None


def triggers(doc: dict) -> dict:
    """`on:` — PyYAML 1.1 parseia a chave `on` como booleano True."""
    return doc.get("on") or doc.get(True) or {}


def main() -> int:
    workflows = sorted(glob.glob(".github/workflows/*.yml"))
    templates = sorted(glob.glob(".github/ISSUE_TEMPLATE/*.yml"))
    # normaliza separadores (glob no Windows devolve backslash — e as chaves
    # de contrato abaixo usam /)
    workflows = [p.replace("\\", "/") for p in workflows]
    templates = [p.replace("\\", "/") for p in templates]
    docs: dict[str, dict] = {}
    for path in workflows + templates:
        doc = load(path)
        if doc is not None:
            docs[path] = doc

    # ── legados: a reestruturação de 01/10 substituiu estes dois ──
    for legacy in (".github/workflows/pipeline.yml", ".github/workflows/close-issues.yml"):
        if docs.get(legacy):
            FAILURES.append(f"{legacy}: workflow legado ainda existe (substituído por ci/_validate/security)")

    # ── ci.yml: gatilhos por função de branch + close-linked integrado ──
    ci_path = ".github/workflows/ci.yml"
    ci = docs.get(ci_path) or {}
    ci_on = triggers(ci)
    push = ci_on.get("push") or {}
    push_branches = push.get("branches") or []
    if not {"develop", "main"} <= set(push_branches):
        FAILURES.append("ci: push deve mirar develop+main (GitFlow)")
    for dev_prefix in ("feature/", "fix/", "chore/", "docs/", "refactor/", "bugfix/"):
        if any(str(b).startswith(dev_prefix) for b in push_branches):
            FAILURES.append(f"ci: push não deve disparar em {dev_prefix}* (validação só no PR)")
    if not {"release/**", "hotfix/**"} <= set(push_branches):
        FAILURES.append("ci: push deve mirar release/** e hotfix/** (branches de versão/correção)")
    pr = ci_on.get("pull_request") or {}
    if "closed" not in (pr.get("types") or []):
        FAILURES.append("ci: on.pull_request.types precisa incluir 'closed' (close-linked)")
    if "develop" not in (pr.get("branches") or []):
        FAILURES.append("ci: on.pull_request.branches precisa incluir 'develop'")
    if (ci.get("permissions") or {}).get("issues") != "write":
        FAILURES.append("ci: permissions.issues precisa ser 'write' (close-linked)")
    jobs = ci.get("jobs") or {}
    close_job = jobs.get("close-linked") or {}
    if not close_job:
        FAILURES.append("ci: job close-linked ausente")
    else:
        job_if = str(close_job.get("if") or "")
        if "merged == true" not in job_if:
            FAILURES.append("ci: guard 'merged == true' ausente no job close-linked")
        if "github-actions[bot]" not in job_if:
            FAILURES.append("ci: guard anti-loop do bot ausente no job close-linked")
        run_blocks = " ".join(
            str(step.get("run") or "") for step in close_job.get("steps") or []
        )
        for word in ("Closes", "GITHUB_STEP_SUMMARY", "achados-security"):
            if word not in run_blocks and word not in job_if:
                FAILURES.append(f"ci: passo do close-linked perdeu '{word}'")
    validate_job = jobs.get("validate") or {}
    if "uses" not in validate_job:
        FAILURES.append("ci: job validate precisa CHAMAR o reusable _validate.yml")
    elif "_validate.yml" not in str(validate_job["uses"]):
        FAILURES.append("ci: job validate deve referenciar ./.github/workflows/_validate.yml")
    if "closed" not in str(validate_job.get("if") or ""):
        FAILURES.append("ci: job validate deve guardar action != 'closed'")

    # ── _validate.yml: reusable com o conjunto de jobs de validação ──
    val = docs.get(".github/workflows/_validate.yml") or {}
    if "workflow_call" not in triggers(val):
        FAILURES.append("_validate: precisa de on.workflow_call (reusable)")
    val_jobs = set((val.get("jobs") or {}).keys())
    for job in ("plan", "gate", "spec", "rust", "front", "e2e", "e2e-visual", "e2e-tauri"):
        if job not in val_jobs:
            FAILURES.append(f"_validate: job '{job}' ausente")

    # ── _publish.yml: reusable de publicação ──
    pub = docs.get(".github/workflows/_publish.yml") or {}
    pub_call = (triggers(pub).get("workflow_call") or {})
    pub_inputs = pub_call.get("inputs") or {}
    for inp in ("tag", "prerelease"):
        if inp not in pub_inputs:
            FAILURES.append(f"_publish: input '{inp}' ausente no workflow_call")
    pub_jobs = set((pub.get("jobs") or {}).keys())
    for job in ("cli", "installer"):
        if job not in pub_jobs:
            FAILURES.append(f"_publish: job '{job}' ausente")

    # ── release.yml: GitFlow + publish no MESMO run ──
    rel = docs.get(".github/workflows/release.yml") or {}
    rel_on = triggers(rel)
    rel_in = (rel_on.get("workflow_dispatch") or {}).get("inputs") or {}
    if "action" not in rel_in:
        FAILURES.append("release: input 'action' ausente (version|rc|promote)")
    else:
        options = (rel_in.get("action") or {}).get("options") or []
        if not {"rc", "promote"} <= set(options):
            FAILURES.append("release: input action deve aceitar rc e promote")
    if "v*" not in ((rel_on.get("push") or {}).get("tags") or []):
        FAILURES.append("release: push de tags v* precisa publicar")
    rel_jobs = rel.get("jobs") or {}
    for job in ("rc", "promote", "validate", "publish-tag", "publish-rc", "publish-promote"):
        if job not in rel_jobs:
            FAILURES.append(f"release: job '{job}' ausente")
    # publish do MESMO run: rc/promote precisam de um job de publish que os siga
    for dep, pub_job in (("rc", "publish-rc"), ("promote", "publish-promote")):
        needs = rel_jobs.get(pub_job, {}).get("needs")
        needs_list = needs if isinstance(needs, list) else ([needs] if needs else [])
        if dep not in needs_list:
            FAILURES.append(f"release: {pub_job} precisa de needs: {dep} (tag do bot não dispara workflow)")

    # ── security.yml: varredura noturna + issue ACHADOS ──
    sec = docs.get(".github/workflows/security.yml") or {}
    if "schedule" not in triggers(sec):
        FAILURES.append("security: precisa de trigger schedule (varredura noturna)")
    sec_jobs = sec.get("jobs") or {}
    if "security" not in sec_jobs:
        FAILURES.append("security: job 'security' ausente")
    else:
        sec_runs = " ".join(
            str(step.get("run") or "") for step in (sec_jobs["security"].get("steps") or [])
        )
        for word in ("cargo audit", "pnpm audit", "uv run pytest", "gh issue create"):
            if word not in sec_runs:
                FAILURES.append(f"security: job perdeu '{word}'")

    # ── relatório ──
    PREFIX_TO_FILE = {
        "ci": ".github/workflows/ci.yml",
        "_validate": ".github/workflows/_validate.yml",
        "_publish": ".github/workflows/_publish.yml",
        "release": ".github/workflows/release.yml",
        "security": ".github/workflows/security.yml",
    }
    broken_files = {PREFIX_TO_FILE[f.split(":")[0]] for f in FAILURES if f.split(":")[0] in PREFIX_TO_FILE}
    for path in workflows + templates:
        print(("ERR" if path in broken_files else "OK "), path)
    if FAILURES:
        print("\nFALHAS:")
        for f in FAILURES:
            print(" -", f)
        return 1
    print(f"\n{len(workflows)} workflows + {len(templates)} templates: YAML válido e contratos de fluxo preservados.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
