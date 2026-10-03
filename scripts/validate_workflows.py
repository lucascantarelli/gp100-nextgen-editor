#!/usr/bin/env python3
"""
Validação sintática + de CONTRATO do pipeline (gate do CI, job `Lint · contratos do pipeline`).

Por que este arquivo existe: o YAML do GitHub aceita muita coisa que só quebra
em produção — um `permissions` que o reusable pede e o caller não concede morre
no LOAD da run (mensagem só na annotation), uma matriz vazia faz a run INTEIRA
falhar sem nenhum job marcado como falha, um token que não sobe de permissão
para `write` falha no meio do job. Nenhuma dessas dá erro de sintaxe.

Os testes de INTEGRAÇÃO reais acontecem onde o workflow é a própria prova: o
fechamento de issues roda no merge, e `scripts/simulate_release.py` executa de
 verdade os blocos `run:` da rc/promote num sandbox git. Aqui travamos o que é
estrutura — e, desde a #68, que a ESTRUTURA nova é o que o projeto adoptou.

Contratos verificados:
  1. existe UM workflow (.github/workflows/ci.yml) e os cinco legados sumiram;
  2. gatilhos: push em develop/main/feature/bugfix/hotfix/release + tag v*;
     PR em develop/main com o tipo `closed`;
  3. permissão MÍNIMA no topo (`contents: read`) — o `write` é por job;
  4. os 11 tipos de job e os 25 jobs existem, com nome no padrão `Tipo · o que é`,
     sem prefixo numérico, e declarados na sequência da execução;
  5. a regra branch → tipos de job mora em `scripts/ci_plan.py` e NÃO volta para
     `if:` espalhado no YAML (duas fontes de verdade é como o gate divergiu);
     e o CodeQL NÃO é configurado aqui (o setup padrão do repo está ligado e o
     GitHub recusa SARIF de configuração avançada enquanto ele estiver ativo);
  6. versionamento e distribuição só por `workflow_dispatch`/tag;
  7. `close-issues` exige `merged == true` (PR fechado sem merge não fecha nada);
  8. o job `Relatório` roda com `if: always()`;
  9. as duas imagens: ci-linux (toolset do Tauri) e ci-base (sem GUI, com uv);
 10. composites preservados (shared-key, add-job-id-key, cache do Playwright);
 11. nenhum action da geração Node 20 (deprecada pelo runner).

Uso: python3 scripts/validate_workflows.py  (exit 1 na primeira violação)
"""
from __future__ import annotations

import glob
import os
import sys

import yaml

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FAILURES: list[str] = []


def load(path: str) -> dict | None:
    try:
        with open(os.path.join(REPO, path), encoding="utf-8") as fh:
            data = yaml.safe_load(fh)
        return data if isinstance(data, dict) else {}
    except Exception as exc:  # noqa: BLE001 — reportar qualquer YAML quebrado
        FAILURES.append(f"{path}: YAML inválido ({exc})")
        return None


def raw(path: str) -> str:
    try:
        with open(os.path.join(REPO, path), encoding="utf-8") as fh:
            return fh.read()
    except OSError:
        return ""


def triggers(doc: dict) -> dict:
    """`on:` — PyYAML 1.1 parseia a chave `on` como o booleano True."""
    return doc.get("on") or doc.get(True) or {}


# ── TIPOS de job × jobs que cada um DEVE ter ─────────────────────────────
# A ordem do dicionário É a sequência de execução pedida (validação → lint →
# compilação → testes → cobertura → segurança → relatório → infra → release →
# distribuição → fechamento). O GitHub não tem `stages:`; a ordem real é o
# grafo `needs:`. Então a sequência é garantida aqui como ORDEM DE DECLARAÇÃO
# no arquivo (que é como a gente lê e como o diff mostra), e o `needs:` garante
# que ela é real.
TIPOS: dict[str, list[str]] = {
    "Validação": ["plan"],
    "Lint": ["lint-workflows", "lint-commits", "lint-rust-fmt", "lint-rust-clippy", "lint-ui"],
    "Compilação": ["build-rust", "build-ui"],
    "Testes": ["test-rust", "test-ui", "test-spec", "test-e2e", "test-e2e-visual", "test-e2e-webview"],
    "Cobertura": ["coverage-ui"],
    "Segurança": ["sec-audit"],
    "Relatório": ["metrics"],
    "Infra": ["ci-images"],
    "Release": ["main-guard", "release-rc", "release-promote", "release-play"],
    "Distribuição": ["dist-ui", "dist-cli"],
    "Fechamento": ["close-issues"],
}
SEQUENCIA_ESPERADA = [j for jobs_ in TIPOS.values() for j in jobs_]

