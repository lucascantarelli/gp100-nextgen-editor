#!/usr/bin/env python3
"""
Resumo visual da run — o estágio de MÉTRICAS.

POR QUE UM ESTÁGIO PRÓPRIO (e com `if: always()`): o `$GITHUB_STEP_SUMMARY` é
onde se olha primeiro quando algo quebrou. Se o resumo morresse junto com o
job que falhou, ele só serviria para anunciar que o job falhou.

O que ele mostra:
  1. veredito no topo (verde/vermelho) e os contadores de stages;
  2. a tabela de ESTÁGIOS com os jobs e o tempo de cada um;
  3. a cobertura agregada, lida dos artefatos `cov-*` que o estágio 4 publica;
  4. o que foi DEIXADO DE FORA nesta branch (a regra branch → estágios),
     porque "por que meu job não rodou?" é a dúvida que mais custa tempo.

Os tempo vêm da API (`actions/runs/<id>/jobs`), não de um cronômetro próprio:
a API já tem `started_at`/`completed_at` de todos os jobs, inclusive os que
foram pulados — que aparecem como 0s, e é informação útil.

Uso: python3 scripts/ci_report.py
     env: GH_TOKEN, GH_REPO, RUN_ID, BRANCH_CLASS, GITHUB_STEP_SUMMARY
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from datetime import datetime

try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
except (AttributeError, OSError):
    pass

# Ordem dos estágios: o número é o prefixo do `name:` do job.
STAGE_ORDER = [
    ("0", "plan · contexto"),
    ("1", "lint"),
    ("2", "build (verificação)"),
    ("3", "test"),
    ("4", "coverage"),
    ("5", "security"),
    ("6", "metrics"),
    ("7", "release / dist"),
    ("8", "close · issues"),
]

ICON = {
    "success": "✅",
    "failure": "❌",
    "cancelled": "🚫",
    "skipped": "⏭️",
    "neutral": "⚪",
    "timed_out": "⌛",
}

# Como ler um coverage-summary.json (formato istanbul): {total: {lines: {pct}}}
COVERAGE_METRICS = [("lines", "linhas"), ("statements", "declarações"), ("functions", "funções"), ("branches", "branches")]


def gh(path: str) -> list[dict]:
    proc = subprocess.run(
        ["gh", "api", path, "--paginate", "--slurp"],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
    )
    if proc.returncode != 0:
        print(f"api indisponível ({path}): {proc.stderr.strip()[:120]}")
        return []
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError:
        return []


def fetch_jobs(repo: str, run_id: str) -> list[dict]:
    """`/actions/runs/<id>/jobs` devolve um OBJETO (`{total_count, jobs:[…]}`),
    e `gh --slurp` embrulha tudo num array de páginas. Normalizar os dois
    formatos aqui — o bug anterior (iterar sobre o dict e chamar `.get` numa
    string) derrubou o estágio inteiro numa run."""
    payload = gh(f"repos/{repo}/actions/runs/{run_id}/jobs")
    jobs: list[dict] = []
    for page in payload:
        if isinstance(page, dict):
            jobs.extend(j for j in page.get("jobs", []) if isinstance(j, dict))
        elif isinstance(page, list):
            jobs.extend(j for j in page if isinstance(j, dict))
    return jobs


def duration(job: dict) -> str:
    start, end = job.get("started_at"), job.get("completed_at")
    if not start or not end:
        return "—"
    fmt = "%Y-%m-%dT%H:%M:%SZ"
    try:
        delta = datetime.strptime(end, fmt) - datetime.strptime(start, fmt)
    except ValueError:
        return "—"
    secs = int(delta.total_seconds())
    return f"{secs // 60}m{secs % 60:02d}s" if secs >= 60 else f"{secs}s"


def coverage_rows(coverage_dir: str) -> list[str]:
    rows: list[str] = []
    for name in ("ui",):
        path = os.path.join(coverage_dir, f"{name}", "coverage-summary.json")
        if not os.path.exists(path):
            continue
        try:
            with open(path, encoding="utf-8") as fh:
                total = json.load(fh).get("total", {})
        except (OSError, json.JSONDecodeError):
            continue
        for key, label in COVERAGE_METRICS:
            value = total.get(key, {}).get("pct")
            if value is None:
                continue
            bar = "🟩" * int(round(value / 5)) + "⬜" * (20 - int(round(value / 5)))
            gate = " (gate 85)" if key in ("lines", "statements", "functions") and value < 85 else ""
            rows.append(f"| app/ui · {label} | {value:.2f}%{gate} | `{bar}` |")
    return rows


def build_summary(jobs: list[dict], klass: str) -> str:
    counts: dict[str, int] = {}
    for job in jobs:
        counts[job.get("conclusion") or job.get("status") or "unknown"] = (
            counts.get(job.get("conclusion") or job.get("status") or "unknown", 0) + 1
        )
    failed = counts.get("failure", 0)
    ok = counts.get("success", 0)
    skipped = counts.get("skipped", 0)
    cancelled = counts.get("cancelled", 0)

    verdict = "🔴 FALHOU" if failed else ("🟢 VERDE" if ok else "⚪ SEM DADOS")
    lines = [
        "## 🧾 Resumo da run",
        "",
        f"**{verdict}** · ✅ {ok} · ❌ {failed} · ⏭️ {skipped} · 🚫 {cancelled}",
        "",
        f"_Branch classificada como **{klass}** — o estágio que não rodou está justificado pela "
        "regra `branch → estágios` (issue #68)._",
        "",
    ]

    by_stage: dict[str, list[dict]] = {}
    for job in jobs:
        name = job.get("name", "?")
        prefix = name.split()[0] if name and name[0].isdigit() else "?"
        by_stage.setdefault(prefix, []).append(job)

    lines += ["### Estágios", "", "| Estágio | Job | Estado | Tempo |", "|---|---|---|---|"]
    for prefix, label in STAGE_ORDER:
        for job in sorted(by_stage.get(prefix, []), key=lambda j: j.get("name", "")):
            state = job.get("conclusion") or job.get("status") or "unknown"
            lines.append(
                f"| `{prefix} {label}` | {job.get('name', '?')} | "
                f"{ICON.get(state, '·')} {state} | {duration(job)} |"
            )
    # Jobs que caíram fora da numeração (ou com nome inesperado) não somem.
    for job in jobs:
        name = job.get("name", "?")
        if not (name and name[0].isdigit()):
            state = job.get("conclusion") or job.get("status") or "unknown"
            lines.append(f"| _sem estágio_ | {name} | {ICON.get(state, '·')} {state} | {duration(job)} |")

    rows = coverage_rows(os.environ.get("COVERAGE_DIR", ".cov"))
    lines += ["", "### Cobertura", ""]
    if rows:
        lines += ["| Produto · métrica | Valor | |", "|---|---|---|", *rows]
    else:
        lines.append("_Nenhum resumo de cobertura publicado nesta run "
                     "(o estágio 4 roda em `dev`, `develop`, `main`, `hotfix` e tag)._")

    slowest = sorted(
        (j for j in jobs if (j.get("conclusion") == "success")),
        key=lambda j: duration(j), reverse=True,
    )[:3]
    if slowest:
        lines += ["", "### 💸 Os 3 jobs mais caros (greenfield de custo)", ""]
        for job in slowest:
            lines.append(f"- `{job.get('name')}` — **{duration(job)}**")

    lines += ["", "---", "", "_Gerado por `scripts/ci_report.py` no estágio `6 metrics`._"]
    return "\n".join(lines)


def main() -> int:
    repo = os.environ.get("GH_REPO", "")
    run_id = os.environ.get("RUN_ID", "")
    klass = os.environ.get("BRANCH_CLASS", "desconhecida")
    summary_path = os.environ.get("GITHUB_STEP_SUMMARY", os.devnull)

    jobs = fetch_jobs(repo, run_id) if repo and run_id else []
    if not jobs:
        print("sem jobs na API (execução local?) — gerando só o esqueleto do relatório")
        jobs = []

    text = build_summary(jobs, klass)
    with open(summary_path, "a", encoding="utf-8") as fh:
        fh.write(text + "\n")
    print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())