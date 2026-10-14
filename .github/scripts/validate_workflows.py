#!/usr/bin/env python3
"""
Validação sintática + de CONTRATO do pipeline (gate do CI, job `Lint · contratos do pipeline`).

Por que este arquivo existe: o YAML do GitHub aceita muita coisa que só quebra
em produção — um `permissions` que o reusable pede e o caller não concede morre
no LOAD da run (mensagem só na annotation), uma matriz vazia faz a run INTEIRA
falhar sem nenhum job marcado como falha, um token que não sobe de permissão
para `write` falha no meio do job. Nenhuma dessas dá erro de sintaxe.

Os testes de INTEGRAÇÃO reais acontecem onde o workflow é a própria prova: o
fechamento de issues roda no merge, e `.github/scripts/simulate_release.py` executa de
 verdade os blocos `run:` da rc/promote num sandbox git. Aqui travamos o que é
estrutura — e, desde a #68, que a ESTRUTURA nova é o que o projeto adoptou.Contratos verificados:
  1. existem EXATAMENTE TRÊS workflows (.github/workflows/ci.yml +
     issues-fechamento.yml + visual-baselines.yml) e os cinco legados sumiram
     (#169 tirou o Fechamento do CI; o #204 tirou a regeneração de baseline
     do dispatch do ci.yml);
  2. gatilhos: push SO em develop/main + tag v*; PR em develop/main SEM o tipo
     `closed` (ele agora mora no issues-fechamento.yml — arrastá-lo no CI
     rodava o pipeline inteiro só para jobs `skipped`). Branch de trabalho
     entra so por `pull_request` -- listar os dois eventos era o que gerava a
     run duplicada a cada push;
  3. permissão MÍNIMA no topo (`contents: read`) — o `write` é por job;
  4. os 10 tipos de job e os 25 jobs existem, com nome no padrão `Tipo · o que é`,
     sem prefixo numérico, e declarados na sequência da execução;
  5. a regra branch → tipos de job mora em `.github/scripts/ci_plan.py` e NÃO volta para
     `if:` espalhado no YAML (duas fontes de verdade é como o gate divergiu);
     e o CodeQL NÃO é configurado aqui (o setup padrão do repo está ligado e o
     GitHub recusa SARIF de configuração avançada enquanto ele estiver ativo);
  6. versionamento e distribuição só por `workflow_dispatch`/tag;
  7. o `close-issues` (workflow próprio) exige `merged == true` (PR fechado sem
     merge não fecha nada);
  8. o job `Relatório` roda com `if: always()` e depende de TODOS os jobs do
     CI — é o ÚLTIMO da run (#169);
  9. as duas imagens: ci-linux (toolset do Tauri) e ci-base (sem GUI, com uv);
  10. composites preservados (shared-key, add-job-id-key, cache do Playwright);
  11. PISO de versao por action (o Node 20 foi descontinuado pelo runner):
     toda action usada esta no mapa e na major minima; entrada sem uso
     tambem falha, senao a regra vira letra morta sem ninguem perceber.
  12. o GRAFO `needs:` na ordem do dono (#169): lint → compilação → segurança →
      testes → cobertura → dist; e a matriz de OS tem o archlinux como
      CONTAINER (`matrix.host`/`matrix.container` em toda job de matriz);
  13. guard EXPLÍCITO (`always()` + `needs.<x>.result != 'failure'`) em toda job
      que depende de um upstream que pode ser skipped (sec-audit, test-ui) — o
      `success()` implícito pula a fase inteira de testes, provado na run
      38168716758 (actions/runner#2205);
  14. os workflows REUTILIZÁVEIS de Distribuição e Release (#245, achado F-12 da
      auditoria de 14/10): um job por arquivo, gatilho só `workflow_call`, sem
      `permissions` (as do caller é que valem) e sem `steps:` no caller; o
      `ci.yml` mantém o grafo e o TETO de linhas — o corpo pesado não volta;
  15. o `simulate_release` (LENTO) roda por ESCOPO (#246, F-13): o `if` do passo
      lê o `scope-ci` do `ci_plan.py`, e a identidade do bot
      (`BOT_NAME`/`BOT_EMAIL`) é idêntica nos três arquivos de release.

Uso: python3 .github/scripts/validate_workflows.py  (exit 1 na primeira violação)
"""
from __future__ import annotations

import glob
import os
import re
import sys

import yaml

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
FAILURES: list[str] = []


