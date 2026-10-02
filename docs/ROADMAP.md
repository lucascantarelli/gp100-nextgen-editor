# 🗺️ ROADMAP EXECUTIVO — Fundação + gp100-core

> **Objetivo:** fechar TODAS as atividades de preparação antes do primeiro commit
> de código de negócio, para que o desenvolvimento flua **sem paradas** para ajuste
> de ambiente, configuração, revisão de spec ou extração de dados. Cada item é uma
> issue com dependências e critério de aceite (Definition of Done) testável.
>
> **Status:** ✅ atual · **Última revisão:** 2026-10-01 · Mapa da doc: `docs/INDEX.md`
>
> **Princípio do não-retrocesso:** nenhum passo de implementação pode depender de
> descoberta nova. Tudo que exigia hardware/pesquisa já está fechado e validado
> (§13 + golden 100%). Se durante a implementação uma premissa falhar, o fluxo é:
> parar → captura nova → golden regenerado → validate 100% → então retomar (regra R3).

---

## 🎫 GESTÃO DE ISSUES — the single source for o trabalho aberto

> **O plano vivo mora no GitHub, não neste documento.** Este ROADMAP guarda o
> histórico do que já foi entregue (fases P/M0/ACHADOS/U/Q/V) e as regras do
> projeto; **todo trabalho aberto é uma issue** com epic, labels e milestone.
> Milestone único da primeira entrega: **v1.0.0**.

### Epics abertos (milestone v1.0.0)

