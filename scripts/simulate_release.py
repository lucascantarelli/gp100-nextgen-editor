#!/usr/bin/env python3
"""
Simulação END-TO-END do release-gitflow (rc1→rcN→promote) num sandbox git
local — sem GitHub e sem Docker. Os blocos `run:` são extraídos DO PRÓPRIO
`.github/workflows/ci.yml` (jobs `release-rc` e `release-promote`) e
executados contra um "origin" bare temporário, com
`${{ inputs/steps/env }}` resolvidos como o Actions faria (GITHUB_OUTPUT
simulado, env de job injetado). O que NÃO é exercido aqui: a execução real no
Actions e o pipeline de distribuição por tag (esse roda no push).

Cenários provados:
  1. rc corta release/x.y.z de develop e etiqueta vX.Y.Z-rc.1
  2. fix em develop + rc de novo → rc.2 REUTILIZANDO a branch
  3. promote → tag final vX.Y.Z, main recebe o SHA da tag, merge --no-ff em
     develop com a PRÓXIMA versão, release branch apagada
  4. promote é idempotente: tag final existente recusa
  5. rc sem bump pendente (sem feat desde a última tag) recusa

Uso: python3 scripts/simulate_release.py [--keep]   (exit 0 = cadeia íntegra;
--keep preserva o sandbox em $TMPDIR para inspeção)
"""
from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
import tempfile

import yaml

# Console do Windows (cp1252) não encoda os acentos/setas dos prints deste
# script — o gate roda no runner Linux (UTF-8), mas o host de dev precisa
# disto para a simulação não morrer no meio.
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
# Desde a #68 o fluxo de release virou um ESTÁGIO do workflow único; os jobs
# se chamam `release-rc` e `release-promote`.
WF = os.path.join(REPO, ".github", "workflows", "ci.yml")
JOBS = {"rc": "release-rc", "promote": "release-promote"}
BOT = {"BOT_NAME": "sim-bot", "BOT_EMAIL": "sim-bot@example.test"}

EXPR = re.compile(r"\$\{\{\s*([a-zA-Z_][\w.]*)\s*\}\}")


def _shq(v: str) -> str:
    """escapa valor para export em aspas simples (prólogo do step)."""
    return "'" + v.replace("'", "'\\''") + "'"


def sh(cmd: str, cwd: str, env: dict | None = None, check: bool = True) -> tuple[int, str]:
    """roda comando git/shell no sandbox; retorna (exit, saída combinada).

    Só para comandos de UMA linha e SEM expansões ($var/$()) — o `bash -c`
    do Windows pode cair no WSL (System32 precede o PATH na busca do
    CreateProcess) e nesse caminho atribuições/expansões não persistem.
    Os steps do workflow NÃO passam por aqui: vão via arquivo .sh.
    """
    full_env = {**os.environ, **BOT, **(env or {})}
    proc = subprocess.run(
        ["bash", "-c", cmd], cwd=cwd, env=full_env,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
    )
    if check and proc.returncode != 0:
        raise AssertionError(f"comando falhou ({proc.returncode}) em {cwd}:\n{cmd}\n{proc.stdout}")
    return proc.returncode, proc.stdout


