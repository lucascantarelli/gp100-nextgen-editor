# knowledge.md — GP-100 NextGen Editor (ler antes de agir)

> Mapa geral da documentação: **`docs/INDEX.md`** · plano vivo: **`docs/ROADMAP.md`**
> (épico da Era Hardware #171) · números (testes, cobertura, jobs): **`docs/INDEX.md` §6**
> — nunca repita contador aqui.

**Premissa (Era Hardware, desde 09/10/2026): o APARELHO REAL é a fonte** (ADR-13).
O que a GP-100 faz se resolve MEDITO no aparelho, não deduzido do mock, do manual
de marketing ou de engenharia reversa. Resposta ao usuário SEMPRE em PT-BR.

Habilidade de entrada: `.agents/README.md` (mapa do harness — skills, prompts,
ordem de leitura). O fluxo de trabalho é `.agents/skills/github-flow/SKILL.md`.

## Estado vivo (atualizar a cada marco — 1–3 linhas, sem prosa)

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
- 10/10 — **`docs/GP100_DEVICE.md` é a fonte canônica do device** (vocabulário:
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
- **Nada de implementar sem issue**; uma issue = um PR (`Closes #N` no corpo);
  descoberta nova no caminho vira issue na hora.
- **Contador vive só no `docs/INDEX.md` §6** (regra #81) — repetido em 2 docs é
  dívida garantida.
- **Palpite de protocolo = para o fluxo**: divergência app × aparelho vira issue
  `protocolo` com evidência; nunca "consertar" o codec/UI no chute (R3).
- Skill nova só se justificar (description dispara em tarefa recorrente); skill
  morta vai para `docs/arquivo/`, nunca é apagada.

## Armadilhas deste host (Windows 11 + Git Bash) — continuam mordendo

- **Toolchain PINADA** `stable-x86_64-pc-windows-gnu` (`rust-toolchain.toml`):
  o host não tem MSVC Build Tools. Não trocar de alvo nem apagar o arquivo.
  O shell Tauri (`packages/app/api`) é MSVC/portável por ADR-7/ADR-8 —
  compila na CI, não no host.
- **PATH do cargo**: fixado no sistema por `scripts/add_cargo_path.ps1` (29/09).
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
  Citada por `packages/core/src/lib.rs` (`SYSEX_EOX`) e `transport/real.rs`.
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
- **Todos os gates de script de uma vez**: `python3 scripts/gates.py` (local;
  a CI roda os mesmos). Só a área: `python3 scripts/gates.py <nome>`.
- **Rust**: `cargo fmt --check` · `cargo clippy --workspace --all-targets --
  -D warnings` · `cargo test` (inclui doc-tests) — skill `rust-practices`.
- **Front**: `pnpm test` / `pnpm test:coverage` (gate 85%) / `pnpm build` /
  e2e `pnpm exec playwright test` — skill `ui-ux-practices` + `testing`.
- **Bump de versão**: `python3 scripts/sync_version.py --set X` + `--check`
  (5 manifests sincronizados). Release: só por `workflow_dispatch`
  (rc → promote) — skill `release`.
- **Baseline visual**: regenerar só por dispatch `update-snapshots=true`
  (linux) e `--update-snapshots=changed` (local); o CI nunca escreve baseline
  sozinho — ver `testing`.
- **docs-sync**: ao fechar marco, commit próprio na mesma PR atualizando
  INDEX/ROADMAP/knowledge — skill `docs-sync`.

## Caminhos canônicos

- `docs/INDEX.md` §6 — estado e **números** (fonte única)
- `docs/ROADMAP.md` — plano executivo vigente (Era Hardware; regras R1–R4)
- `docs/GP100_DEVICE.md` — o QUE a GP-100 é (fonte canônica do device)
- `docs/PROTOCOL.md` §13 — a FORMA do fio · `docs/protocol_golden.json` — a mesma
  coisa, executável (especificação de FORMA; consumo via core)
- `docs/DECISIONS.md` — ADRs; mudança de decisão = novo ADR
- `docs/H2_CHECKLIST.md` — roteiro do gate de escrita (única porta de escrita)
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
- Commits conventional (`tipo(escopo): assunto`) com footer Codebuff quando o
  agente escreve; docs-sync em commit PRÓPRIO na mesma PR.
- Teste novo acompanha código novo (unidade em `src/`, contrato em `tests/`) —
  skill `testing`.
- Decisão/estado de issue: a **issue é a fonte**; doc não duplica plano de issue.
