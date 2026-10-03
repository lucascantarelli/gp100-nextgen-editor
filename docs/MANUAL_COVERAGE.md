# 🎛️ MANUAL_COVERAGE — cobertura do manual oficial na UI (Fase 1)

> **Status:** 🔨 vivo · **Criado:** 30/09/2026
> **Fontes:** docs/UI_REFERENCE.md §1 (app oficial + manual), `files/GP-100_Online Manual_EN_Firmware V2.0.pdf`
> (extração em `analysis/manual_streams.txt`), firmware V2.1 (drum/looper/Global EQ), all.prst (99 presets).
> **Regra do owner (30/09):** toda funcionalidade do manual deve estar implementada E testada
> antes de avançar de fase. Esta matriz é a prova viva — nada passa para a Fase 2 com 🔴/🟡.
> **Testes:** ✅ = e2e Playwright (14/14, `pnpm e2e`) · unit vitest (23/23, `pnpm test`).

## 1. Topbar e identidade (manual §"Painel de controle")

| # | Item do manual/app oficial | Estado na nossa UI | Implementação | Teste |
|---|---|---|---|---|
| T1 | Logo GP-100 + identidade | ✅ "GP-100 NextGen · editor não-oficial" (sem copiar marca) | TopBar.tsx | R1 e2e |
| T2 | LED de conexão (ON) | ✅ live-dot verde pulsando + "conectado"/"aguardando device" + Boot na navbar | TopBar | R1/R5 e2e |
| T3 | Patch navbar `Pnn Nome` | ✅ display 1-based (achado P25) + **◀ ▶ ciclo P01–P99 (30/09)** | TopBar + App.stepPreset | R1/R2 e2e |
| T4 | DRUM: Volume/Speed/BPM/compasso | ✅ 87 ritmos firmware em 5 gêneros; BPM campo numérico alinhado (30/09) | DrumPanel.tsx | drum e2e + assert de alinhamento |
| T5 | Master VOL | ✅ slider 0–99 persistente (prévia local) | TopBar + App (localStorage) | R3 e2e (sem erro) |
| T6 | Stomp Mode FS1/FS2 | 🟡 **bloqueado pós-Fase 1** — precisa captura de writes (G3–G6); UI não mente: não expõe | — | — |
| T7 | Kill switch | ✅ botão presente (prévia local; sem efeitos na Fase 1) | TopBar.tsx | R3 e2e |
| T8 | ⚙ Settings | ✅ abre modal 6 abas | SettingsModal.tsx | R4 e2e |
| T9 | ⇄ mover (extra nosso) | ✅ trava do drag com aria-pressed — no CABEÇALHO DO PALCO (remanejado da navbar) | EmptyBoard | R3 e2e |

## 2. Coluna do patch (manual §1.1 — centro)

| # | Item | Estado | Implementação | Teste |
|---|---|---|---|---|
| P1 | ◀ ▶ navegação de presets | ✅ ciclo com wrap (P01↔P99); navbar+LED+biblioteca em sincronia | App.stepPreset | R1 e2e (30/09) |
| P2 | Patch BPM [120] | 🟡 visível como nota "no device (aguarda captura)" — knob BPM do painel é o parâmetro real; escrita pós-G3–G6 | TopBar (title) | — |
| P3 | EXP Setting | 🔴 Fase 3 — expressão/footswitch precisa de captura; não há UI falsa | — | — |

## 3. Biblioteca (manual §1.1 — direita)

