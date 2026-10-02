#!/usr/bin/env python3
"""
Tempos por JOB e por STEP de uma run do GitHub Actions — a régua das mudanças
de custo do CI (issue #41: antes/depois da imagem ci-linux + caches).

Por que existe: "o CI ficou mais rápido" só vale com número. Este script é a
medição reproduzível do antes/depois, por job e por step, direto da API (sem
depender de captura de tela de log).

Uso:
    python3 scripts/ci_timings.py 36946076269            # tabela de uma run
    python3 scripts/ci_timings.py 36946076269 --steps    # + steps de cada job
    python3 scripts/ci_timings.py --compare 36946076269 36950000000
    python3 scripts/ci_timings.py <run> --repo owner/repo

Depende só do `gh` autenticado (mesmo CLI do ci.yml) + stdlib. Jobs pulados
(skipped) aparecem com 0s amarelo → o quadro completo fica visível.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime


def gh_api(path: str) -> list[dict]:
    """GET paginado na API do GitHub (--slurp junta as páginas num array)."""
    proc = subprocess.run(
        ["gh", "api", path, "--paginate", "--slurp"],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        sys.exit(f"gh api falhou para {path}:\n{proc.stderr.strip()}")
    data = json.loads(proc.stdout or "[]")
    if isinstance(data, dict):  # resposta única (não paginada)
        return [data]
    pages: list[dict] = []
    for page in data:
        pages.extend(page if isinstance(page, list) else [page])
    return pages


def repo_slug() -> str:
    proc = subprocess.run(
        ["gh", "repo", "view", "--json", "nameWithOwner", "-q", ".nameWithOwner"],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        sys.exit("não consegui descobrir o repo (rode dentro do checkout ou use --repo)")
    return proc.stdout.strip()


def seconds(ini: str | None, fim: str | None) -> int | None:
    if not ini or not fim:
        return None
    a = datetime.fromisoformat(ini.replace("Z", "+00:00"))
    b = datetime.fromisoformat(fim.replace("Z", "+00:00"))
    return round((b - a).total_seconds())


def run_info(run_id: int, repo: str) -> dict:
    data = gh_api(f"repos/{repo}/actions/runs/{run_id}")
    return data[0] if data else {}


def job_timings(run_id: int, repo: str) -> dict[str, dict]:
    jobs = gh_api(f"repos/{repo}/actions/runs/{run_id}/jobs")
    out: dict[str, dict] = {}
    for job in jobs:
        steps = []
        for step in job.get("steps") or []:
            dur = seconds(step.get("started_at"), step.get("completed_at"))
            if dur is not None:
                steps.append((step.get("name") or "?", dur))
        out[job.get("name") or f"job-{job.get('id')}"] = {
            "total": seconds(job.get("started_at"), job.get("completed_at")),
            "conclusion": job.get("conclusion"),
            "steps": steps,
        }
    return out


def fmt(dur: int | None) -> str:
    return "  0s" if dur is None else f"{dur:>4}s"


def show(run_id: int, repo: str, with_steps: bool) -> None:
    info = run_info(run_id, repo)
    jobs = job_timings(run_id, repo)
    wall = seconds(info.get("created_at"), info.get("updated_at"))
    print(f"\nrun {run_id} — {info.get('event')} · {info.get('head_branch')} · "
          f"{info.get('conclusion')} — parede {fmt(wall).strip()} ({len(jobs)} jobs)")
    for name, data in sorted(jobs.items(), key=lambda kv: kv[1]["total"] or 0, reverse=True):
        print(f"  {fmt(data['total'])}  {name}  [{data['conclusion']}]")
        if with_steps:
            for step_name, dur in sorted(data["steps"], key=lambda s: s[1], reverse=True):
                print(f"           {dur:>4}s  {step_name}")


def compare(before_id: int, after_id: int, repo: str, with_steps: bool) -> None:
    before_info = run_info(before_id, repo)
    after_info = run_info(after_id, repo)
    before = job_timings(before_id, repo)
    after = job_timings(after_id, repo)
    wall_b = seconds(before_info.get("created_at"), before_info.get("updated_at"))
    wall_a = seconds(after_info.get("created_at"), after_info.get("updated_at"))
    print(f"\nANTES  run {before_id} ({before_info.get('created_at')}) — parede {wall_b}s")
    print(f"DEPOIS run {after_id} ({after_info.get('created_at')}) — parede {wall_a}s")
    if wall_b and wall_a:
        print(f"DELTA da parede: {wall_a - wall_b:+}s\n")
    i = 0
    print(f"  {'job':<58} {'antes':>6} {'depois':>7} {'delta':>7}")
    for name in sorted(before.keys() | after.keys()):
        b = (before.get(name) or {}).get("total")
        a = (after.get(name) or {}).get("total")
        if b is None and a is None:
            continue
        delta = None if (b is None or a is None) else a - b
        marca = "" if delta is None else (" ⬅ novo" if b is None else (" ⬅ removido" if a is None else ""))
        i += 1
        print(f"  {name[:58]:<58} {str(b):>6} {str(a):>7} {str(delta):>7}{marca}")
    if with_steps:
        print("\n  steps com delta ≥ 3s (jobs em comum):")
        for name in sorted(before.keys() & after.keys()):
            sb = dict((before[name]["steps"]))
            sa = dict((after[name]["steps"]))
            for step in sorted(sb.keys() & sa.keys()):
                if abs(sa[step] - sb[step]) >= 3:
                    print(f"    {sa[step] - sb[step]:+}s  {name[:40]} :: {step[:60]}")
    print()


def main() -> int:
    ap = argparse.ArgumentParser(description="Tempos (job/step) de runs do Actions")
    ap.add_argument("runs", nargs="+", type=int, help="run id do GitHub Actions")
    ap.add_argument("--compare", action="store_true", help="2 runs: tabela antes/depois")
    ap.add_argument("--steps", action="store_true", help="detalhar steps (e diff de steps no compare)")
    ap.add_argument("--repo", default=None, help="owner/repo (default: o do checkout)")
    args = ap.parse_args()

    repo = args.repo or repo_slug()
    if args.compare:
        if len(args.runs) != 2:
            sys.exit("--compare exige exatamente 2 runs (antes depois)")
        compare(args.runs[0], args.runs[1], repo, args.steps)
    else:
        for run_id in args.runs:
            show(run_id, repo, args.steps)
    return 0


if __name__ == "__main__":
    sys.exit(main())