class _LoaderSemChaveDuplicada(yaml.SafeLoader):
    """O PyYAML aceita chave repetida (a última vence em silêncio); o GitHub
    RECUSA no LOAD — a run nasce com 0 jobs, conclusion=failure e sem log. Já
    aconteceu (#169: `dist-sdist` ficou com DOIS `needs:` depois do patch do
    grafo, e o gate local passou). Este loader transforma o silêncio em erro.
    """

    def construct_mapping(self, node, deep=False):  # noqa: ANN001 — assinatura do PyYAML
        chaves = [self.construct_object(k, deep=deep) for k, _ in node.value]
        repetidas = sorted({c for c in chaves if isinstance(c, str) and chaves.count(c) > 1})
        if repetidas:
            raise yaml.YAMLError(
                f"chave(s) duplicada(s) {repetidas} na linha {node.start_mark.line + 1} "
                "— o GitHub recusa no LOAD (run com 0 jobs e sem log)"
            )
        return super().construct_mapping(node, deep)


def load(path: str) -> dict | None:
    try:
        with open(os.path.join(REPO, path), encoding="utf-8") as fh:
            data = yaml.load(fh, Loader=_LoaderSemChaveDuplicada)
        return data if isinstance(data, dict) else {}
    except Exception as exc:  # noqa: BLE001 — reportar qualquer YAML quebrado
        FAILURES.append(f"{path}: YAML inválido ({exc})")
        return None


def sem_comentarios(texto: str) -> str:
    """O texto sem as linhas de comentário — comentário NÃO é instrução.

    O `ci.yml` cita os arquivos extraídos em comentário de propósito (para o
    próximo agente achar o corpo), e citar `make_sdist.py` num comentário não
    pode valer como "o corpo voltou". Mesma regra do `ci-base/Dockerfile`.
    """
    return "\n".join(
        linha for linha in texto.splitlines() if not linha.lstrip().startswith("#")
    )


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
# A ordem do dicionário É a sequência de execução pedida (#169: validação →
# lint → compilação → infra → segurança → testes → cobertura → distribuição →
# release → relatório). O GitHub não tem `stages:`; a ordem real é o grafo
# `needs:`. Então a sequência é garantida aqui como ORDEM DE DECLARAÇÃO no
# arquivo (que é como a gente lê e como o diff mostra), e o `needs:` (contrato
# 12) garante que ela é real. O Fechamento saiu daqui: é o workflow próprio
# `.github/workflows/issues-fechamento.yml`.
TIPOS: dict[str, list[str]] = {
    "Validação": ["plan"],
    "Lint": ["lint-workflows", "lint-commits", "lint-rust-fmt", "lint-rust-clippy", "lint-ui"],
    "Compilação": ["build-rust", "build-ui"],
    "Infra": ["ci-images"],
    "Segurança": ["sec-audit"],
    "Testes": ["test-rust", "test-ui", "test-spec", "test-e2e", "test-e2e-visual", "test-e2e-webview"],
    "Cobertura": ["coverage-ui"],
    # `dist-sdist` publica o tarball de FONTE (com o `ui/dist` embutido) de que
    # o PKGBUILD do Arch precisa (#72). Fica entre o instalador e o CLI porque é
    # artefato de distribuição — não gate, não teste.
    "Distribuição": ["dist-ui", "dist-sdist", "dist-cli"],
    "Release": ["main-guard", "release-rc", "release-promote", "release-play"],
    "Relatório": ["metrics"],
}
SEQUENCIA_ESPERADA = [j for jobs_ in TIPOS.values() for j in jobs_]

