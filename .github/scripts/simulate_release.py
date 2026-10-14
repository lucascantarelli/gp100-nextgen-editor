#!/usr/bin/env python3
"""
Simulação END-TO-END do release-gitflow (rc1→rcN→promote) num sandbox git
local — sem GitHub e sem Docker. Os blocos `run:` são extraídos DOS PRÓPRIOS
WORKFLOWS que rodam de verdade — os reutilizáveis `.github/workflows/
release-rc.yml` e `release-promote.yml` (+ o contrato do caller no `ci.yml`,
#245) — e executados contra um "origin" bare temporário, com
`${{ inputs/steps/env }}` resolvidos como o Actions faria (GITHUB_OUTPUT
simulado, env de job injetado, `with:` do caller resolvido para os inputs do
workflow chamado). O que NÃO é exercido aqui: a execução real no Actions e o
pipeline de distribuição por tag (esse roda no push).

No CI este gate roda por ESCOPO (#246): schedule, tag/dispatch e PR que toca a
maquinaria de release (`ci_plan.py` → `scope-ci`).

Cenários provados:
  1. rc corta release/x.y.z de develop e etiqueta vX.Y.Z-rc.1
  2. fix em develop + rc de novo → rc.2 REUTILIZANDO a branch
  3. promote → tag final vX.Y.Z, main recebe o SHA da tag, merge --no-ff em
     develop com a PRÓXIMA versão, release branch apagada
  4. promote é idempotente: tag final existente recusa
  5. rc sem bump pendente (sem feat desde a última tag) recusa

Uso: python3 .github/scripts/simulate_release.py [--keep]   (exit 0 = cadeia íntegra;
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

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
# Desde a #68 o fluxo de release virou um ESTÁGIO do workflow único; os jobs
# se chamam `release-rc` e `release-promote`.
# No Windows o CreateProcess procura System32 ANTES do PATH: um `bash` solto
# cai no bash.exe do WSL (nem sempre funcional neste host) em vez do git-bash.
# `shutil.which` respeita o PATH de verdade — o mesmo bash que o Actions usa.
def _bash() -> str:
    """Bash real do host — NUNCA o launcher do WSL (#206).

    No Windows o CreateProcess procura System32 ANTES do PATH, e o
    `C:\\Windows\\System32\\bash.exe` é o launcher do WSL: em host sem distro
    todo `sh()` falha com "Falha ao anexar o disco"; em host COM distro o bash
    seria o da distro, não o do repo. `shutil.which`/`where` respeitam o PATH,
    mas num PATH padrão do Windows o System32 vem primeiro — então varremos o
    PATH na ordem e pegamos o primeiro `bash.exe` que NÃO é do System32 nem o
    alias da Microsoft Store (WindowsApps, que também intercepta). Sem
    candidato (runner ubuntu da CI), cai no `bash` do PATH, que é o da distro.
    """
    if os.name == "nt":
        for pasta in os.environ.get("PATH", "").split(os.pathsep):
            base = os.path.basename(pasta.rstrip("\\/")).lower()
            if base in ("system32", "windowsapps"):
                continue
            cand = os.path.join(pasta, "bash.exe")
            if os.path.isfile(cand):
                return cand
    return shutil.which("bash") or "bash"


BASH = _bash()


def _python_do_host() -> str:
    """O interpretador que o sandbox usa no lugar de `python3` (quando precisa).

    O texto dos workflows chama `python3` — é o binário do runner Linux, onde o
    gate roda de verdade. No host do dono, NEM `python` NEM `python3` do PATH
    são o interpretador: os dois são os STUBS da Microsoft Store (WindowsApps),
    que respondem "Python was not found" e matam a simulação no primeiro
    `python3 .github/scripts/sync_version.py` — o gate local reprovava sem ter
    nada de errado no workflow. A tradução é para o MESMO interpretador que roda
    este script, no caminho estilo MSYS que o bash do Git entende (o caminho tem
    espaço, então vai entre aspas).
    """
    exe = sys.executable or "python"
    if os.name == "nt":
        exe = "/" + exe[0].lower() + exe[2:].replace("\\", "/")
    return f'"{exe}"'


PYTHON_DO_HOST = _python_do_host()

# ── DE ONDE VÊM OS BLOCOS (#245) ───────────────────────────────────────────
# Desde a extração da #245 os jobs de release moram em workflows
# REUTILIZÁVEIS (`workflow_call`), e não mais dentro do `ci.yml`. O simulador
# executa os `run:` dos blocos: ele lê o arquivo que TEM os blocos, e confere no
# `ci.yml` que o caller chama aquele arquivo e passa os inputs certos.
#
# O `env` de topo do arquivo do job é o que os steps leem (`BOT_NAME`/
# `BOT_EMAIL`): `env:` de topo do caller NÃO atravessa um `workflow_call`, então
# o `ci.yml` não tem mais essas duas chaves.
WF = {
    "rc": os.path.join(REPO, ".github", "workflows", "release-rc.yml"),
    "promote": os.path.join(REPO, ".github", "workflows", "release-promote.yml"),
}
CI = os.path.join(REPO, ".github", "workflows", "ci.yml")
# etapa do simulador → (job no ci.yml, job no arquivo reutilizável)
JOBS = {"rc": "rc", "promote": "promote"}
CALLER = {"rc": "release-rc", "promote": "release-promote"}


def locais_no_lock(texto: str) -> dict[str, str]:
    """`nome -> versao` dos pacotes LOCAIS de um `Cargo.lock`.

    Mesma regra do `sync_version.py`: pacote de CAMINHO e o que nao tem
    `source = ...` no bloco (terceiro sempre tem) — a versao deles e a da
    release e tem de casar com os manifests (#231).
    """
    locais: dict[str, str] = {}
    for bloco in texto.split("[[package]]")[1:]:
        if re.search(r"^source\s*=", bloco, re.M):
            continue
        nome = re.search(r'^name\s*=\s*"([^"]+)"', bloco, re.M)
        versao = re.search(r'^version\s*=\s*"([^"]+)"', bloco, re.M)
        if nome and versao:
            locais[nome.group(1)] = versao.group(1)
    return locais


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
        [BASH, "-c", cmd], cwd=cwd, env=full_env,
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
        # Os 5 manifests que `sync_version.py` governa (#73) — antes eram 3, e
        # faltando `tauri.conf.json`/`package.json` a cadeia rc/promote rodava
        # num repo que nao se parecia com o real: o bug so aparecia no repo de
        # verdade, nunca aqui. O `frontendDist` aponta para um `dist/` minimo
        # para o conf ser um conf de verdade.
        manifests = {
            "Cargo.toml": '[package]\nname = "sim"\nversion = "0.1.0"\n',
            "packages/app/api/Cargo.toml": '[package]\nname = "sim-api"\nversion = "0.1.0"\n',
            "packages/app/api/tauri.conf.json": '{\n  "version": "0.1.0",\n  "build": {\n    "frontendDist": "../ui/dist"\n  }\n}\n',
            "packages/app/ui/package.json": '{\n  "name": "sim-ui",\n  "version": "0.1.0"\n}\n',
            "version.json": '{\n  "version": "0.1.0"\n}\n',
            # ... e os DOIS lock files (#231): desde que o bump reescreve a
            # versao dos pacotes LOCAIS neles (crate de caminho = o que nao
            # tem `source`), um sandbox sem lock nao se parece com o repo real
            # — e o "--set" morreria aqui enquanto a cadeia de verdade passava.
            # Mesma licenca do #73: o sandbox tem de ter o que os jobs leem.
            "Cargo.lock": (
                "# gerado\nversion = 4\n\n"
                '[[package]]\nname = "sim"\nversion = "0.1.0"\n'
            ),
            "packages/app/api/Cargo.lock": (
                "# gerado\nversion = 4\n\n"
                '[[package]]\nname = "sim-api"\nversion = "0.1.0"\n\n'
                '[[package]]\nname = "serde"\nversion = "1.0.0"\n'
                'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
            ),
        }
        for path, content in manifests.items():
            full = os.path.join(self.seed, path)
            os.makedirs(os.path.dirname(full), exist_ok=True)
            with open(full, "w", encoding="utf-8", newline="") as fh:
                fh.write(content)

        # O `run:` dos jobs chama `python3 .github/scripts/sync_version.py` (issue #73),
        # entao o sandbox precisa do script — sem ele a cadeia morre com
        # "can't open file .../sync_version.py" e o gate do Lint reprova.
        script_dst = os.path.join(self.seed, ".github", "scripts")
        os.makedirs(script_dst, exist_ok=True)
        shutil.copyfile(
            os.path.join(REPO, ".github", "scripts", "sync_version.py"),
            os.path.join(script_dst, "sync_version.py"),
        )
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

    def remote_file(self, full_ref: str, path: str) -> str:
        """Conteudo de `path` num ref do bare; `full_ref` = refs/heads/<b> | refs/tags/<t>."""
        _, out = sh(f"git --git-dir=origin.git show {full_ref}:{path}", self.root)
        return out

    def remote_lock_versions(self, full_ref: str) -> dict[str, str]:
        """Versoes dos pacotes LOCAIS nos DOIS lock files daquele ref (#235)."""
        locais: dict[str, str] = {}
        for lock in ("Cargo.lock", "packages/app/api/Cargo.lock"):
            for nome, versao in locais_no_lock(self.remote_file(full_ref, lock)).items():
                locais[f"{lock}:{nome}"] = versao
        return locais


class Workflow:
    """executa os run blocks de um job de release com contexto simulado.

    Os jobs vêm dos arquivos reutilizáveis (`release-*.yml`); o `ci.yml` é lido
    para o CONTRATO do caller — `uses:` apontando para o arquivo certo e os
    inputs chegando por `with:` (um input obrigatório sem valor recusa a run no
    LOAD, e esse defeito é invisível em teste de unidade do YAML).
    """

    def __init__(self, root: str):
        self.jobs: dict[str, dict] = {}
        self.env_do_job: dict[str, dict[str, str]] = {}
        for caminho in WF.values():
            with open(caminho, encoding="utf-8") as fh:
                doc = yaml.safe_load(fh)
            env_arq = {k: str(v) for k, v in (doc.get("env") or {}).items()}
            for job_id, job in (doc.get("jobs") or {}).items():
                self.jobs[job_id] = job
                self.env_do_job[job_id] = env_arq
        with open(CI, encoding="utf-8") as fh:
            self.ci_jobs = (yaml.safe_load(fh) or {}).get("jobs") or {}
        self.inputs: dict[str, str] = {}
        self.steps: dict[str, dict[str, str]] = {}
        self.wf_env: dict[str, str] = {}

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

        texto = EXPR.sub(sub, text)
        if os.name == "nt":
            # O sandbox roda o MESMO conteúdo, só com o binário análogo do host
            # (`_python_do_host` explica por que não é um `python` cru).
            texto = re.sub(r"\bpython3\b", PYTHON_DO_HOST, texto)
        return texto

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
        self.steps = {}
        job_id = JOBS.get(job, job)
        if job_id not in self.jobs:
            raise AssertionError(
                f"job '{job_id}' não existe nos reutilizáveis (tem: {', '.join(self.jobs)})"
            )
        self.wf_env = self.env_do_job[job_id]
        # Os inputs do JOB CHAMADO vêm do `with:` do caller, resolvido com os
        # inputs do dispatch — é assim que o Actions faz, e é o que faz a
        # simulação cobrir o encanamento (nome de input errado, valor faltando).
        caller = CALLER.get(job)
        if caller is None:
            self.inputs = dict(inputs)
        else:
            esperado = "./" + os.path.relpath(WF[job], REPO).replace(os.sep, "/")
            job_caller = self.ci_jobs.get(caller) or {}
            if str(job_caller.get("uses") or "") != esperado:
                raise AssertionError(
                    f"o caller `{caller}` do ci.yml não chama {esperado!r} "
                    f"(chama {job_caller.get('uses')!r})"
                )
            self.inputs = dict(inputs)
            passados = job_caller.get("with") or {}
            self.inputs = {k: self.resolve(str(v)) for k, v in passados.items()}
            if self.inputs.get("version") is None:
                # o reutilizável lê `inputs.version` (input do workflow_call):
                # sem a chave o corpo compararia com vazio e cairia no caminho
                # errado — melhor falhar aqui, nomeando o caller.
                raise AssertionError(
                    f"o caller `{caller}` não passa `version` — o job chamado lê esse input"
                )
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
            argv = [BASH, "-x", f".gh-step-{sid}.sh"] \
                if os.environ.get("SIM_TRACE") == "1" else [BASH, f".gh-step-{sid}.sh"]
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

    print("3b) a TAG e a develop saem com LOCK e MANIFEST na MESMA versao (#235)")
    # O `sync_version.py --set` reescreve tambem os 2 `Cargo.lock` desde #231;
    # se o commit do job nao os levar junto, a arvore publicada fica com
    # manifest 0.2.0 e lock 0.3.0 e o `--check` do Lint reprova no push da tag
    # (distribuicao inteira `skipped`) — foi exatamente o que aconteceu no
    # ciclo da v0.2.1 (run 38568214477).
    esperados = ("refs/tags/v0.2.0", "0.2.0"), ("refs/heads/develop", "0.3.0")
    for ref, versao in esperados:
        fora = {n: v for n, v in sandbox.remote_lock_versions(ref).items() if v != versao}
        assert not fora, f"{ref}: lock fora de {versao} — {fora}"
        assert sandbox.remote_file(ref, "version.json").find(versao) > 0, ref

    print("4) promote de novo → recusa (idempotente)")
    wd = sandbox.workdir("promote2")
    wf.run_job("promote", wd, inputs={"action": "promote", "version": ""},
               expect_failure=True)

    print("5) rc sem bump pendente → recusa (exige feat ou input version)")
    wd = sandbox.workdir("rc3", branch="develop")
    wf.run_job("rc", wd, inputs={"action": "release", "version": "", "source": "develop"}, expect_failure=True)

    print("\n✅ cadeia rc1→rcN→promote íntegra no sandbox (release-*.yml executados de verdade)")
    if keep:
        print(f"sandbox preservado em {root}")
        return 0
    shutil.rmtree(root, ignore_errors=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
