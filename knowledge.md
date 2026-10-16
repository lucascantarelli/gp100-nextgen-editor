> ✅ **atual** · última revisão 2026-10-16.

# knowledge.md — GP-100 NextGen Editor (ler antes de agir)

> Mapa geral da documentação: **`docs/INDEX.md`** · plano vivo: **`docs/planejamento/ROADMAP.md`**
> (épico da Era Hardware #171) · números (testes, cobertura, jobs): **`docs/INDEX.md` §6**
> — nunca repita contador aqui.

**Premissa (Era Hardware, desde 09/10/2026): o APARELHO REAL é a fonte** (ADR-13).
O que a GP-100 faz se resolve MEDITO no aparelho, não deduzido do mock, do manual
de marketing ou de engenharia reversa. Resposta ao usuário SEMPRE em PT-BR.

Entrada: **`AGENTS.md`** (raiz — o que um harness carrega sozinho) e `.agents/README.md`
(mapa do harness). Papéis e estado da demanda (`flow:*`): `.agents/agents/README.md`; fluxo:
`.agents/skills/github-flow/SKILL.md`; dono ausente: **modo ausente** (`.agents/README.md`).
O que **não** se faz é a lista do [`AGENTS.md`](AGENTS.md) §5 — aqui é ponteiro, não cópia.

## Estado vivo (atualizar a cada marco — 1–3 linhas, sem prosa)

- 16/10 — **a higiene do harness virou gate (#267)**: `check_harness.py` cobra caminho,
  `description`, contrato de papel e **uma casa por regra** — a tabela de fontes únicas é
  SAÍDA dele (`--tabela` gera; editar à mão reprova) e o #273 liga gate → job.
- 15/10 — **equipe multiagente (harness v2)**: o cliente **não carrega** agente definido no
  projeto (medido, 3 layouts); contorno = papel é contrato de arquivo + estado em label
  `flow:*` + `git worktree`. Entrada nova: `AGENTS.md` — medição em `docs/analises/`.
- 14/10 — **#261 (`scope-rust` do plano)**: a lista de crates Rust da CI sai dos `members` do
  manifesto do WORKSPACE (= `parents[2]` de `.github/scripts/`, armadilha da #168) e o plano
  **falha alto** — o `except OSError` fazia PR de Rust rodar a CI sem lint/compile/teste.

- Os **11 marcos anteriores** (14/10 → 09/10) estão em
  [`docs/arquivo/knowledge-estado-vivo-2026-10.md`](docs/arquivo/knowledge-estado-vivo-2026-10.md).

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
  (arquivado na #168). Em shell antigo: `export PATH="/c/Users/Canta/.cargo/bin:$PATH"` —
  PREPEND, e caminho em estilo POSIX (`C:\...` quebra a lista no `:` do drive).
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
  BINÁRIO (já corrompeu os arquivos de ícone — contagem no `docs/INDEX.md` §6). Arquivo com identidade byte-a-byte entra
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
  `packages/core/src/codec.rs` (`op_payload`; a citação anterior apontava `golden.rs`, que
  **nunca** foi o dono — o gate de obsolescência doc × módulo (#275) pegou o ponteiro errado).

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
