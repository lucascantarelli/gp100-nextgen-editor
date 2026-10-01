#!/usr/bin/env python3
"""
Validação sintática dos workflows/templates do GitHub (gate do pipeline).

O teste de INTEGRAÇÃO real do close-issues acontece quando um PR mergeia em
develop (o workflow é a própria prova — marcada no summary da run). Validá-lo
com `act` exigiria Docker dentro do gate; aqui travamos o CONTRATO:

  1. TODO workflow e template de issue parseia como YAML;
  2. close-issues.yml: gatilho (PR closed em develop), permissions e guards;
  3. pipeline.yml: gatilhos do GitFlow (push develop+main, PR → develop,
     tags v* para release);
  4. release.yml: workflow_dispatch com input `action` (rc|promote).

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

    # ── close-issues: o contrato do fechamento automático ──
    ci = docs.get(".github/workflows/close-issues.yml") or {}
    pr = triggers(ci).get("pull_request") or {}
    if "closed" not in (pr.get("types") or []):
        FAILURES.append("close-issues: on.pull_request.types precisa incluir 'closed'")
    if "develop" not in (pr.get("branches") or []):
        FAILURES.append("close-issues: on.pull_request.branches precisa incluir 'develop'")
    if (ci.get("permissions") or {}).get("issues") != "write":
        FAILURES.append("close-issues: permissions.issues precisa ser 'write'")
    jobs = ci.get("jobs") or {}
    close_job = jobs.get("close-linked") or {}
    if not close_job:
        FAILURES.append("close-issues: job close-linked ausente")
    else:
        job_if = str(close_job.get("if") or "")
        if "merged == true" not in job_if:
            FAILURES.append("close-issues: guard 'merged == true' ausente no job")
        run_blocks = " ".join(
            str(step.get("run") or "") for step in close_job.get("steps") or []
        )
        for word in ("Closes", "GITHUB_STEP_SUMMARY"):
            if word not in run_blocks and word not in job_if:
                FAILURES.append(f"close-issues: passo de extração/fechamento perdeu '{word}'")

    # ── pipeline: gatilhos do GitFlow ──
    pl = docs.get(".github/workflows/pipeline.yml") or {}
    pl_on = triggers(pl)
    push = pl_on.get("push") or {}
    if not {"develop", "main"} <= set(push.get("branches") or []):
        FAILURES.append("pipeline: push deve mirar develop+main (GitFlow)")
    if "develop" not in ((pl_on.get("pull_request") or {}).get("branches") or []):
        FAILURES.append("pipeline: pull_request deve mirar develop (integração)")
    if "v*" not in (push.get("tags") or []):
        FAILURES.append("pipeline: tags v* devem disparar o release")
    if "workflow_dispatch" not in pl_on:
        FAILURES.append("pipeline: workflow_dispatch (play) ausente")

    # ── release: cadeia rc→final sob workflow_dispatch ──
    rel = docs.get(".github/workflows/release.yml") or {}
    rel_in = (triggers(rel).get("workflow_dispatch") or {}).get("inputs") or {}
    if "action" not in rel_in:
        FAILURES.append("release: input 'action' ausente (rc|promote)")
    else:
        options = (rel_in.get("action") or {}).get("options") or []
        if not {"rc", "promote"} <= set(options):
            FAILURES.append("release: input action deve aceitar rc e promote")
    rel_jobs = rel.get("jobs") or {}
    for job in ("rc", "promote"):
        if job not in rel_jobs:
            FAILURES.append(f"release: job '{job}' ausente")

    # ── relatório ──
    broken = {f.split(":")[0] for f in FAILURES}
    for path in workflows + templates:
        print(("ERR" if path in broken else "OK "), path)
    if FAILURES:
        print("\nFALHAS:")
        for f in FAILURES:
            print(" -", f)
        return 1
    print(f"\n{len(workflows)} workflows + {len(templates)} templates: YAML válido e contratos de gatilho preservados.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