| # | Item | Estado | Implementação | Teste |
|---|---|---|---|---|
| B1 | Abas Factory/User | ✅ as DUAS abas funcionam (#11): Factory com os 99 reais; User salva/abre/exclui patches do dono (prévia local — a escrita no device ainda não tem canal, §5) | LibraryPanel.tsx + userPatches.ts | R2 e R2b e2e + unit |
| B2 | NO. / Patch Name / Style (99) | ✅ os 99 REAIS do all.prst (nome+tipo), P01..P99 1-based | presetData.ts (GERADO) | R2 e2e + unit (99) |
| B3 | Busca por nome/nº/estilo | ✅ case-insensitive; nº 1-based ("98" acha P98); empty state com dica | LibraryPanel.tsx | R2 e2e |
| B4 | Abrir preset | ✅ deviceSelectPreset REAL §13.10 (fallback dev determinístico); corrente destacada+aria-selected; **o pedalboard redesenha a CADEIA REAL do patch** (20 dos 99 têm a cadeia trocada) | App.openPreset + presetChains.ts (GERADO) | R2 e R2b e2e + unit |
| B5 | Teclado (listbox) | ✅ setas+Enter; foco visível; alvo ≥32px | LibraryPanel.tsx | R1/R2/R6 e2e |
| B6 | Import/Export/Rename/Save no device | 🔴 escrita no device — depende do canal USB do `save_preset` (§4 do PROTOCOL); a UI já tem o fluxo de usuário (B7) em prévia local | — | — |
| B7 | Patch de usuário (salvar/abrir/excluir) | ✅ PRÉVIA LOCAL (#11): snapshot da cadeia corrente, U## no LED/navbar, excluir devolve a fábrica; persiste em localStorage; retrato imutável | userPatches.ts + App.openUserPatch/saveUserPatch/deleteUserPatch | R2b e2e + 8 unit |

## 4. Board / cadeia (manual §"Rodapé da cadeia")

| # | Item | Estado | Implementação | Teste |
|---|---|---|---|---|
| C1 | 9 slots PRE DST AMP NR CAB EQ MOD DLY RVB | ✅ ordem do sinal; display LED (nº âmbar + nome + tipo) | EmptyBoard.tsx | R3 e2e + unit |
| C2 | Effects List (trocar o efeito do módulo) | ✅ **UI completa em prévia local** (#19): lista os algoritmos do módulo com busca, o atual marcado, e trocar reflete no palco com os knobs novos nos defaults. Falta SÓ a escrita: `change-effect` (`0x47` da família `0x4X`) sem formato validado — BLOCKERS 10b, roteiro em CAPTURE_PLAN (CAPTURA 5) | effects.ts + PedalModal.tsx | R7b e2e + unit (effects + interações) |
| C3 | Trava do drag | ✅ drag só com ⇄ ativo (protege knobs futuros) | EmptyBoard.tsx | R3 e2e |
| C4 | Cabos de sinal + LED por pedal | 🔴 Fase 2 (checklist UI_REFERENCE §3) | — | — |
| C5 | Warns ⚠ sob slots | 🟡 aguardando mapeamento confirmado do app oficial (§1.1) — não copiado às cegas | — | — |

## 5. Bateria — DRUM (firmware V2.1; UI_REFERENCE §7.1)

| # | Item | Estado | Implementação | Teste |
|---|---|---|---|---|
| D1 | Estilos por gênero | ✅ 87 REAIS: Electronic 12 · Rock 33 · Pop 14 · World 20 · Jazz 8 (rótulos de gênero inferidos — firmware só grava "Electronic") | drumData.ts (GERADO) | drum e2e + unit |
| D2 | Gênero → estilo dependente | ✅ troca de gênero reseta para o 1º estilo do grupo | DrumPanel.tsx | drum e2e (Rock 33 → World 20) |
| D3 | BPM (40–240) | ✅ campo numérico SEM spinners, mesmo tamanho dos selects (30/09: `.gp-num{box-sizing}`; era 258×46) | DrumPanel + design.css | drum e2e (assert 240×32) |
| D4 | Compassos | ✅ os 8 reais: 2/4 · 3/4 · 4/4 · 6/4 · 7/4 · 6/8 · 7/8 · 9/8 | drumData.ts | drum e2e + unit |
| D5 | Volume/Speed (0–99) | ✅ sliders (prévia local persistente `gp100.drum.v2`) | DrumPanel.tsx | unit |
| D6 | Play/stop | ✅ toggle ⏵/⏹ NA NAVBAR (sem abrir o painel) + **Espaço global** | TopBar + useGlobalShortcuts | R-ATALHOS e2e |
| D7 | "100 patterns" do marketing | 🟡 mantemos os 87 REAIS da tabela do firmware (decisão §7.1) | — | — |

## 6. Looper (specs Valeton/manual v1.8; UI_REFERENCE §7.2)

| # | Item | Estado | Implementação | Teste |
|---|---|---|---|---|
| L1 | Rec VOL · Play VOL · P-VOL (0–99) | ✅ os 3 ganhos do menu LOOPER como faders com sulco e capuz metálico (#12) | LooperPanel.tsx + looper.css | looper e2e + deck unit |
| L2 | Pre/Post — 90s PRE / 45s POST | ✅ chave de duas posições no sulco, com a ativa acesa; máximo muda com o modo | LooperPanel + LOOP_SECONDS_* | looper e2e + deck unit |
| L3 | Transporte REC/PLAY/DUB/STOP/REW | ✅ máquina de estados como pedaleira; PLAY disabled sem fita; **R global = REC**; teclas com chanfro + LED de estado (#12) | LooperPanel + useGlobalShortcuts | looper + R-ATALHOS e2e + deck unit |
| L4 | CLEAR com confirmação | ✅ 2 toques (✕ → ok?) | LooperPanel.tsx | looper e2e |
| L5 | Contador mm:ss | ✅ **7 segmentos** com o `mm:ss` exato no texto acessível (#12) | LooperPanel.tsx | looper e2e (anda + REW) + deck unit |
| L6 | Specs 24-bit/44.1kHz/SNR 110dB | ✅ nota única no mix (duplicação removida 30/09) | LooperPanel.tsx | — |
| L7 | *(visual, #12)* Rolo / cabeçote / capstan do caminho da fita | ✅ **deck em 1 SVG** com rolos, ponte, cabeçotes, capstan e fita; pacote de fita dirigido pelos segundos | LooperPanel.tsx | deck unit + visual e2e |
| L8 | *(visual, #12)* VU com face iluminada | ✅ régua VU não-linear (-30→+3 dB), zona vermelha no +1, agulha em repouso | LooperPanel.tsx | visual e2e |

## 7. Settings (manual §"menu GLOBAL" + captura §1.2)

| # | Item | Estado | Implementação | Teste |
|---|---|---|---|---|
| S1 | 6 abas General/Global EQ/About/Info Frame/Help/Release Note | ✅ | SettingsModal.tsx | R4 e2e |
| S2 | General: Input/Normal Level, USB Audio, Hint Mode, Tap Tempo, Language | ✅ persiste LOCAL (badge "prévia local"; escrita pós G3–G6). **Noise Gates/Noise Mode removidos 30/09 — não existem no device** (review Q-2); Language desabilitado até o i18n | SettingsModal + localStorage | R4 e2e (reload mantém + ausência dos fantasmas) |
| S3 | Global EQ 5 bandas FREQ/Q/GAIN + L-CUT/H-CUT | ✅ placeholders desabilitados (estrutura REAL do firmware) | SettingsModal.tsx | R4 e2e |
| S4 | Footswitch Mode | ✅ select presente, desabilitado (captura pendente) | SettingsModal.tsx | R4 e2e |
| S5 | About/Info Frame | ✅ dados reais: firmware V2.1, software 1.2.0, 185 algs/639 controles (30/09) | SettingsModal.tsx | unit a11y |
| S6 | Help — atalhos | ✅ Espaço/R/Esc documentados (tabela) + guardas | SettingsModal.tsx | R-ATALHOS e2e |
| S7 | Esc/clique-fora/✕ fecham | ✅ | SettingsModal.tsx | R4 + R-ATALHOS e2e |

## 8. Extras nossos (além do manual)

| # | Item | Estado | Teste |
|---|---|---|---|
| X1 | Atalhos globais Espaço/R/Esc com precedência e guardas de a11y | ✅ | 3 e2e + 6 unit |
| X2 | Responsividade 1440/1280/1024 (9→3×3; biblioteca empilha; looper auto-fit) | ✅ | 3 e2e (asserts de layout) |
| X3 | Boot com barra de progresso + pushes (protocolo §13.10) | ✅ | R5 e2e |
| X4 | Alvos ≥32px, foco visível, reduced-motion | ✅ | R6 e2e |
| X5 | e2e na CI contra `pnpm dev` (Chromium) — gate de release | ✅ _validate.yml job `e2e` | CI |
| X6 | Regressão estética (toHaveScreenshot, 9 baselines por viewport/plataforma) | ✅ | e2e-visual (CI) + visual.spec |
| X7 | Smoke do shell Tauri REAL no webview (tauri-driver, Linux) | ✅ | e2e-tauri (CI) |

## Placar: **33 ✅ · 5 🟡 · 5 🔴** — nenhuma 🔴/🟡 é silenciosa (ou está explícita na UI como aguardando captura, ou é Fase 2/3 documentada).

### Legenda
- ✅ implementado + testado · 🟡 parcial/aguardando captura de writes (G3–G6) ou decisão · 🔴 fase futura (2/3)
- **Próximo 🔴 da Fase 2 (U-3):** pedais 1-por-vez (COMP primeiro) — desbloqueia C2/C3/C4 e os knobs do dicionário.
- Capturas de writes (G3–G6) desbloqueiam: T6 (Stomp), P2/P3 (Patch BPM/EXP), B6 (Save/Import/Export), S2/S3/S4 (escritas globais).
