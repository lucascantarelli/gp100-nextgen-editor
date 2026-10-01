# 🤝 CONTRIBUTING — Fluxo de contribuição (GP-100 NextGen Editor)

> Vale para humanos **e** agentes. O ciclo completo: **issue → branch → commits →
> PR → CI → merge em develop → issue fecha sozinha**. Nada aqui é teórico: cada
> etapa aponta para o template, workflow ou gate que a executa.

---

## 0. TL;DR — o ciclo em 6 passos

1. **Issue** — escolha um template ([bug](../.github/ISSUE_TEMPLATE/bug.yml) ·
   [feature](../.github/ISSUE_TEMPLATE/feature.yml) ·
   [refactor](../.github/ISSUE_TEMPLATE/refactor.yml) ·
   [epic](../.github/ISSUE_TEMPLATE/epic.yml) ·
   [protocolo](../.github/ISSUE_TEMPLATE/protocolo.yml)) ou pegue uma da
   [FASE ACHADOS](ROADMAP.md) (prioridade máxima).
2. **Branch** GitFlow a partir de `develop` (§2).
3. **Commits** no padrão conventional (§3) — o gate da CI rejeita fora do padrão.
4. **Gates locais** da sua área (§4) — o PR nasce verde.
5. **PR** para `develop` com o [template](../.github/PULL_REQUEST_TEMPLATE.md),
   tabelas preenchidas e `Closes #N` no **corpo** (§5).
6. **CI verde + merge** (squash) → o job `close-linked` do
   [ci.yml](../.github/workflows/ci.yml) fecha a issue com comentário de
   rastreabilidade.

---

## 1. Issues — templates e automações

| Template | Quando usar | Ponto-chave |
|---|---|---|
| 🐞 [Bug](../.github/ISSUE_TEMPLATE/bug.yml) | algo não funciona | rode `uv run pytest` e `cargo test` antes; cole a saída |
| ✨ [Feature](../.github/ISSUE_TEMPLATE/feature.yml) | nova funcionalidade | indique a fase do ROADMAP (ou "fora de escopo") |
| ♻️ [Refactor](../.github/ISSUE_TEMPLATE/refactor.yml) | dívida técnica | liste as **garantias de não-regressão** (testes que devem seguir verdes) |
| 🏗️ [Epic](../.github/ISSUE_TEMPLATE/epic.yml) | fase do ROADMAP desdobrada | checklist de filhas `- [ ] #N` é o medidor de progresso |
| 🔌 [Protocolo](../.github/ISSUE_TEMPLATE/protocolo.yml) | divergência com o device | regra R3: captura nova → golden → só então código |

**Regras de casa:**

- **Epic não fecha pelo PR de uma filha.** Filhas nunca usam `Closes #N`
  apontando para o epic — referencie com menção simples `#N`. O epic fecha
  manualmente quando o checklist completa (ou vire o vínculo em menção).
- **Label `achados-security`**: issue automática do
  [security.yml](../.github/workflows/security.yml) noturno. Prioridade máxima;
  o `close-linked` **nunca** a fecha — só manualmente, com a correção provada.
- **Label `ci-lite`** no PR de WIP: a matriz da CI roda só o Windows (barato);
  tire a label para a revisão final.
- **Vínculo de fechamento vive no CORPO do PR** (não no título, não em
  comentário): `Closes #N`, `Fixes #N` ou `Resolves #N`.

---

## 2. Branches — GitFlow

| Branch | Papel | PR de destino |
|---|---|---|
| `main` | produção/histórico de releases (protegida) | só via `release/*` |
| `develop` | integração — **destino padrão dos PRs** | — |
| `feature/<slug>` | trabalho de qualquer fase | `develop` |
| `hotfix/<slug>` | correção urgente de produção | `main` **e** `develop` |
| `release/x.y.z` | estabilização de rc (freeze de features) | `main` + backport em `develop` |

- Inclua o número da issue no nome quando houver: `feature/42-knob-drag`
  (rastreabilidade que sobrevive fora do GitHub).
