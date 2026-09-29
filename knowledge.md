# knowledge.md — GP-100 NextGen Editor (ler antes de agir)

> Mapa geral da documentação (o que consultar para cada assunto): **`docs/INDEX.md`**.

Projeto: substituto do Valeton Suite para a pedaleira GP-100, por engenharia reversa
(local, sem depender de hardware para ~95% do trabalho). Resposta ao usuário SEMPRE em PT-BR.

## Estado vivo (atualizar aqui a cada marco)
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
  testes verdes; CLI já bloqueia --real. ⚠️ PATH: adicionar C:\Users\Canta\.cargo\bin.
- P3 ✅ (28/09): suíte de regressão `analysis/tests/test_protocol.py` — `uv run pytest`
  = 9/9 (~4s): golden 5 provas com critérios objetivos + hash da baseline + knob_map 13/14
  (exceção CAB/Mic). Gate de qualquer PR que toque spec/decoders.
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
  (op 0/1 = sair/entrar modo edição) + re-sync 11000008 (31 regs zeros) + status 12000001.
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