class Sandbox:
    """origin bare + clone de trabalho, como runner efêmero do Actions."""

    def __init__(self, root: str):
        self.root = root
        self.origin = os.path.join(root, "origin.git")
        self.seed = os.path.join(root, "seed")

    def setup(self) -> None:
        sh("git init --bare -b main origin.git", self.root)
        sh("git clone -q origin.git seed", self.root)
        manifests = {
            "Cargo.toml": '[package]\nname = "sim"\nversion = "0.1.0"\n',
            "packages/app/api/Cargo.toml": '[package]\nname = "sim-api"\nversion = "0.1.0"\n',
            "version.json": '{\n  "version": "0.1.0"\n}\n',
        }
        for path, content in manifests.items():
            full = os.path.join(self.seed, path)
            os.makedirs(os.path.dirname(full), exist_ok=True)
            with open(full, "w", encoding="utf-8") as fh:
                fh.write(content)
        for repo in (self.seed,):
            sh('git config user.name sim && git config user.email sim@example.test', repo)
        sh('git add -A && git commit -qm "chore: base 0.1.0"', self.seed)
        sh("git tag v0.1.0", self.seed)
        sh("git checkout -qb develop", self.seed)
        sh('git commit -q --allow-empty -m "feat(ui: x): primeira feature"', self.seed)
        sh("git push -q origin main develop && git push -q origin v0.1.0", self.seed)

    def workdir(self, name: str, branch: str | None = None) -> str:
        wd = os.path.join(self.root, name)
        args = f"-b {branch} " if branch else ""
        sh(f"git clone -q {args}origin.git {name}", self.root)
        sh("git config user.name sim && git config user.email sim@example.test", wd)
        sh("git fetch -q origin --tags --force", wd)
        return wd

    def remote_has(self, ref: str) -> bool:
        code, out = sh(f"git ls-remote origin {ref}", self.root, check=False)
        return code == 0 and bool(out.strip())

    def remote_manifest_version(self, ref: str) -> str:
        # ref chega como origin/main|develop; no bare é refs/heads/<nome>
        branch = ref.replace("origin/", "")
        _, out = sh(f"git --git-dir=origin.git show refs/heads/{branch}:version.json", self.root)
        return re.search(r'"version":\s*"([^"]+)"', out).group(1)


class Workflow:
    """executa os run blocks de um job do ci.yml com contexto simulado."""

    def __init__(self, root: str):
        with open(WF, encoding="utf-8") as fh:
            doc = yaml.safe_load(fh)
        self.jobs = doc["jobs"]
        self.wf_env = {k: str(v) for k, v in (doc.get("env") or {}).items()}
        self.inputs: dict[str, str] = {}
        self.steps: dict[str, dict[str, str]] = {}

    def resolve(self, text: str) -> str:
        def sub(m: re.Match) -> str:
            path = m.group(1)
            if path.startswith("inputs."):
                return self.inputs.get(path.split(".", 1)[1], "")
            if path.startswith("steps."):
                _, sid, _, key = path.split(".")
                return self.steps.get(sid, {}).get(key, "")
            if path.startswith("env."):
                return self.wf_env.get(path.split(".", 1)[1], "")
            if path == "github.event.inputs.tag":
                return ""
            if path in ("github.repository", "github.server_url", "github.actor"):
                return f"example/{path.split('.')[-1]}"
            raise AssertionError(f"expressão não simulada: ${{{{ {path} }}}}")

        return EXPR.sub(sub, text)

    def run_job(self, job: str, cwd: str, inputs: dict[str, str],
                expect_failure: bool = False) -> None:
        """executa os steps run do job; `uses:` vira setup do runner.

        Cada step vai para um .sh DENTRO do clone (untracked; o release.yml
        usa `git add` com caminhos explícitos) e roda `bash arquivo.sh` —
        NUNCA `bash -c`: além de ser como o Actions executa de verdade,
        o `-c` através do interop WSL do Windows perde atribuições, expan-
        sões e o env injetado (descoberto à base de trace -x + sondas).
        O env (workflow + step + magic do Actions) entra por `export` no
        prólogo; GITHUB_OUTPUT/GITHUB_STEP_SUMMARY ficam relativos ao cwd.
        """
        self.inputs = inputs
        self.steps = {}
        job_id = JOBS.get(job, job)
        if job_id not in self.jobs:
            raise AssertionError(f"job '{job_id}' não existe em ci.yml (tem: {', '.join(self.jobs)})")
        for idx, step in enumerate(self.jobs[job_id]["steps"]):
            if "run" not in step:
                continue
            sid = re.sub(r"[^\w-]", "_", str(step.get("id") or f"step{idx}"))
            out_name = f".gh-output-{sid}"
            step_env = {
                **self.wf_env,
                **{k: self.resolve(str(v)) for k, v in (step.get("env") or {}).items()},
                "GITHUB_OUTPUT": out_name,
                "GITHUB_STEP_SUMMARY": ".gh-summary",
            }
            prologue = "\n".join(f"export {k}={_shq(v)}" for k, v in step_env.items())
            script = prologue + "\n" + self.resolve(str(step["run"])) + "\n"
            step_path = os.path.join(cwd, f".gh-step-{sid}.sh")
            with open(step_path, "w", encoding="utf-8", newline="\n") as fh:
                fh.write(script)
            out_file = os.path.join(cwd, out_name)
            if os.path.exists(out_file):
                os.remove(out_file)
            argv = ["bash", "-x", f".gh-step-{sid}.sh"] \
                if os.environ.get("SIM_TRACE") == "1" else ["bash", f".gh-step-{sid}.sh"]
            proc = subprocess.run(
                argv, cwd=cwd, env={**os.environ, **BOT},
                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
            )
            os.remove(step_path)
            code, out = proc.returncode, proc.stdout
            if step.get("id"):
                self.steps[step["id"]] = {}
                if os.path.exists(out_file):
                    with open(out_file, encoding="utf-8") as fh:
                        for line in fh:
                            if "=" in line:
                                k, v = line.rstrip("\n").split("=", 1)
                                self.steps[step["id"]][k] = v
            print(f"    [{job}/{step.get('id', '?')}] exit={code}")
            if code != 0:
                if expect_failure:
                    print("      (falha ESPERADA — guard funcionou)")
                    return
                raise AssertionError(
                    f"step {job}/{step.get('id', '?')} falhou:\n"
                    f"--- script ---\n{script}\n--- saída ---\n{out}"
                )
        if expect_failure:
            raise AssertionError("esperava falha do guard e o job terminou verde")