- ⚠️ **Por que o PR aponta para develop e a issue ainda fecha:** o GitHub
  nativo só fecha por `Closes #N` em merge na branch **default** (`main`).
  No GitFlow a integração acontece em `develop` — o job `close-linked`
  ([ci.yml](../.github/workflows/ci.yml)) cobre essa lacuna via API (§5).
- **Push em `feature/*`/`fix/*`/`chore/*` NÃO roda CI** (decisão de custo de
  01/10): valide localmente e abra o PR — o CI dispara no PR e na integração.

```bash
git checkout develop && git pull
git checkout -b feature/42-knob-drag
```

---

## 3. Commits — conventional commits

Formato: `tipo(escopo): assunto` — validado pelo job **gate** do pipeline.

| Tipo | Uso | | Escopo | Área |
|---|---|---|---|---|
| `feat` | funcionalidade | | `ui` | packages/app/ui (front React) |
| `fix` | correção | | `shell` | packages/app/api (gp100-ui Tauri) |
| `refactor` | interna, sem mudança de comportamento | | `core` | gp100-core |
| `docs` | documentação | | `cli` | gp100-cli |
| `test` | testes apenas | | `spec` | analysis/ + golden |
| `chore`/`ci`/`deps` | manutenção/CI/dependências | | `docs` | documentação |

- Breaking change: `feat!:` ou `BREAKING CHANGE:` no corpo — o versionador
  corta major no próximo play.
- O **subject** nunca fecha issue; o vínculo de fechamento vai no corpo do PR.
- Exemplo: `feat(ui): knob com arrasto vertical e teclado (#42)`

---

## 4. Gates locais por área (antes do PR)

Só a área que você tocou — o CI filtra o resto por caminhos.

| Área | Gates |
|---|---|
| **Front** (`packages/app/ui`) | `npx tsc -b` · `pnpm lint` · **`pnpm test:coverage`** (gate 85% statements/functions/lines — [vite.config.ts](../packages/app/ui/vite.config.ts)) · `pnpm build` · `pnpm exec playwright test` (e2e completo) |
| **Baselines visuais** (mudou layout/paleta) | `pnpm exec playwright test visual.spec.ts --update-snapshots` → inspecione os diffs → commite os PNGs |
| **Rust core/cli** (raiz) | `cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings` · `cargo test` |
| **Shell Tauri** (`packages/app/api`) | gates do front (`ui/dist` alimenta o binário) + `cargo clippy/test -p gp100-ui` |
| **Spec** (`analysis/`, golden) | `uv run pytest` (10/10 — trava da especificação) |

- **Teste acompanha código novo** — padrão de DoD das issues; coverage caiu
  abaixo de 85%? O gate falha e pede teste, não exceção.
- Regras **R1–R4** do [ROADMAP](ROADMAP.md) continuam valendo (protocolo
  adivinhado não entra; round-trip `.prst` é sagrado).

---

## 5. PR — review, CI e fechamento da issue

**Abertura** ([template](../.github/PULL_REQUEST_TEMPLATE.md)): o quê/por quê ·
tabela **antes → depois** · tabela de **gates executados** (marque o que rodou)
· ambiente verificado · R1–R4 · política de conteúdo (nenhum material
proprietário do device).

