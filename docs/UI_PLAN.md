# 🧭 UI_PLAN — Planejamento da Fase M1 (Editor UI)

> **Status:** ⏳ planejado · **Última revisão:** 2026-09-29 · **Pré-requisitos:** ✅ Fase M0 concluída (M0.7 CLI + M0.8 docs) · gate **H1** só para o modo `--real` de leitura
> **Fontes de verdade deste plano:** `docs/VISION.md` §5–§9 (arquitetura/stack/UX/features), `docs/DECISIONS.md` ADR-1..6, `docs/ROADMAP.md` (M0/H e regras R1–R4), `docs/BLOCKERS.md` itens 4/11 (editor + escrita validada), API da `Session` (ADR-6).
>
> Este documento é o **planejamento issue-a-issue da M1**. O panorama de produto
> (por que, para quem, features) é da VISION; decisões estruturais novas que
> surgirem aqui viram **ADR-7+** em `docs/DECISIONS.md` antes do código.

---

## 1. Objetivo e escopo

Substituir a janela única do Suite oficial por um editor desktop **Tauri 2 +
React/TS** que fala com a GP-100 **exclusivamente através do `gp100-core`**
(regra R1: a UI nunca fala protocolo). Escopo da M1:

| ✅ DENTRO da M1 | ❌ FORA (fase/decisão própria) |
|---|---|
| Conexão ao device (**mock** default; real só pós-H1) | Live mode, setlists, BPM (M3) |
| Boot/scan com progresso + `info` | Cloud sync, tone match IA (M3) |
| Biblioteca de presets (import `all.prst`, busca/tag) | Biblioteca versionada git-like (M3) |
| Editor de cadeia (9 slots fixos, bypass, troca de efeito) | Laboratório de IRs completo (M2; upload cru é da M1) |
| Painel de knobs com ranges/defaults do dicionário | Diff visual avançado/undo ilimitado persistente (M1.5 faz undo de sessão) |
| Set/save/upload IR **com política de hardware** (§7) | SnapTone manager (M2) |
| Undo/diff de sessão, temas dark/light, i18n (pt-BR/en/es/zh) | Empacotamento MSI/AppImage/dmg (M2) |
| | Banco SQLite da biblioteca (M2 — decisão ADR própria; M1 lê os `.prst` direto) |

## 2. Arquitetura (proposta → ADR no spike)

```
┌──────────────────────────────────────────────────────────┐
│ ui/ — React 19 + TS + Vite 8 (dark/light, i18n)          │
│   Zustand (estado de sessão) + bindings ts-rs do core    │
└──────────────▲───────────────────────────────────────────┘
               │ IPC Tauri (commands + events de progresso/push)
┌──────────────┴───────────────────────────────────────────┐
│ src-tauri/ (crate gp100-ui) — commands Rust              │
│   DeviceActor: dono ÚNICO da Session (D8), fila mpsc;    │
│   commands longos rodam spawn_blocking (ADR-3, sync);    │
│   eventos: progresso do boot, pushes (pending_pushes)    │
└──────────────▲───────────────────────────────────────────┘
               │ Session<T: DeviceTransport> (ADR-4/ADR-6)
┌──────────────┴───────────────────────────────────────────┐
│ gp100-core: golden/codec/transport(mock|real)/session    │
└──────────────────────────────────────────────────────────┘
```

Decisões estruturais (cada uma vira linha de ADR no M1.0 se confirmada no spike):
1. **DeviceActor (D8):** a `Session` tem consumidor único — um actor task
   com fila `mpsc` possui a `Session`; commands enviam requisições e recebem
   resultados por canal. Nada de `Mutex<Session>` compartilhado com a UI
   (evita interleave de transações e respeita D1/D8).
2. **Long ops com progresso:** `boot()` (2299 transações na S1) roda em
   `spawn_blocking` e emite eventos de progresso (nº de transações, pp atual)
   — a UI mostra barra, não trava.
3. **Tipos no front via `ts-rs`:** `BootReport`, `StatePage`, `UserIrTable`,
   `IrUploadReport` e o modelo de preset do core geram TS — zero drift de
   schema (já previsto na VISION §6).
4. **Pushes = eventos, não poll:** o backlog D7 é drenado pelo actor em
   intervalo curto e reemitido como evento Tauri (`device://push`).

## 3. Superfície IPC proposta

| Command | Assinatura (resumo) | Notas |
|---|---|---|
| `device_connect` | `(backend: "mock" \| "real")` | `real` bloqueado sem gate H1 (ver §5) |
| `device_info` | → pp corrente, nome, tipo, nº de presets | estado do mock/actor |
| `device_boot` | → eventos de progresso → `BootReport` | barra de progresso; cancelável no M1.6+ |
| `list_user_irs` | → `UserIrTable` | 20 slots |
| `dump_preset` | `(pp) → Vec<StatePage>` | 9 páginas via `scan_state` |
| `set_param` | `(chain_slot, code, ctrl, value)` | fire-and-forget D4; UI aplica undo local |
| `save_preset` | `(pp, pp_type, name)` | **dry-run default** (§5) |
| `upload_ir` | `(ir_slot, wav_path)` | validação WAV + blob 15B; ACK por chunk |
| `pending_pushes` | → `Vec<Vec<u8>>` | consumido pelo actor, reemitido como evento |

## 4. Telas e módulos (mapa de UI)

