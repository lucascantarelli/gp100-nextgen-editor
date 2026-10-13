#!/usr/bin/env python3
"""
Achado de segurança = issue na FASE ACHADOS (label `achados-security`).

REGRA (fail-safe deliberada): qualquer auditoria com exit ≠ 0 vira achado —
inclusive quando a FALHA É DA FERRAMENTA, não da dependência. Perder um alerta
por causa de um `cargo audit` quebrado é bem pior do que um alarme a mais, e
um alarme a mais é o jeito que se aprende a ignorar o vermelho.

Se já existe issue aberta da label, comenta nela em vez de abrir outra: cinco
issues para cincoCVEs da mesma ferramenta é ruído, um issue com cinco
comentários é fila de trabalho.

Uso: python3 scripts/security_report.py
     env: GH_TOKEN, RUN_URL + os outcomes dos steps no arquivo .audit-outcomes
"""
from __future__ import annotations

import os
import re
import subprocess
import sys
from datetime import datetime, timezone

try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
except (AttributeError, OSError):
    pass

LABEL = "achados-security"
AUDITS = [
    ("cargo audit (workspace)", "/tmp/cargo-audit.txt"),
    ("cargo audit (gp100-ui)", "/tmp/cargo-audit-ui.txt"),
    ("pnpm audit (runtime)", "/tmp/pnpm-audit.txt"),
]


def sh(*args: str) -> subprocess.CompletedProcess:
    return subprocess.run(args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)


def outcomes() -> dict[str, str]:
    """Lê os `conclusion` que o workflow gravou (step outcome por step id)."""
    path = ".audit-outcomes"
    result: dict[str, str] = {}
    if os.path.exists(path):
        with open(path, encoding="utf-8") as fh:
            for line in fh:
                if "=" in line:
                    k, v = line.strip().split("=", 1)
                    result[k] = v
    return result


def head(text: str, lines: int = 80) -> str:
    """Primeiras linhas do log, com o que quebraria um bloco de markdown."""
    body = "".join(text.splitlines(keepends=True)[:lines])
    return body.replace("`", "\\`").replace("$", "\\$")


def main() -> int:
    results = outcomes()
    failed = [name for name, _ in AUDITS if results.get(name.replace(" ", "_"), "success") == "failure"]
    run_url = os.environ.get("RUN_URL", "")
    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")

    # Resumo SEMPRE — mesmo sem achado, o run mostra o que foi auditado.
    summary = ["### 🔒 Auditorias de dependência", "", "| Auditoria | Resultado |", "|---|---|"]
    for name, _ in AUDITS:
        state = results.get(name.replace(" ", "_"), "success")
        summary.append(f"| {name} | {'❌ falhou' if state == 'failure' else '✅ ok'} |")
    summary.append("")
    summary.append("_Detecção aqui · remediação no dependabot.yml. Não bloqueia o merge: "
                   "`glib` tem advisory documentada em SECURITY.md sem upgrade possível com o Tauri 2._")
    with open(os.environ.get("GITHUB_STEP_SUMMARY", os.devnull), "a", encoding="utf-8") as fh:
        fh.write("\n".join(summary) + "\n")

    if not failed:
        print("sem achados — nada a registrar")
        return 0

    sections = []
    for name, path in AUDITS:
        if name not in failed:
            continue
        text = ""
        if os.path.exists(path):
            with open(path, encoding="utf-8", errors="replace") as fh:
                text = fh.read()
        sections.append(f"<details><summary>{name}</summary>\n\n```\n{head(text)}\n```\n\n</details>")

    body = "\n".join([
        "## 🔒 Achado de segurança (varredura)",
        "",
        "**Regra:** FASE ACHADOS do `docs/ROADMAP.md` — prioridade MÁXIMA, resolve "
        "antes de qualquer issue de fase. Corrigido = fechar esta issue + linha "
        "A-xx no ROADMAP com a prova.",
        "",
        f"**Falhas:** {', '.join(failed)}",
        "",
        *sections,
        "",
        f"_Issue automática do CI — {run_url}_",
    ])

    existing = sh("gh", "issue", "list", "--label", LABEL, "--state", "open",
                  "--json", "number", "--jq", ".[0].number // empty").stdout.strip()
    # `|| true`: um label novo ainda não existe no primeiro achado.
    sh("gh", "label", "create", LABEL, "--color", "D93F0B",
       "--description", "Achado de segurança (auditoria de dependência)", "--force")

    if existing and re.fullmatch(r"\d+", existing):
        sh("gh", "issue", "comment", existing,
           "--body", f"Novo achado em {today} — {', '.join(failed)} — job: {run_url}")
        print(f"issue #{existing} atualizada")
    else:
        proc = sh("gh", "issue", "create", "--title",
                  f"🔒 ACHADOS: falha em {', '.join(failed)} ({today})",
                  "--label", LABEL, "--body", body)
        print(proc.stdout.strip() or "issue criada")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())