# O GRAFO `needs:` NA ORDEM DO DONO (#169). Posição no arquivo não executa;
# `needs:` executa. Este mapa é a política transformada em igualdade de
# conjunto: mudar a ordem sem mudar a política = atualizar aqui, e o diff do
# gate é a revisão da mudança.
GRAFO_ESPERADO: dict[str, set[str]] = {
    # Lint ← Validação (o `plan` é o único que nunca pula)
    "lint-workflows": {"plan"},
    "lint-commits": {"plan"},
    "lint-rust-fmt": {"plan"},
    "lint-rust-clippy": {"plan"},
    "lint-ui": {"plan"},
    # Compilação ← Lint
    "build-rust": {"plan", "lint-rust-fmt", "lint-rust-clippy"},
    "build-ui": {"plan", "lint-ui"},
    # Segurança ← Compilação/Infra — ANTES dos testes (#169 moveu para cá; o
    # `if: always()` do sec-audit é o que deixa os build-* skipped não bloquearem)
    "sec-audit": {"plan", "ci-images", "build-rust", "build-ui"},
    # Testes ← Segurança (+ build da área, + imagem quando consomem)
    "test-rust": {"plan", "build-rust", "sec-audit"},
    "test-ui": {"plan", "build-ui", "sec-audit"},
    "test-e2e": {"plan", "build-ui", "sec-audit"},
    "test-e2e-visual": {"plan", "build-ui", "sec-audit"},
    "test-spec": {"plan", "ci-images", "sec-audit"},
    "test-e2e-webview": {"plan", "build-rust", "ci-images", "sec-audit"},
    # Cobertura ← Testes
    "coverage-ui": {"plan", "build-ui", "test-ui"},
    # Distribuição ← Cobertura (UI) ou do teste do artefato (CLI, sem coverage)
    "dist-ui": {"plan", "coverage-ui"},
    "dist-sdist": {"plan", "coverage-ui"},
    "dist-cli": {"plan", "test-rust"},
    # Release ← Validação (dist nunca coexiste com dispatch — ver o comentário
    # do bloco de Release no ci.yml)
    "main-guard": {"plan"},
    "release-rc": {"plan"},
    "release-promote": {"plan"},
    "release-play": {"plan"},
}

# ── OS WORKFLOWS REUTILIZÁVEIS (#245, achado F-12) ───────────────────────────
#
# Distribuição e Release saíram do `ci.yml` (1777 linhas / 29 jobs) para
# arquivos com `workflow_call`. Valor da tabela: arquivo → job do `ci.yml` que o
# chama. As regras que o gate trava aqui valem para TODOS eles:
#   * UM job por arquivo — o check de um job reutilizável aparece como
#     `<job do caller> / <job do chamado>`, então um arquivo com os três jobs
#     guardados por input faria a run ganhar `skipped` que não existiam;
#   * gatilho SÓ `workflow_call` — push/pull_request aqui rodaria a
#     distribuição/release FORA do grafo do `ci.yml`;
#   * `permissions` NUNCA declarado aqui: os jobs recebem as do job que chama
#     (o `contents: write` do caller é o que publica na Release). Declarar dos
#     dois lados cria uma segunda política — e a que vale é a interseção, que
#     ninguém lê.
REUSABLE: dict[str, str] = {
    ".github/workflows/dist-instalador.yml": "dist-ui",
    ".github/workflows/dist-sdist.yml": "dist-sdist",
    ".github/workflows/dist-cli.yml": "dist-cli",
    ".github/workflows/release-rc.yml": "release-rc",
    ".github/workflows/release-promote.yml": "release-promote",
    ".github/workflows/release-play.yml": "release-play",
}

# O TETO do `ci.yml` (#245). O número não é estético: são 29 jobs e 1777 linhas
# ANTES da extração, e o objetivo dela é o arquivo voltar a ser o GRAFO (o grafo
# + a validação, medidos hoje, são ~1350). Sem teto, o próximo bloco pesado
# reentra no arquivo e ninguém vê o tamanho voltar.
CI_MAX_LINHAS = 1400

# O CORPO pesado que NÃO pode voltar para o `ci.yml` (comentário não conta — a
# extração é citada em comentário de propósito). Cada token é a assinatura de um
# bloco que hoje vive em um reutilizável.
CORPO_NO_REUSABLE = (
    ("tauri build --features real-device", ".github/workflows/dist-instalador.yml"),
    ("make_sdist.py", ".github/workflows/dist-sdist.yml"),
    ("sync_version.py --set", ".github/workflows/release-*.yml"),
    ("softprops/action-gh-release", ".github/workflows/dist-*.yml"),
)

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