# Legados: a consolidação da #68 os substituiu por um workflow só.
LEGACY = (
    "_validate.yml",
    "_publish.yml",
    "release.yml",
    "security.yml",
    "container.yml",
    "pipeline.yml",
    "close-issues.yml",
)

NODE20_REFS = (
    "actions/cache@v4",
    "actions/download-artifact@v4",
    "actions/upload-artifact@v3",
    "docker/setup-buildx-action@v3",
    "docker/login-action@v3",
    "docker/metadata-action@v5",
    "docker/build-push-action@v6",
)


def main() -> int:
    # A console do Windows e cp1252 por padrao, e as mensagens deste arquivo usam
    # `→` e `·`. Sem isto, uma falha que CONTEM um caractere fora do cp1252
    # morre com UnicodeEncodeError no `print` — o gate estoura um traceback em
    # vez de dizer o que reprovou, e parece bug do gate.
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass
    workflows = sorted(p.replace("\\", "/") for p in glob.glob(os.path.join(REPO, ".github/workflows/*.yml")))
    workflows = [p[len(REPO) + 1:] for p in workflows]
    docs: dict[str, dict] = {}
    for path in workflows:
        doc = load(path)
        if doc is not None:
            docs[path] = doc

    # ── 1. UM workflow só, e os legados fora ──────────────────────────────
    for legacy in LEGACY:
        if f".github/workflows/{legacy}" in docs:
            FAILURES.append(f".github/workflows/{legacy}: legado ainda existe (a #68 consolidou tudo em ci.yml)")
    if set(workflows) != {".github/workflows/ci.yml"}:
        FAILURES.append(f"esperado exatamente 1 workflow (ci.yml); existem: {workflows}")

    ci = docs.get(".github/workflows/ci.yml") or {}
    if not ci:
        print("FALHAS:\n - ci.yml ausente")
        return 1

    on = triggers(ci)
    push = on.get("push") or {}
    pr = on.get("pull_request") or {}
    push_branches = set(push.get("branches") or [])
    pr_branches = set(pr.get("branches") or [])

    # ── 2. gatilhos ──────────────────────────────────────────────────────
    # `push` só onde NÃO existe PR. Branch de trabalho entra pelo
    # `pull_request`, que além de não duplicar o custo testa o MERGE SINTÉTICO
    # (cabeça do PR fundida na base) — o que realmente vai entrar. Push em
    # `feature/**` com PR aberto rodava o pipeline inteiro duas vezes.
    for needed in ("develop", "main"):
        if needed not in push_branches:
            FAILURES.append(f"ci: on.push.branches precisa de {needed} (integração e produção)")
    # Regressão: branch de trabalho NUNCA pode voltar ao `push` — cada push
    # passaria a custar duas runs (push + pull_request/synchronize).
    for proibido in ("feature/**", "bugfix/**", "hotfix/**", "release/**"):
        if proibido in push_branches:
            FAILURES.append(
                f"ci: on.push.branches nao deve ter {proibido} — a branch de trabalho e coberta "
                "pelo `pull_request`; no `push` o GitHub dispara os DOIS eventos e o pipeline roda "
                "duas vezes por push, sem informacao nova na segunda"
            )
    for needed in ("develop", "main", "hotfix/**", "release/**"):
        if needed not in pr_branches:
            FAILURES.append(f"ci: on.pull_request.branches precisa de {needed}")
    if "v*" not in (push.get("tags") or []):
        FAILURES.append("ci: on.push.tags precisa de v* (a release publica no push da tag)")
    if "develop" not in pr_branches or "main" not in pr_branches:
        FAILURES.append("ci: on.pull_request.branches precisa de develop e main")
    if "closed" not in (pr.get("types") or []):
        FAILURES.append("ci: on.pull_request.types precisa incluir 'closed' (fechamento de issue)")
    if "schedule" not in on:
        FAILURES.append("ci: precisa de on.schedule (auditoria noturna)")

    # ── 3. permissão mínima no topo ──────────────────────────────────────
    perms = ci.get("permissions") or {}
    if perms.get("contents") != "read":
        FAILURES.append("ci: permissions.contents no topo precisa ser 'read' — o `write` é por job")
    if perms.get("issues") != "write":
        FAILURES.append("ci: permissions.issues precisa ser 'write' (fechamento de issues)")
    if perms.get("packages") != "read":
        FAILURES.append("ci: permissions.packages precisa ser 'read' (pull das imagens no ghcr.io)")
    if perms.get("actions") != "read":
        FAILURES.append("ci: permissions.actions precisa ser 'read' (o job `Relatório` lista os jobs)")

    jobs = ci.get("jobs") or {}

    # ── 4. os tipos de job e a ordem de declaração ──────────────────────
    for tipo, expected in TIPOS.items():
        for job in expected:
            if job not in jobs:
                FAILURES.append(f"ci: tipo '{tipo}' perdeu o job '{job}'")

    tipo_de = {j: t for t, jobs_ in TIPOS.items() for j in jobs_}
    for job_id, job in jobs.items():
        name = str(job.get("name") or "")
        # O prefixo numérico saiu de propósito (a lista de checks do PR é
        # alfabética e o número só servia para ordenar a leitura). Este check
        # impede que ele volte por descuido.
        if name[:1].isdigit():
            FAILURES.append(
                f"ci: job '{job_id}' voltou a ter prefixo numérico no name: {name!r} — "
                "o padrão é `Tipo · o que é`"
            )
        esperado_tipo = tipo_de.get(job_id)
        if esperado_tipo and not name.startswith(f"{esperado_tipo} · "):
            FAILURES.append(
                f"ci: job '{job_id}' deveria ser `{esperado_tipo} · …`, mas é {name!r}"
            )
    desconhecidos = sorted(set(jobs) - set(tipo_de))
    if desconhecidos:
        FAILURES.append(f"ci: jobs fora de qualquer tipo: {desconhecidos}")

    declarados = list(jobs)
    if declarados != SEQUENCIA_ESPERADA:
        FAILURES.append(
            "ci: ordem de declaração fora da sequência "
            f"({' → '.join(SEQUENCIA_ESPERADA)}); veio {(' → '.join(declarados))}"
        )

    # ── 5. a REGRA branch → tipos de job mora no ci_plan.py, e só lá ──────────
    plan_raw = raw("scripts/ci_plan.py")
    ci_raw = raw(".github/workflows/ci.yml")
    for token in ("def classify", "def scopes", "stage-test", "stage-dist", "stage-release"):
        if token not in plan_raw:
            FAILURES.append(f"ci_plan.py: perdeu '{token}' (é a fonte de verdade da regra)")
    if "ci_plan.py" not in ci_raw:
        FAILURES.append("ci: o job `plan` precisa chamar scripts/ci_plan.py")
    if "codeql-action" in ci_raw:
        FAILURES.append(
            "ci: CodeQL não pode ser configurado aqui — o repo tem o setup PADRÃO ligado "
            "e o GitHub recusa SARIF de configuração avançada enquanto ele estiver ativo"
        )
    # Se a regra voltar a ser `if:` espalhada, os dois lugares divergem — e o
    # lugar que diverge é sempre o que ninguém lê.
    if "startsWith(github.ref, 'refs/heads/release" in ci_raw or "startsWith(github.ref, 'refs/heads/hotfix" in ci_raw:
        FAILURES.append("ci: regra branch → tipos de job NÃO pode voltar como `if:` no YAML (fonte única é ci_plan.py)")

    # ── 6. versionamento e dist por evento, nunca por push/PR ────────────
    # A main só recebe o sha de uma tag v*; como o GitHub não deixa expressar
    # isso como regra em repo pessoal, o guard transforma a regra em ALARME.
    guard = jobs.get("main-guard") or {}
    guard_if = str(guard.get("if") or "")
    if "refs/heads/main" not in guard_if:
        FAILURES.append("ci: `Release · guarda da main` precisa rodar em push na main")
    guard_run = " ".join(str(s.get("run") or "") for s in (guard.get("steps") or []))
    if "--points-at" not in guard_run:
        FAILURES.append("ci: `Release · guarda da main` precisa conferir se o sha é o de uma tag v*")

    for job_id in ("release-rc", "release-promote", "release-play"):
        job_if = str((jobs.get(job_id) or {}).get("if") or "")
        if "workflow_dispatch" not in job_if:
            FAILURES.append(f"ci: {job_id} precisa ser só por workflow_dispatch (versionar no push é tag prematura)")
    for job_id in ("dist-ui", "dist-cli"):
        job_if = str((jobs.get(job_id) or {}).get("if") or "")
        if "stage-dist" not in job_if:
            FAILURES.append(f"ci: {job_id} precisa passar pelo stage-dist do plano (artefato só na tag)")
        if (jobs.get(job_id) or {}).get("permissions", {}).get("contents") != "write":
            FAILURES.append(f"ci: {job_id} precisa de permissions.contents: write (anexa na Release)")

    # ── 7. fechamento de issue só em PR MERGEADO ─────────────────────────
    close = jobs.get("close-issues") or {}
    close_if = str(close.get("if") or "")
    for guard in ("pull_request", "closed", "merged == true", "github-actions[bot]"):
        if guard not in close_if:
            FAILURES.append(f"ci: `8 close` perdeu o guard '{guard}' (PR fechado sem merge NÃO fecha issue)")
    close_run = " ".join(str(s.get("run") or "") for s in (close.get("steps") or []))
    for token in ("achados-security", "GITHUB_STEP_SUMMARY"):
        if token not in close_run:
            FAILURES.append(f"ci: `8 close` perdeu '{token}'")

    # ── 8. métricas SEMPRE (mesmo com falha) ─────────────────────────────
    metrics_if = str((jobs.get("metrics") or {}).get("if") or "")
    if "always()" not in metrics_if:
        FAILURES.append("ci: `6 metrics` precisa de if: always() (o resumo é onde se vê o que quebrou)")
    metrics_needs = (jobs.get("metrics") or {}).get("needs")
    needs = metrics_needs if isinstance(metrics_needs, list) else [metrics_needs]
    if "plan" not in [n for n in needs if n]:
        FAILURES.append("ci: `6 metrics` precisa depender só de `plan` — um `needs` vermelho atrasa o resumo")

    # ── 9. as duas imagens ───────────────────────────────────────────────
    images_job = jobs.get("ci-images") or {}
    if (images_job.get("permissions") or {}).get("packages") != "write":
        FAILURES.append("ci: `Infra · imagens de container` precisa de packages: write (push no ghcr.io)")
    if "pull_request" not in str(images_job.get("if") or ""):
        FAILURES.append(
            "ci: `Infra · imagens de container` não pode publicar em PR — a imagem é infraestrutura "
            "versionada, e publicar de um PR colocaria código não revisado no ghcr.io"
        )
    image_matrix = str((images_job.get("strategy") or {}).get("matrix") or "")
    for image in ("ci-linux", "ci-base"):
        if image not in raw(".github/workflows/ci.yml"):
            FAILURES.append(f"ci: imagem {image} não está na matriz de `Infra · imagens de container`")
    if image_matrix.count("Dockerfile") < 2:
        FAILURES.append("ci: `Infra · imagens de container` precisa das DUAS imagens (matrix com 2 Dockerfiles)")

    tauri_docker = raw(".github/docker/ci-linux/Dockerfile")
    for tool in ("tauri-driver", "libwebkit2gtk-4.1-dev", "webkit2gtk-driver", "xvfb", "pnpm fetch"):
        if tool not in tauri_docker:
            FAILURES.append(f"ci-linux/Dockerfile: perdeu '{tool}' (toolset do Tauri/WebKit)")
    base_docker = raw(".github/docker/ci-base/Dockerfile")
    if not base_docker:
        FAILURES.append("ci-base/Dockerfile ausente")
    else:
        for token in ("uv", "rust", "node"):
            if token not in base_docker:
                FAILURES.append(f"ci-base/Dockerfile: precisa de {token} (jobs sem GUI também rodam Python e pnpm)")
        # Comentário NÃO é instrução: os arquivos explicam POR QUE não levam
        # WebKitGTK, e mencionar o pacote no texto não o instala.
        instructions = "\n".join(
            line for line in base_docker.splitlines() if not line.lstrip().startswith("#")
        )
        for forbidden in ("libwebkit2gtk", "xvfb", "webkit2gtk-driver"):
            if forbidden in instructions:
                FAILURES.append(
                    f"ci-base/Dockerfile: instala '{forbidden}', que é da ci-linux — aqui é peso morto (e ~1,5 GB)"
                )

    # Só o diff não publica: a run que introduz a imagem pode morrer ANTES do
    # push (foi o que travou este repo — a imagem nunca existiu e todo job que
    # a consome ficou em `manifest unknown` para sempre, porque nenhum diff
    # seguinte tocava `.github/docker`). A sonda no registro é o que torna o
    # bootstrap auto-curativo, e a verificação pós-push é o que impede o
    # "verde que não publicou".
    images_run = "\n".join(
        str(s.get("run") or "") for s in (images_job.get("steps") or [])
    )
    if "check_base_images.py" not in images_run:
        FAILURES.append(
            "ci: `Infra · imagens de container` precisa rodar `check_base_images.py` antes do build — "
            "tag invalida no FROM so aparece DEPOIS de o runner subir camadas, e sem nome de arquivo"
        )
    if "manifest inspect" not in images_run:
        FAILURES.append(
            "ci: `Infra · imagens de container` precisa sondar a tag no ghcr.io (`docker manifest inspect`) — "
            "decidir só pelo diff trava o bootstrap quando a imagem some do registro"
        )
    for consumer in ("test-spec", "sec-audit", "test-e2e-webview"):
        needs_raw = (jobs.get(consumer) or {}).get("needs")
        needs_list = needs_raw if isinstance(needs_raw, list) else [needs_raw]
        if "ci-images" not in [n for n in needs_list if n]:
            FAILURES.append(
                f"ci: '{consumer}' consome a imagem mas não depende de `ci-images` — roda em paralelo "
                "e morre em `manifest unknown` na MESMA run em que a imagem é publicada"
            )

    # `container.image` NÃO aceita o contexto `env` (a run morre no LOAD com
    # 0 jobs e mensagem genérica). A ref é escrita literal; o `env` do topo
    # documenta a fonte, e este check é o que impede as duas de divergirem.
    wf_env = {k: str(v) for k, v in (ci.get("env") or {}).items()}
    for job_id, env_key in (("test-spec", "CI_IMAGE_BASE"), ("sec-audit", "CI_IMAGE_BASE"),
                            ("test-e2e-webview", "CI_IMAGE_TAURI")):
        image = str(((jobs.get(job_id) or {}).get("container") or {}).get("image") or "")
        if "env." in image:
            FAILURES.append(
                f"ci: '{job_id}' usa ${{{{ env.* }}}} em container.image — contexto inválido ali "
                "(a run morre no LOAD); escreva a ref literal"
            )
        expected = wf_env.get(env_key, "").replace("${{ github.repository }}", "<repo>")
        actual = image.replace("${{ github.repository }}", "<repo>")
        if expected != actual:
            FAILURES.append(f"ci: '{job_id}' usa {actual!r} mas o env {env_key} diz {expected!r} — diverge")

    # ── 10. composites: as alavancas de custo ───────────────────────────
    composites = {
        ".github/actions/setup-rust/action.yml": ("shared-key", "cache-workspace-crates", "GP100_CI_IMAGE"),
        ".github/actions/setup-node-pnpm/action.yml": ("GP100_CI_IMAGE", "npm_config_store_dir", "install"),
        ".github/actions/build-front/action.yml": ("pnpm build",),
        ".github/actions/playwright-setup/action.yml": ("actions/cache", "ms-playwright", "--with-deps"),
        ".github/actions/version-bump/action.yml": ("next-version", "release", "bump"),
    }
    for path, tokens in composites.items():
        text = raw(path)
        if not text:
            FAILURES.append(f"{path}: action ausente")
            continue
        for token in tokens:
            if token not in text:
                FAILURES.append(f"{path}: perdeu '{token}'")
    if "add-job-id-key" not in raw(".github/actions/setup-rust/action.yml"):
        FAILURES.append("setup-rust: shared-key exige add-job-id-key: false (senão a chave volta a ser por job)")

    # ── 11. geração Node 20 (o runner força Node 24 e avisa) ────────────
    scan = workflows + sorted(
        p[len(REPO) + 1:].replace("\\", "/") for p in glob.glob(os.path.join(REPO, ".github/actions/*/action.yml"))
    )
    for path in scan:
        text = raw(path)
        for ref in NODE20_REFS:
            if ref in text:
                FAILURES.append(f"{path}: '{ref}' mira Node 20 (deprecado) — subir a major")

    # ── relatório ───────────────────────────────────────────────────────
    for path in sorted(docs):
        print(("ERR " if path == ".github/workflows/ci.yml" and FAILURES else "OK  "), path)
    if FAILURES:
        print("\nFALHAS:")
        for failure in FAILURES:
            print(" -", failure)
        return 1
    print("\n1 workflow + 5 actions compostas + 2 imagens: YAML válido e contratos preservados.")
    return 0


if __name__ == "__main__":
    sys.exit(main())