| Epic | Escopo | Filhas |
|---|---|---|
| [#13](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/13) | **FASE U** — UI por etapas (casca → pedais) | [#19](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/19) |
| [#14](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/14) | **FASE V** — UI/UX enterprise e modernização visual | #8 · #9 · #10 · #11 · #12 · #20 · #30 |
| [#15](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/15) | **M2** — IR lab, SnapTone, biblioteca e empacotamento | #24 · #25 · #26 · #17 |
| [#16](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/16) | **M3** — Diferenciais (live mode, cloud, tone match) | (panorama) |
| [#17](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/17) | **Release v1.0.0** — binários, `.deb` e Arch | #27 · #28 · #29 |
| [#18](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/18) | **Gate H** — validação em hardware real | #21 · #22 · #23 |

### Issues filhas (por área)

| # | Título | Área |
|---|---|---|
| [#19](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/19) | U-3: renderizar os 9 pedais no board | ui |
| [#8](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/8)–[#12](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/12) | Modernização da UI (tuner, design system, topbar, biblioteca, looper) | ui/design |
| [#20](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/20) | V-8: edge cases de IPC nível 2 | ui |
| [#30](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/30) | i18n do editor (pt-BR/en/es/zh) | ui |
| [#24](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/24)–[#26](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/26) | IR lab · SnapTone/NAM · biblioteca SQLite | core/ui |
| [#27](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/27)–[#29](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/29) | Release multiplataforma (Win/macOS · `.deb` · Arch) | release |
| [#21](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/21)–[#23](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/23) | Gate H: H1 leitura · H2 escrita · H3 congelamento | re |

### Labels

`area:ui` · `area:core` · `area:cli` · `area:api` · `area:ci` · `area:docs` ·
`area:release` · `area:re` · `design` · `packaging` · `epic` · `feature` ·
`bug` · `documentation` · `accessibility` · `priority:high|medium|low`.

---

## FASE P — PREPARAÇÃO (não escreve lógica de negócio)

> **Agentes e skills — quem executa o quê** (detalhes em `.agents/skills/`):
> - **Buffy (agente principal)** — orquestra, decide, executa issues P/M0 por ordem;
>   dono de docs (INDEX/knowledge) e do fluxo R3.
> - **`spec-baseline`** — congelar/alterar baseline da spec (P1/H3): hashes,
>   bump de versão, justificativa no PROTOCOL.md.
> - **`protocol-validate`** — rodar e interpretar validate_golden/validate_knob_map
>   (gate de qualquer PR que toque spec, decoders ou capturas); reporta cobertura.
> - **`core-dev`** — implementação Rust das issues M0.1–M0.7 dentro das decisões
>   de P5 (nunca adivinhar protocolo: consulta golden/§13; R4 é sagrado).
> - **`proxy-build`** — compilar o proxy winmm e propagar a DLL para `suite_local/`.
> - **`capture-analyze`** — nova sessão de captura com o owner: copiar log,
>   decodificar, regenerar golden (sempre seguida de `protocol-validate`).
> - **`docs-sync`** — após qualquer marco: atualizar INDEX.md, knowledge.md e
>   status das issues deste roadmap.
> - **`rust-practices`** — gates de qualidade e estilo de TODO código Rust
>   (fmt/clippy/test/doc-tests, doc-comments PT-BR, `deny(missing_docs)` no core).

### P0. Ambiente Python sob uv — ✅ FEITO 28/09 (decisão do owner)
- **Responsável:** Buffy · **O quê:** migrar de venv manual para **astral uv**
  (`pyproject.toml` + `uv.lock` versionados; venv recriável com `uv sync`).
- **DoD:** ✅ `uv sync` reproduz o ambiente (capstone, pefile, ziglang 0.16 no venv,
  pytest no grupo test); validate_golden 100% rodando com o novo venv; proxy não
  depende mais de python global.

### P1. Congelar a especificação do protocolo (baseline v1.0) — ✅ FEITO 28/09
- **Responsável:** skill `spec-baseline` · **Estimativa:** 30 min
- **O quê:** fixar `docs/protocol_golden.json` + `analysis/parameters.json` como
  baseline imutável de desenvolvimento: hash sha256 dos dois arquivos registrado
  no banner do `PROTOCOL.md` §13.
- **Por quê:** implementar contra um alvo que muda é o maior gerador de retrabalho.
- **DoD:** ✅ hashes registrados (Baseline v1.0 no §13). Mudança futura = captura
  nova → build_golden → validate 100% → novo hash + justificativa (regras R2/R3).

### P2. Workspace Rust + toolchain + qualidade — ✅ FEITO 28/09
- **Responsável:** skill `core-dev` · **Estimativa:** 1h
- **Python:** ambiente migrado para **astral uv** ✅ (`pyproject.toml` + `uv.lock`
  na raiz, **venv único na raiz: `.venv/`** — o antigo `analysis/.venv` foi
  removido; deps: capstone, pefile, **ziglang no venv** — proxy não depende mais
  do Python global; pytest no grupo `test`).
  Comandos: `uv sync --all-groups` (recria venv idêntico), `uv pip install ...`,
  `uv run pytest` (sempre da raiz).
- **Rust:** instalado via winget (rustup 1.29.1, rustc 1.98.1). Toolchain pinado
  em `rust-toolchain.toml` como **stable-x86_64-pc-windows-gnu** — o host não tem
  MSVC Build Tools e o `link` do PATH é o GNU coreutils do Git Bash; o alvo gnu
  usa o linker MinGW embutido no rustup (zero dependência externa).
  ✅ PATH DO SISTEMA RESOLVIDO 29/09: `C:\Users\Canta\.cargo\bin` adicionado ao
  PATH de MÁQUINA (HKLM) via `scripts/add_cargo_path.ps1` (idempotente; preserva
  REG_EXPAND_SZ e faz broadcast WM_SETTINGCHANGE). Workaround de sessão
  (`export PATH=...`) só é necessário em terminais abertos ANTES do fix.
- **O quê (feito):** workspace `gp100-core` (lib) + `gp100-cli` (bin);
  `rust-toolchain.toml`; workspace deps `serde`/`serde_json`/`thiserror`;
  feature `real-device` declarada (default = mock, política de hardware);
  `.gitignore` (`/target`); smoke test valida o header SysEx contra o §13.1;
  CLI já bloqueia `--real` sem `--i-know-what-im-doing`.
- **DoD:** ✅ `cargo build && cargo test && cargo clippy --workspace --all-targets --
  -D warnings && cargo fmt --check` verdes; `uv run pytest` 9/9.

### P3. Suíte de regressão Python (trava da especificação) — ✅ FEITO 28/09
- **Responsável:** skill `protocol-validate` · **Estimativa:** 1h
- **O quê:** `analysis/tests/test_protocol.py` (pytest) executando:
  `validate_golden.py` (5 provas, exige 100% em B–E) e `validate_knob_map.py`
  (13/14 + 1 exceção conhecida), com exit code correto p/ CI.
  **+ teste de hash da baseline** (golden não muda sem bump formal).
- **DoD:** ✅ `uv run pytest` = 9/9 verde em ~4s. Rodar antes de qualquer PR que
  toque spec ou decoders.

### P4. Fixtures de replay (capturas segmentadas) — ✅ FEITO 28/09
- **Responsável:** skill `capture-analyze` (parte de extração) · **Estimativa:** 1–2h
- **O quê:** gerar `analysis/fixtures/` com as 4 capturas fatiadas por fase
  (boot/scan, edits, save, ir-upload) em JSON compacto por transação
  (payloads hex), a partir dos loaders existentes — consumidas pelos testes de
  replay do M0 sem tocar os `.jsonl` originais.
- **DoD:** ✅ `analysis/make_fixtures.py` → `analysis/fixtures/` (boot/knobs/save/ir
  em JSONL compacto + `manifest.json` com paridade e sha do golden). Paridade 100%:
  boot 2299 OUT (4597 msgs; 10 SEM-HDR excluídos), knobs 89+3, save 77,
  IR 1186 (2 BEGIN + 592 chunks + 592 ACKs). Capturas `.jsonl` intocadas;
  gate no pytest (`test_fixtures_parity`).

### P5. Decisões de design pré-assinadas (ADR-lite, 1 página) — ✅ FEITO 28/09
- **Responsável:** Buffy + `core-dev` (rascunho) · **Estimativa:** 30 min
- **O quê:** registrar em `docs/DECISIONS.md` as escolhas que evitam debate no meio:
  1. endian/nibble: fio usa pp/PG/CRC **BE**, effectCode e float **LE**, payloads
     de objeto **nibble-expandidos** (hi primeiro) — com exemplos do golden;
  2. erros: `thiserror` com `ProtocolError` tipado (shape inválida, timeout,
     ack inesperado);
  3. transporte síncrono com timeout por transação (3s, como o pairing do golden);
  4. `DeviceTransport` trait: `send_raw`, `recv_raw(timeout)`, `open`, `close`;
  5. feature-flag `WRITE_VERIFIED` só afeta transporte REAL (mock sempre permite).
- **DoD:** ✅ `docs/DECISIONS.md` (ADR-1..5) e citado no `gp100-core/src/lib.rs`
  (README do workspace vem no M0.8).

### P6. Checklist "pronto para codar" — ✅ FEITO 28/09
- **DoD da fase:** P1–P5 concluídos + `docs/INDEX.md` e `knowledge.md` apontando
  para este roadmap. A partir daqui **zero paradas de preparação são permitidas**;
  qualquer descoberta nova entra pelo fluxo R3, nunca por patch ad-hoc no código.
- **Status:** ✅ Fase P 100% (P0–P5). Próximo passo: M0.1 (modelo do dicionário).

---

## FASE M0 — gp100-core (só lógica, só mock)

### M0.1 Modelo de dados do dicionário — ✅ FEITO 29/09
- **Responsável:** skill `core-dev` · Depende: P1, P2 · **Estimativa:** 2–3h
- **O quê:** structs serde (`Algorithm`, `Control`, `Dictionary`) lendo
  `parameters.json`; validação de integridade na carga (códigos únicos, ranges
  min<max); embed do JSON via `include_str!`.
- **DoD:** ✅ `gp100-core/src/model.rs`: carrega 185 algs/639 controles;
  rejeita corrompido (4 testes de rejeição); lookup O(1).
  **Achados estruturais (R1, documentados no módulo):** identidade do alg é a
  tripla `(module,nibble,index)` — Boost/14 Boost existem em PRE **e** DST com
  defaults divergentes (Bright "1" vs "0"); knobs bidirecionais vêm min>max
  (Pitch.L-Pitch 0..-24 → `Control::range()` normaliza); `observed_*`
  ausente/NULL em 68 algs; `default` é string. Lookup: `algorithm_in_module`
  (primária) + `algorithm` (fallback first-wins, semântica do knob_map).
  `ProtocolError` (ADR-2) nasce aqui. 11 unit + 1 doc-test, clippy -D, CI verde.

### M0.2 Modelo `.prst` com round-trip byte-idêntico — ✅ FEITO 29/09
- **Responsável:** skill `core-dev` · Depende: M0.1 · **Estimativa:** 4–6h
- **O quê:** modelo completo do XML (preset_info, presets/pp*, Effect params_0..14,
  ppCtrl, ppEXP1, ppIRInfo) com preservação de atributos desconhecidos; writer que
  reproduz byte-a-byte o arquivo original.
- **DoD:** ✅ `gp100-core/src/preset.rs`: parser/writer que trata LAYOUT COMO DADO
  (ordem de attrs, quebras dentro das tags e indents são registrados e reproduzidos;
  sem algoritmo de wrap adivinhado — R1). Round-trip byte-idêntico provado nos 3
  `.prst` (`tests/roundtrip_prst.rs`: 8 contratos — all.prst=99 presets, double-parse,
  edição estável, `Dub&amp;Vibe` verbatim, strict de dialecto). `.prst` agora com
  `-text` no .gitattributes (mesma lição EOL do golden). Views tipadas
  PresetView/EffectView sem esconder ppCtrl/ppEXP1.

### M0.3 Consumidor do golden-file — ✅ FEITO 29/09
- **Responsável:** skill `core-dev` · Depende: P1, P2 · **Estimativa:** 3–4h
- **O quê:** parser do `protocol_golden.json`; gerador de request por template
  (substitui segmentos `var`); matcher de resposta (aceita por len+const);
  API: `Template::build_request(vars) -> Sysex`, `Template::matches(data) -> Option<Vars>`.
- **DoD:** ✅ `gp100-core/src/golden.rs` + `tests/golden_consumer.rs` (8 contratos):
  propriedade "extract → build == exemplo byte a byte" PROVADA para os 40
  templates; despacho by-len (ACK 4B × tabela 75B × resync no mesmo endereço
  `12001002`); build strict; `GoldenFile::embedded()` (OnceLock);
  `decode_envelope` com trim no 1º F7. Achado documentado: `example.*` do
  golden são PAYLOADS (não SysEx).

### M0.4 Codec de fio — ✅ FEITO 29/09
- **Responsável:** skill `core-dev` · Depende: M0.3 · **Estimativa:** 3–4h
- **O quê:** envelope SysEx (`F0 21 25 7F 47 50 2D 64 | FUNC | ADDR | DATA | F7`),
  trim no 1º F7, detecção de truncamento (SEM-HDR), nibble expand/collapse,
  helpers semânticos: `set_param(slot, effect_code, ctrl, value)` (§13.11),
  `meta_block(pp, ppType, name)` (§13.12), `ir_begin(slot)`, `ir_chunk(slot, idx, data)`
  (§13.7), decodificador de página de User IR (§13.12, nome+CRC).
- **DoD:** ✅ `gp100-core/src/codec.rs`: primitivas nibble expand/collapse STRICT
  (par ímpar ou nibble >0x0F → `InvalidShape`) + helpers semânticos PROVADOS byte a
  byte contra as fixtures P4 em `tests/codec_wire.rs`: 92 knobs (§13.11), 2 saves
  (§13.12), 1186 frames IR (§13.7, último chunk dup 0x0226 slot 1) + roundtrip de
  nibble em chunk real; erros de shape tipados (ProtocolError); `WireWrite =
  ([u8;4], Vec<u8>)` = forma canônica dos blocos que a FSM envia. Trim no 1º F7 e
  SEM-HDR já vivem no golden (M0.3); página de User IR 13xx deliberadamente FORA
  (o mock do M0.5 precisa só do shape); regras de SEQUÊNCIA da FSM (ordem de writes,
  quirk de re-leitura, salto de página) = M0.6. 6 unit + 4 contratos;
  fmt/clippy -D warnings/testes/CI verdes (commit 6422cd1).

### M0.5 Transporte + mock device — ✅ FEITO 29/09
- **Responsável:** skill `core-dev` (mock) · Depende: M0.4, P5 · **Estimativa:** 4–6h
- **Progresso pré-issue (29/09):** esqueleto PRONTO e CI verde —
  `gp100-core/src/transport.rs` com a trait `DeviceTransport` (ADR-4
  verbatim) + `TransportError` tipado (Closed/OpenFailed/SendFailed/
  **DeviceGone**/RecvTimeout, com semântica de reconexão no mesmo objeto);
  6 contratos em `tests/transport_trait.rs` (trait pub/object-safe via
  implementador externo). Restam: `MockDevice` (contrato D1–D8 do ADR-6)
  e `RealDevice` feature-gated.
- **O quê:** `DeviceTransport` trait ✅ (esqueleto); `MockDevice` que responde conforme o golden
  (boot: tabelas/scan §13.10; página de estado; ACK de chunk; SEM resync pós-save
  — D3 do ADR-6: save é fire-and-forget; burst de fim de sessão não é emitido);
  `RealDevice` (midir/WinMM/ALSA) **atrás de feature** e desabilitado por default.
- **DoD:** ✅ diálogo completo mock↔codec numa sessão sintética: boot → scan 198 pp →
  set param → save → upload IR 2 slots, sem timeouts.
  `transport/mock.rs`: estado derivado de `all.prst` (99 presets, 20 ppIRCRC de
  fábrica do container `<ppIRInfo>` RAIZ — achado M0.5) + dicionário; despacho por
  `match_request` (D5); respostas via **`Template::build_response`** (API nova,
  simetria provada nos 15 exemplos IN); ACK por chunk; fire-and-forget (D3);
  `queue_push` p/ push intercalado (D7/D8). **Achados:** golden congela
  `set_param` por instância (9 templates 10xx0002) → shape validado pelo CODEC;
  meta6 (t6) é const (pp não entra); scan 1302 não abre com pp; by-len decide pelo
  endereço da RESPOSTA. `RealDevice` fica p/ pós-H (feature `real-device`).

### M0.6 FSM de sessão + testes de replay — ✅ FEITO 29/09
- **Responsável:** skill `core-dev` + `protocol-validate` no fim · Depende: M0.5, P4 · **Estimativa:** 4h
- **O quê:** `Session::{boot, scan_state, set_param, save_preset, upload_ir,
  list_user_irs}` ✅ implementados sobre o golden com as regras D1–D8 do
  ADR-6 rev.3 (transação dona do endpoint; espera em LOOP FILTRANTE com
  backlog de pushes; save fire-and-forget; sem retry; InvalidShape tipado);
  testes de replay em `tests/replay_fixtures.rs` (ReplayTransport compara
  func+addr+payload hex dos OUTs com a captura e devolve os INs na ordem).
- **DoD:** ✅ replay 100% das 4 fixtures — boot 2299/2299 OUTs byte-a-byte
  (framing; páginas geradas e dados de chunk do blob divergem por desenho,
  filtrados com tag), knobs 92/92 e save 9/9 byte-a-byte EXATOS, IR
  594/594 OUTs + 592 ACKs no framing. Divergência = falha com diff hex.
  **Achados (provados no replay):** ciclo do pp atual 0x0100 = 2 selects →
  2 meta6, 2 opens → PÁGINA 0 duas vezes (não 3º meta6), reqs pg0..7 →
  pág1..8, pg8 → IN 13010005; T3 nomes é FIRE-AND-FORGET (61 leituras, 57
  respostas — device omitiu 4 do banco 00 [idx 01/03/06/0a]; D4/D7);
  select da sonda 1302 é CONST "0000" (sem pp); template 11000008 é mixed
  2 vars + const 0000; T1 tem 1 resposta de tabela que chega TARDIA (após
  o 1º select do scan) — backlog D7 absorve. `GP100_TRACE=1` dumpa o
  wait_for (diagnóstico).

### M0.7 gp100-cli — ✅ FEITO 29/09
- **Responsável:** skill `core-dev` · Depende: M0.6 · **Estimativa:** 2–3h
- **O quê:** ✅ os 5 subcomandos contra o MOCK (`info`, `list-user-irs`,
  `dump-preset <pp>`, `set-param --dry-run`, `save --dry-run`) + `--log <arq>`
  gravando TODOS os frames no MESMO schema das fixtures P4 (requisito do
  `docs/H1_CHECKLIST.md`) + política de hardware em camada única efetiva:
  `--real` é recusado SEMPRE nesta fase (exit 2), mesmo com
  `--i-know-what-im-doing`; escrita efetiva = pós-H2 (WRITE_VERIFIED).
  Parser zero-dep decidido na issue (clap adiado — superfície pequena;
  reabrir na M1 com ADR-lite). **Extensões no core:**
  `Session::{select_preset, state_page}` (rotas §13.10 do replay) e MOCK
  completado para o pareamento D1 provado pela captura: select→meta6 com o
  pp (199↔199), página ecoa [pp][PG], PG8 → 4B em `13010005`, tabela ecoa
  a página pedida — gaps que o SMOKE do CLI expôs (a M0.5 só provava
  TAMANHO das respostas). 9 testes do CLI (parser/política/logger P4) +
  2 do mock atualizados à evidência; 74 testes, clippy/fmt/pytest verdes.
- **DoD:** ✅ todos os subcomandos funcionam no mock (smoke do binário);
  `--help`/usage documenta a política (stderr + exit 2 nas violações).

### M0.8 Documentação do core — ✅ FEITO 29/09 · **FASE M0 100%**
- **Responsável:** agente principal (Buffy) + `docs-sync` do INDEX · Depende: M0.7 · **Estimativa:** 1h
- **O quê:** ✅ `gp100-core/README.md` (arquitetura por fases, fontes de
  verdade, como construir/testar, exemplos por camada, CLI, modelo de testes,
  erros/convenções) + **contrato `tests/readme_examples.rs`** que executa os
  exemplos da doc 1:1 (doc que mente quebra o `cargo test` — pegou na revisão
  um import faltante no exemplo da Session). Onboarding do README raiz
  revisado (INDEX primeiro; hands-on de 10 min com o CLI).
- **DoD:** ✅ um contribuidor novo compila, testa e entende o core em <15min
  (README raiz → INDEX → README do core → hands-on).

## FASE ACHADOS — CORREÇÕES E MELHORIAS (PRIORIDADE MÁXIMA)

> **Regra (decisão do owner, 29/09):** achados de reviews/auditorias, warns de
> versão e vulnerabilidades de deps **entram nesta fase com prioridade MAIOR
> que qualquer issue de fase (P/M/H/M1+)** e são resolvidos ANTES de abrir a
> próxima issue de roadmap. Encontrou → registra aqui com ID (A-xx) + prova →
> corrige → commit próprio → CI verde.

### A-1. Vulnerabilidades npm do front (vitest/@vitest/mocker) — ✅ FEITO 29/09
- **Origem:** `pnpm audit` (2 moderadas, GHSA-82fw-gwwq-j7x9) · **Prioridade:** máxima
- **O quê:** ✅ upgrade das 7 dev-deps atrasadas (vitest 3→5, vite 7→8, eslint
  plugins, jsdom, globals) + `@types/node`; audit **0 vulnerabilidades**;
  gates do front verdes (10/10 testes, lint, build tsc+vite).
  **TypeScript fixado em 6.0** (o TS 7.0 quebra o typescript-eslint 8.x —
  exceção documentada; subir os dois juntos quando o plugin suportar ≥7.1).

### A-2. Fundação de UI do M1.0 (front + design system + CI) — ✅ FEITO 29/09
- **Origem:** revisão de estado · **Prioridade:** máxima
- **O quê:** ✅ front `ui/` (React 19 + Vite 8 + Vitest 5 + ESLint 10 + TS 6.0) com
  design system da paleta palco Valeton (`docs/UI_DESIGN.md`: tokens de
  Fibonacci, contraste AA medido, identidade "pedalboard ao vivo"), skill
  `ui-ux-practices`, testes de token/a11y/estado e job `ui` na CI.

### A-4. Completar o spike M1.0 — workspace `src-tauri` + `device_info` — ✅ FEITO 30/09
- **Origem:** auditoria `ls src-tauri` (ausente) · **Prioridade:** máxima
- **O quê:** ✅ `src-tauri/` (crate `gp100-ui`, Tauri 2) com o command
  `device_info` contra o MockDevice (DTO camelCase = `ui/src/ipc/types.ts`,
  testado contra o mock real) e o front `ui/` consumindo via `invoke` com
  fallback mockado. **ADR-7:** o Tauri 2 não suporta windows-gnu (build
  script morre com STATUS_ACCESS_VIOLATION no pin da casa) — crate FORA do
  workspace, toolchain MSVC própria (pin local) validada pela CI.
- **DoD:** ✅ CI verde nos 3 OSes (clippy/test/build do shell; Windows prova
  o Tauri em MSVC). `pnpm tauri dev` no host exige MSVC local (instalação
  é decisão do owner); até lá, dev da UI = `pnpm dev` no browser (fallback
  mock do ipc) + CI.
- **Depois dela:** M1.1 (DeviceActor + boot).

### A-5. Baselines visuais win32 desatualizadas + visual SKIPado no CI — 🔴 aberto (issue #45)
- **Origem:** `pnpm e2e` local na árvore LIMPA enquanto a #20 fechava (02/10) ·
  **Prioridade:** alta (achado)
- **O quê:** 12 baselines visuais falham no local (topbar ×7 viewports, board
  1280/1024/800, lib 1280, looper 800). Prova de que é PRÉ-EXISTENTE: com todas
  as mudanças da #20 revertidas (`git stash`) o `topbar 1440×900` falha
  IDÊNTICO (4129 px, ratio 0.06). O diff é redistribuição horizontal do banner
  (mesmos elementos, espaçamento diferente) e `e2e/__screenshots__` foi
  commitado por último em `6366b08` (01/10 15:14), ANTES da mudança visual do
  TopBar/App em `3bb6a3e` (01/10 19:12).
- **Buraco maior:** as baselines são por plataforma e o repo tem **54 `-win32`
  e 0 `-linux`** → no ubuntu o `skipIfBaselineMissing` SKIPA todo o visual (a
  regressão estética não é gate de nada no CI).
- **DoD:** baselines win32 regeneradas + linux geradas pelo input
  `update-snapshots` (artefato → commit) + `e2e-visual` comparando nas duas
  plataformas.

### A-3. Toolchain/versões base — ✅ FEITO 29/09
- **Origem:** `cargo update --dry-run` + `pnpm outdated` · **Prioridade:** máxima
- **O quê:** ✅ Rust stable 1.98.1 (atual), crates sem updates pendentes no
  lockfile (midir 0.9.1, serde 1.0.229, thiserror 2.0.21 atuais); front nas
  releases atuais pós A-1. CI valida Windows (2 jobs) + Linux + macOS + UI.

## FASE H — GATE DE HARDWARE (só após M0 100% — ✅ **M0 FECHADA 29/09**)

> **Pré-requisito de software do H1 FECHADO (29/09):** `RealDevice` implementado
> (`transport/real.rs`, midir/WinMM, feature `real-device` default OFF) e o CLI
> de campo compila com `--features real-device` — `--real` abre o device com
> dupla confirmação; escrita segue bloqueada (H2/WRITE_VERIFIED). Falta só a
> pedaleira + owner (roteiro: `docs/H1_CHECKLIST.md`; kit: `scripts/h1_field.sh`).

> ⚠️ **Paralelismo:** a **FASE M1 (Editor UI) pode começar em paralelo** — ela
> roda contra o MOCK (política de hardware, ADR-4/ADR-5). O modo real da UI/CLI
> só existe após H1/H2. O planejamento issue-a-issue da M1 está em
> **`docs/UI_PLAN.md`** (M1.0–M1.6 com DoD, arquitetura DeviceActor, política
> de escrita na UI e estratégia de testes). **MAIS UMA VEZ: achados da FASE
> ACHADOS (A-xx) têm prioridade máxima e precedem qualquer M1.x.**

### H1. Primeiro contato real (somente leitura)
- **Responsável:** owner no hardware + skill `capture-analyze` p/ divergências · Depende: M0.7 · **Estimativa:** 1h
- **O quê:** `gp100-cli --real info` + `list-user-irs` + `dump-preset` com a
  pedaleira; log de divergência vs mock. **Operacional passo-a-passo:
  `docs/H1_CHECKLIST.md`** (pré-requisitos, níveis de comparação
  framing×estado, log de divergência, fluxo R3, critérios de saída).
- **DoD:** leitura real idêntica ao mock; divergências → fluxo R3.

### H2. Escrita real dos 3 fluxos capturados
- **Responsável:** owner no hardware (confirmação no display) + `protocol-validate` · Depende: H1 · **Estimativa:** 1–2h
- **O quê:** `set-param` (knob), `save`, `upload-ir` no device real, um por vez,
  com read-back/verificação display (como na S4).
- **DoD:** 3 fluxos verificados em campo → flip `WRITE_VERIFIED=true` no transporte
  real (já sancionado pelo BLOCKERS item 11).

### H3. Congelamento pós-hardware
- **Responsável:** skill `spec-baseline` + `capture-analyze` · Depende: H2 · **Estimativa:** 1h
- **O quê:** nova captura com o gp100-core no fio → golden v1.1 se houver ajuste.
- **DoD:** baseline atualizada + validate 100%.

---

## 📋 Pendências de planejamento (documentos por criar, na ordem)

- ~~**Checklist operacional H1**~~ ✅ **CRIADO 29/09**: `docs/H1_CHECKLIST.md`
  (pré-requisitos, procedimento de campo, níveis framing×estado, log de
  divergência, fluxo R3). Requisito de campo novo identificado: log de fio no
  CLI (`--log`) no schema das fixtures P4 — entrar na M0.7/prep-H1.
- **Planejamento fino da M2** (IR lab, SnapTone manager, banco da biblioteca
  [SQLite], empacotamento) → **epic #15** com as filhas #24–#26.
- **M3** fica no nível de panorama (VISION §9) até o gate H passar → **epic #16**.
- **ADR-7+** à medida que os spikes fecharem as decisões em aberto do
  `docs/UI_PLAN.md` §9.
- **Distribuição/instaladores** (Windows/macOS · `.deb` · Arch) → **epic #17**
  com as filhas #27–#29.

## FASE U — UI POR ETAPAS (casca → pedais; decisão do owner 30/09)

> **Fonte de verdade desta fase: `docs/UI_REFERENCE.md`** (inventário de achados,
> capturas do app oficial, checklist vivo da fase dos pedais e plano §4).
> Executa a M1.2/M1.3 do `docs/UI_PLAN.md` **por etapas aprovadas pelo owner**:
> cada etapa termina com **teste manual** (roteiros em `docs/UI_TEST_PLAN.md`)
> e aprovação antes da próxima.

### U-1. Casca da UI + navegação de presets de fábrica (sem pedais) — ✅ FEITO
- **Responsável:** Buffy (UI) · Depende: A-4, M1.1 · **Estimativa:** 1 sessão
- **O quê:** organização geral da UI no layout do oficial (referencial §1 de
  UI_REFERENCE): topbar (logo, conexão, Stomp/DRUM/Master VOL prévia, ⚙),
  biblioteca com os **99 presets de fábrica** (nome/tipo do all.prst; abrir =
  REAL §13.10), coluna do patch (◀ ▶ + nome), **board VAZIO** com os 9 lugares
  marcados (PRE/DST/AMP/NR/CAB/EQ/MOD/DLY/RVB) e trava "⇄ mover" no header.
  NENHUM pedal desenhado nesta fase.
- **DoD:** navegação completa de presets funcional (busca, setas, abrir), board
  vazio com lugares definidos, roteiro de teste manual executado, aprovação do
  owner, CI verde (tsc/lint/vitest/build).

### U-2. Modal Settings (6 abas) + trava do drag — ✅ FEITO (trava virou o cadeado 🔒/🔓 no palco)
- **Responsável:** Buffy (UI) · Depende: U-1 · **Estimativa:** 1 sessão
- **O quê:** modal Settings com as abas do oficial (General/Global EQ/About/Info
  Frame/Help/Release Note); General persiste LOCAL (localStorage, "prévia
  local" — escrita global só pós captura G3–G6); Global EQ placeholder com 5
  páginas; trava do drag-and-drop integrada ao header (drag só com ela ATIVA).
- **DoD:** modal navegável por teclado (Esc fecha, foco preso), aba General
  persiste local, CI verde + teste manual.

### U-3. Pedais fase 2 (1 cadeia por entrega; 1 efeito por vez) — 🔜 **issue #19**
- **Responsável:** Buffy (UI) · Depende: U-1/U-2 aprovados · **Estimativa:** várias sessões
- **O quê:** adicionar os pedais um por um na ordem da cadeia (PRE → DST → AMP
  → NR → CAB → EQ → MOD → DLY → RVB): modelagem SVG por variante, knobs do
  dicionário com valor editável (textbox), modal de edição do pedal, LED
  verde/vermelho. Cada efeito é validado ISOLADO (só ele no board) e depois em
  TODAS as 9 posições (tamanho/espaçamento/linhas) antes do próximo — checklist
  completo em `docs/UI_REFERENCE.md` §3.
- **DoD:** cadeia completa entregue após rodadas de teste manual aprovadas;
  knobs sem sobreposição em qualquer posição; travinha ativa para reordenar.

### U-4. Roteiros de teste manual — ✅ FEITO 30/09 (+ Playwright)
- **Responsável:** Buffy (docs) · Depende: U-1 · **Estimativa:** 2h
- **O quê:** ✅ `docs/UI_TEST_PLAN.md` com roteiros numerados por etapa (casca,
  settings, cada pedal da fase U-3, posição do board, presets/fábrica, teclado)
  **+ Playwright implementado** (`packages/app/ui/e2e/shell.roteiros.spec.ts`):
  R1–R6 + drum/looper executados no Chromium (8/8 ✅, rodada 30/09 registrada na
  doc). Achado corrigido: alvo <32px no PushLog.
- **DoD:** ✅ roteiro executável pelo owner sem contexto do chat; achados voltam
  para `docs/UI_REFERENCE.md` §3; `pnpm e2e` na suíte da UI.

### U-5. E2e completo da casca na CI + cobertura do manual — ✅ FEITO 30/09
- **Responsável:** Buffy (UI) · Depende: U-1/U-2/U-4 · **Estimativa:** 1 sessão
- **O quê:** ✅ job `e2e` no pipeline (Playwright contra o `pnpm dev` no Chromium;
  mesmo filtro do front) e como **gate do release** (`tag-release` precisa dele).
  Suíte: R1–R6 + drum + looper + atalhos globais + responsividade (3 viewports)
  = **14 e2e ✅ · 23 unit ✅**. Achados corrigidos: BPM do drum com tamanho
  divergente (258×46 vs 240×32 — UA só aplica border-box a select; `.gp-num`
  ganhou `box-sizing`), navegação ◀ ▶ do manual implementada, redundâncias do
  looper removidas (estado da fita numa linha só). Matriz de cobertura:
  `docs/MANUAL_COVERAGE.md` (33 ✅ · 5 🟡 · 5 🔴 — nada silencioso).
- **DoD:** ✅ e2e verde na CI como condição de tag; cobertura do manual explícita;
  alinhamento dos componentes travado por teste (responsivo + BPM).

### U-6. Regressão estética dos painéis (toHaveScreenshot) — ✅ FEITO 30/09
- **Responsável:** Buffy (UI) · Depende: U-5 · **Estimativa:** 1 sessão
- **O quê:** ✅ `e2e/visual.spec.ts` — 9 baselines (board/looper/biblioteca ×
  1440/1280/1024) com `toHaveScreenshot`: animações congeladas, tolerância 1% e
  **baseline por plataforma** (fontes divergem Win/Linux — template
  `{arg}-{platform}`). Sem baseline no CI = SKIP (não quebra a 1ª execução);
  local sem baseline = cria sozinho; input `update-snapshots` do workflow
  gera as baselines linux e sobe artefato para commit. Validadas localmente
  (win32, 2 rodadas idênticas).
- **DoD:** ✅ job `e2e-visual` na CI; promoção p/ gate de release assim que as
  baselines `linux` forem commitadas (1º run do input update-snapshots).

### U-7. Smoke do shell Tauri real na CI (tauri-driver) — ✅ FEITO 30/09
- **Responsável:** Buffy (UI) · Depende: U-5, ADR-7 (sobe no CI) · **Estimativa:** 1 sessão
- **O quê:** ✅ job `e2e-tauri` (Ubuntu): build debug do `gp100-ui` com o
  `ui/dist` embutido (backend mock, sem hardware) → `tauri-driver` +
  `WebKitWebDriver` sob `xvfb-run` (receita oficial: deps
  `webkit2gtk-driver`+`xvfb`, `WEBKIT_DISABLE_DMABUF_RENDERER=1`) → Selenium
  (`e2e/tauri.smoke.mjs`): banner no webview, 3 painéis no DOM e a biblioteca
  com os 99 presets REAIS dentro do webview. Nota: roda quando front OU rust
  mudam; não é gate de release ainda (amadurecer 1º) — promover depois.
- **DoD:** ✅ prova de que a casca sobe no WEBVIEW na CI, não só no Chromium.

### FASE Q — QUALIDADE DA UI (code review do owner, 30/09)

> Origem: review do owner — campos falsos na UI (Noise Gates inexistentes),
> nome do preset congelado nos displays, vocabulário interno visível ao
> usuário e "os testes por que não pegaram?". Estes issues NÃO reabrem
> discussão: são dívida técnica nomeada, com prioridade antes de novos
> recursos de UI.

- **Q-1. Vocabulário interno fora da UI** — ✅ FEITO 30/09
  - ✅ Limpo: "fase 2", "(D7)", "captura G3–G6", "Fase M", "gate H1",
    "U-1/U-2", "§13.12", "docs/MANUAL_COVERAGE.md" nos textos da UI;
    mensagens centralizadas em `src/i18n/messages.ts` (fonte única, base do
    i18n de 4 idiomas que o próprio Settings já prevê).
  - ✅ Varredura exaustiva: TODO componente migra para MSG (TopBar,
    ConnectionBar, Library, EmptyBoard, Drum, PushLog, Looper, Settings, App,
    Pedal, Pedalboard, PresetCase + decorativos AmpHead/Knob); achei e migrei
    resíduos que a 1ª passada deixou (aria do pedal/rolo, "capstan", placa
    "GP-100", "0x" do pp, ternários "ok?"/"✕").
  - ✅ Lint custom `local/no-user-literals` (eslint.config.js, `error` em
    src/components/** + App.tsx): trava JSXText com letras, literais em
    title/placeholder/aria-label/label/alt, templates sem interpolação e
    ternários com ramo literal. Gates: 0 violações (unit 27/27, e2e 32/32).
- **Q-2. Campos falsos na UI (não existem no device)** — ✅ FEITO 30/09
  - Noise Gate (1)/(2) e Noise Mode removidos do Settings (nada de noise no
    manual/firmware extraído); APP Language agora DESABILITADO com "disponível
    em uma próxima versão" (o select ativo que não trocava idioma era o mesmo
    engano); unit test trava a ausência dos fantasmas.
- **Q-3. Nome do preset congelado (fallback ignorava o pp)** — ✅ FEITO 30/09
  - `localMockBoard(pp)` devolvia sempre "It's GP100" — navbar, LED do board
    e ConnectionBar não acompanhavam a seleção. Fix: fallback usa o MESMO
    artefato da biblioteca (presetData). Corrigido em conjunto o pp/ppType
    do ConnectionBar (pp fixo 0x0000).
  - Lição de teste: os asserts verificavam só o NÚMERO (`/^P25 /`) — nunca o
    nome. Agora e2e cobre o NOME COMPLETO nos 3 locais (biblioteca → navbar,
    ◀ ▶ → navbar+LED+seleção da lista).
- **Q-4. Efeito colateral dentro de updater de estado** — ✅ FEITO 30/09
  - `stepPreset` chamava `openPreset()` dentro do updater do `setPp`
    (updater tem que ser puro; StrictMode executa 2×). Fix: updater puro +
    `useEffect([pp])` abre o preset corrente.
- **Q-5. Varredura completa de dados estáticos/mock** — ✅ FEITO 30/09
  - Auditoria dos 3 fallbacks (info/board/lib) contra os artefatos gerados:
    nome/tipo do corrente e a biblioteca já vinham do all.prst; o board usa
    knobs/ranges do dicionário, mas os 9 `code` eram transcritos à mão e
    6/9 divergiam do nibble/index reais (só o Bog RedM — validado por
    captura — coincidia). Fix: code DERIVADO do artefato (nibble<<24 | index).
  - Números exibidos ao usuário (About/Release Note/drum do painel: 99
    presets, 87 ritmos, 185/639 do catálogo) agora são derivados dos
    ARTEFATOS (FACTORY_PRESETS, DRUM_GENRES, FX_MODULES) — regenerar o
    dicionário atualiza a UI (regra §8-3 do UI_REFERENCE de verdade).
  - Duplicações eliminadas: a lista PRE…RVB existia em 3 cópias (2 em
    device.ts + 1 em EmptyBoard) → `CHAIN_FAMILIES`/`ARCHETYPE_OF` em
    ipc/types.ts (fonte única); tipo `DrumBeat` órfão removido.
  - Revisão pré-Fase 2 de Pedal/Pedalboard/Knob: sem dado mock hardcoded —
    knobs/nomes/codes chegam do board; restam só formato de payload SET e
    dimensões de layout (protocolo/arte, não dado do device).
  - Hardcoded deliberadamente mantido (mock de Fase 1, sem captura que o
    valide): irSlotsWithCrc=20, 90/45 s do looper, 2297 transações, V2.1,
    ppTypeName do corrente. Unit trava o que já é derivável (87 ritmos).
- **Q-6. Cobertura de interação ponta-a-ponta da casca** — ✅ FEITO 30/09
  - `e2e/interacoes.spec.ts`: master VOL (display numérico + reload), drum
    volume/speed, looper Rec/Play/P-VOL + rota PRE/POST, kill switch por
    teclado e Settings campo a campo (input/normal level, USB Audio, Hint
    Mode, Tap Tempo) — cada campo muda → UI reflete → persiste (localStorage
    comprovado e reload restaura). Regra Q-3 aplicada: assert do valor
    TROCADO, nunca só do inicial.
- **Q-7. Error states e empty states sem cenário** — ✅ FEITO 30/09
  - Gancho de teste `gp100.debug.failDevice` (="info"|"boot"|"board"|"all")
    no fallback de `src/ipc/device.ts` — fora do fallback (webview real)
    não tem efeito; sem a chave, custo zero.
  - Unit `tests/device.fail.test.ts`: deviceInfo rejeita com a flag (info/all)
    e segue mock sem ela.
  - `e2e/estados.spec.ts` (flag setada antes do load via addInitScript):
    info falha → casca de pé + "Device desconectado"; boot falha →
    role="alert" amigável (MSG.connBootError, sem stack técnica); board
    falha → banner amigável (MSG.errOpenPreset); "all" → nenhum crash.
- **Q-8. Comentários/docstrings de código apontando para o chat** — ✅ FEITO 30/09
  - Varredura exaustiva de src/, tests/ e e2e/ (artifacts/ gerados ficam de
    fora): zero referências a decisões de chat, datas de rodada ou issues em
    comentários — cada um agora explica o PORQUÊ técnico local; as decisões
    continuam documentadas nos docs vivos (ROADMAP, UI_REFERENCE §8, TEST_PLAN).
  - Bônus da varredura: 1 vazamento REAL de vocabulário interno ao usuário —
    o tooltip do modo engenheiro do Knob exibia "SET §13.11"; agora mostra
    "SET" + addr/code/ctrl/payload.
- **Q-9. Revisão Rust (core/api) sob as mesmas lentes** — 🟢 BAIXA · saudável
  - Evidência: 0 `unwrap` fora de testes no core/api, 0 TODO/FIXME real,
    gates clippy -D warnings na CI. Manter o padrão; nada a fazer agora.

## 🎨 FASE V — UI/UX enterprise (plano do owner 30/09)

> Alinhamento já respondido pelo owner: drums = drawer inferior (A);
> coverage mínimo 85%; `/deprecated_docs` entra no .gitignore ao final;
> merge de PR = revisão do agente (CI verde, sem comentários) + comando do usuário.

- **V-1. Tema "Valeton Violet" (dark + light)** — ✅ FEITO 30/09
  - Paleta do acabamento jewel violet do GP-100VT (pesquisado no site
    oficial): obsidiana `#0C0910` + violeta anodizado `#7C3AED` e brilho
    neon `#9333EA` — visual vivo/brilhante de mesa tecnológica, não retro.
  - Tokens novos: `--border`, `--accent-text` (TEXTO AA sobre escuro),
    `--on-accent` (texto sobre violeta), `--silver`/`--steel`; contrastes
    medidos e travados em tokens.ts (todos ≥ 4.5:1; on-accent 5.5/7.6).
- **V-2. Hierarquia do shell (looper no topo, sem lacunas)** — ✅ FEITO 30/09
  - Ordem no App: looper full-width em cima; abaixo, biblioteca (300px,
    scroll interno, `minHeight: 0`) à esquerda e pedalboard com
    `align-items: stretch` — alturas iguais, zero lacuna vertical;
    empilhado ≤1100px o palco volta ao topo via CSS `order`.
  - Asserts e2e novos: lib ≤300px, à esquerda do board, topos alinhados.
- **V-3. Navbar consolidada** — ✅ FEITO 30/09 · ampliado (seção "Conexão" REMOVIDA)
  - Faixa "backend simulado (mock)…" removida da ConnectionBar; vira badge
    `Mock Device` com LED (âmbar=mock, verde=conectado) no cluster de conexão.
  - Mover/Kill/⚙ integrados à navbar à direita, botão vidro (classe
    `btn-glass`: 34px fixos, blur sutil, borda violeta, hover com glow).
  - **Consolidação total (pedido do owner):** a seção permanente "Conexão"
    da página foi apagada — status, badge e BOOT vivem só na navbar (botão
    Boot com `aria-busy`); progresso do boot = faixa fina no banner e erro =
    `role="alert"`, ambos só durante o boot/falha. `ConnectionBar.tsx` e
    `bootPp` do useBoot removidos; 14 baselines visuais regeneradas;
    asserts e2e migrados (R1/R5, estados, erro-boot).
  - **Viewport única + remanejamento harmônico (pedido do owner):** a página
    inteira virou uma "mesa" (`main` = grid navbar → meio com scroll →
    rodapé; os 99 presets NÃO esticam mais a página — lista delimitada
    `max-height 38vh` rolando por dentro). Botões remanejados: **⇄ mover**
    → cabeçalho do PALCO (controla o drag dos slots, onde faz sentido);
    **⭘ kill** → cluster do master (mute junto do volume, como no hardware);
    **⚙** → cluster do master (global, sempre clicável — no rodapé o drawer
    do drum o cobriria); rodapé novo = chassi da pedaleira (IN · GP · OUT).
- **V-4. Drums em drawer inferior** — ✅ FEITO 30/09 (opção A)
  - DrumPanel reescrito como drawer full-width (`.drum-drawer`, animação
    suave, fecha com Esc pela precedência global já existente); palco 100% limpo.
- **V-5. Regressão visual expandida** — ✅ FEITO 30/09
  - 48 baselines win32: 3 painéis + topbar + 2 erros × 7 viewports
    (800×600, 1024, 1280, 1440, 1920×1080, 2560×1440, 2560×1080) + tema
    claro (emulateMedia, sufixo `-light`) × 6. Regeneradas com o tema novo;
    71/71 e2e ✅ com comparação limpa na 2ª execução.
- **V-6. Código morto (parte da auditoria de QA)** — ✅ FEITO 30/09
  - Removidos: `useDevice.ts`, `PresetCase.tsx`, `AmpHead.tsx` (0
    importadores; varredura por grep + tsc + lint confirmando) e suas chaves
    MSG. Mantidos DE PROPÓSITO: Pedal/Pedalboard/Knob/fxModels — fundação
    declarada da Fase 2 (pedais voltam ao board um efeito por vez).
- **V-7. Pendências do plano enterprise** — ✅ tudo entregue (os edge cases de IPC viraram a issue #20, fechada em 02/10):
  - ✅ FEITO 01/10 — Page Object Model nos e2e: `e2e/pages/_pages.ts`
    (Shell/Brand/Library/Board/Looper/Drum/Settings/VU) com os 5 specs
    reescritos 1:1 (mesmos asserts, zero mudança de comportamento);
    `_helpers.ts` permanece como módulo de medidas geométricas. Bônus:
    restaurado o `toHaveScreenshot` dos painéis/tema claro (perdido em
    edição anterior) — o guard retomado pegou drift real de 2px no palco
    e as baselines foram regeneradas; 72/72 e2e ✅.
  - ✅ FEITO 02/10 — **edge cases de IPC de nível 2** (issue #20):
    falha/retry/backoff na porta única do front (`retry` 3× com backoff
    120→240 ms + jitter, timeout de 8 s POR tentativa; o `boot` fica fora da
    política — a recuperação dele é o ⟳). Gancho `gp100.debug.failDevice`
    ganhou `select`/`set_param` (eram void-infallible), o modo transitório
    `op:n` (exercita o backoff de verdade) e `boot-mid` (disconnect no meio
    do boot). A navegação de preset deixou de ser otimista: `pp`/nome só mudam
    DEPOIS do select confirmado, e a falha vira banner com AÇÃO de retry.
    Push log valida (`F0…F7`/par/hex) e dedupa repetição consecutiva (`×N`).
    Números: **111 unit · 88,4% stmts · 85,9% fns · 90,2% lines** (gate 85) e
    +3 e2e novos (`e2e/ipc.edge.spec.ts`).
  - ✅ FEITO 01/10 — GitFlow completo + cadeia rc sob `workflow_dispatch`:
    `.github/workflows/release.yml` (release-gitflow) com `action=rc`
    (corta/reutiliza release/x.y.z de develop, etiqueta vX.Y.Z-rc.N —
    o pipeline publica PRERELEASE; N incrementa sozinho) e
    `action=promote` (tag final vX.Y.Z, merge --no-ff em main, backport
    em develop, branch apagada; conflito aborta sem empurrar nada
    parcial; idempotente por tag existente). Guard de prerelease no
    pipeline para tags -rc (CLI + instalador). Validação sintática dos
    workflows no gate do pipeline (`scripts/validate_workflows.py`:
    YAML dos workflows + contratos de fluxo ci/_validate/_publish/release/
    security — o teste de integração real do close-linked continua sendo o
    próprio merge em develop; act exigiria Docker no gate). Proteção de
    branches = configuração do repo (não-workflow).
  - ✅ FEITO 01/10 — **simulação end-to-end da cadeia no gate**
    (`scripts/simulate_release.py`): extrai os blocos `run:` DO PRÓPRIO
    release.yml e os executa num sandbox git local (origin bare + clones
    descartáveis), com `${{ inputs/steps/env }}` resolvidos e
    GITHUB_OUTPUT simulado — prova rc1→rc2 (branch reutilizada, N
    incrementa)→promote (tag final, merge --no-ff em main, backport em
    develop, branch apagada), idempotência do promote e os guards de
    recusa (rc sem bump pendente / promote com tag existente). Roda no
    gate do pipeline (runner ubuntu: git+bash+python3) e local via
    `python3 scripts/simulate_release.py` (`--keep` preserva o sandbox;
    steps rodam via arquivo .sh como o Actions faz — `bash -c` perde
    atribuições/expansões no interop WSL do Windows, diagnosticado e
    documentado no próprio harness).
  - ✅ FEITO 30/09 — fechamento de issue no merge em develop (a parte
    "vinculação issue↔PR" da nota): o job `close-linked` do `ci.yml`
    (era close-issues.yml; integrado em 01/10) extrai `Closes/Fixes/Resolves #N`
    do corpo do PR mergeado em develop e
    fecha via API com comentário de rastreabilidade (PR + sha + run).
    Guards: PR do github-actions[bot], issues `achados-security` (fechamento
    manual) e idempotência (já fechada = confirmada). O GitHub nativo só
    fecha na branch default (main); no GitFlow este workflow cobre develop.
  - ✅ FEITO 30/09 — templates novos (parte da centralização): issue `epic`
    (checklist de filhas como medidor de progresso) e `refactor` (com
    garantias de não-regressão); bug/feature ganharam áreas do editor
    (gp100-ui/shell), `CI / workflow` e fases atuais do ROADMAP; PR template
    reescrito com tabelas de gates executados, ambiente verificado e
    antes→depois (R1–R4 e política de conteúdo mantidos). YAML dos 8
    arquivos validado por parse.
  - ✅ FEITO 01/10 — **gate de coverage 85%** no front: `test:coverage`
    (vitest + @vitest/coverage-v8) com thresholds statements/functions/lines
    = 85 no `vite.config.ts` (branches medidos, sem gate — ganho por
    incremento; excluindo só main.tsx/design.css/VuPanel, coberto por e2e
    visual). A CI trava pelo build-front action (`pnpm test:coverage`),
    válido para front, ui-rust, e2e-tauri e releases. Suite subiu 27 → 77
    unit (tsc/lint ✅): ipc/device completo (boot em lotes de 64 com cap
    travado em unit, board/library/comandos, gancho failDevice por
    operação, **branch de webview Tauri** via mock parcial de
    @tauri-apps/core+event), useBoot (auto 1× sob StrictMode, manual,
    throttle rAF, erro→retry, reset), fundação do palco (fxModels
    variante+fallback, Pedalboard linhas/ordem/pulsos, Pedal LED+ValueBox,
    Knob teclado+arrasto+ciclo) e interações do App (◀/▶ ciclo, master,
    faixas de boot/erro com recuperação pelo ⟳, erro do openPreset, FSM
    completa do looper, busca da biblioteca, 6 abas do Settings,
    persistência do General, push log com cap de 100, drum on/off/BPM/
    compasso, VU modo + drag do EmptyBoard). Números: **91.6% lines ·
    89.9% stmts · 86.6% fns · 81.6% branches**. **Mapa de edge cases de
    IPC cobertos** vs. pendentes (nível 2): (a) info falha → navbar off
    (e2e R5); (b) boot falha → alerta + ⟳ recupera (e2e + unit); (c) board
    falha → erro amigável (e2e + unit); (d) boot lento → lotes de 64,
    UI fluida (unit); (e) events unlisten/dupla assinatura (unit); (f)
    library/sem falha — resolve sempre (unit). **Pendentes na ocasião** (falha
    em select/set_param, retry/backoff de command, disconnect mid-boot,
    dedupe/parse de push hex inválido) → ✅ TODOS ENTREGUES na **issue #20**
    (02/10): ver a entrada da #20 acima e também o mapa de testes
    (`tests/ipc.retry.test.ts`, `tests/ipc.push.test.ts`,
    `tests/device.fail.test.ts` e `e2e/ipc.edge.spec.ts`).
  - ✅ FEITO 01/10 — fluxo de contribuição documentado (parte da
    centralização): `docs/CONTRIBUTING.md` liga o ciclo completo
    issue → branch (GitFlow, develop como integração) → conventional
    commits → gates por área (coverage 85% incluso) → PR (tabelas do
    template) → fechamento automático da issue pelo job `close-linked`;
    regras de casa escritas (epic não fecha por filha, vínculo de
    fechamento no CORPO do PR, ci-lite, achados-security manual).
    Indexado no INDEX.md (rota + inventário) e no seletor de issues
    (contact link do config.yml).

## ❌ Deliberadamente FORA de escopo agora (não reabrir)
- Firmware update (política V2+) · layout byte-a-byte da página 13xx (só mock precisa
  de shape) · CRC de slot IR ocupado · ppEXP1/ppCtrl fino · G3–G6 de captura ·
  UI/Tauri (M1, depois do gate H).

## 📐 Regras do não-retrocesso (R1–R4)
- **R1** Spec é insumo, não produto: código nunca "adivinha" — consulta golden/§13.
- **R2** Toda mudança de spec passa por captura → `build_golden` → `validate_golden`
  100% → bump de baseline → só então código.
- **R3** Divergência com device real: logar, interromper o fluxo, tratar como R2.
- **R4** Round-trip `.prst` é sagrado: qualquer mudança no modelo passa pelo teste
  byte-idêntico dos 99 presets.