def main() -> int:
    keep = "--keep" in sys.argv
    root = tempfile.mkdtemp(prefix="sim-release-")
    print(f"sandbox: {root}\n")

    sandbox = Sandbox(root)
    sandbox.setup()
    wf = Workflow(root)

    print("1) rc: corta release/0.2.0 de develop e etiqueta v0.2.0-rc.1")
    wd = sandbox.workdir("rc1", branch="develop")
    wf.run_job("rc", wd, inputs={"action": "release", "version": "", "source": "develop"})
    assert wf.steps["prep"]["full"] == "0.2.0-rc.1", wf.steps["prep"]
    assert sandbox.remote_has("refs/tags/v0.2.0-rc.1")
    assert sandbox.remote_has("refs/heads/release/0.2.0")

    print("2) fix em develop + rc de novo → rc.2 REUTILIZANDO a branch")
    sh('git commit -q --allow-empty -m "fix(ui: x): correção entre rcs"', sandbox.seed)
    sh("git push -q origin develop", sandbox.seed)
    wd = sandbox.workdir("rc2", branch="develop")
    wf.run_job("rc", wd, inputs={"action": "release", "version": "", "source": "develop"})
    assert wf.steps["prep"]["full"] == "0.2.0-rc.2", wf.steps["prep"]

    print("3) promote → tag final, merge main, backport develop, branch apagada")
    wd = sandbox.workdir("promote")
    wf.run_job("promote", wd, inputs={"action": "promote", "version": ""})
    assert sandbox.remote_has("refs/tags/v0.2.0")
    # main recebe o SHA da tag; develop sai da release JA na próxima versão.
    assert sandbox.remote_manifest_version("origin/main") == "0.2.0"
    assert sandbox.remote_manifest_version("origin/develop") == "0.3.0"
    assert not sandbox.remote_has("refs/heads/release/0.2.0")

    print("4) promote de novo → recusa (idempotente)")
    wd = sandbox.workdir("promote2")
    wf.run_job("promote", wd, inputs={"action": "promote", "version": ""},
               expect_failure=True)

    print("5) rc sem bump pendente → recusa (exige feat ou input version)")
    wd = sandbox.workdir("rc3", branch="develop")
    wf.run_job("rc", wd, inputs={"action": "release", "version": "", "source": "develop"}, expect_failure=True)

    print("\n✅ cadeia rc1→rcN→promote íntegra no sandbox (ci.yml executado de verdade)")
    if keep:
        print(f"sandbox preservado em {root}")
        return 0
    shutil.rmtree(root, ignore_errors=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