# Piso de versão por action. O GitHub descontinuou o runtime Node 20 (set/2025):
# o runner força Node 24 e emite aviso em TODA run. O contrato é um PISO, e não
# uma lista de versões ruins -- assim uma versão que ninguém catalogou ainda
# falha em vez de passar, e subir o piso é uma edição de uma linha.
# Referência: https://github.blog/changelog/2025-09-19-deprecation-of-node-20-on-github-actions-runners/
PISO_VERSAO_ACTION = {
    "actions/cache": 6,
    "actions/checkout": 7,
    "actions/download-artifact": 7,
    "actions/setup-node": 7,
    "actions/upload-artifact": 7,
    "Swatinem/rust-cache": 2,
    "astral-sh/setup-uv": 10,
    "awalsh128/cache-apt-pkgs-action": 1,
    "docker/build-push-action": 7,
    "docker/login-action": 4,
    "docker/metadata-action": 6,
    "docker/setup-buildx-action": 4,
    "softprops/action-gh-release": 3,
    "taiki-e/install-action": 2,
}


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

    # ── 1. DOIS workflows (ci.yml + fechamento), e os legados fora ────────
    for legacy in LEGACY:
        if f".github/workflows/{legacy}" in docs:
            FAILURES.append(f".github/workflows/{legacy}: legado ainda existe (a #68 consolidou tudo em workflows próprios)")
    esperados = {
        ".github/workflows/ci.yml",
        ".github/workflows/issues-fechamento.yml",
        ".github/workflows/visual-baselines.yml",
        *REUSABLE,
    }
    if set(workflows) != esperados:
        FAILURES.append(f"esperado exatamente 3 workflows de pipeline + {len(REUSABLE)} reutilizáveis {sorted(esperados)}; existem: {workflows}")

    ci = docs.get(".github/workflows/ci.yml") or {}
    if not ci:
        # `load()` já registrou o motivo REAL (ex.: chave duplicada); imprimir
        # só "ci.yml ausente" escondeu o diagnóstico e custou uma rodada de CI.
        print("FALHAS:")
        for failure in FAILURES or ["ci.yml ausente"]:
            print(" -", failure)
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
    if "closed" in (pr.get("types") or []):
        FAILURES.append(
            "ci: on.pull_request.types NÃO pode ter 'closed' — o fechamento de issues saiu do CI (#169) e "
            "mora em issues-fechamento.yml; arrastá-lo aqui rodava o pipeline inteiro só para jobs skipped"
        )
    if "schedule" not in on:
        FAILURES.append("ci: precisa de on.schedule (auditoria noturna)")

    # ── 2b. o workflow PRÓPRIO do Fechamento (#169) ──────────────────────
    fech = docs.get(".github/workflows/issues-fechamento.yml") or {}
    if fech:
        if fech.get("name") != "Issues · Fechamento de issues vinculadas":
            FAILURES.append(
                f"fechamento: name precisa ser 'Issues · Fechamento de issues vinculadas' (pedido na #169); é {fech.get('name')!r}"
            )
        fech_pr = (triggers(fech).get("pull_request") or {})
        if "closed" not in (fech_pr.get("types") or []):
            FAILURES.append("fechamento: precisa do gatilho pull_request.types = [closed]")
        for needed in ("develop", "main"):
            if needed not in set(fech_pr.get("branches") or []):
                FAILURES.append(f"fechamento: on.pull_request.branches precisa de {needed}")
        if (fech.get("permissions") or {}).get("issues") != "write":
            FAILURES.append("fechamento: permissions.issues precisa ser 'write' (fecha as issues)")
        fech_close = (fech.get("jobs") or {}).get("close-issues") or {}
        fech_if = str(fech_close.get("if") or "")
        for guard in ("pull_request", "closed", "merged == true", "github-actions[bot]"):
            if guard not in fech_if:
                FAILURES.append(f"fechamento: `close-issues` perdeu o guard '{guard}' (PR fechado sem merge NÃO fecha issue)")
        fech_run = " ".join(str(s.get("run") or "") for s in (fech_close.get("steps") or []))
        for token in ("achados-security", "GITHUB_STEP_SUMMARY"):
            if token not in fech_run:
                FAILURES.append(f"fechamento: `close-issues` perdeu '{token}'")

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

    # ── 4b. os REUTILIZÁVEIS de Distribuição/Release e os seus callers (#245) ─
    ci_raw = raw(".github/workflows/ci.yml")
    for path, caller in REUSABLE.items():
        doc_r = docs.get(path)
        if not doc_r:
            FAILURES.append(
                f"{path}: workflow reutilizável ausente — o corpo do job `{caller}` tem de viver aqui (#245)"
            )
            continue
        gatilhos_r = set(triggers(doc_r)) - {None}
        if gatilhos_r != {"workflow_call"}:
            FAILURES.append(
                f"{path}: gatilho precisa ser SÓ workflow_call (tem {sorted(str(g) for g in gatilhos_r)}) — "
                "um push/pull_request aqui roda a distribuição/release FORA do grafo do `ci.yml`"
            )
        jobs_r = doc_r.get("jobs") or {}
        if len(jobs_r) != 1:
            FAILURES.append(
                f"{path}: precisa de EXATAMENTE um job (tem {sorted(jobs_r)}) — o check de um job reutilizável "
                "aparece como `<caller> / <job>`; com N jobs guardados por input a run ganha N-1 `skipped` novos"
            )
        if doc_r.get("permissions"):
            FAILURES.append(
                f"{path}: `permissions` não se declara aqui — os jobs recebem as do job que chama "
                "(o `contents: write` do caller é o que publica); dos dois lados viram duas políticas"
            )
        job_caller = jobs.get(caller) or {}
        chamada = str(job_caller.get("uses") or "")
        if chamada != f"./{path}":
            FAILURES.append(f"ci: `{caller}` precisa de `uses: ./{path}`; veio {chamada!r}")
        if job_caller.get("steps"):
            FAILURES.append(
                f"ci: `{caller}` é um caller de workflow — não pode ter `steps:` (o corpo vive em {path}); "
                "`runs-on`/`timeout-minutes`/`env`/`outputs` também não são aceitos num job que usa `uses:`"
            )
        for chave in ("runs-on", "timeout-minutes", "steps", "env", "outputs"):
            if chave in job_caller:
                FAILURES.append(
                    f"ci: `{caller}` não pode declarar '{chave}' — num job que chama um workflow reutilizável "
                    f"esse valor pertence a {path}"
                )
        # input obrigatório sem valor = a run morre no LOAD, com mensagem só na annotation
        call_r = triggers(doc_r).get("workflow_call") or {}
        if not isinstance(call_r, dict):
            call_r = {}
        exigidos = {
            nome
            for nome, spec in (call_r.get("inputs") or {}).items()
            if isinstance(spec, dict) and spec.get("required")
        }
        passados = set(job_caller.get("with") or {})
        faltando = exigidos - passados
        if faltando:
            FAILURES.append(
                f"ci: `{caller}` não passa {sorted(faltando)} para {path} — input obrigatório vazio recusa a run no LOAD"
            )
        sobrando = passados - set((call_r.get("inputs") or {}))
        if sobrando:
            FAILURES.append(
                f"ci: `{caller}` passa {sorted(sobrando)} e {path} não declara esse input (a run é recusada no LOAD)"
            )

    # O CORPO não volta: os blocos extraídos só podem reaparecer como comentário,
    # que é como o `ci.yml` aponta para o arquivo (e é o que o teto de linhas
    # existe para impedir de crescer de novo).
    ci_sem_comentario = sem_comentarios(ci_raw)
    for token, destino in CORPO_NO_REUSABLE:
        if token in ci_sem_comentario:
            FAILURES.append(
                f"ci: o corpo '{token}' voltou para o `ci.yml` — ele vive em {destino} (#245); "
                "o caller só mantém nome, grafo e matriz"
            )
    linhas_ci = ci_raw.count("\n") + 1
    if linhas_ci > CI_MAX_LINHAS:
        FAILURES.append(
            f"ci: {linhas_ci} linhas (teto {CI_MAX_LINHAS}) — o `ci.yml` é grafo + validação; "
            "um bloco pesado novo vai para um workflow reutilizável (#245)"
        )

    declarados = list(jobs)
    if declarados != SEQUENCIA_ESPERADA:
        FAILURES.append(
            "ci: ordem de declaração fora da sequência "
            f"({' → '.join(SEQUENCIA_ESPERADA)}); veio {(' → '.join(declarados))}"
        )

    # ── 5. a REGRA branch → tipos de job mora no ci_plan.py, e só lá ──────────
    plan_raw = raw(".github/scripts/ci_plan.py")
    for token in ("def classify", "def scopes", "stage-test", "stage-dist", "stage-release"):
        if token not in plan_raw:
            FAILURES.append(f"ci_plan.py: perdeu '{token}' (é a fonte de verdade da regra)")

    # ── 5b. o GATE LENTO RODA POR ESCOPO (#246, achado F-13) ─────────────
    #
    # `simulate_release` executa a cadeia rc→promote de verdade (~2 min) e
    # protegia uma cadeia que muda raramente em TODO PR. A regra é do
    # `ci_plan.py` (`CI_PATHS` → `scope-ci`) e o único lugar que a lê é o `if` do
    # passo. A checagem existe para o `if` não voltar a ser `true` fixo (o que
    # devolveria o custo em silêncio) nem o escopo desaparecer do plano.
    if "CI_PATHS" not in plan_raw or "scope-ci" not in plan_raw:
        FAILURES.append(
            "ci_plan.py: perdeu 'CI_PATHS'/'scope-ci' — é o escopo que governa o gate `simulate_release` "
            "(#246: a cadeia de release é provada onde ela pode quebrar, não em todo PR)"
        )
    passo_sim = next(
        (
            step
            for step in ((jobs.get("lint-workflows") or {}).get("steps") or [])
            if "simulate_release.py" in str(step.get("run") or "")
        ),
        None,
    )
    if passo_sim is None:
        FAILURES.append(
            "ci: `lint-workflows` perdeu o passo que roda `simulate_release.py` — é a prova da cadeia rc→promote"
        )
    elif "scope-ci" not in str(passo_sim.get("if") or ""):
        FAILURES.append(
            "ci: o passo do `simulate_release.py` precisa de `if: needs.plan.outputs.scope-ci == 'true'` "
            "(#246) — sem ele o gate de ~2 min volta a rodar em todo PR"
        )

    # ── 5c. a IDENTIDADE DO BOT é a mesma nos três reutilizáveis (#245) ──
    #
    # `env:` de topo NÃO atravessa um `workflow_call` e o `env` não é contexto
    # aceito no `with:`, então `BOT_NAME`/`BOT_EMAIL` são repetidos nos arquivos
    # que commitam. Repetição de literal só é segura com contrato que trava a
    # igualdade — o mesmo padrão da ref de imagem de container.
    identidade: dict[str, set[str]] = {"BOT_NAME": set(), "BOT_EMAIL": set()}
    for path in REUSABLE:
        env_r = (docs.get(path) or {}).get("env") or {}
        for chave in identidade:
            if chave in env_r:
                identidade[chave].add(str(env_r[chave]))
    for chave, valores in identidade.items():
        if len(valores) != 1:
            FAILURES.append(
                f"{chave} divergente entre os reutilizáveis de release: {sorted(valores)} — "
                "o commit do bot e o guard do fechamento dependem desse nome"
            )
    if "BOT_NAME" in (ci.get("env") or {}):
        FAILURES.append(
            "ci: `BOT_NAME` no env de topo do `ci.yml` ficou sem consumidor depois da extração (#245) — "
            "a identidade do bot vive nos arquivos que commitam"
        )
    if "ci_plan.py" not in ci_raw:
        FAILURES.append("ci: o job `plan` precisa chamar .github/scripts/ci_plan.py")
    if "codeql-action" in ci_raw:
        FAILURES.append(
            "ci: CodeQL não pode ser configurado aqui — o repo tem o setup PADRÃO ligado "
            "e o GitHub recusa SARIF de configuração avançada enquanto ele estiver ativo"
        )
    # Se a regra voltar a ser `if:` espalhada, os dois lugares divergem — e o
    # lugar que diverge é sempre o que ninguém lê.
    if "startsWith(github.ref, 'refs/heads/release" in ci_raw or "startsWith(github.ref, 'refs/heads/hotfix" in ci_raw:
        FAILURES.append("ci: regra branch → tipos de job NÃO pode voltar como `if:` no YAML (fonte única é ci_plan.py)")

    # TAG → CHANGED=ALL no passo do diff (#228). A tag publica a ÁRVORE, não
    # um diff: numa tag MOVIDA o `before` é um SHA válido e o diff pode sair
    # vazio — `CHANGED=(none)` → escopos false → testes/cobertura/dist
    # `skipped` → run VERDE com a tag sem instalador e sem Release (run
    # 38508338840). O `ci_plan.py` já marca `tag` com suíte + dist; o gate
    # abaixo impede que um "enxugar" futuro devolva a tag ao ramo do diff.
    diff_step = next(
        (str(step.get("run") or "")
         for step in ((jobs.get("plan") or {}).get("steps") or [])
         if step.get("id") == "base"),
        "",
    )
    if "refs/tags/" not in diff_step or 'CHANGED="ALL"' not in diff_step:
        FAILURES.append(
            "ci: o passo 'Diff contra a base do evento' precisa forçar CHANGED=ALL em ref de tag "
            "(#228 — tag movida tem `before` válido e diff vazio, e a distribuição some em silêncio)"
        )

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
    # A checagem saiu daqui para o contrato 2b: o `close-issues` não é mais um
    # job do ci.yml — é o workflow `.github/workflows/issues-fechamento.yml`.
    if "close-issues" in jobs:
        FAILURES.append("ci: `close-issues` voltou para o ci.yml — o Fechamento é workflow próprio (#169)")

    # ── 8. métricas SEMPRE (mesmo com falha) ─────────────────────────────
    metrics_if = str((jobs.get("metrics") or {}).get("if") or "")
    if "always()" not in metrics_if:
        FAILURES.append("ci: `6 metrics` precisa de if: always() (o resumo é onde se vê o que quebrou)")
    # O Relatório é o ÚLTIMO job (#169): precisa depender de todos os outros
    # para resumir a run INTEIRA. O `if: always()` é o que garante que ele
    # roda mesmo quando algum desses `needs` vermelhou.
    metrics_needs = (jobs.get("metrics") or {}).get("needs")
    needs = set(metrics_needs) if isinstance(metrics_needs, list) else {metrics_needs}
    faltando = set(jobs) - {"metrics"} - needs
    if faltando:
        FAILURES.append(
            "ci: `Relatório · resumo da run` precisa depender de TODOS os jobs do CI (#169 — roda por "
            f"último); faltando: {sorted(faltando)}"
        )

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

    # ── 11. piso de versão das actions (Node 20 descontinuado) ──────────
    scan = workflows + sorted(
        p[len(REPO) + 1:].replace("\\", "/") for p in glob.glob(os.path.join(REPO, ".github/actions/*/action.yml"))
    )
    usados = set()
    for path in scan:
        for acao, versao in re.findall(r"uses:\s*([\w.-]+/[\w./-]+?)@v(\d+)", raw(path)):
            usados.add(acao)
            piso = PISO_VERSAO_ACTION.get(acao)
            if piso is None:
                FAILURES.append(
                    f"{path}: action '{acao}' fora do mapa de pisos — declarar a major minima "
                    f"em PISO_VERSAO_ACTION (sem isso a deprecacao volta em silencio)"
                )
            elif int(versao) < piso:
                FAILURES.append(
                    f"{path}: '{acao}@v{versao}' abaixo do piso v{piso} (Node 20 descontinuado) — subir a major"
                )
    for acao in sorted(set(PISO_VERSAO_ACTION) - usados):
        FAILURES.append(
            f"PISO_VERSAO_ACTION declara '{acao}' mas nenhuma action usa — piso obsoleto, "
            f"a regra parada deixa de ser conferida"
        )

    # ── 12. o GRAFO na ordem do dono + Arch na matriz (#169) ─────────────
    for job_id, esperado in GRAFO_ESPERADO.items():
        needs_raw = (jobs.get(job_id) or {}).get("needs")
        conjunto = set(needs_raw) if isinstance(needs_raw, list) else {needs_raw}
        if conjunto != esperado:
            FAILURES.append(
                f"ci: `{job_id}` precisa depender de {sorted(esperado)} (ordem do dono, #169); "
                f"veio {sorted(x for x in conjunto if x)}"
            )
    # Toda job que consome matriz de OS precisa aceitar a entrada `archlinux`
    # como CONTAINER (#169): `matrix.host` no runs-on e `matrix.container` na
    # imagem (expressão vazia = roda direto no runner hospedado). Sem os dois,
    # a entrada archlinux morre no LOAD da run inteira.
    for job_id in ("lint-rust-clippy", "build-rust", "test-rust", "build-ui"):
        job = jobs.get(job_id) or {}
        if str(job.get("runs-on") or "") != "${{ matrix.host || matrix.os }}":
            FAILURES.append(f"ci: `{job_id}` precisa de `runs-on: ${{{{ matrix.host || matrix.os }}}}` (Arch, #169)")
        imagem = str((job.get("container") or {}).get("image") or "")
        if imagem != "${{ matrix.container || '' }}":
            FAILURES.append(f"ci: `{job_id}` precisa de `container.image: ${{{{ matrix.container || '' }}}}` (Arch, #169)")
    for token in ("archlinux:base-devel", "ARCH_OS", "ARCH_CONTAINER"):
        if token not in plan_raw:
            FAILURES.append(f"ci_plan.py: perdeu '{token}' — a matriz de OS não tem mais o archlinux (#169)")

    # ── 13. guard EXPLÍCITO onde a cadeia tem job que pode ser skipped ───
    # O `success()` IMPLÍCITO pula o job quando QUALQUER need foi skipped —
    # mesmo todos os outros verdes (run 38168716758: `sec-audit` verde e a
    # fase de testes INTEIRA skipped; actions/runner#2205). Na prática quem
    # dispara isso é o `ci-images` skipped em PR, que o `sec-audit` consome.
    # Então toda job que depende de um upstream sujeito a skip precisa do
    # `always()` + `needs.<x>.result != 'failure'` na cara — é o padrão que o
    # `test-spec`/`test-e2e-webview` já usavam.
    GUARDS_EXPLICITOS = {
        "sec-audit": "needs.sec-audit.result != 'failure'",
        "test-ui": "needs.test-ui.result != 'failure'",
    }
    for job_id, spec in jobs.items():
        # O Relatório é a Única exceção: ele precisa rodar JUSTAMENTE quando
        # algo falhou (é o lugar onde se lê a falha) — `always()` sem guard.
        if job_id == "metrics":
            continue
        needs_raw = spec.get("needs")
        lista = [n for n in (needs_raw if isinstance(needs_raw, list) else [needs_raw]) if n]
        cond = str(spec.get("if") or "")
        for upstream, guard in GUARDS_EXPLICITOS.items():
            if upstream in lista and ("always()" not in cond or guard not in cond):
                FAILURES.append(
                    f"ci: `{job_id}` depende de `{upstream}` (que pode ser skipped) mas o `if` não tem "
                    f"`always() && {guard}` — o success() implícito pula o job (actions/runner#2205)"
                )

    # ── 14. visual-baselines: regeneração SEM alavanca de release (#204) ──
    # O workflow existe por causa de DUAS runs reais (38070949933 e a do
    # incidente rc.5 de 13/10): um dispatch para regenerar baselines com
    # `action` vazio APLICou o `default: release` do ci.yml e o job
    # `Release · RC` criou branch + tag que não existiam. O arquivo separado
    # resolve por construção — e ESTE contrato é o que impede ele de
    # reengravidar: se um dia alguém acrescentar um input, uma permissão
    # `write` ou um `git push` aqui, o gate falha antes de qualquer dispatch.
    vb = docs.get(".github/workflows/visual-baselines.yml")
    if not vb:
        FAILURES.append("visual-baselines.yml: ausente (a regeneração de baseline voltaria para dentro do ci.yml)")
    else:
        vb_on = triggers(vb)
        gatilhos_vb = set(vb_on) - {None}
        if gatilhos_vb != {"workflow_dispatch"}:
            FAILURES.append(
                "visual-baselines: gatilhos precisam ser SÓ workflow_dispatch "
                f"(encontrado: {sorted(str(g) for g in gatilhos_vb)}) — push/schedule/tag dariam "
                "gatilho automático a uma operação manual de teste"
            )
        vb_inputs = (vb_on.get("workflow_dispatch") or {}).get("inputs") or {}
        if vb_inputs:
            FAILURES.append(
                "visual-baselines: workflow_dispatch não pode ter inputs "
                f"(encontrado: {sorted(vb_inputs)}) — é o input esquecido que causou os incidentes "
                "das runs 38070949933 e rc.5; a ref alvo vem do próprio dispatch"
            )
        vb_perms = vb.get("permissions") or {}
        if vb_perms.get("contents") != "read":
            FAILURES.append(
                "visual-baselines: permissions.contents no topo precisa ser 'read' — a trava "
                "de fábrica que impede commit/tag/push neste workflow"
            )
        vb_raw = raw(".github/workflows/visual-baselines.yml")
        # Proibidos em QUALQUER lugar do arquivo (topo ou job): a segurança é
        # do arquivo inteiro, não de uma declaração que alguém pode mover.
        for proibido in ("contents: write", "git push", "git tag", "action-gh-release", "gh release"):
            if proibido in vb_raw:
                FAILURES.append(
                    f"visual-baselines: contém {proibido!r} — este workflow regenera baseline, "
                    "nunca escreve no repositório nem publica nada"
                )
        # O produto do workflow: o flag de escrita e o artefato que o entrega.
        if "--update-snapshots=all" not in vb_raw:
            FAILURES.append(
                "visual-baselines: sem `--update-snapshots=all` — sem ele o playwright COMPARA "
                "(papel do ci.yml) e o workflow não regenera nada"
            )
        if "visual-snapshots" not in vb_raw:
            FAILURES.append(
                "visual-baselines: sem o artefato `visual-snapshots` — a saída é o produto; "
                "sem artefato o workflow regenera e joga fora"
            )
        if "if-no-files-found: error" not in vb_raw:
            FAILURES.append(
                "visual-baselines: artefato precisa de `if-no-files-found: error` — aqui o "
                "artefato é o produto, e verde sem nada gerado é a mentira mais cara"
            )

    # ── relatório ───────────────────────────────────────────────────────
    for path in sorted(docs):
        print(("ERR " if path == ".github/workflows/ci.yml" and FAILURES else "OK  "), path)
    if FAILURES:
        print("\nFALHAS:")
        for failure in FAILURES:
            print(" -", failure)
        return 1
    print(
        f"\n3 workflows de pipeline + {len(REUSABLE)} reutilizáveis + 5 actions compostas "
        "+ 2 imagens: YAML válido e contratos preservados."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())