**CI — 5 workflows com UMA função cada** (reestruturado em 01/10; issues #31–#34):

| Arquivo | Função | Dispara em |
|---|---|---|
| [ci.yml](../.github/workflows/ci.yml) | triggers + `close-linked` | PR para develop/main/release · push develop/main/release/hotfix |
| [_validate.yml](../.github/workflows/_validate.yml) | a régua completa (reusable) | chamado pelo ci.yml e pelo release.yml |
| [release.yml](../.github/workflows/release.yml) | version/rc/promote + publish | tag `v*` · dispatch manual |
| [_publish.yml](../.github/workflows/_publish.yml) | CLI + instalador (reusable) | chamado pelo release.yml |
| [security.yml](../.github/workflows/security.yml) | audits RustSec/npm + issue ACHADOS | agendado (06:30 UTC) · dispatch |

Dentro do `_validate`: gate (fmt + conventional commits) · spec (se
`analysis/` mudou) · matrizes Rust/front 3-OS **filtradas por caminhos** ·
e2e Playwright · e2e visual (baselines por plataforma) · smoke do shell
Tauri real. Docs-only não sobe Rust/front (jobs aparecem como skipped, sem
custo). **Push em `feature/*`/`fix/*`/`chore/*` não roda CI** — a validação
acontece no PR (econômico de propósito). O publish depende do `validate`
completo: tag não sai com a casca quebrada.

**Merge** (squash): subject conventional limpo, base `develop`, CI verde
(visual divergente só com decisão de baseline documentada).

**Fechamento automático**
([close-issues.yml](../.github/workflows/close-issues.yml)):

1. No merge em `develop`, extrai `Closes/Fixes/Resolves #N` do **corpo**;
2. Fecha cada issue via API com comentário de rastreabilidade
   (PR · sha do merge · link da run) — o resumo da run traz a tabela
   issue → resultado;
3. **Guards**: PR mergeado apenas; PRs do `github-actions[bot]` ignorados;
   `achados-security` nunca fecha por aqui; já fechada = confirmada (idempotente);
4. A issue sai do board na integração; o **release para `main`** é a saída oficial.

**Checklist relâmpago antes de abrir:**

- [ ] `Closes #N` no corpo (ou justificou vínculo simples)
- [ ] Gates da área verdes localmente (tabela do template preenchida)
- [ ] Teste novo acompanha o comportamento novo
- [ ] Docs vivos atualizados (§7)

---

## 6. Releases — cadeia rc1→rcN→tag final

- **Push de tag `v*`** → `release.yml` valida (reusable) e publica a GitHub
  Release com CLI de campo (Windows/gnu) + instalador NSIS do app (MSVC).
  (Tags criadas pelo `rc`/`promote` não disparam workflow — o publish roda no
  MESMO run que cortou a tag.)
- **▶ Play (`release.yml`, `action=version`)**: valida tudo (reusable),
  calcula o semver pelos conventional commits desde a última tag e só
  corta/publica com bump pendente (ou publique uma tag existente via input
  `publish-tag`).
- **Cadeia de Release Candidates** ([release-gitflow](../.github/workflows/release.yml),
  `workflow_dispatch`):
  1. **`action=rc`** — corta `release/x.y.z` de develop (ou reutiliza a
     branch), aplica `x.y.z-rc.N` nos manifests e etiqueta `vX.Y.Z-rc.N`;
     o pipeline roda a matriz cheia e publica como **PRERELEASE** (CLI +
     instalador marcados como prerelease). Versão: input `version`
     explícito, ou o semver pendente (exige `feat` desde a última tag).
     Correções entre rcs: PR em develop → rode `rc` de novo (o N
     incrementa sozinho).
  2. **`action=promote`** — remove o `-rc.N`, etiqueta `vX.Y.Z` FINAL
     (pipeline publica a release real), faz merge `--no-ff` em `main`,
     backport em `develop` e apaga a release branch. Conflito de merge =
     falha explícita para resolução manual (nada parcial é empurrado;
     promote é idempotente — tag existente recusa).
- **Proteção das branches** (`main`/`develop`) é configuração do
  repositório (Settings → Branches) — não vive nos workflows.
- **Ensaio da cadeia (gate + local)**: `python3 scripts/simulate_release.py`
  extrai os blocos `run:` do release.yml e os executa num sandbox git
  temporário (origin bare + clones) — prova rc1→rc2→promote, idempotência
  e os guards de recusa ANTES de qualquer uso real. Roda automaticamente
  no gate do pipeline; local precisa só de git+bash+python3 (+pyyaml).
  `--keep` preserva o sandbox em $TMPDIR para inspeção.

---

## 7. Docs vivos — atualizar junto do código

| Documento | Quando tocar |
|---|---|
| [ROADMAP.md](ROADMAP.md) | status de issue/fase concluída (com prova e data) |
| [UI_TEST_PLAN.md](UI_TEST_PLAN.md) | nova rodada de testes/ajustes de UI |
| [INDEX.md](INDEX.md) | documento novo ou papel alterado (é o mapa de navegação) |
| [DECISIONS.md](DECISIONS.md) | decisão supersedida → novo ADR marcando o anterior |
