#!/usr/bin/env python3
"""
Resumo visual da run — o job `Relatório`.

POR QUE UM ESTÁGIO PRÓPRIO (e com `if: always()`): o `$GITHUB_STEP_SUMMARY` é
onde se olha primeiro quando algo quebrou. Se o resumo morresse junto com o
job que falhou, ele só serviria para anunciar que o job falhou.

O que ele mostra:
  1. veredito no topo (verde/vermelho) e os contadores de stages;
  2. a tabela de ESTÁGIOS com os jobs e o tempo de cada um;
  3. a cobertura agregada, lida dos artefatos `cov-*` que o job `Cobertura` publica;
  4. o que foi DEIXADO DE FORA nesta branch (a regra branch → tipos de job),
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

# Ordem de execução dos tipos de job. O `name:` é `Tipo · o que é`, então o
# tipo é o texto ANTES do ` · ` — é isso que agrupa o resumo na ordem real,
# que é justamente o que a lista de checks do PR (alfabética) não entrega.
STAGE_ORDER = [
    "Validação",
    "Lint",
    "Compilação",
    "Testes",
    "Cobertura",
    "Segurança",
    "Relatório",
    "Infra",
    "Release",
    "Distribuição",
    "Fechamento",
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


def job_tipo(job: dict) -> str:
    """Tipo de um job = texto antes do ` · ` no `name:`.

    É o que substituiu o prefixo numérico: o `name:` é `Tipo · o que é`, então
    agrupar por tipo devolve a ordem de execução mesmo com a lista do GitHub
    em ordem alfabética.
    """
    name = str(job.get("name") or "")
    return name.split(" · ")[0].strip() if " · " in name else "?"


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
        f"_Branch classificada como **{klass}** — o que não rodou está justificado pela "
        "regra `branch → tipos de job` (issue #68)._",
        "",
    ]

    by_stage: dict[str, list[dict]] = {}
    for job in jobs:
        by_stage.setdefault(job_tipo(job), []).append(job)

    lines += ["### Tipos de job", "", "| Tipo | Job | Estado | Tempo |", "|---|---|---|---|"]
    for tipo in STAGE_ORDER:
        for job in by_stage.get(tipo, []):
            state = job.get("conclusion") or job.get("status") or "unknown"
            lines.append(
                f"| **{tipo}** | {job.get('name', '?')} | "
                f"{ICON.get(state, '·')} {state} | {duration(job)} |"
            )
    # Jobs cujo nome nao segue `Tipo · ...` nao somem do relatorio.
    for job in jobs:
        if job_tipo(job) not in STAGE_ORDER:
            name = job.get("name", "?")
            state = job.get("conclusion") or job.get("status") or "unknown"
            lines.append(f"| _sem tipo_ | {name} | {ICON.get(state, '·')} {state} | {duration(job)} |")

    rows = coverage_rows(os.environ.get("COVERAGE_DIR", ".cov"))
    lines += ["", "### Cobertura", ""]
    if rows:
        lines += ["| Produto · métrica | Valor | |", "|---|---|---|", *rows]
    else:
        lines.append("_Nenhum resumo de cobertura publicado nesta run "
                     "(o job `Cobertura` roda em `dev`, `develop`, `main`, `hotfix` e tag)._")

    slowest = sorted(
        (j for j in jobs if (j.get("conclusion") == "success")),
        key=lambda j: duration(j), reverse=True,
    )[:3]
    if slowest:
        lines += ["", "### 💸 Os 3 jobs mais caros (greenfield de custo)", ""]
        for job in slowest:
            lines.append(f"- `{job.get('name')}` — **{duration(job)}**")

    lines += ["", "---", "", "_Gerado por `scripts/ci_report.py` no job `Relatório`._"]
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