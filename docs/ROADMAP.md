# 🗺️ ROADMAP EXECUTIVO — Fundação + gp100-core

> **Objetivo:** fechar TODAS as atividades de preparação antes do primeiro commit
> de código de negócio, para que o desenvolvimento flua **sem paradas** para ajuste
> de ambiente, configuração, revisão de spec ou extração de dados. Cada item é uma
> issue com dependências e critério de aceite (Definition of Done) testável.
>
> **Status:** ✅ atual · **Última revisão:** 2026-09-29 · Mapa da doc: `docs/INDEX.md`
>
> **Princípio do não-retrocesso:** nenhum passo de implementação pode depender de
> descoberta nova. Tudo que exigia hardware/pesquisa já está fechado e validado
> (§13 + golden 100%). Se durante a implementação uma premissa falhar, o fluxo é:
> parar → captura nova → golden regenerado → validate 100% → então retomar (regra R3).

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
> de escrita na UI e estratégia de testes).

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
  [SQLite], empacotamento) — espelhar o formato do `docs/UI_PLAN.md` quando a
  M1.0 fechar.
- **M3** fica no nível de panorama (VISION §9) até o gate H passar.
- **ADR-7+** à medida que os spikes fecharem as decisões em aberto do
  `docs/UI_PLAN.md` §9.

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
