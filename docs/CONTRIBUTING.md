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
6. **CI verde + merge** (squash) → o job `Fechamento · issues` do
   [ci.yml](../.github/workflows/ci.yml) fecha a issue com comentário de
   rastreabilidade (só quando o PR foi MESMO mergeado).

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
- **Label `achados-security`**: issue automática da auditoria noturna
  (`Segurança · auditorias`). Prioridade máxima; o
  `Fechamento · issues` **nunca** fecha uma issue com essa label — só
  manualmente, com a correção provada.
- **A label `ci-lite` foi abolida (#68)**: ela reduzia a matriz para Windows
  em silêncio, sem ninguém registrar o motivo. Todo PR roda a matriz inteira.
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
| `release/x.y.z` | estabilização de rc (freeze de features) | vira `main` pelo SHA da tag + `develop` avança para a próxima |

- Inclua o número da issue no nome quando houver: `feature/42-knob-drag`
  (rastreabilidade que sobrevive fora do GitHub).
- ⚠️ **Por que o PR aponta para develop e a issue ainda fecha:** o GitHub
  nativo só fecha por `Closes #N` em merge na branch **default** (`main`).
  No GitFlow a integração acontece em `develop` — o job `Fechamento · issues`
  ([ci.yml](../.github/workflows/ci.yml)) cobre essa lacuna via API (§5).
- **Push e PR** disparam a CI em toda branch de desenvolvimento. O filtro é o
  de *caminho* (`scripts/ci_plan.py`): um PR que só mexe em `docs/` não sobe
  Rust nem front.

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
| **Spec** (`analysis/`, `scripts/`, golden) | `uv run pytest` (trava da especificação) |
| **Gates de script** (pipeline, empacotamento, versão, spec, release) | **`python3 scripts/gates.py`** — roda todos de uma vez, na ordem do CI |

- **`gates.py` é o atalho para "os gates de script"**: `validate_workflows`,
  `check_bundle`, `sync_version`, `check_deadcode`, `h1_compare`, `pytest`,
  `check_base_images` e `simulate_release`. Rode `python3 scripts/gates.py --list`
  para ver a lista, ou passe nomes para rodar só alguns:
  `python3 scripts/gates.py check_bundle validate_workflows`.
  Todas as dependências (`pyyaml`, `jsonschema`) estão no `pyproject.toml`, então
  `uv sync --all-groups` basta — nada de instalar pacote na linha de comando
  (#80).
- **`scripts/` está no escopo do `spec`** (`scripts/ci_plan.py`): o pytest testa
  scripts de gate, e mexer só no script deixaria o job de spec como `skipped` —
  os testes ficariam verdes sem rodarem contra a mudança (#21).
- **Piso de versão das actions** — o GitHub descontinuou o runtime Node 20
  (set/2025): o runner força Node 24 e emite aviso em **toda** run. O contrato é
  um **piso de major por action** em `PISO_VERSAO_ACTION`
  ([validate_workflows.py](../scripts/validate_workflows.py)), não uma lista de
  versões ruins — assim uma major que ninguém catalogou ainda falha em vez de
  passar. Adicionar uma action nova? Declare o piso dela no mesmo commit, senão o
  gate cobra. Subir uma major? Suba o piso junto.
  O `actionlint` roda no mesmo job (`Lint · contratos do pipeline`) e é o que
  pega erro de expressão (`${{ }}`, `secrets`, `inputs`) que o YAML aceitaria.
- **Teste acompanha código novo** — padrão de DoD das issues; coverage caiu
  abaixo de 85%? O gate falha e pede teste, não exceção.
- Regras **R1–R4** do [ROADMAP](ROADMAP.md) continuam valendo (protocolo
  adivinhado não entra; round-trip `.prst` é sagrado).

---

## 4b. Binários de campo (H1 leitura / H2 escrita)

O CLI de campo tem **duas camadas**, e elas não são interchangeáveis:

```bash
# H1 — leitura real. A escrita é IMPOSSÍVEL neste binário.
cargo build --release -p gp100-cli --features real-device

# H2 — escrita real dos 3 fluxos capturados (set-param, save, upload-ir).
cargo build --release -p gp100-cli --features real-device,write-verified
```

`write-verified` é **feature de compilação, default OFF** (ADR-5): não existe
flag, variável de ambiente ou argumento que abra a escrita num binário que não
foi compilado com ela. `write-verified` implica `real-device`, então listar as
duas é redundante — mas explícito é melhor que implícito num comando que alguém
vai colar com a pedaleira ligada.

Ao mexer em transporte/`Session`/CLI, **compile nos dois modos**: o desligado
prova que nada de escrita entrou por acidente; o ligado prova que a trava não
virou erro de compilação nem de runtime. `cargo test --workspace` roda os dois
(14 testes de CLI em cada modo).

Runbooks: [H1_CHECKLIST.md](H1_CHECKLIST.md) (leitura) e
[H2_CHECKLIST.md](H2_CHECKLIST.md) (escrita — **muda estado do aparelho do
usuário**; os 3 fluxos são um por vez, com verificação no display).

---

## 5. PR — review, CI e fechamento da issue

**Abertura** ([template](../.github/PULL_REQUEST_TEMPLATE.md)): o quê/por quê ·
tabela **antes → depois** · tabela de **gates executados** (marque o que rodou)
· ambiente verificado · R1–R4 · política de conteúdo (nenhum material
proprietário do device).

**CI — UM workflow, 11 tipos de job** (consolidado na #68; a divisão anterior em
6 arquivos por função veio da #31–#34 e da imagem de container da #41):

O `name:` de cada job é `Tipo · o que é`, sem número. O GitHub não tem
`stages:` como o GitLab — a doc de migração do próprio GitHub diz que o
equivalente é o `needs:`. Duas consequências, ambas assumidas: a ordem de
execução é o grafo `needs:` (e a ordem de declaração no arquivo), e a **lista
de checks de um PR sai em ordem alfabética**. Por isso o job `Relatório`
reconta a sequência real no step summary.

| Tipo | Jobs | Quando roda |
|---|---|---|
| `Validação` | contexto: o que mudou, tipo da branch, matrizes | sempre |
| `Lint` | contratos do pipeline · mensagens de commit · rust fmt · rust clippy · eslint | **sempre**, em toda branch |
| `Compilação` | rust `cargo check` · front (tsc + vite) | **sempre** — build ≠ distribuição |
| `Testes` | rust (unit) · vitest · pytest · e2e Chromium · e2e visual · e2e webview | dev · develop · main · hotfix · tag |
| `Cobertura` | vitest com instrumentação + gate 85% | dev · develop · main · hotfix · tag |
| `Segurança` | cargo audit · pnpm audit · npm audit · issue ACHADOS | sempre |
| `Relatório` | resumo visual no step summary | **sempre** (`if: always()`), mesmo com falha |
| `Infra` | imagens de container do próprio CI | push que mexe em `.github/docker/` (ou tag ausente no registro) |
| `Release` | guarda da main · rc · promote · play | dist só na tag `v*`; versionamento só por dispatch |
| `Distribuição` | instaladores (3 OS) · CLI de campo | só na tag `v*` |
| `Fechamento` | fecha as issues do PR **mergeado** | só em PR fechado com merge |

Um job cujo `if:` é falso aparece como *skipped* na lista de checks: o GitHub
cria um check para todo job declarado e não há como esconder. Só sumiria se o
job não existisse naquele workflow — e espalhar em vários arquivos é
exatamente o custo que a #68 eliminou.

A tabela **branch → tipos de job** mora em `scripts/ci_plan.py` (uma fonte de
verdade, testável localmente), e não em `if:` espalhado no YAML — foi
justamente a duplicação que fez a regra divergir antes.

Imagens de CI no ghcr.io: `ci-linux` (Ubuntu + WebKitGTK/GTK/xvfb/tauri-driver,
para o shell Tauri) e `ci-base` (Debian slim + Rust/uv/Node, para os jobs sem
GUI). Alpine foi descartado: `alsa-sys` e crates com código C/linkam contra
glibc.

A régua é o conjunto de jobs dos tipos `Lint` a `Segurança`. As
matrizes Rust/front são **filtradas por caminhos** (`scripts/ci_plan.py`): docs
puro não sobe Rust/front — os jobs aparecem como *skipped*, sem custo. O gate
do front (`Lint · UI` e `Cobertura`) roda em **jobs próprios**, em
paralelo com a matriz de compilação 3-OS, para o Setup Node de um não serializar
com o do outro (issue #50). A cobertura é job separado de propósito: é uma
métrica com gate, não parte de "os testes passaram".

**Uma run por push, não duas.** O gatilho `push` cobre só `develop`, `main` e as
tags `v*`. Branch de trabalho (`feature/*`, `bugfix/*`, `hotfix/*`, `release/*`)
entra pelo `pull_request` — que além de não custar o dobro testa o **merge
sintético** (cabeça do PR fundida na base), ou seja, o que realmente entra. O
`push` num branch com PR aberto dispara os DOIS eventos no GitHub, e o pipeline
inteiro rodava duas vezes sem informação nova na segunda. O validador proíbe
`feature/**`/`bugfix/**`/`hotfix/**`/`release/**` no `push` para isso não voltar. O que muda entre elas não é o gatilho, é a **tabela**
de `scripts/ci_plan.py` (a develop e a tag rodam a suíte inteira;
a release branch, por ser um freeze já testado da develop, roda lint + compilação).

### Custo do CI — imagem de container e caches (#41)

O que é **fixo** (toolchain, bibliotecas de sistema, driver) não pertence ao
job: virou camada da imagem publicada no ghcr.io.

- **Imagem `ci-linux`** (`:1` estável + `sha-<curto>` de auditoria;
  Dockerfile em [.github/docker/ci-linux/](../.github/docker/ci-linux/Dockerfile)):
  Rust stable+clippy+rustfmt, `tauri-driver` compilado, WebKitGTK 4.1/GTK3/
  appindicator/rsvg/ALSA (dev), `webkit2gtk-driver`, `xvfb`, mesa (software
  rendering) e Node 22 + pnpm 11 com **store aquecido** pelo lockfile do front.
  Pacote público (repo público) → os jobs fazem pull anônimo.
- **Onde o container roda:** `ui-rust (ubuntu · container)` e `e2e smoke —
  shell Tauri real`. Ali o pull (~30s) se paga contra o que a imagem elimina:
  apt do webkit/gtk (57s) e, no smoke, apt 45s + `cargo install tauri-driver`
  17s. `front`, `e2e` e `e2e visual` **continuam no runner hospedado** — o pull
  custaria mais que o Chromium que eles instalariam; ali a alavanca é o cache
  do `~/.cache/ms-playwright` (chave = hash do `pnpm-lock.yaml`).
- **Caches de Rust:** por **workspace** (`shared-key` = `ws-raiz` para
  core/cli/gate/publish-cli; `ws-api` para ui-rust/smoke/installer). O input
  `key` antigo somava à chave automática POR JOB — nenhum job compartilhava
  cache. O ui-rust do Windows ainda usa `cache-workspace-crates`
  (experimento: o MSVC recompilava o `gp100-ui` inteiro, ~269s).
- **ui-rust em 2 OS:** Windows (MSVC, ADR-7) + Linux (container). O macOS do
  front usa `macos-15-intel` (x86_64): todo label `macos-*` arm64 carrega a
  anotação de fila do GitHub; o Intel não (suporte até ~08/2027).
- **Rebuild da imagem:** o job `Infra · imagens de container` publica quando a definição em
  `.github/docker/` mudou **ou** quando a tag `:1` não existe no ghcr.io. A
  segunda metade é o que torna o bootstrap possível: decidir só pelo diff trava
  para sempre quando a run que introduziu a imagem morre antes do push (ninguém
  mais toca `.github/docker`, e todo job que consome fica em `manifest
  unknown`). Quem decide publicar e não confirma a tag no registro **falha** —
  verde sem imagem é pior que vermelho, porque a quebra aparece depois, em
  outro job, sem ligação com a causa. Os jobs que consomem a imagem dependem
  dele no `needs:`, então não correm em paralelo ao push. Bump de tag (`:1` →
  `:2`) é manual e só quando a mudança for incompatível — os jobs referenciam
  `:1`.
- **Medição (antes/depois):** `python3 scripts/ci_timings.py <run-id> --steps`
  e `--compare <antes> <depois>` — tabela por job/step direto da API do
  Actions (só `gh` + stdlib). Mudança de custo entra com número.
- **Números do #41** (runs `36946076269` → `36954177254`, matriz completa):
  parede **405s → 140s (−65%)**; `ui-rust (windows)` **388s → 119s** (o
  `cache-workspace-crates` cortou 234s do clippy/test/build MSVC — o maior
  ganho isolado); `e2e smoke` **240s → 85s** (apt + `cargo install` fora do
  caminho e cache da api compartilhado); `ui-rust (macos)` saiu (96s).
  Contrapartidas honestas: o front do macOS em `macos-15-intel` custou +34s
  (runner Intel é mais lento; o preço de matar a anotação de fila do arm64) e
  e2e/e2e-visual ficaram ~neutros (o cache de browser evita download, não o
  resto do job).
- **Números do #43** (gate do front em 1 OS — runs `36954858195` →
  `36956509001`): o passo com gate (lint + coverage + build) custava **30s no
  macOS** e **20s no Windows**; agora esses dois rodam só `Build (tsc + vite)`
  em **7s** e **6s** (~37s de runner a menos por run), com o gate preservado
  no ubuntu (31s, inalterado).

**Merge** (squash): subject conventional limpo, base `develop`, CI verde
(visual divergente só com decisão de baseline documentada).

**Fechamento automático**
(job `Fechamento · issues` do [ci.yml](../.github/workflows/ci.yml)):

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

- **Push de tag `v*`** → a suíte inteira roda (lint → build → test →
  coverage → security) e só então os jobs `Distribuição · *` publicam na GitHub
  Release: instalador do app por plataforma (`.msi`/`.exe`, `.dmg`,
  `.deb`/`.AppImage`) e CLI de campo (Windows/gnu). Tags criadas pelo
  `rc`/`promote` não disparam workflow — o publish roda no MESMO run que
  cortou a tag.
- **▶ Play (`workflow_dispatch`, `action=play`)**: calcula o semver pelos
  conventional commits desde a última tag e só corta com bump pendente.
- **Cadeia de Release Candidates** (`workflow_dispatch`):
  1. **`action=release`** — corta `release/x.y.z` da `develop` (ou reutiliza
     a branch), aplica `x.y.z-rc.N` nos manifests e etiqueta `vX.Y.Z-rc.N`,
     publicando como **PRERELEASE**. `source=main` faz o fluxo de hotfix
     (patch em vez de minor). Versão: input `version` explícito, ou o semver
     pendente. Correções entre rcs: rode `action=release` de novo (o N
     incrementa sozinho).
  2. **`action=promote`** — remove o `-rc.N`, etiqueta `vX.Y.Z` FINAL, faz
     `main` receber o **SHA da tag**, faz merge `--no-ff` da release na
     `develop` **avançando a develop para a próxima versão**, e apaga a
     release branch. Conflito de merge = falha explícita para resolução
     manual; promote é idempotente (tag existente recusa).
- **A `main` só recebe o SHA de uma tag `v*`.** Em repositório **pessoal** o
  GitHub não permite restrição de escrita em branch (a API recusa com
  "Only organization repositories can have users and team restrictions"), e
  regra de branch filtra *quem* é protegido, não *de onde* veio o push. A
  regra é aplicada pelo job `Release · guarda da main`, que roda em todo
  push na `main` e **falha a run** se o sha não for o de uma tag `v*`. O que
  o servidor garante: histórico linear, sem force-push, sem deleção. Em repo
  de organização dá para trocar a guarda por `restrictions.push` de verdade.
- **Ensaio da cadeia (gate + local)**: `python3 scripts/simulate_release.py`
  extrai os blocos `run:` do `ci.yml` (jobs `release-rc` e
  `release-promote`) e os executa num sandbox git temporário — prova
  rc1→rc2→promote, a `develop` avançando para a próxima versão, idempotência
  e os guards de recusa ANTES de qualquer uso real. Roda no job
  `Lint · contratos do pipeline`; local precisa só de git+bash+python3 (+pyyaml).
  `--keep` preserva o sandbox em $TMPDIR para inspeção.

---

## 7. Docs vivos — atualizar junto do código

| Documento | Quando tocar |
|---|---|
| [ROADMAP.md](ROADMAP.md) | status de issue/fase concluída (com prova e data) |
| [UI_TEST_PLAN.md](UI_TEST_PLAN.md) | nova rodada de testes/ajustes de UI |
| [INDEX.md](INDEX.md) | documento novo ou papel alterado (é o mapa de navegação) |
| [DECISIONS.md](DECISIONS.md) | decisão supersedida → novo ADR marcando o anterior |
