# knowledge.md — GP-100 NextGen Editor (ler antes de agir)

> Mapa geral da documentação: **`docs/INDEX.md`** · plano vivo: **`docs/planejamento/ROADMAP.md`**
> (épico da Era Hardware #171) · números (testes, cobertura, jobs): **`docs/INDEX.md` §6**
> — nunca repita contador aqui.

**Premissa (Era Hardware, desde 09/10/2026): o APARELHO REAL é a fonte** (ADR-13).
O que a GP-100 faz se resolve MEDITO no aparelho, não deduzido do mock, do manual
de marketing ou de engenharia reversa. Resposta ao usuário SEMPRE em PT-BR.

Habilidade de entrada: **`AGENTS.md`** (raiz — o que um harness carrega sozinho) e
`.agents/README.md` (mapa do harness — papéis, skills, prompts, ordem de leitura).
Os papéis da equipe e o estado da demanda (`flow:*`) são `.agents/agents/README.md`.
O fluxo de trabalho é `.agents/skills/github-flow/SKILL.md`.
Dona ausente? **Modo ausente do dono** (mesmo `.agents/README.md`): fila
autônoma na ordem de prioridade, UMA issue por PR; decisão crítica vira
comentário NA ISSUE e a fila avança.

## Estado vivo (atualizar a cada marco — 1–3 linhas, sem prosa)

- 15/10 — **equipe multiagente (harness v2)**: a limitação foi **medida**, não suposta —
  este cliente não carrega agente definido no projeto em nenhum dos 3 layouts testados
  (`.agents/<id>.ts`, `.agents/agents/<id>.ts`, `.agents/agents/<id>.md`); a única
  spawnável é a injetada (`researcher-web`). Contorno: papel = contrato de arquivo
  (`.agents/agents/`: triagem→implementador→revisor→verificador) + estado em labels
  `flow:*` no GitHub + `git worktree` para paralelismo. Entrada nova: `AGENTS.md`.

- 13/10 — **#168 (`scripts/` dissolvida)**: CI/gates → `.github/scripts/`;
  spec/dados → `tools/` (suíte Spec em  `tools/tests/`); empacotamento →
  `packaging/`; RE → `docs/arquivo/scripts-era-RE/`. `uv run python
  .github/scripts/gates.py` roda a suíte de gates (contagem no INDEX §6). Armadilhas novas: root em
  `.github/scripts/` é `parents[2]` de `__file__`; `simulate_release` resolve
  o git-bash via `shutil.which` (System32/WSL precede o PATH) e traduz
  `python3`→`python` no sandbox Windows.
- 13/10 — **#167 (docs categorizada)**: `docs/` virou árvore (`dev/` `tecnico/`
  `planejamento/` `analises/` `referencias/` `arquivo/`) com TODOS os caminhos
  do repo atualizados (sed + link checker); `SECURITY.md` da raiz virou ponteiro
  para `docs/tecnico/seguranca.md`; golden bump v1.3 só de proveniência;
  `files/` triado no disco (instalador/DebugView/logs fora — 97→16 MB).
- 14/10 — **#243 (actor fatiado) entregue**: `packages/app/api/src/actor.rs`
  saiu de 2 702 linhas para um módulo (`actor.rs` = handle + spawn, sob o teto de
  600) + `actor/`: `estado` (o que a thread possui), `relogio` (loop com prazos +
  read-back), `fila` (`Request`), `tipos` (DTOs do IPC), `backends`,
  `{leitura,escrita,boot,diagnostico}` e `tests`. Comportamento idêntico: 27
  contratos do actor antes e depois, mesmos 40 testes do crate. O par
  `sink`+`debounce` virou `Cadencia` (escrita marca o estado e avisa a tela
  junto) e o orçamento do crate ficou declarado em `docs/dev/ARCHITECTURE.md`
  §4.1 — a cobrança automática do Rust é a **#248**.
