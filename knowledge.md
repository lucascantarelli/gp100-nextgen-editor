# knowledge.md — GP-100 NextGen Editor (ler antes de agir)

> Mapa geral da documentação (o que consultar para cada assunto): **`docs/INDEX.md`**.

Projeto: substituto do Valeton Suite para a pedaleira GP-100, por engenharia reversa
(local, sem depender de hardware para ~95% do trabalho). Resposta ao usuário SEMPRE em PT-BR.

## Estado vivo (atualizar aqui a cada marco)
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
- `docs/DECISIONS.md` — ADR-1..5 do gp100-core (P5); mudar decisão = novo ADR
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
- Ferramentas novas de análise = script em `analysis/` (não heredocs longos).
- Descobertas de protocolo vão para docs/PROTOCOL.md com evidência (VA/hex da captura), nunca só conversa.
- Não commitar sem pedido; não tocar em `analysis/nsis_app/` (é material extraído, read-only).