1. **ConnectionBar** — backend ativo (mock/real), status, pp corrente, botão boot.
2. **LibraryPanel** — presets do `all.prst` importado (+ pastas locais): busca,
   tags, favoritos; abrir = carregar no editor (mock sempre; device pós-H).
3. **ChainEditor** — cadeia horizontal 9 slots (VISÃO §8.2): bypass por clique,
   troca de efeito por slot (dicionário: 116 efeitos/9 módulos), drag para
   reordenar **com mapeamento para os slots fixos do hardware**.
4. **ParamPanel** — knobs/sliders com ranges reais do dicionário (185/639),
   incluindo knobs bidirecionais (min>max normalizado — achado M0.1) e o
   controle oculto do CAB (params_0..14 preservados).
5. **IRLab (stub na M1)** — lista dos 20 slots (nome via `list_user_irs`);
   upload de .wav cru (validação + `upload_ir`); visualização fica na M2.
6. **DiffBar / UndoStack** — diff antes/depois por parâmetro; undo/redo de
   sessão (VISION §8.2).
7. **Settings** — idioma, tema, política de escrita (dry-run flag persistida,
   VISION §7.2.1), caminho do device.

i18n: reutilizar as strings EN/CN da firmware (VISION R1.3) + pt-BR como
língua do projeto; es via comunidade.

## 5. Segurança de hardware na UI (não negociável)

Aplicação direta de VISION §7.2 + BLOCKERS §4 + ADR-5/ADR-6:
1. **Mock é o único backend até o gate H1**; `device_connect("real")` só existe
   após H1 e a **escrita** só após H2 (`WRITE_VERIFIED`) — a UI não tem bypass.
2. **Dry-run default** na 1ª execução (flag persistida): save/upload mostram o
   payload que iria ao fio e pedem confirmação explícita.
3. **Dupla confirmação** em toda escrita (diálogo com resumo: o que, para onde,
   quantos bytes) — espelha a política `--i-know-what-im-doing` do CLI.
4. Journaling local de escritas (diff + timestamp) — começar em memória na M1,
   SQLite na M2 (mesmo ADR da biblioteca).

## 6. Fases da M1 (issues — entrar no ROADMAP quando a M0 fechar)

| Issue | Entrega | DoD |
|---|---|---|
| **M1.0** Spike Tauri | workspace `src-tauri` + `ui/`; 1 command `device_info` contra o mock; CI estendida (pnpm lint/test) | `pnpm tauri dev` mostra info do mock; CI verde com os 3 gates |

**Ferramentas fixadas no M1.0:** React 19, Vite 8, Vitest 5, ESLint 10,
TypeScript 6.0 (TS 7 aguarda suporte do typescript-eslint ≥7.1), Node 22 +
pnpm 11 (lockfile congelado; upgrade deliberado).
UI segue o design system `docs/UI_DESIGN.md` (paleta palco Valeton, escala
de Fibonacci, WCAG 2.2 AA) com gate próprio no CI (job `ui`).
| **M1.1** Conexão + boot | DeviceActor + commands `device_*` + ConnectionBar + progresso | boot contra o mock com barra; pushes visíveis em log da UI |
| **M1.2** Biblioteca | import de `all.prst` (parser do core), lista/busca/favoritos | 99 presets listados com nome/tipo; abrir carrega no editor |
| **M1.3** Editor | ChainEditor + ParamPanel (knobs/dicionário) + `set_param` | knobs limitados pelos ranges; bypass/troca refletem no estado |
| **M1.4** Fluxos de escrita | save/upload-IR com dry-run + dupla confirmação | contra o mock: save persiste no estado do mock; IR aceita WAV válido e rejeita inválido |
| **M1.5** Conforto | undo/diff de sessão, temas, i18n (4 línguas) | undo cobre knobs/bypass/troca; 4 línguas trocáveis |
| **M1.6** Preparação ao H1 | modo leitura `real` (atrás de flag), log de divergência vs mock | checklist H1 executável pela UI; divergências viram issue R3 |

## 7. Estratégia de testes (mesma régua do core)

- **Rust (commands/actor):** contratos contra `MockDevice` (o mesmo da M0.5) —
  `cargo test` estendido ao crate `gp100-ui`.
- **Front (vitest):** reducers de undo/diff, normalização de ranges, i18n.
- **E2E (Playwright, webdriver do Tauri):** feliz-path boot→editar→save no mock.
- **Regra R1 na UI:** nenhum componente importa `gp100-core` direto — só via
  commands; lint proíbe o import (regra nova a registrar na M1.0).

## 8. Riscos e watchlist

| Risco | Mitigação |
|---|---|
| Boot longo trava a UI | actor + `spawn_blocking` + eventos de progresso (M1.1) |
| Drift de tipos core↔TS | `ts-rs` gerado no build; drift = erro de CI |
| Escrita acidental no real | §5 (3 camadas: backend gate, dry-run, dupla confirmação) |
| Escopo inchando (features M3 aliciando) | tabela §1 FORA é vinculante; nova feature = issue/ROADMAP |
| ts-rs/Tauri versões | pinar versões no M1.0; upgrade deliberado |

## 9. Decisões que este plano DELIBERA deixar para o M1.0 (spike → ADR)

- Gestor de estado do front (Zustand é a proposta; alternativas: Redux Toolkit, Jotai).
- Estilo de bindings (`ts-rs` vs `specta` — o Tauri moderno puxa `specta`).
- Estrutura de pastas final (`src-tauri/` + `ui/` na raiz do workspace).
- Journaling em memória (M1) → SQLite (M2) numa tacada só ou por etapas.