- 14/10 — **CI enxuta (#245/#246)**: Distribuição e Release saíram do
  `ci.yml` (1777 → ~1350 linhas) para 6 workflows `workflow_call`, com o
  `ci.yml` ficando com o grafo, o nome dos jobs e a matriz de OS. O `env:` de
  topo do caller NÃO atravessa o `workflow_call` (nem é contexto do `with:`) —
  runner/args vêm por `with:` e o resto (toolchain do CLI, `BOT_NAME`/
  `BOT_EMAIL`) mora no arquivo chamado. Check de job reutilizável que RODA sai
  como `<caller> / <job>`; `skipped` (todo PR) sai com o nome do caller, então
  required check de PR não muda. E o `simulate_release` (~2 min) só roda por
  escopo (`scope-ci`): schedule, tag/dispatch e PR de release.
- 13/10 — **#164 (sync bilateral) entregue**: estado tipado único no actor
  (`idle/sync/pending/failed/offline`; `device_info` semeia e `device://sync`
  transiciona), read-back pós-escrita (debounce 800 ms) + refresh de 10 s pela
  sequência do boot (`reload_paginas_correntes` — `PROTOCOL.md` §13.15,
  ADR-15), chip na navbar com ação reler/reconectar e reconciliação "o
  aparelho vence". Limite honesto: knob físico não emite SysEx — é poll,
  não push.
- 13/10 — **regeneração de baseline virou workflow dedicado** (#204):
  `visual-baselines.yml` (`workflow_dispatch` sem inputs, `contents: read`)
  regenera as baselines `-linux`; o input `update-snapshots` saiu do `ci.yml`.
  Motivo: o `default: release` do input `action` fazia o dispatch de baseline
  cortar rc junto (runs 38070949933 e a do incidente rc.5 de 13/10 — revertido).
  Contrato dos 3 workflows de pipeline + dos 6 reutilizáveis de dist/release
  (#245) no `validate_workflows.py` (contratos 14/15).
- 12/10 — **Navbar enxuta (diretriz do dono)**: cluster de PATCH (◀ ▶ + nome)
  removido da navbar — troca vive na LISTA e o rótulo no display LED do palco;
  DeviceBadge sem miniatura SVG (dot `.live-dot`/`.idle-dot` + status honesto).
- 11/10 — **#170 (harness do agente) em execução**: skills da Era HW
  (`hardware-dev`, `testing`, `release` novas; 5 da Era RE arquivadas em
  `docs/arquivo/skills-era-RE/`), `.agents/prompts/` com 6 templates, templates
  de issue com contrato obrigatório (gate `check_issue_templates.py`) e este
  arquivo reescrito. O knowledge da era antiga ficou em
  `docs/arquivo/knowledge-era-RE_2026-10.md`.
- 11/10 — **#169 (CI na ordem do dono) ✅**: `ci.yml` com estágios em grafo
  (validação → lint → compilação → infra → segurança → testes → cobertura →
  distribuição → release), Relatório por último, Fechamento em
  `issues-fechamento.yml` próprio, Arch Linux na matriz via container.
- 10/10 — **`docs/referencias/GP100_DEVICE.md` é a fonte canônica do device** (vocabulário:
  User Patch ≠ User IR ≠ Setlist; fila de lacunas de leitura). O aparelho está
  ligado nesta máquina e é consultável por sessão (ver `hardware-dev`).
- 09/10 — **Era Hardware aberta** (épico #171, milestone v1.1.0); roadmap
  reescrito, era anterior arquivada em `docs/arquivo/`.

## O que NÃO fazer (regras de era, não sugestões)

- **Nada de engenharia reversa**: spec não é reconstruída de Suite/proxy/captura.
  `docs/arquivo/analysis/` é histórico/evidência; o que importa de lá migrou para
  `packages/core/data/` (#163). Skills da Era RE estão arquivadas — não
  ressuscite o processo delas.
- **Nada de escrever no aparelho fora das travas** (ADR-5): escrita exige a
  feature `write-verified` + gate H2 passado + roteiro assinado. Não crie
  terceira camada (env var, flag, subcomando) para contornar.
- **Nada de contornar gate vermelho** (CI ou local): sem relaxar `-D warnings`,
  sem pular job, sem `--no-verify`. CI vermelha não vira marco.
- **Nada de implementar sem issue**; uma issue = um PR; descoberta nova no
  caminho vira issue na hora — o ciclo é a skill `github-flow`.
- **Contadores**: a casa é o `docs/INDEX.md` §6 (skill `docs-sync`) — número em
  dois docs é dívida garantida.
- **Palpite de protocolo = para o fluxo**: divergência app × aparelho vira issue
  `protocolo` com evidência; nunca "consertar" o codec/UI no chute.
- Skill nova só se justificar (description dispara em tarefa recorrente); skill
  morta vai para `docs/arquivo/`, nunca é apagada.

## Armadilhas deste host (Windows 11 + Git Bash) — continuam mordendo

- **gnu 1.99.0 (update 13/10, 03:23) quebra o build script do crate `tauri`**
  (#223): compila e o exe segfaulta ao rodar (0xc0000005, módulo "unknown",
  qualquer path; bytes estáveis, Defender sem detecção). Efeito: `tauri
  build`/`check`/`dev` com o pin gnu morrem. **App shell = MSVC** por ADR-7
  (`RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc`, o mesmo do launch.json):
  compila e builda verde (NSIS+MSI gerados em 13/10). gnu pin segue válido para
  core/cli/library (cargo test/build fresh rodam).

- **Toolchain PINADA** `stable-x86_64-pc-windows-gnu` (`rust-toolchain.toml`):
  o host não tem MSVC Build Tools. Não trocar de alvo nem apagar o arquivo.
  O shell Tauri (`packages/app/api`) é MSVC/portável por ADR-7/ADR-8 —
  compila na CI, não no host.
- **PATH do cargo**: fixado no sistema por `docs/arquivo/scripts-era-RE/add_cargo_path.ps1`
  (arquivado na #168, fix de 29/09).
  Em shell da sessão antiga: `export PATH="/c/Users/Canta/.cargo/bin:$PATH"` —
  PREPEND (winget/choco sombreiam por append) e caminho em estilo POSIX
  (`C:\...` no PATH quebra a lista no `:` do drive).
- **`python3` no Git Bash do Windows NÃO existe** (alias da Microsoft Store):
  usar `uv run python …` da raiz. Na CI (ubuntu) `python3` funciona.
- **Python sob uv**: `uv sync --all-groups` na RAIZ (venv único `.venv/`);
  rodar tudo como `uv run …` da raiz.
- **Console cp1252**: `print()` de Unicode (`→`, `—`, acento) estoura
  `UnicodeEncodeError`; `subprocess.run(..., text=True)` pode dar
  `UnicodeDecodeError` dentro do reader thread. `sys.stdout.reconfigure(errors=
  "replace")` ou ler bytes com `errors="replace"`.
- **Arquivo escrito em modo texto vira CRLF** no Windows e suja o diff contra o
  `* text=auto eol=lf`: escrever com `newline="\n"`; gerador novo = rodar a
  suite e exigir `git status` limpo. **Nunca** aplicar normalização de EOL em
  BINÁRIO (já corrompeu os 16 ícones). Arquivo com identidade byte-a-byte entra
  no `.gitattributes` com `-text` antes do 1º commit (golden, .prst, capturas,
  fixtures).
- **CJK acidental em texto PT-BR**: ao redigir/ editar, varrer
  `0x3000–0x9FFF` (também cirílico `0x0400–0x04FF`) antes do commit — já
  aconteceu 5+ vezes em docstrings e comentários longos.
- **`bash -o pipefail`**: `cmd | tail` engole o exit 1 do gate. Rode gate sem
  pipe ou com pipefail — e confirme o exit code, não só a saída.
- **Heredoc `python - <<'EOF'` no Git Bash** é frágil (aspas/try-except):
  script não-trivial vira arquivo temporário e roda (o `analysis/` virou
  cápsula de histórico na #163 — não jogue lixo novo lá).
- `%TEMP%` real = `C:\Users\Canta\AppData\Local\Temp` (o `/tmp` do Git Bash é
  outro). O proxy grava `%TEMP%\midi_trace.jsonl`.
- **`zip` não existe no Git Bash**: `7z a -tzip` (também na CI, runner Windows).
- **Chave YAML repetida passa pelo PyYAML** (a última vence) e derruba o load
  no GitHub — run nasce com 0 jobs e sem log. `validate_workflows.py` tem o
  loader anti-duplicata; use o mesmo padrão em YAML próprio.
- **`success()` implícito pula job quando upstream foi skipped**
  (actions/runner#2205): guard explícito `always() && needs.x.result !=
  'failure'` atrás de job sujeito a skip.
- Tags/labels do GitHub: `gh issue create --label X` falha se a label não
  existe (criar com `gh label create --force`); **tag empurrada com
  `GITHUB_TOKEN` não dispara workflow** (por isso a cadeia de release publica
  artefatos no mesmo run).

## Armadilhas de campo e de captura (código e checklists H citam ESTA seção)

- **Suite oficial FECHADO durante a sessão** (occupancy de MIDI): qualquer
  programa com a porta aberta dá `OpenFailed` no `open()`. E logo após
  abrir/conectar, o Windows **reenumera** o device — o 1º `open` pode falhar
  com "nenhuma porta MIDI"; **retry em ~5s** resolve (não é driver morto).
  Citada pelos checklists H2–H5 e pela mensagem de erro de `real.rs`.
- **Buffers MIM_LONGDATA do proxy chegam com cauda stale**, e resposta longa
  traz `F7` EMBUTIDO no meio (**paginação, NÃO fim de mensagem**). A regra do
  codec é **trim no 1º `F7`** — já aplicada nos decoders/`RealDevice`.
  Citada por `packages/core/src/lib.rs` (`SYSEX_EOX`) e `packages/core/src/transport/real.rs`.
- **Linha sem `F7` final não é frame**: o proxy morre no flush e a cauda vem
  preenchida com `ff` — cortar/interpretar isso fabrica payload que nunca
  existiu (lição da baseline v1.1: 63 de 77 mensagens de save eram inventadas).
- **Log de captura é `append-only`**, com truncamentos SEM-HDR (ring buffer);
  separar sessões por gaps >30s. Nunca "consertar" isso no codec. Citada por
  `packages/core/src/golden.rs` (`op_payload`).

## Workflows canônicos (nunca fazer na mão)

- **Consultar o aparelho (leitura pura)**: `cargo build --release -p gp100-cli
  --features real-device` e `./target/release/gp100-cli.exe --real
  --i-know-what-im-doing <sub>`. Efeito colateral: o pedal troca de patch no
  display durante varredura (avisar o owner). Detalhes, anti-brick e evidência:
  skill `hardware-dev`.
- **Todos os gates de script de uma vez**: `uv run python .github/scripts/gates.py` (local;
  a CI roda os mesmos). Só a área: `uv run python .github/scripts/gates.py <nome>`.
- **Rust**: `cargo fmt --check` · `cargo clippy --workspace --all-targets --
  -D warnings` · `cargo test` (inclui doc-tests) — skill `rust-practices`.
- **Front**: `pnpm test` / `pnpm test:coverage` (gate 85%) / `pnpm build` /
  e2e `pnpm exec playwright test` — skill `ui-ux-practices` + `testing`.
- **Bump de versão**: `uv run python .github/scripts/sync_version.py --set X` + `--check`
  (5 manifests sincronizados). Release: só por `workflow_dispatch`
  (rc → promote) — skill `release`.
- **Baseline visual**: regenerar SÓ pelo workflow dedicado
  `visual-baselines.yml` (`gh workflow run visual-baselines.yml --ref <branch>`,
  `contents: read`, sem inputs) e `--update-snapshots=changed` (local); o CI
  nunca escreve baseline sozinho — ver `testing`. Motivo (#204): a regeneração
  saiu do dispatch do `ci.yml` porque o `default: release` do input `action`
  cortava rc junto com uma regeneração de baseline (runs 38070949933 e a do
  incidente rc.5 de 13/10).
- **docs-sync**: ao fechar marco, commit próprio na mesma PR atualizando
  INDEX/ROADMAP/knowledge — skill `docs-sync`.

## Caminhos canônicos

- `docs/INDEX.md` §6 — estado e **números** (fonte única)
- `docs/planejamento/ROADMAP.md` — plano executivo vigente (Era Hardware; princípios **P1–P7** no §1.2)
- `docs/referencias/GP100_DEVICE.md` — o QUE a GP-100 é (fonte canônica do device)
- `docs/tecnico/PROTOCOL.md` §13 — a FORMA do fio · `docs/referencias/protocol_golden.json` — a mesma
  coisa, executável (especificação de FORMA; consumo via core)
- `docs/dev/DECISIONS.md` — ADRs; mudança de decisão = novo ADR
- `docs/arquivo/H2_CHECKLIST.md` — roteiro do gate de escrita (única porta de escrita)
- `packages/core/data/` — dados canônicos (fixtures/manifest) — #163
- `.agents/` — harness do agente (skills, prompts, README)
- `docs/arquivo/` — era anterior (ROADMAP, skills, este knowledge) — histórico,
  não processo
- `docs/arquivo/analysis/` — a Era de RE inteira, arquivada na #163 (capturas,
  evidência dos gates H, decoders, proxy, manual): é PROVENIÊNCIA — de onde veio
  cada número — e não fonte de spec nem insumo de runtime (o guard reprova
  `analysis/` dentro de `packages/`)

## Convenções

- Código Rust: skill `rust-practices` (doc-comment PT-BR com evidência em item
  `pub`, `deny(missing_docs)`, sem `unwrap` na lib, `thiserror` tipado).
- Front: skill `ui-ux-practices` (invoke só em `ui/src/ipc/`, estados
  explícitos, a11y, tokens, i18n).
- Commits: formato e footer na skill `github-flow`; docs-sync em commit PRÓPRIO
  na mesma PR.
- Teste novo acompanha código novo — onde cada um vive: skill `testing`.
- Decisão/estado de issue: a **issue é a fonte**; doc não duplica plano de issue.
