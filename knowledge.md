# knowledge.md — GP-100 NextGen Editor (ler antes de agir)

> Mapa geral da documentação (o que consultar para cada assunto): **`docs/INDEX.md`**.

Projeto: substituto do Valeton Suite para a pedaleira GP-100, por engenharia reversa
(local, sem depender de hardware para ~95% do trabalho). Resposta ao usuário SEMPRE em PT-BR.

## Estado vivo (atualizar aqui a cada marco)
- 02/10 — **EDGE CASES DE IPC NÍVEL 2 ✅ (issue #20)**: falha/retry/backoff na
  porta única do front (3 tentativas, backoff 120→240 ms com jitter ±30%, timeout
  8 s POR tentativa; **boot fora da política**), gancho `gp100.debug.failDevice`
  com modo TRANSITÓRIO (`op:n`) e `boot-mid` (disconnect no meio do boot) e log de
  pushes validado/deduplicado (push inválido ignorado; repetição consecutiva vira
  `×N`). A navegação de preset deixou de ser otimista (o `pp` só muda DEPOIS do
  select confirmado — a UI nunca mostra preset que o device recusou).
  Front: 111 unit ✅ (cobertura 88,4% stmts · 85,9% fns · 90,2% lines) e e2e 63 ✅
  + 3 novos de `ipc.edge` (select transitório/permanente e mid-boot).
  **Achado A-5 / issue #45 RESOLVIDO no mesmo dia** — 12 baselines win32
  desatualizadas (drift pré-existente, provado com `git stash`) e 0 `-linux` no
  repo (o visual SKIPava no CI): win32 regeradas + 48 `linux` geradas pelo
  dispatch `update-snapshots` (artefato `visual-snapshots`) e o skip silencioso
  removido do spec — baseline ausente agora FALHA o job. Lições na seção de IPC
  abaixo.
- 01/10 — **PAUSA PARA AUDITORIA + GESTÃO POR ISSUES**: revisão completa de ui/cli/app/docs/CI.
  Gates verdes: front tsc/lint/unit 88/coverage 87,6%/build ✅; Rust fmt/clippy -D/test ✅
  (corrigido 1 erro real de clippy — `needless_borrow` em `pedalboard.rs`); pytest 10/10 ✅.
  **Afinador do palco** entregue (TunerPanel: display sempre visível no lugar do VU,
  LED próprio, escala ♭→♯, botão on/off visual, REF PITCH 435–445; botão mover virou
  cadeado 🔒/🔓; navbar com logo à esquerda e controles à direita; **kill switch** agora
  tem ação real e reversível — mute global de master+drum).
  **GitHub organizado:** milestone **v1.0.0** + labels por área/tipo/prioridade
  (`area:*`, `design`, `packaging`, `priority:*`) + 6 epics (#13–#18) e 12 filhas
  (#19–#30). O ROADMAP agora **aponta para as issues** (o plano vivo mora no GitHub).
- 30/09 — **REFATORAÇÃO ESTRUTURAL packages/ ✅ + pipeline único reescrito**: monorepo
  `packages/{core,cli,app/{ui,api}}` (ver seção Monorepo abaixo), limpeza de gestão
  (M0.x/M1.x/ROADMAP/DoD) de todos os comentários de código, READMEs curtos em
  core/cli/app, CI com caminhos novos + fix da matriz vazia. Provas locais: fmt/
  clippy/test 15 suítes ✓, pytest 10/10 ✓, front lint+vitest+build ✓, actionlint ✓.
  COMMITADO E PUSHED (1a8e08e refactor + acc9f9a docs-sync + d69186b fix re.M +
  3fe8294 docs-sync); CI verde (run 36765355879) — os skips estavam CORRETOS
  (push só de CI/docs não mapeia projeto). Materialização completa das matrizes
  ainda não provada pós-re.M (próximo push de packages/ ou o play).
- 30/09 — **midir PINADO em 0.10 (experimento 0.11 FECHADO)**: o 0.11 puxa crates
  `windows-*` com raw-dylib e é IMPOSSÍVEL no host de campo hoje — dlltool GNU
  moderno (2.44+) rejeita a machine que o rustc passa (`Machine 'x86_64_w64_mingw32'
  not supported`; provado em WinLibs 2.47 E mingw-builds 2.46 com um .def simples)
  e o llvm-dlltool do rustup compila mas o loader rejeita o import (segfault no
  load). Configuração final do host que builda o 0.10: llvm-dlltool COPIADO como
  `~/.cargo/bin/dlltool.exe` + `~/.cargo/bin` PREPENDIDO no PATH (o winget WinLibs
  sombreava por append — foi desinstalado; ver lições). PR #3 (0.11) descartado até
  rustc/binutils convergirem.
- 30/09 — **TS 7 travado no Dependabot (entry npm)**: typescript-eslint 8.x suporta
  só TS <6.1 (`Error: typescript-eslint does not support TS 7.0.` na importação do
  flat config — lint morre ANTES de lintar; o build passa pois vite só transpila).
  PR #2 (typescript 7.0.2) fechou em failure nos 3 OS; `ignore: typescript >=7`
  no dependabot.yml impede a reincidência semanal. Reabrir quando o peer range do
  typescript-eslint cobrir TS 7.
- 29/09 — RealDevice ✅ (H1 pronto em software): `gp100-core/src/transport/real.rs`
  (midir 0.9/WinMM, feature `real-device` via dep:midir; callback→fila
  compartilhada, trim no 1º F7 na entrada, despacho de porta por nome
  "gp-100", SysEx completo num send, reconexão ADR-4; Box<dyn> transport).
  CLI: `--real` = dupla confirmação + feature (2 camadas testadas); build de
  campo `cargo build --release -p gp100-cli --features real-device` (smoke:
  sem device = OpenFailed limpo). Falta SÓ a pedaleira (H1_CHECKLIST).
- 29/09 — KIT DE CAMPO DO H1 PRONTO: build release do CLI + referências do mock
  (`analysis/h1_reference/`, -text) + runbook `scripts/h1_field.sh rehearsal|field`
  (ensaio executado: 0 divergências de framing) + `docs/H1_REPORT.md` (template
  com o PLANO DE BACKUP fixo §7: divergência ⇒ parar, preservar evidência,
  diagnóstico só com o log, R3 no escritório). Falta SÓ o RealDevice + pedaleira.
- 29/09 — REVIEW FINAL DA FASE M0: ✅ APROVADA (8/8 issues com DoD cumprido;
  78 testes Rust em 14 suites + pytest 10/10 + provas A–E 100%; bytes
  congelados íntegros; docs 100% sincronizados — resíduos "7/8" e tabela
  de ADRs do DECISIONS sem M0.1/M0.2/M0.7 corrigidos). M0 FECHADA; próximo
  = M1.0 (spike Tauri) e/ou gate H1 (pedaleira, H1_CHECKLIST).
- M0.8 ✅ (29/09) — **FASE M0 100%**: `gp100-core/README.md` (arquitetura,
  exemplos, testes, CLI) + contrato `tests/readme_examples.rs` que executa os
  exemplos da doc 1:1 (pegou import faltante no exemplo da Session); onboarding
  do README raiz revisado (INDEX primeiro + hands-on de 10 min). Próximo:
  M1.0 (spike Tauri, docs/UI_PLAN.md) e/ou gate H1 (docs/H1_CHECKLIST.md).
- M0.7 ✅ (29/09): `gp100-cli` — 5 subcomandos contra o mock + `--log` no schema
  P4 (requisito do H1) + `--real` bloqueado (exit 2; dupla confirmação não
  contorna o gate). Extensões: `Session::{select_preset, state_page}`; MOCK
  completado p/ o pareamento D1 do replay S1: select→meta6 com o pp, página
  ecoa [pp][PG], PG8 → `13010005` (4B), tabela ecoa a página pedida — o SMOKE
  do CLI expôs que a M0.5 só provava TAMANHO das respostas do mock. Parser
  zero-dep (clap adiado p/ M1, ADR-lite lá). 74 testes.
- 29/09 — H1_CHECKLIST.md CRIADO (`docs/`): operacional do gate H1 (só leitura;
  níveis de comparação framing×estado; fluxo R3; watchlist). Requisito de campo
  novo: CLI com `--log` no schema das fixtures P4 (entra na M0.7/prep-H1).
- 29/09 — REVIEW DE DOCUMENTAÇÃO: README/VISION/ROADMAP/INDEX sincronizados ao
  estado real (M0 6/8; "próximo marco = M0" estava defasado; venv/.prst/decisões
  corrigidos) + **`docs/UI_PLAN.md` CRIADO** (planejamento completo da M1:
  escopo, arquitetura DeviceActor, IPC, telas, política de hardware, testes,
  riscos M1.0–M1.6). Pendências de planejamento registradas no ROADMAP:
  checklist H1, plano fino M2, ADR-7+ pós-spike.
- M0.6 ✅ (29/09): `gp100-core/src/session.rs` — FSM completa (boot/scan_state/
  set_param/save_preset/upload_ir/list_user_irs/pending_pushes) e replay
  100% das 4 fixtures byte-a-byte (`tests/replay_fixtures.rs`: boot 2299/2299
  OUTs, knobs 92/92, save 9/9 exatos, IR 594/594+592 ACKs; divergência de
  framing = falha com diff hex). ACHADOS DO BOOT (provados no replay):
  (1) ciclo do pp atual 0x0100 DUPLICADO = 2 selects → 2 meta6; 2 opens →
  PÁGINA 0 duas vezes (NÃO é um 3º meta6); reqs pg0..7 → pág1..8; pg8 →
  IN 13010005 (14 INs p/ 13 OUTs: 1 pág0 dup); (2) T1 tem 1 resposta de
  tabela que chega TARDIA, DEPOIS do 1º select do scan — o backlog D7
  absorve (fila não é por endpoint); (3) T3 nomes é FIRE-AND-FORGET:
  61 leituras, 57 respostas — o device OMITIU 4 respostas (banco 00, idx
  01/03/06/0a) e o Suite seguiu; esperar por-leitura = Timeout; respostas
  de nomes = pushes de contexto (D2) → backlog (D7); (4) select da sonda
  1302 é CONST "0000" no golden (sem pp); (5) template 11000008 é mixed
  2 vars + const 0000 (não 4 vars). Próximo: M0.7 CLI.
- M0.5 ✅ (29/09): `gp100-core/src/transport/mock.rs` — MockDevice respondendo
  CONFORME O GOLDEN (despacho `match_request`, respostas via
  `Template::build_response` — API nova com simetria provada nos 15 exemplos
  IN; D5 das respostas). Estado = all.prst (99 presets; **ppIRInfo é container
  NA RAIZ <GP>**, irmão de preset_info; ppIRNum = índice global, NÃO slot) +
  dicionário. D1–D8: fila FIFO global, ACK por chunk, fire-and-forget (sem
  resync pós-save), queue_push p/ push intercalado (D7), DeviceGone+reconexão.
  ACHADOS: golden congela set_param POR INSTÂNCIA (9 templates 10xx0002 com
  consts de knobs) → o SHAPE §13.11 é validado pelo CODEC no mock; meta6 (t6)
  é const; scan 1302 não abre com pp; by-len decide pelo endereço da
  RESPOSTA (t8 lê 13010004 e responde em 13010003); índices de chunk IR
  NUNCA caem em 128-255 (F7 cru no idx — a razão provável dos gaps).
- Esqueleto do session PRONTO (29/09, pré-M0.6): `gp100-core/src/session.rs` —
  assinaturas do ADR-6 rev.3 COMPILANDO com corpos `todo!("M0.6: …")`
  explícitos (os 7 métodos da FSM panicam apontando a issue; contrato em
  `tests/session_skeleton.rs` prova Session sobre transporte EXTERNO e
  placeholders não-silenciosos). Tipos de saída MÍNIMOS (BootReport,
  StatePage opaca, UserIrTable, IrUploadReport). M0.6 restante = corpos
  (D1–D8: match_response, backlog D7) + replay byte-a-byte das fixtures.
- Esqueleto do transport PRONTO (29/09, pré-M0.5): `gp100-core/src/transport.rs` —
  trait `DeviceTransport` verbatim do ADR-4 (open/close/send_raw/recv_raw;
  bytes crus, sync/bloqueante, ciclo de vida do CHAMADOR) + `TransportError`
  tipado (Closed/OpenFailed/SendFailed/DeviceGone/RecvTimeout). `DeviceGone`
  = desconexão física NO MEIO da sessão; `open()` no MESMO objeto RECONECTA;
  `open()` sem device = OpenFailed — decisão travada em
  `tests/transport_trait.rs` (implementador EXTERNO loopback prova trait
  pub/object-safe/dyn). Contrato de msg: `recv_raw` devolve UMA msg
  `F0..1º F7` (trim no 1º F7 é do RealDevice; F7 no meio = paginação).
  M0.5 restante = SÓ o MockDevice (D1–D8) e o RealDevice feature-gated.
- 29/09 — FECHAMENTO DO DIA: M0.4 ✅ no remoto com CI verde; ADR-6 rev.3 ACEITO
  (D1–D8 travados, watchlist H1 completa); save RE-DERIVADO do log cru (D3
  definitiva); quirk dos 32 ACKs tardios FECHADO (§13.7/§13.12 corrigidos);
  op_payload travado em teste (40 testes); skills revisadas (regra de testes
  explícita + auditoria com 5 achados). Próximo: M0.5 com contrato pronto.
- SAVE RE-DERIVADO do log cru (29/09, `analysis/derive_save_ops.py`): o save
  NÃO tem resposta IN (S4: zero msgs nos 11s pós-ops; S2: ops isoladas, ±118s
  de qualquer 11xx). `12000001` NÃO fecha ciclo de op (na S2 chegou 22s ANTES
  das ops). Bursts `11000008`×N+`12000001` = SINCRONIZAÇÃO de tabela do
  Suite/app (S2: precedidos de 61 requests OUT; S4: cópia espontânea +11,4s
  após o save, janela com 0 OUT). §13.12 CORRIGIDO; D3 do ADR-6 agora
  DEFINITIVA (save fire-and-forget; ciclo de ops = S4: op0 ×2 [0;+578ms] →
  op1 ×2 [+593ms]; fim dos writes = commit). QUIRK FECHADO (§13.7): os 32
  ACKs tardios `12001002` da S2 (slot 1, idx 0x208..0x226, últimas msgs do
  log, 0 OUT na janela) = burst de FIM DE SESSÃO como o da S4
  (`11000008`+`12000001`) — flush do ring do proxy no close (hipótese
  principal); não é resposta de save nem retransmissão; FSM não modela (D7).
  §13.7 também corrigido: o frame 0x226 duplicado NÃO é `0F`×15 — payload é
  a cauda REAL do blob; marcador de fim = a duplicação. GAPS menores: op
  `00020000` BE confirmado contra fixture (`00010000` = 1) e TRAVADO em teste
  (`op_payload_vector` no codec.rs, 29/09 — gap fechado); S2 índices de chunk em páginas
  intercaladas 0-127/256-383/512-550 (dois slots × páginas alternadas).
- ADR-6 ✅ ACEITO (29/09, rev.3): `docs/DECISIONS.md` — assinaturas da FSM
  `Session<T: DeviceTransport>` (boot/scan_state/set_param/save_preset/
  upload_ir/list_user_irs) + regras D1–D8 (D3 definitiva: save fire-and-forget
  com ciclo de ops da S4; D7 backlog de IN não solicitado; D8 consumidor único
  do IN; chain_slot × ir_slot; blob múltiplo de 15B strict) + alternativas
  rejeitadas + watchlist do gate H1. CONTRATO do MockDevice (M0.5) e base da
  issue M0.6 — caminho livre para abrir a issue.
- M0.4 ✅ (29/09): `gp100-core/src/codec.rs` — codec de fio PURO e sem estado:
  nibble_expand/collapse strict (par ímpar ou nibble >0x0F = InvalidShape);
  `set_param` (§13.11: payload 20B nibble-exp [code u32 LE][ctrl][00][f32 LE],
  addr `10 [slot 1..9] 00 02`); `meta_block` (§13.12: 5 writes na ordem capturada
  11000000/11000004/11000005/11000007/12000002, nome ASCII 12B trunc+pad);
  `op_payload` (00020000, op u16 BE em [4..5]); `ir_begin`/`ir_chunk`/
  `ir_chunk_ack` (§13.7: begin cru `00 [slot] 00 00 01 00 00 0a` ANTES dos chunks;
  chunk 33B [slot][idx u16 BE]+30 nibbles=15B reais; ACK [slot][idx][01]).
  Contratos (`tests/codec_wire.rs`) reproduzem as fixtures P4 byte a byte:
  92 knobs + 2 saves + 1186 frames IR (último chunk dup 0x0226 slot 1).
  `pub type WireWrite = ([u8;4], Vec<u8>)` = forma canônica dos blocos.
  Fixture guarda PAYLOAD puro (cortes rebuilt[13..33]/[13..46]); regras de
  SEQUÊNCIA da FSM (ordem/quirk/salto) = M0.6; página IR 13xx fora (deliberado).
- M0.3 ✅ (29/09): `gp100-core/src/golden.rs` — GoldenFile/Template/Pattern com
  build_request (SysEx completo, vars posicionais) + matches_response (len+consts,
  extrai vars; by-len despacha por comprimento no 12001002: ACK 4B / tabela 75B /
  resync). Propriedade extract→build==exemplo provada nos 40 templates.
  ⚠️ example.* do golden são PAYLOADS (não SysEx completo).
  API v2 (revisão pós-M0.3): endpoints TIPADOS (u8,[u8;4]) com índices O(1);
  3 endpoints OUT têm DOIS templates (caso congelado do boot × geral:
  13010002/13010004/13020004) → request_template(func,addr,vars) desambigua
  por var_count; GoldenFile::build_request(func,addr,vars) = rota da FSM.
  IN compartilhado com bytes ambíguos (13010001 push×req): dispatch = ordem
  do arquivo; FSM resolve pelo contexto do que pediu (M0.6).
- M0.2 ✅ (29/09): `gp100-core/src/preset.rs` — round-trip byte-idêntico dos 3 .prst
  (R4 provado em tests/roundtrip_prst.rs; layout = DADO: quebras/indents registrados).
  Efeitos em ordem x DESCENDE no arquivo (RVB->PRE); ppName com `&amp;` verbatim;
  set_attr estrito. **.prst também é `-text` no .gitattributes** (CI pegou EOL de novo).
- M0.1 ✅ (29/09): `gp100-core/src/model.rs` — dicionário serde (185/639) com
  validação na carga e lookup O(1). ACHADOS (R1): identidade = (module,nibble,index)
  (Boost/14 Boost dual-módulo PRE/DST, defaults divergentes); knobs bidirecionais
  min>max (Pitch.L-Pitch 0..-24 → range()); observed_* ausente/NULL em 68;
  default é string. ProtocolError (ADR-2) definido. CI verde.
- CI GitHub Actions (29/09): gates `uv run pytest` + `cargo fmt/clippy/test` por push/PR
  (.github/workflows/ci.yml, windows-latest pela toolchain gnu pinada; badge no README).
  Golden protegido com `-text` no .gitattributes: o hash da baseline (§13) cobre os EOLs
  e o blob no repo é byte-idêntico ao congelado (CRLF) — renormalizado no commit da CI.
  validate_knob_map.py: caminho absoluto `D:\GP-100 app` → relativo à raiz (CI incluída).
- PATH do cargo RESOLVIDO (29/09): `scripts/add_cargo_path.ps1` (admin, 1x) adicionou
  `C:\Users\Canta\.cargo\bin` ao PATH de MÁQUINA (HKLM; preserva REG_EXPAND_SZ +
  broadcast WM_SETTINGCHANGE). Terminal novo acha cargo SEM export; workaround de
  sessão (`export PATH=...`) só para terminais abertos antes do fix.
- Skill NOVA `rust-practices`: gates e estilo de todo código Rust (fmt/clippy/test
  com doc-tests; doc-comments PT-BR com evidência; `deny(missing_docs)` no core).
- P4+P5 ✅ (28/09): `analysis/make_fixtures.py` → `analysis/fixtures/` (boot/knobs/save/ir
  + manifest c/ paridade e sha do golden) — 2299 OUT boot, 89+3 knobs, 77 save,
  1186 IR (2 BEGIN + 592 chunks + 592 ACKs), 100%; gate no pytest (10/10).
  `docs/DECISIONS.md` ADR-1..5 pré-assinados (endian/nibble, ProtocolError/thiserror,
  timeout 3s síncrono, trait DeviceTransport, WRITE_VERIFIED). FASE P 100% → M0.1.
- P3 ✅ (28/09): suíte de regressão `analysis/tests/test_protocol.py` — `uv run pytest`
  = 9/9 (~4s): golden 5 provas com critérios objetivos + hash da baseline + knob_map 13/14
  (exceção CAB/Mic). Gate de qualquer PR que toque spec/decoders.
- P2 ✅ (28/09): Rust instalado (rustup 1.29.1 / rustc 1.98.1 via winget+elevação).
  Toolchain PINADO windows-gnu em rust-toolchain.toml (host sem MSVC; `link` do PATH
  é o GNU coreutils). Workspace gp100-core+gp100-cli criado; clippy -D warnings + fmt +
  testes verdes;  CLI já bloqueia --real. ⚠️ PATH: adicionar C:\Users\Canta\.cargo\bin.
- **Plano de execução vigente: docs/ROADMAP.md** — Fase P (congelar spec P1, workspace P2,
  regressão P3, fixtures P4, ADR P5) → M0 (core só com mock) → Gate H (hardware).
  Seguir a ordem das issues; descoberta nova entra pelo fluxo R3 do roadmap, nunca por patch ad-hoc.
- Protocolo de fio FECHADO: docs/PROTOCOL.md §13 (SysEx Valeton, não é o envelope de objetos §10–11).
- Sessão 3 ANALISADA (§13.11): knob pela UI = write semântico `12 | 10 [slot 1..9] 00 02 | payload 20B
  nibble-expandido = [effectCode u32 LE][ctrl u8][00][float32 LE valor físico]`, SEM ACK e sem push de página.
  `slot` = posição na cadeia (1=PRE..9=RVB), NÃO o nibble; `ctrl` = pos do controls[] = params_N do .prst.
  Mapa/89 edits: analysis/knob_map.json (gerado por validate_knob_map.py; dump_edit_writes.py = extração bruta).
  ATENÇÃO: CAB tem controle oculto (Mic, ctrl 1) que NÃO está no parameters.json — preservar params_N 0..14.
- Sessão 4 ANALISADA (§13.12): save pela UI = writes 11xx (metadados c/ nome ASCII) + ops `00020000`
  (op 0/1 = sair/entrar modo edição; ciclo S4 = op0 ×2 → op1 ×2 — §13.12 RE-DERIVADO:
  o burst 11000008+12000001 é FIM DE SESSÃO/sincronização, NÃO parte do save).
  SEM readback 13xx no save; **persistência CONFIRMADA pelo usuário no display da pedaleira**
  (slot recebeu o preset com os valores editados da sessão 3 ⇒ save persiste o ESTADO AO VIVO).
  Item 11 do BLOCKERS FECHADO; WRITE_VERIFIED=true para os fluxos capturados (knob/save/IR).
  capture-session4.jsonl contém a session3 inteira (log é append-only): separar sessões por gaps >30s.
- Gaps restantes de captura (G3–G6 do capture_gaps.md): settings globais/BPM/drum, knob físico,
  footswitch/EXP, info de firmware — opcionais, não bloqueiam o gp100-core.
- 12001002 DECIFRADO (§13.12): tabela dos 20 User IRs = [slot u8] + nibble-exp 37B
  = nome ASCII 32B (0xFF=vazio) + byte[32] (flag) + CRC32 BE (vazio = ppIRCRC do .prst!) + trailer.
  11000007 = 50B sempre zeros (reservado). Ver decode_types_table.py.
- 0x817 RESOLVIDO (§13.8); `.prst` = XML puro (§13.9); scan de presets §13.10 (d[3]=NÚMERO DA PÁGINA).

## Ambiente (Windows + Git Bash) — ARMADILHAS que já morderam
- **Python sob uv (28/09)**: `pyproject.toml` + `uv.lock` na raiz; deps (capstone,
  pefile, **ziglang no venv**, pytest no grupo test). Recriar ambiente =
  `uv sync --all-groups` (da RAIZ). Venv ÚNICO = `.venv/` na raiz (o antigo
  `analysis/.venv` foi removido). Rodar: `uv run python ...` (sempre da raiz).
  O proxy NÃO depende mais de python global (ziglang 0.16 no venv).
- `print()` de texto Unicode quebra no console (cp1252): sempre `sys.stdout.reconfigure(errors="replace")`.
- Heredoc `python - <<'EOF'` no Git Bash: cuidado com try/except e aspas dentro do bloco;
  para scripts não-triviais, escrever arquivo em `analysis/` e rodar.
- `%TEMP%` real do usuário = `C:\Users\Canta\AppData\Local\Temp` — o `/tmp` do Git Bash é outro
  (válido só p/ processos filhos dele). O proxy grava em `%TEMP%\midi_trace.jsonl`.
- Console/tasklist em PT-BR retorna texto com encoding estranho; preferir PowerShell quando precisar de estrutura.

## Caminhos canônicos
- `.venv/` (raiz) — ÚNICO venv do projeto (uv); `analysis/.venv` NÃO existe mais
- `docs/ROADMAP.md` — plano executivo vigente (issues P/M0/H com responsável e DoD)
- `docs/DECISIONS.md` — ADR-1..6 do gp100-core (P5 + ADR-6 da FSM); mudar decisão = novo ADR
- `scripts/add_cargo_path.ps1` — fix do PATH do cargo no sistema (HKLM; idempotente)
- `analysis/fixtures/` — fixtures de replay por fase (regenerar: `uv run python analysis/make_fixtures.py`)
- `README.md` — porta de entrada do repo (panorama, workflows de regeneração, onboarding)
- `analysis/captures/` — logs de captura (session1.jsonl, session2.jsonl, ir_slot*.bin)
- `analysis/suite_local/` — cópia gravável do Suite + winmm.dll do proxy (É ESSA que o usuário abre)
- `analysis/winmm.dll` — build atual; copiar p/ suite_local após cada rebuild
- `docs/PROTOCOL.md` — referência única do protocolo (§13 = campo); `docs/BLOCKERS.md` — matriz de status
- `analysis/parameters.json` — dicionário canônico de 185 algoritmos/639 controles
- `files/patches/*.prst` — biblioteca XML exportada (all.prst = 99 presets)
- `analysis/nsis_app/` — extração do instalador (108MB; não indexar — ver .codebuffignore)

## Workflows (nunca fazer na mão)
- **Golden-file do protocolo**: `analysis/build_golden.py` regenera `docs/protocol_golden.json`
  (templates request→resposta de TODAS as capturas, com padrões de payload const/var e
  semântica por endereço). É a especificação executável do §13 — consumir DE LÁ, não reparsear logs.
  Pairing: OUT→IN por (func,addr) timeout 3s; pushes 13xx por família; writes sem resposta = fire-and-forget.
- **Validação do golden**: `analysis/validate_golden.py` = 5 provas (accounting IN 100%/OUT
  99,57%; knobs 89/89+3/3; boot/scan 2299/2299; save 77/77 — metadados gerados do .prst;
  upload IR 1186/1186 — framing por regras, dados = inventário).
  Rodar SEMPRE depois de qualquer mudança no build_golden ou nas capturas.
  Regras de largura confirmadas: select/open pp = u16 BE; página = pp u16BE + PG u16BE + 01 (5B);
  chaves 11000008 = [banco u8][índice u8]; ring buffer do proxy pode truncar mensagens (ignorar SEM-HDR).
  ORDEM DO UPLOAD DE IR (correção v2): `10050001` = BEGIN/reserva do slot, vem ANTES dos chunks;
  fim = último chunk `0F`×15 duplicado, SEM commit no fio.
  Metadados de save: 11000000 = zeros4 + pp u16BE + zeros2 + nome ASCII 12B; 11000005 = ppType u16BE;
  12000002 = zeros4 + pp u16BE + zeros2. Tudo disponível no .prst (ppID/ppType).
- Proxy winmm: `uv run python analysis/build_proxy.py` (gera impl+build C,
  cruza def↔C↔winmm real, compila com zig, valida exports). Depois: `cp analysis/winmm.dll analysis/suite_local/`.
- Analisar captura: copiar `%TEMP%\midi_trace.jsonl` → `analysis/captures/sessionN.jsonl`, rodar
  `analysis/decode_wire.py` (formas/transações) e/ou `analysis/check_session2.py` (assinaturas/diff).
  Log antigo (pré-fix da aspa) é reparado automaticamente pelos loaders.
- Sessão de captura com o usuário: FECHAR todas as instâncias do Suite antes (instância única
  mata a nossa silenciosamente); rodar `analysis/suite_local/GP-100.exe`; pausas de 5–10s entre ações.
- Buffers MIM_LONGDATA chegam com cauda stale: trim no 1º F7 é a regra (já embutido nos decoders).
  Paginação com F7 embutido NÃO é fim de mensagem.

## Convenções
- Código Rust segue `.agents/skills/rust-practices/SKILL.md` (gates fmt/clippy/test
  com doc-tests; doc-comments PT-BR com evidência; sem unwrap na lib; `deny(missing_docs)`).
- Commits: `docs-sync` tem commit PRÓPRIO e SEPARADO do código (skill `docs-sync`);
  NUNCA misturar ROADMAP/knowledge/INDEX no commit da issue — relembrado no core
  review 29/09 (6609485 misturou; padrão correto = 6422cd1 → d2fa925).
- Ferramentas novas de análise = script em `analysis/` (não heredocs longos).
- Descobertas de protocolo vão para docs/PROTOCOL.md com evidência (VA/hex da captura), nunca só conversa.
- Não commitar sem pedido; não tocar em `analysis/nsis_app/` (é material extraído, read-only).

## UI e estado (29/09 — M1.0 em curso)
- **Fase ACHADOS (A-xx) no ROADMAP**: achados de review/warns/vulns têm
  PRIORIDADE MÁXIMA e precedem issues de fase (decisão do owner).
- Paleta da UI = "palco": preto-quente #141210 + âmbar Valeton #ffa938;
  variantes de tema nas cores de produto (vermelho/lavanda). Rácios WCAG
  medidos em docs/UI_DESIGN.md §2 (token novo = rácio novo medido).
- Identidade "pedalboard AO VIVO" (UI_DESIGN §6): cadeia em 1 clique, edição
  direta (D4), LED pulsando `.live-dot`, regra dos 2 cliques.
- Front fixado: React 19, Vite 8, Vitest 5, ESLint 10, **TypeScript 6.0**
  (TS 7 nativo quebra typescript-eslint 8.x — só subir junto do plugin ≥7.1),
  Node 22+pnpm 11. `@types/node` é dev-dep obrigatória (Vitest 5 + node: no
  teste de tokens).
- pnpm 11: allowlist de build scripts vai em `pnpm-workspace.yaml`
  (`allowBuilds`), NÃO no package.json (`pnpm.onlyBuiltDependencies` é legado).
- CI (reestrutura do owner, 30/09): UM JOB POR PROJETO × matrix dos 3 OSes
  (core/cli/ui em windows+linux+macos) + fmt rápido em ubuntu + pytest único.
  Rust stable do RUNNER fora do Windows (pin gnu via RUSTUP_TOOLCHAIN só no
  runner Windows). Linux + feature real-device exige `pkg-config` +
  `libasound2-dev` (alsa-sys compila C) — instalado no job cli/ubuntu.

## gp100-ui / Tauri (30/09 — ADR-7)
- **Tauri 2 NÃO suporta windows-gnu**: build script do `tauri` morre com
  STATUS_ACCESS_VIOLATION (0xC0000005) no gnu, mesmo com dlltool resolvido
  (llvm-dlltool via -C dlltool=). Crate gp100-ui FORA do workspace
  (`exclude`), `rust-toolchain.toml` próprio (canal "stable" portável —
  ADR-8; o alvo MSVC vem de RUSTUP_TOOLCHAIN na CI e do default do host).
- Estrutura canônica Tauri: commands em SUBMÓDULO (commands.rs) —
  generate_handler! no mesmo módulo do #[tauri::command] colide os macros
  ocultos __cmd__<name> (E0255).
- generate_context! (linux/macos) exige icons/icon.png **RGBA** (RGB puro
  rejeita: "icon is not RGBA"); .ico só serve ao Windows.
- tauri::Error::Setup recebe SetupError (não String) no 2.x atual.
- CI job gp100-ui: front buildado ANTES do cargo (generate_context! embute
  ui/dist); linux precisa libwebkit2gtk-4.1-dev + gtk3 + ayatana + librsvg;
  fmt do crate roda DENTRO de src-tauri (projeto solto, sem --all).
- rustfmt rejeita vírgula final dentro de generate_handler![] (macro com
  proc-macro span — fmt local com a gnu é suficiente p/ validar).

## M1.1 (30/09 — DeviceActor + boot com barra)
- **DeviceActor (D8)**: thread ÚNICA dona da `Session<MockDevice>`; fila
  mpsc serializa commands (sem Mutex<Session> compartilhado). Posse via
  `Option<Session>`: `info`/drain tomam a Session com `take()`, extraem
  via `into_transport()` e REMONTAM (backlog D7 de observação descartado).
- **Progresso do boot SEM callbacks emprestados**: `&mut dyn FnMut` não é
  Send — não atravessa a fila do actor. Padrão: command passa
  `mpsc::Sender<BootProgress>` (Send); actor fecha closure em volta do
  sender; `boot_with_progress(Some(&mut cb))` no core (hook OBSERVACIONAL:
  `boot()` canônico do ADR-6 intocado, replay byte-a-byte segue válido).
- **boot() NUNCA tinha rodado sobre o MockDevice** (replay transport até
  então). Rotas que o mock precisou: t12 (meta6 da sonda @13020001, const
  00000c1c0140 — o select da sonda é CONST 0000, 2B, não vazio), t16
  (pg8 da sonda → 4B @13020005), abertura → PÁGINA 0 196B (S1 rows 89–93:
  "open open → pág0 pág0" — o t7 do golden pareava open com meta6,
  mis-pairing inofensivo enquanto o boot só rodava no replay) e eco
  pp/PG do REQUEST nas páginas (arm 13xx: example do req de leitura é
  vazio → fill por sub-padrão/zeros, não por example_var do request).
- Inventário do boot: DEFAULT da Session = 0..198 (2297 tx); os 2299 da
  captura exigem inventário da S1 (0x0100 primeiro + pp corrente
  duplicado, quirk §13.4). 0x0100 = 256 NÃO está em 0..198.
- **Backlog D7 no DEVICE**: nomes são fire-and-forget (D4) e o mock
  RESPONDE a eles → 61 pushes ficam na inbox do mock pós-boot;
  `drain_inbox()` (FIFO global) os devolve → log `device://push` da UI
  (DoD "pushes visíveis"). boot limpo NÃO deixa inbox vazia por si só.
- **Pegadinhas do actor**: (1) Drop em handle Clone derruba o actor
  quando um CLONE sai de escopo (matou thread de teste) — shutdown
  EXPLÍCITO no ciclo do run(); (2) sem chamada de produção, clippy -D
  warnings mata `shutdown`/handle como dead code (Clone não conta); (3)
  clippy local do src-tauri morre no linker (GNU `link` do PATH intercepta
  o link.exe MSVC — ADR-7; CI é a prova, host só fmt + cargo check
  parcial).
- UI: barra com `role=progressbar` + throttle rAF (2297 beats não
  renderizam 2297 vezes — ~30 fps); Tauri 2 emite evento com `emit` do
  trait `Emitter` (`use tauri::Emitter`) — `core:default` já cobre
  `listen` no front; `.idle-dot` adicionada ao design.css.

## Monorepo packages/ (30/09 — refatoração estrutural)
- **Mapa de renomeação** (menções antigas em entradas históricas abaixo = caminhos da época):
  `gp100-core/` → `packages/core/` · `gp100-cli/` → `packages/cli/` · `ui/` →
  `packages/app/ui/` · `src-tauri/` → `packages/app/api/` (pedido do owner: "api é a API
  Rust do projeto"). Nomes de CRATES não mudaram (gp100-core/gp100-cli/gp100-ui =
  identidade de API em lockfiles/CLI/docs). Root limpo: só configs + docs/analysis/files/scripts.
- **Workspaces Cargo**: raiz = `members = ["packages/core","packages/cli"]`,
  `exclude = ["packages/app/api"]` (ADR-7 continua: Tauri exige MSVC, raiz é gnu).
- **Landmines de caminho já re-costurados** (não re-morder): `include_str!` ganhou +1 nível
  (core→`../../../docs/protocol_golden.json` e `../../../analysis/parameters.json`;
  mock→`../../../../files/patches/all.prst`); testes com `CARGO_MANIFEST_DIR` dão
  DOIS `pop()` até a raiz; path-deps `../core` (cli) e `../../core` (api);
  `frontendDist: "../ui/dist"` segue válido (ui e api são irmãos em app/);
  shim do tauri na CI = `packages/app/ui/node_modules/.bin/tauri.cmd` (run de
  `packages/app/api` com `../../ui/...`); node_modules do ui foi REFEITO após o
  mv (o store pnpm guarda caminho absoluto — mv quebra o install com
  ERR_PNPM_ABORTED_REMOVE_MODULES_DIR; rm -rf + CI=true pnpm install resolve).
- **CI aponta para**: `packages/app/api` (workdir ui-rust/release/audits),
  `packages/app/ui` (pnpm), regex do plan = `packages/core/|packages/cli/|packages/app/`.

## Pipeline único — lições de CI (30/09)
- **MATRIZ VAZIA CRASHA A RUN INTEIRA (não use nunca)**: job com
  `strategy.matrix: ${{ fromJSON(...) }}` e matriz `"include":[]` NÃO materializa
  nenhum job e a RUN TERMINA `failure` sem NENHUM job failed (comportamento do
  runner, community discussion 27096). FIX no `plan` (hoje em `_validate.yml`): emite
  BOOLEANOS (`run-rust/run-front/run-spec`) e os jobs ganham
  `if:`; matrizes dinâmicas só onde o plan garante ≥1 item; tag-release/release-*
  usam matriz FIXA de 1 item. Job filtrado = "skipped" (0 min), run verde.
- **CAUSA-RAIZ da failure 36747668222 era DUPLA**: além do crash da matriz vazia,
  o filtro de mudanças do plan usava `re.search(pat, changed)` SEM `re.M` — o
  `^` só casa no início da STRING INTEIRA e CHANGED é multi-linha: NENHUM
  projeto "mudava" (matrizes vazias sempre que o 1º arquivo do diff não batia).
  A refatoração packages/ expôs o bug (run 36764807538: push de packages/
  inteiro → tudo skipped). Fix: `re.I | re.M` + simulação local do diff antes
  de subir (o log do runner NÃO mostra o VALOR dos outputs do plan — validar
  localmente com o mesmo diff é o único caminho).
- **actionlint é o gate local do workflow** (`/tmp/actionlint` valida workflow +
  composite actions de uma vez); lição de schema: outputs de job NÃO podem ter
  chave duplicada (case-insensitive) — pegou `run-spec` duplicado no plan.
- **Dependabot × TS 7 (PR #2)**: bump `typescript 6.0.3→7.0.2` matou o lint nos 3 OS
  (tseslint 8.71 peer `<6.1.0`; erro na importação do config, antes de lintar).
  O build passa com TS 7 — só o lint quebra; diagnóstico por reprodução local
  (`pnpm add -D typescript@7.0.2 && pnpm lint`). Trava: `ignore:
  - dependency-name: "typescript"\n    versions: [">=7.0.0"]` na entry npm.
- **Dependabot × midir (PR #3)**: PR CLOSED nunca mergeado, mas CI 14/14 verde
  (compila no runner). No host de campo o 0.11 é impossível hoje (ver entrada do
  Estado vivo): dlltool GNU 2.44+ não gera o import lib do rustc e llvm-dlltool
  gera um que o loader rejeita. Armadilha EXTRA descoberta: o winget WinLibs
  entra no PATH do USUÁRIO e SOMBRIA ferramentas colocadas por append
  (`export PATH="$PATH:x"` NÃO vence — o dlltool quebrado 2.47 venceu o
  llvm-dlltool e dava `os error 1006` nos `kernel32.dll_imports.lib`); a
  configuração que funciona é PREPEND (`export PATH="/c/Users/Canta/.cargo/bin:
  $PATH"`). Defender EXONERADO como causa do 1006 — as exclusões de teste
  (processo dlltool.exe + pasta target/) foram REMOVIDAS; nada ficou.
- **Estado final dos MinGW no host (30/09)**: o WinLibs do winget foi
  DESINSTALADO (pacote inútil — dlltool quebrado e sombreava o PATH; uninstall
  do winget limpou o PATH do usuário sozinho). O mingw-builds 16.1 do choco
  (`C:\ProgramData\mingw64\mingw64\bin`, no PATH da MÁQUINA) FICOU, mas seu
  dlltool 2.46 é TÃO quebrado para o rustc quanto o do WinLibs — não use para
  import libs raw-dylib. Serve como GCC/binutils MANUAL: `gcc`, `objdump`,
  `x86_64-w64-mingw32-gcc` (útil p/ inspecionar DLLs e toolchains de C). O
  dlltool que o rustc encontra segue sendo o llvm-dlltool copiado em
  `~/.cargo/bin/dlltool.exe` (PREPEND no PATH vence o mingw64 da máquina).
- **Cargo PATH no Git Bash ad-hoc**: `export PATH="/c/Users/Canta/.cargo/bin:$PATH"
  (o scripts/add_cargo_path.ps1 fixa no sistema, mas shells novos da sessão podem
  não herdar). Duas regras: PREPEND (winget/choco adicionam dirs ao PATH do
  usuário que sombreiam por append) e caminho em ESTILO POSIX (Windows `C:\...`
  no PATH quebra a lista POSIX no `:` do drive).

## Infra CI (30/09 — security + release; 01/10 reestruturou em 5 workflows;
02/10 adicionou `container.yml` — imagem ci-linux, #41)
- **Reestruturação 01/10 (issues #31–#34)**: o `pipeline.yml` (805 linhas, tudo
  junto) virou `ci.yml` (triggers por FUNÇÃO de branch + close-linked integrado)
  → `_validate.yml` (reusable com plan/gate/spec/rust/front/e2e/visual/smoke) +
  `release.yml` (publish por tag + version/rc/promote) → `_publish.yml` (reusable)
  e `security.yml` (auditorias noturnas). `close-issues.yml` deixou de existir
  (job `close-linked` do ci.yml). Composite actions novas: `playwright-setup` e
  `tauri-linux-deps` (esta APOSENTADA no #41 — a imagem `ci-linux` cobre).
  `develop` foi CRIADA (antes: só main; o fluxo GitFlow
  inteiro — close-issues, rc→promote — estava morto sem ela).
- **LIÇÃO 01/10 (publish × GITHUB_TOKEN)**: tag empurrada com GITHUB_TOKEN NÃO
  dispara workflows (`on: push: tags`) — por isso rc/promote/version publicam no
  MESMO run via reusable `_publish.yml`; o trigger de tag cobre só tag humana.
- **LIÇÃO 01/10 (baselines visuais no CI)**: em CI o Playwright NÃO cria baseline
  nova (`updateSnapshots` resolvido não é sinal confiável) — o skip do
  `visual.spec.ts` usa `UPDATE_SNAPSHOTS=true` (env do ci.yml) + `--update-snapshots=all`
  para gerar. Run 36935738610: 48 falhas por depender do default.
- **LIÇÃO 01/10 (smoke Tauri)**: `e2e/tauri.smoke.mjs` subia 3 níveis e caía em
  `packages/` → procurava `packages/packages/app/api/target/debug/gp100-ui`
  (4 níveis = raiz do repo).
- **LIÇÃO 01/10 (Dependabot × glib)**: `security_update_not_possible` recorrente
  (Tauri 2 trava gtk-rs 0.18; fix do RUSTSEC-2024-0429 só em 0.20+) → `ignore`
  documentado no dependabot.yml + alerta dispensado como risco aceito no SECURITY.md.
- **security.yml (noturno 06:30 UTC)**: pytest + cargo audit (2 lockfiles) +
  pnpm audit --prod + outdated informativo. Achado = **exit code das steps**
  (outcome), NUNCA grep de log (grep pegou crash de toolchain como "achado"
  → issue falso-positiva #1, fechada com documentação). RUSTUP_TOOLCHAIN=stable
  no job: o pin gnu da raiz quebra qualquer cargo no Linux (lição ADR-7 de novo).
- **Dependabot (30/09)**: `.github/dependabot.yml` — 4 entries SEMANAIS
  (github-actions `/`, npm `/packages/app/ui`, cargo `/packages/app/api`,
  docker `/.github/docker/ci-linux` — base Ubuntu da imagem de CI, #41;
  segunda 09:00 UTC = 06:00 BRT), groups p/ 1 PR/ecossistema/semana, limit 5.
  Entry npm tem `ignore: typescript >=7` (trava TS 7 — ver lições). Labels
  provisionadas ANTES via `gh label create --force` (dependencies, rust,
  npm, github-actions) — o Dependabot NÃO cria label e update com label
  inexistente falha (mesma lição do achados-security).
- **Dependabot × rust-toolchain.toml (ADR-8)**: o updater roda em container
  LINUX e não expõe RUSTUP_TOOLCHAIN — canal com triple Windows
  (`stable-x86_64-pc-windows-gnu/msvc`) nem parseia no rustup de lá
  ("target tuple in channel name") e TODA dependência falha com
  `dependency_file_not_resolvable` (provado nas runs 36702829331/…9326).
  Fix: src-tauri usa `channel = "stable"` (portável — resolve p/ o triple
  do host; alvo exato vem de RUSTUP_TOOLCHAIN na CI). Entry `cargo /`
  (raiz) NÃO existe — DELIBERADO: o pin gnu é load-bearing no host (sem
  VS Build Tools) e fica; lockfile raiz segue com `cargo update`
  deliberado (padrão A-1/A-3) + security noturno deteta vulns.
  Detalhe: deps de PATH atravessam a fronteira — o midir do core entrou
  no grafo do src-tauri e o PR da entry src-tauri editou
  `gp100-core/Cargo.toml` (a CI de 14 jobs provará o bump).
- Validação de dependabot.yml local: `uv run --with pyyaml python -c ...`
  (PyYAML NÃO está no venv do projeto — usar overlay do uv, sem tocar no
  pyproject.toml).
- **release.yml (tag v*)**: CLI de campo gnu (gates + smoke 2/3 + zip com kit
  H1 + SHA256) e instalador NSIS via `tauri build` (MSVC). Artefatos sempre
  publicados como artifacts; GitHub Release só em tag.
- **tauri-cli (armadilhas):** (1) acha o tauri.conf.json DESCENDO do cwd;
  (2) exige crate MEMBRO de workspace cargo (sem [workspace] próprio → panic
  Option::unwrap em rust.rs) — src-tauri tem [workspace] próprio e o build
  roda de dentro dele; (3) `pnpm exec` da raiz do repo não funciona (raiz não
  é pacote pnpm) — usar o shim `packages/app/ui/node_modules/.bin/tauri(.cmd)`.
- **Actions node24:** checkout@v7, setup-node@v7, setup-uv@v10.2.0 (o repo
  do setup-uv NÃO publica major tag — pino sempre a versão exata!),
  upload-artifact@v7, rust-cache@v2. Geração nova das actions do Docker
  (setup-buildx@v4, login@v4, metadata@v6, build-push@v7) e actions/cache@v6
  (até 01/10: @v3/@v5/@v6 e cache@v4 miravam Node 20 — o runner avisa
  "Node.js 20 is deprecated... forced to run on Node.js 24"). Contrato no
  validate_workflows.py: referência node20 falha o gate.
- **LIÇÃO #43 (gate do front em 1 OS)**: lint + `vitest --coverage` são
  plataforma-independentes (jsdom/eslint puros) — rodar o trio em 3 OS só
  duplicava custo. O PLANO elege o OS do gate (ubuntu; no `ci-lite`, Windows,
  que é o único OS que sobe) e a matriz carrega `full` por OS; o que fica nos
  3 é o `pnpm build` (tsc + vite), que é o typecheck sensível à plataforma.
  Passo medido: macOS 30s→7s, Windows 20s→6s.
- **LIÇÃO #43 (cache é ESCOPADO por branch — cuidado ao medir)**:
  `workflow_dispatch` numa branch NÃO enxerga os caches de `develop` (só os do
  branch DEFAULT) — a 1ª rodada de uma branch sai fria (Chromium +29s, store
  do pnpm +13s, `ws-api` +23s, pull da imagem +32s) e a parede mente
  (36955948885: +39s "de regressão" que era só cache frio). Meça na 2ª rodada
  da branch, ou via PR (que herda o cache da branch base).
- **LIÇÃO #41 (rust-cache `rustc -vV` × pin da raiz)**: o probe do cache roda
  DENTRO da composite `setup-rust` e NÃO enxergava o `RUSTUP_TOOLCHAIN`
  exportado via GITHUB_ENV — o pin gnu do `rust-toolchain.toml` da raiz vencia
  e o rustup respondia "target tuple in channel name" (##[error] no job: o
  cache cai no fallback e o job fica verde, mas a annotation suja; era
  PRÉ-EXISTENTE ao #41). Fix: `RUSTUP_TOOLCHAIN` no `env:` do JOB, com
  expressão sobre `matrix` (o `runner.os` só existe em step; `matrix` vale em
  env de job) — env de job chega a todo step, inclusive dentro de composite.
- **LIÇÃO #41 (imagem de CI × hot path)**: o que é FIXO não pertence ao job.
  Rust+clippy/rustfmt, `tauri-driver` compilado, WebKitGTK/GTK/ALSA dev,
  `webkit2gtk-driver`, xvfb+mesa e Node/pnpm com store aquecido viraram camada
  da imagem `ghcr.io/<repo>/ci-linux` (Dockerfile versionado em
  `.github/docker/ci-linux/`), publicada pelo `container.yml` (`:1` estável +
  `sha-<curto>`). Eliminou por run: apt 57s (ui-rust Linux) e apt 45s +
  `cargo install` 17s (smoke Tauri). **ONDE o container NÃO se paga:**
  e2e/e2e-visual/front — o pull da imagem custa mais que o Chromium que ele
  substituiria; nesses a alavanca é o cache do `~/.cache/ms-playwright`
  (chave = hash do pnpm-lock.yaml). Regra: container para toolchain/servidor
  gráfico; cache para o resto. Medir com `scripts/ci_timings.py`.
- **LIÇÃO #41 (bootstrap de workflow_dispatch)**: dispatch de workflow que só
  existe numa branch de trabalho → 404 ("not found on the default branch"). A
  1ª publicação da imagem precisou de um gatilho de push TEMPORÁRIO na própria
  branch, removido no mesmo PR. Dispatch manual depois do merge: ok.
- **LIÇÃO #41 (callee × caller: startup_failure no LOAD)**: declarar
  `permissions: packages: read` no reusable `_validate.yml` derrubou TODO o run
  antes de qualquer job — "The workflow is requesting 'packages: read', but is
  only allowed 'packages: none'" (o caller ci.yml/release.yml não concedia).
  A mensagem NÃO aparece em log (a run nem tem logs) e o actionlint não pega:
  só na aba **Annotations** da run. Callee pede, caller CONCEDE. Contrato
  travado no `validate_workflows.py` (lição de leitura obrigatória).
- **LIÇÃO #41 (cache do cargo por WORKSPACE)**: o input `key` do
  Swatinem/rust-cache apenas SOMA à chave automática POR JOB
  (`add-job-id-key` default true) — nenhum job compartilhava cache. Compartilhar
  é `shared-key` (por workspace: `ws-raiz` | `ws-api`) + `add-job-id-key:
  "false"`. Guard extra (validado no gate): `gate` usa `cache: "false"` —
  `cargo fmt` não compila e não deve baixar GB do cache compartilhado.
- **LIÇÃO #41 (HOME nos jobs de container)**: o runner executa o container com
  HOME=/github/home; por isso RUSTUP_HOME/CARGO_HOME da imagem vivem em `/opt`
  (sem isso o rustup não acha toolchain nenhuma) e o store do pnpm em
  `/opt/pnpm-store`, passado por env `npm_config_store_dir` (pnpm lê config por
  env `npm_config_*`). Ferramenta faltando = falha no BUILD do Dockerfile
  (camada de sanidade), nunca no job.
- **LIÇÃO #41 (macOS arm64 × anotação de fila)**: TODO label `macos-*` padrão é
  arm64 e carrega o aviso "capacity constraints ... longer queue times" do
  GitHub (ruído de infra, não erro do repo). O front migrou para
  `macos-15-intel` (x86_64; casa com Windows/Linux) e o ui-rust saiu do macOS —
  2 OS (Windows MSVC + Linux/container), com a UI coberta pelo front/vitest
  3-OS.
- **Labels:** ubuntu-24.04 (migração p/ 26 em 19/10/2026), macos-15-intel
  (x86_64 desde o #41 — mata a anotação de fila do arm64; EOL do Intel
  ~08/2027, reavaliar antes do v1.0.0).
- `zip` não existe no Git Bash do runner Windows: `7z a -tzip` fallback.
- `gh issue create --label` falha se o label não existir: criar com --force
  antes (idempotente).
- **LIÇÃO #51 (outputs de step × id)**: `steps.<id>.outputs` só existe com
  `id` EXPLÍCITO no step que escreveu o `$GITHUB_OUTPUT`. O resumo do
  `close-linked` referenciava `steps.close-linked.*` num step SEM id: o
  actionlint acusou ("property não definida") e os totais saíam VAZIOS em toda
  run — sem falhar, então ninguém notava. Fix: `id: close-linked` no step de
  fechamento; os totais agora aparecem no `$GITHUB_STEP_SUMMARY`.
- **LIÇÃO #51 (título de PR no run = injeção)**: `github.event.pull_request.title`
  interpolado INLINE num `run:` é entrada NÃO confiável — qualquer pessoa abre
  um PR com título `"; curl …` e o shell executa. Regra: SEMPRE por `env:`
  (ex.: `PR_TITLE`) e `"$PR_TITLE"` no script; o actionlint checa isso a cada
  run (mesma classe do `BODY` da extração, que já nasceu por env).
- **LIÇÃO #50 (job próprio × parede do front)**: o gate do front (lint +
  `vitest --coverage`) morava na leg ubuntu da matriz `front`, atrás do Setup
  Node e ANTES do build — soma no caminho crítico. Extraído para o job
  `front-gate` (ubuntu; Windows no ci-lite) ele roda EM PARALELO com os 3
  builds: a parede vira o MÁXIMO (gate × builds), não a soma. O `build-front`
  ficou só build (o input `full` morreu) e os contratos do
  `validate_workflows.py` acompanharam (job novo na lista, `gate-os` no plano,
  `pnpm build` no composite).

---

## Edge cases de IPC nível 2 (02/10 — issue #20)

- **Estado otimista é mentira**: `stepPreset` trocava o `pp` e um
  `useEffect([pp])` disparava o select — quando o select falhava, navbar/LED/
  biblioteca exibiam um preset que o device NÃO aceitou. Fix: `pp`/nome só mudam
  depois do `deviceBoard` confirmar (o `openPreset` faz select+leitura e é a
  única fonte); falha vira banner com AÇÃO (retry) e a UI fica no preset REAL.
- **Retry/backoff mora na PORTA ÚNICA do front** (`ipc/device.ts`): o backend já
  tem timeout por TRANSAÇÃO (D6, ADR-6) e o actor serializa a fila (D8) — retry
  no front é uma nova transação, observável em teste. Leitura sempre retentável;
  `select`/`set_param` reenviam o MESMO destino/valor (fire-and-forget §13.11 é
  idempotente). **Boot FORA da política** (2297 transações: retry automático
  mascararia device morto) — a recuperação dele é o ⟳ do usuário.
- **Gancho de falha com modo TRANSITÓRIO**: `gp100.debug.failDevice = "op:n"`
  falha as n PRÓXIMAS chamadas e DECREMENTA a chave — o que sobra na chave vira o
  contador de tentativas que o teste usa para PROVAR o backoff (`info:5` → sobra
  `info:2` quando a política faz 3 tentativas). `"boot-mid"` emite progresso real
  até ~40% e então rejeita: o cenário de cabo puxado com a UI aberta, sem mexer
  no core (que não tem transporte que falha).
- **Timeout é o que impede o spinner eterno**: cada tentativa corre contra
  `COMMAND_TIMEOUT_MS` (8 s), provado no branch Tauri com `invoke` que nunca
  resolve (fake timers: 3 timeouts + 2 backoffs).
- **Log de push só serve se for legível**: validar (`F0…F7`, tamanho par, só hex)
  e deduplicar repetição CONSECUTIVA na MESMA linha (`×N`) — o boot repete a
  resposta de tabela dezenas de vezes (backlog D7). Push inválido devolve o MESMO
  array (zero render). Parsing/dedupe é PURO (`ipc/push.ts`) — testável sem DOM.
- **Achado A-5 (issue #45, RESOLVIDO)**: `toHaveScreenshot` compara a baseline da
  PLATAFORMA (`{arg}-{platform}`); o repo tinha 54 `-win32` e **0 `-linux`** → no CI
  ubuntu tudo SKIPAVA (o visual não era gate de nada) e o dev local acumulava drift
  silencioso (12 diffs na árvore limpa). **Receita para nascer/adicionar baseline:**
  1) local, `npx playwright test visual.spec.ts --update-snapshots=changed` (só o que
  difere); 2) no CI, `gh workflow run ci.yml --ref <branch> -f update-snapshots=true`
  → o job ubuntu escreve as `-linux` e sobe o artefato `visual-snapshots`;
  3) `gh run download <id> -n visual-snapshots -D /tmp/x` e copiar SÓ os `-linux`;
  4) commitar os dois. O `test.skip` por baseline ausente **não existe mais**: no CI
  a ausência falha o job (o Playwright lá nunca escreve baseline sozinho). Antes de
  culpar o próprio PR, PROVE com `git stash` (mesmo nº de pixels = pré-existente) —
  foi assim que o drift de 12 baselines ficou separado do #20.
- **`--update-snapshots=all` reescreve TAMBÉM o que já batia** (re-encode muda o
  byte do PNG): use `=changed` no local e filtre por `-g "<nome>"` quando o alvo é
  um grupo só (evita commitar baseline alheia sem mudança real de pixels).
- **Baseline gerada ANTES do rebase ENVELHECE** (aconteceu no #45): as `-linux`
  nasceram numa árvore sem o botão de retry que o #46 adicionou e o banner do
  `erro-preset` ficou 38px contra 50px do código — o PR abriu vermelho com
  "Expected an image 1400px by 38px, received 1400px by 50px". Regra: gere/baixe
  o artefato DEPOIS do rebase final (ou re-dispare o `update-snapshots`); e
  compare com `cmp -s` antes de copiar, para commit só do que mudou de verdade.
- **O job `e2e` também roda o `visual.spec.ts`** (`playwright test` pega todos os
  specs): gerar baseline pelo input deixa o `e2e` vermelho enquanto o
  `e2e-visual` escreve — é o sintoma de artefato velho, não de bug. Se um dia
  isso incomodar, separe os projects (visual × funcional) em vez de duplicar.

---

## DeviceGone ponta-a-ponta (02/10 — issue #48)

- **Tipo perdido é bug de UI**: `TransportError::DeviceGone` existia desde o M0.5
  e o `real.rs` documentava que desconexão física nem vira `DeviceGone` (vira
  RecvTimeout) — resultado: NENHUM teste exercitava device morrendo e a FSM
  ACHATAVA o erro em `InvalidShape` ("transporte saudável"). O front decide
  diferente (LED off + retry explícito, sem retry automático), então o tipo agora
  ATRAVESSA: `tx_err`/`wait_for` mapeiam para `ProtocolError::DeviceGone { why }`
  e o `open()` do mock pós-morte também devolve `DeviceGone` (device não
  ressuscita).
- **A falha entra no MOCK — nunca num transporte novo**: `MockFault::DieAfter(n)`
  + `with_fault()` (e `transactions()` para asserção) no `MockDevice`. O mock é
  o test double DECLARADO do projeto: o caminho testado é exatamente o de
  produção (transporte → FSM → actor → command → UI) e o `RealDevice` fica
  intocado (ADR-4/5). A morte PRECEDE o parse: `sent` incrementa ANTES do
  decode e devolve `DeviceGone` — quem sumiu do fio não avalia shape.
- **Gancho de shell por env**: `GP100_DEBUG_FAULT=die-after:<n>` lido no `run()`
  do gp100-ui (parser puro `parse_debug_fault`; valor malformado = backend
  saudável — env de debug não derruba app). Exclusivo do backend MOCK; o
  transporte real nunca lê.
- **O smoke prova o cenário com UMA sessão**: o tauri-driver sobe com o env no
  `spawn` (herança wrapper → WebKitWebDriver → app) e `die-after:60` derruba o
  device no MEIO do boot do mount (transação 61, início do scan). O smoke
  assere o alerta amigável (XPath `contains(., …)` — e um XPath NEGATIVO prova
  que "MockFault"/"die-after" não vazam), depois clica o ⟳: a 2ª falha FIXA o
  estado — LED off (classe `.idle-dot`, sem depender de `getText`), NENHUMA
  barra de progresso e retry habilitado. Nada de esperar boot saudável.
- **LIÇÃO #48 (beats de boot × webview do CI)**: o boot de 2297 transações
  emite 2297 eventos `device://progress`; o webview do CI (WebKitGTK sob xvfb,
  software rendering) leva **~65 s** para drená-los (run 36999196897: o
  `.live-dot` apareceu exatamente quando o wait de 60 s estourou). Por isso o
  smoke do #48 NÃO espera o boot terminar — e um wait de LED "on" com 60 s era
  flake por construção. Fica registrado como candidato a otimização (o front
  já throttla por rAF; o custo está na travessia dos eventos).
- **Morte não espera a janela de 3 s**: o teste do actor (`DieAfter(300)`) falha
  em <2 s — se o tipo voltasse a ser achatado num timeout, levaria 3 s+ e o
  teste pega isso (o boot completo de 2297 leva ~0,2 s; 300 transações são
  instantâneas).

---

## Pedal real no palco (02/10 — issue #19, fatia 1)

- **`Pedal`/`Pedalboard` existiam desde o `8eb40af`, mas estavam DESLIGADOS**: o
  App renderizava o `EmptyBoard` (só os 9 lugares) e o comentário do topo dizia
  "knobs/toggle/set_param entram um efeito por vez". A fatia 1 troca o palco por
  `Stage` (cabeçalho LED/trava/tuner + sockets): `PEDAL_FAMILIES_READY` (hoje
  `PRE`) decide quais slots viram pedal REAL do `device_board`; o resto fica
  placeholder. `Pedalboard` (3 linhas + cabos) segue como o ALVO da fase em que
  os 9 pedais estiverem no ar — não é código morto.
- **Semântica do set_param no fio**: `device_set_param(slot, code, ctrl, value)`
  usa o `slot` em 1..=9 (o `BoardSlot.slot` da UI é 0..8 → `+1`); `ctrl` = `pos`
  do dicionário; `value` f32 em unidades de display. O `code` do efeito vem do
  ARTEFATO (`(nibble<<24)|index` — no COMP do mock, `0`).
- **Toggle/switch/combox ainda são LOCAIS**: o protocolo capturado só tem o SET
  f32 (§13.11) — não existe comando de toggle de efeito nem encoding de opção de
  switch/combox. O LED verde/vermelho e o ciclo de opções são prévia local; o
  único caminho de escrita do pedal é o knob numérico (com `errSetParam` novo no
  banner do App para falha permanente — nada silencioso, regra do #20).
- **Layout do palco por CONTEÚDO**: o `.board-slots` deixou o `repeat(9, 1fr)`
  (placeholders iguais) por `minmax(132px, max-content)` — um pedal real tem
  ~420px de largura mínima e não cabe num `1fr` de 3×3; agora são 3 colunas até
  1700px e 9 acima, com `overflow-x: auto` de segurança. O `measureShell` do e2e
  passou a contar colunas POR LINHA e a medir sobreposição (larguras diferentes
  quebraram o `sameSlotWidth`/`pitchUniform` antigos de propósito).
- **Baselines mudam em cascata**: o palco mais alto estica a biblioteca
  (`stretch`, alturas iguais) e ENCURTA a faixa de alerta — a `main` usa
  `auto 1fr auto` e, com alerta visível, é ele que ocupa a linha `1fr`
  (erro-boot 1920: 153→39px; lib 1440: 463→909px). Além de `board-*`, `lib-*` e
  `erro-boot-*` precisam de baseline nova nas DUAS plataformas (win32 local com
  `--update-snapshots=changed`; linux via dispatch `update-snapshots=true`).
