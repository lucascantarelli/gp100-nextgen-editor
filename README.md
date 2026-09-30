# GP-100 NextGen Editor

[![pipeline](https://github.com/lucascantarelli/gp100-nextgen-editor/actions/workflows/pipeline.yml/badge.svg)](https://github.com/lucascantarelli/gp100-nextgen-editor/actions/workflows/pipeline.yml)

> 📇 **Mapa da documentação:** `docs/INDEX.md` — rotas por objetivo, fonte de
> verdade por assunto e inventário anotado. Comece por lá se está lost.

Aplicativo multiplataforma (Windows/Linux/macOS) para a pedaleira **Valeton GP-100**,
construído por engenharia reversa para substituir o "Valeton Suite" oficial
(Windows-only, shell CEF). **Núcleo em Rust (`gp100-core`) + UI em Tauri 2/React.**

**Status atual (29/09):** a engenharia reversa está **concluída e validada em campo**
(protocolo comprovado byte-a-byte contra 4 sessões de captura) e a **Fase M0
está 100% concluída (8/8)**: model (M0.1), preset round-trip (M0.2), golden
consumer (M0.3), codec de fio (M0.4), transporte + MockDevice (M0.5), FSM de
sessão com **replay byte-a-byte das 4 fixtures** (M0.6), **gp100-cli com
`--log` no schema P4** (M0.7) e documentação do core com exemplos que rodam
(M0.8) — tudo no remoto com CI verde (78 testes Rust + pytest 10/10).
Próximos: **M1** (UI, `docs/UI_PLAN.md`) e/ou o **gate H** de hardware
(`docs/H1_CHECKLIST.md`). Planejamento issue-a-issue: `docs/ROADMAP.md`.

> 🔒 **Política de segurança de hardware:** nenhum fluxo de escrita sai sem captura
> validada (`WRITE_VERIFIED`), e **update de firmware está fora de escopo** (V2+, e só
> com bootloader documentado). Ver `docs/BLOCKERS.md` §4 e `docs/VISION.md` §7.

---

## 1. O que já foi conquistado (panorama da RE)

| Camada | Resultado | Onde |
|---|---|---|
| **Modelo de dados** | 185 algoritmos / 639 controles com nomes, ranges e defaults, validado em 3 vias (XML oficial do Suite + 909 slots de patches reais + strings da firmware) | `analysis/parameters.json` |
| **Formato de preset** | `.prst` = XML puro; `effectCode = (nibble<<24) \| index`; cadeia de 9 slots fixos (`x=0..8`); `params_N` 0..14 (incl. controles ocultos como o Mic do CAB) | `docs/PROTOCOL.md` §13.9 |
| **Protocolo de fio** | SysEx `F0 21 25 7F 47 50 2D 64 \| FUNC \| ADDR(4B) \| DATA \| F7`; ler estado (páginas `13xx`), setar parâmetro (envelope semântico `10xx0002` com float32), save (metadados `11xx` + ops `00020000`), upload de IR com ACK por chunk | `docs/PROTOCOL.md` §13.1–13.12 |
| **Especificação executável** | 40 templates request→resposta extraídos das capturas, com padrões de payload (const/var) e semântica por endereço | `docs/protocol_golden.json` |
| **Validação** | Accounting IN 100% / OUT 99,6% (resto = truncamentos do ring buffer); regeneração byte-a-byte: knobs 92/92, boot/scan 2299/2299 | `analysis/validate_golden.py` |
| **Persistência comprovada** | O save da UI persiste o **estado ao vivo** (confirmado no display da pedaleira após a sessão 4) | `docs/PROTOCOL.md` §13.12 |

**Como chegamos lá:** um proxy `winmm.dll` (129 exports) instrumenta o Suite oficial
sem nenhum risco ao device — intercepta `midiOutShortMsg/LongMsg` e os buffers de
entrada, e loga tudo em JSONL com timestamps. As 4 sessões de captura cobriram:
boot/scan de presets (S1), upload de IRs mono+estéreo (S2), edição de knobs pela UI
(S3) e save com confirmação de persistência (S4).

---

## 2. Estrutura do repositório

```
├── packages/                  # monorepo de código (root limpo = só configs)
│   ├── core/                  # gp100-core (lib Rust): model, preset, golden,
│   │   │                      #   codec, transport (mock/real), session
│   │   ├── README.md          # arquitetura do core + exemplos que rodam
│   │   └── tests/             # contratos caixa-preta (fixtures P4, replay)
│   ├── cli/                   # gp100-cli (bin Rust): info/list-user-irs/… via
│   │                          #   mock + --log no schema P4 (insumo do gate H1)
│   └── app/                   # aplicativo desktop (Tauri 2 + React)
│       ├── ui/                # front React/TS: vite, vitest, tokens, ipc/
│       ├── api/               # backend do shell (crate gp100-ui): commands,
│       │                      #   DeviceActor — workspace MSVC próprio (ADR-7)
│       └── README.md          # divisão front/backend + como rodar
├── docs/
│   ├── INDEX.md               # mapa da documentação (comece por aqui)
│   ├── VISION.md              # visão, arquitetura, stack, features (rev. v1.2)
│   ├── ROADMAP.md             # plano executivo P/M0/H com issues e DoD
│   ├── UI_PLAN.md             # planejamento issue-a-issue da UI (M1)
│   ├── PROTOCOL.md            # referência do protocolo (§1–12 arquivo, §13 fio)
│   ├── protocol_golden.json   # especificação executável (40 templates, baseline)
│   ├── DECISIONS.md           # ADR-lite: 6 decisões estruturais do gp100-core
│   ├── BLOCKERS.md            # matriz de riscos/bloqueios (11/12 resolvidos)
│   └── CAPTURE_PLAN.md        # plano das capturas (histórico)
├── analysis/                  # laboratório de RE
│   ├── parameters.json        # dicionário canônico (185 algs / 639 controles)
│   ├── knob_map.json          # mapa de 89 edições knob→fio (sessão 3)
│   ├── capture_gaps.md        # gaps de captura G1–G6 (G1/G2 fechados)
│   ├── captures/              # session1–4.jsonl + ir_slot*.bin (brutos)
│   ├── build_proxy.py         # gera/compila/valida o proxy winmm (zig)
│   ├── midi_proxy.c …         # fontes do proxy
│   ├── suite_local/           # Suite instrumentado (GP-100.exe + winmm.dll)
│   ├── build_golden.py        # extrai os templates → docs/protocol_golden.json
│   ├── validate_golden.py     # 5 provas de validação (accounting + geração A–E)
│   ├── validate_knob_map.py   # valida o envelope do knob vs dicionário/.prst
│   ├── make_fixtures.py       # fatia as capturas → fixtures/ (replay do M0)
│   ├── fixtures/              # boot/knobs/save/ir + manifest (paridade vs golden)
│   ├── decode_wire.py …       # decoders de captura
│   ├── FINDINGS_*.md          # achados da RE estática do exe (capstone)
│   ├── nsis_app/              # extração do instalador (read-only, não indexar)
│   └── captures/              # session1–4.jsonl + ir_slot*.bin (append-only)
├── files/                     # artefatos oficiais de entrada (firmware, instaladores,
│   └── patches/*.prst         #   patches: all.prst = 99 presets)
├── scripts/
│   └── add_cargo_path.ps1     # fix do PATH do cargo no sistema (HKLM, idempotente)
├── .venv/                     # Python do projeto (uv, VENV ÚNICO na raiz)
├── knowledge.md               # memória operacional do agente (estado vivo, armadilhas)
└── .agents/skills/            # workflows sob demanda (proxy-build, capture-analyze…)
```

---

## 3. Ambiente

Windows + Git Bash (desenvolvido em `D:\GP-100 app`). Requisitos:

- **[uv](https://docs.astral.sh/uv/)** (gestão Python, raiz): `uv sync --all-groups`
  recria o venv único `.venv/` na raiz (capstone, pefile, **ziglang no venv**, pytest).
  Rodar comandos da raiz com `uv run python ...`.
- **Rust** stable (M0+, toolchain windows-gnu pinada em `rust-toolchain.toml`) e
  **Node 20+** (M1, Tauri/React). Terminal novo sem `cargo`? Fix permanente:
  `scripts/add_cargo_path.ps1` (uma vez, como admin). Gates de código Rust:
  `.agents/skills/rust-practices/SKILL.md`
- **CI (GitHub Actions):** os mesmos gates (`uv run pytest` + `cargo fmt/clippy/test`)
  rodam a cada push e PR (pipeline único: `.github/workflows/pipeline.yml` —
  matrix 3-OS, security noturno, versionamento semver automático e release
  por tag; passos reutilizáveis em `.github/actions/*`)
- Agente/IA: ver `knowledge.md` (armadilhas) e `.agents/skills/` (workflows:
  proxy-build, capture-analyze, new-session, spec-baseline, protocol-validate,
  core-dev, docs-sync, rust-practices)

Armadilhas conhecidas (cp1252, `%TEMP%` real vs `/tmp`, log append-only, F7 embutido
em mensagens paginadas) estão catalogadas no `knowledge.md`.

---

## 4. Workflows de regeneração

### 4.1 Proxy winmm (instrumentação do Suite)
```bash
uv run python analysis/build_proxy.py
cp analysis/winmm.dll analysis/suite_local/     # obrigatório após o build
```
O `build_proxy.py` gera o C de forwarding, cruza exports (129) com a winmm real,
compila com zig e valida o resultado.

### 4.2 Nova sessão de captura (requer a pedaleira)
1. **Feche TODAS as instâncias do Suite** (instância única mata a nossa silenciosamente)
2. Rode `analysis/suite_local/GP-100.exe` — o proxy loga em `%TEMP%\midi_trace.jsonl`
3. Execute o roteiro da sessão (ver `analysis/capture_gaps.md` para os gaps abertos
   G3–G6); pausas de ~5s entre ações separam as fases na timeline
4. Feche o Suite e copie o log:
   ```bash
   cp "$LOCALAPPDATA/Temp/midi_trace.jsonl" analysis/captures/sessionN.jsonl
   ```
5. Analise:
   ```bash
   uv run python analysis/decode_wire.py analysis/captures/sessionN.jsonl
   ```
   ⚠️ O log é **append-only** (sessões novas contêm as antigas — segmente por gaps >30s)
   e mensagens sem `F7` são truncamentos do ring buffer (ignore).

### 4.3 Golden-file do protocolo
```bash
uv run python analysis/build_golden.py      # gera docs/protocol_golden.json
uv run python analysis/validate_golden.py   # 5 provas (deve dar 100%)
uv run pytest                               # suíte de regressão (após P3)
```
O golden cobre as 4 sessões. Regras de largura de campo confirmadas:
select/open de preset = pp **u16 BE**; página = `pp u16BE + PG u16BE + 01` (5B);
chaves da tabela de nomes = `[banco u8][índice u8]`.

### 4.4 Dicionário e mapa de knobs
```bash
uv run python analysis/validate_knob_map.py # revalida knob_map.json
```

---

## 5. Roadmap

- **M0 — gp100-core (Rust) — ✅ 100% (8/8, 29/09):** model (M0.1), preset
  round-trip byte-idêntico (M0.2), golden consumer (M0.3), codec de fio
  (M0.4), transporte + MockDevice D1–D8 (M0.5), FSM de sessão com replay
  byte-a-byte das 4 fixtures (M0.6), gp100-cli com `--log` no schema P4
  (M0.7) e docs do core com contrato de exemplos (M0.8) — CI verde.
  *Aceite da fase: replay byte-a-byte das capturas 1–4 — atingido na M0.6.*
- **H — gate de hardware (entre M0 e a escrita real):** H1 (leitura real) →
  H2 (escrita dos 3 fluxos capturados) → H3 (golden v1.1 se houver ajuste).
  Detalhes no ROADMAP. A M1 pode começar em paralelo (mock), mas o modo real
  da UI/CLI só existe após H1/H2.
- **M1 — Editor UI (Tauri 2 + React/TS):** biblioteca (import `all.prst`), editor de
  cadeia, knobs com ranges reais, diff/undo; device mock primeiro, hardware depois
  (set/save/IR já verificados em campo; `WRITE_VERIFIED=true` só para fluxos capturados).
  **Planejamento issue-a-issue: `docs/UI_PLAN.md` (M1.0–M1.6 com DoD).**
- **M2 — IR lab + SnapTone manager + empacotamento:** laboratório de IRs (upload já
  funcional), gestor de NAM, i18n (pt-BR/en/es/zh — strings da firmware reutilizáveis),
  MSI/AppImage/dmg.
- **M3 — Diferenciais:** biblioteca versionada git-like, live mode, cloud opt-in,
  tone match IA, A/B blind test (lista completa em `docs/VISION.md` §9).Dívidas de baixa prioridade (não bloqueiam nada): layout byte-a-byte da
página de estado 13xx, campo 0x00BC/0x00B4 do blob de IR, semântica de
ppEXP1/ppCtrl, capturas G3–G6 (globals/BPM, knob físico, footswitch).
(A tabela de TIPOS da `12001002` e o schema do `11000007` foram FECHADOS
em §13.12 — não são dívidas.)

---

## 6. Roteiro para novos contribuidores

**Onboarding (leia nesta ordem):**
1. `docs/INDEX.md` — o mapa da documentação (rotas por objetivo, fontes de verdade)
2. `docs/VISION.md` — o produto que estamos construindo e por quê
3. `docs/PROTOCOL.md` §13 — o protocolo de fio (a narrativa)
4. `docs/protocol_golden.json` — a mesma coisa, byte-a-byte (a especificação executável)
5. `packages/core/README.md` — **a arquitetura do código com exemplos que RODAM**
   (contrato `tests/readme_examples.rs` garante que a doc não mente)
6. `docs/BLOCKERS.md` — o que está resolvido e o que é risco
7. `knowledge.md` — armadilhas de ambiente que já morderam alguém

**Primeiro hands-on (10 min):**
```bash
cargo build --workspace
cargo run -p gp100-cli -- info
cargo run -p gp100-cli -- dump-preset 0x0007
cargo test --workspace
```

**Regras da casa:**
- **Preserve o round-trip**: qualquer preset aberto→salvo sem mudanças deve gerar
  XML byte-idêntico (teste de regressão obrigatório)
- **Consuma o golden-file**, nunca reparseie logs de captura
- **Escrita no device**: somente os 3 fluxos capturados (set §13.11, save §13.12,
  IR §13.7); fluxo novo = captura própria primeiro, feature-flag depois
- **Nunca toque em rotinas de firmware update** (política V2+)
- Descobertas de protocolo vão para o `PROTOCOL.md` com evidência — nunca só conversa
- Não commite sem pedido; `analysis/nsis_app/` e `files/` são material de origem

**Boas primeiras tarefas (estado 29/09 — M0 concluída):**
- M1.0: spike Tauri (UI falando com o mock — ver `docs/UI_PLAN.md`)
- M1.1–M1.6: conexão/boot, biblioteca, editor, fluxos de escrita (UI_PLAN)
- Gate H1 em campo: `gp100-cli --real` com o roteiro do `docs/H1_CHECKLIST.md`
  (requer a pedaleira + owner)

---

## 7. Nota legal

Projeto de **interoperabilidade**, desenvolvido por análise de artefatos obtidos
legalmente (instaladores públicos, firmware do próprio dispositivo, documentação
oficial). Não distribua firmware, instaladores ou material proprietário da Valeton
por este repositório. Use por sua conta e risco — a política de segurança de
hardware existe para que a sua pedaleira sobreviva ao desenvolvimento.
