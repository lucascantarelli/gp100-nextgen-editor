# 🧪 UI_TEST_PLAN — Roteiros de teste manual da UI (por fase)

> **Status:** 🔨 vivo · **Criado:** 30/09/2026 · **Como usar:** executar no browser
> (`pnpm dev` em `packages/app/ui`, porta 5173) OU no shell Tauri. Marcar ✅/❌ por
> item e devolver os achados para `docs/UI_REFERENCE.md` §3. Cada fase só passa
> para a próxima com a aprovação do owner (decisão 30/09).

---

## FASE 1 — Casca (U-1/U-2): navegação, biblioteca, board vazio, settings

### R1. Topbar e identidade
1. Logo GP-100 + tag do backend (mock) visíveis; LED "conectado" verde (fallback dev).
2. Navbar mostra o patch corrente no formato `P25 Mist` (nome REAL de fábrica).
3. Resize da janela: topbar não quebra (wrap limpo), nada sobrepõe.
4. Tab atravessa TODOS os controles com foco visível (âmbar); ordem lógica.

### R2. Biblioteca de fábrica (99 presets reais do all.prst)
1. Painel direito lista 99 linhas `P00…P98` com nome e estilo (Rock/Funk/…).
2. Buscar "mist" → só `P25 Mist`; buscar "Rock" → filtra estilos; buscar "98" → `P98`.
   Busca sem resultado mostra dica (empty state com hint, não tela vazia).
3. Clicar em qualquer linha: display LED do board e a navbar trocam para o preset
   (fallback dev: determinístico; no shell Tauri: §13.10 REAL).
4. Linha corrente fica destacada (âmbar) e com aria-selected.
5. Teclado: setas + Enter selecionam um preset (listbox nativo de botões).
6. Tab "User Patch": aparece bloqueada com explicação (save é da fase de escrita).

### R3. Board vazio (9 lugares)
1. Palco mostra 9 slots nomeados PRE DST AMP NR CAB EQ MOD DLY RVB na ordem do sinal.
2. NENHUM pedal desenhado; cada slot diz "pedal na fase 2".
3. Display LED exibe nº do preset (2 dígitos âmbar) + nome + tipo.
4. `⇄ mover` OFF: slots não são arrastáveis (cursor normal).
5. `⇄ mover` ON: botão fica âmbar (aria-pressed=true), slots com cursor grab e
   feedback de drop (borda tracejada âmbar ao arrastar sobre um slot).
6. Kill switch e DRUM/Master VOL: cliques não soltam erro (prévia local).

### R4. Modal Settings (⚙)
1. ⚙ abre o modal com 6 abas: General | Global EQ | About | Info Frame | Help |
   Release Note; badge "prévia local" visível.
2. General: sliders (Input/Normal Level), toggles (Noise Gate 1/2, USB Audio),
   selects (Hint/Noise Mode), checkboxes Tap Tempo (PRE/MOD/DLY) e idioma — todos
   operam e PERSISTEM: recarregar a página mantém os valores (localStorage).
3. Global EQ: seletor 1/5…5/5 e controles desabilitados (aguardando captura G3–G6).
4. Esc fecha; clique no fundo fecha; ✕ fecha; foco não "escapa" do modal.
5. `prefers-reduced-motion` respeitado (sem animações).

### R5. Boot e pushes
1. Botão de boot na NAVBAR roda o script com barra de progresso (faixa fina
   abaixo do banner, só durante o boot — não há seção de conexão na página).
2. No fim, o display LED dá o flip (classe `led-flip`) uma única vez.
3. Drawer "pushes do device" abre e mostra o log (ou vazio) sem erro.

### R6. Acessibilidade (gate manual do UI_DESIGN §7)
- [ ] Alvos clicáveis ≥32×32 em toda a casca.
- [ ] Nenhum estado SÓ pela cor (toggles têm posição + aria-checked; abas têm
      aria-selected; trava tem aria-pressed).
- [ ] Zoom 200% mantém uso; janela 800×600 sobrevive (scroll horizontal ok).

**Critério de aprovação da Fase 1:** R1–R6 todos ✅ pelo owner no browser.

---

## 📋 EXECUÇÃO — Fase 1 + Drum/Looper (30/09, Playwright · Chromium 1440×900)

> Executada como **testes e2e automatizados** (`pnpm e2e` — Playwright, 8 testes
> em `e2e/shell.roteiros.spec.ts`, cada um cita o ID do roteiro). O painel do
> browser do agente estava instável; a suíte roda o MESMO app no Chromium real
> (headless), contra o `pnpm dev` na :5173.

| Roteiro | Resultado | Observações |
|---|---|---|
| R1 Topbar/identidade | ✅ | logo, conexão, patch `P01 It's GP100`; foco de teclado visível |
| R2 Biblioteca (99) | ✅ | 99 reais do all.prst; busca nome/nº/estilo; empty com dica; abrir→seleção+navbar; User Patch bloqueada |
| R3 Board vazio + trava | ✅ | 9 slots PRE→RVB; LED 2 dígitos; `⇄ mover` liga `draggable`; kill sem erro |
| R4 Settings | ✅ | 6 abas + badge; General PERSISTE no reload (localStorage); Global EQ 5 bandas desabilitadas; Footswitch Mode presente (desabilitado); Esc/clique-fora fecham |
| R5 Boot/pushes | ✅ | boot sem alerta; status conectado; drawer abre |
| R6 Acessibilidade | ✅ (após fix) | alvos ≥32px (achado: botão "Limpar" do PushLog com 28px → corrigido p/ 32); reduced-motion = 0.01ms; 800×600 sobrevive |
| Drum (gestão de ritmos) | ✅ | 87 estilos/5 gêneros; gênero→estilo dependente (Rock 33, World 20); compassos reais 2/4…9/8; chip reflete Samba·140 BPM |
| Looper (máquina de fita) | ✅ | PLAY desabilitado com fita vazia; REC→status+timer; REC de novo = PLAY; REW zera; STOP; PRE·90s/POST·45s; CLEAR com confirmação; rack Rec/Play/P-VOL |

**Achados corrigidos nesta rodada:**
1. PushLog "Limpar" com 28px (< mínimo 32×32 do UI_DESIGN §2) → `minHeight: 32`.
2. `scripts.e2e = "playwright test"` adicionado ao package.json (Playwright 1.63,
   Chromium local; config `reuseExistingServer` para conviver com o `pnpm dev`).

**Limitações conhecidas (anotar no teste manual do owner):**
- Drag-and-drop real (HTML5 DnD) validado por handlers/atributos; a REORDENAÇÃO
  visual com feedback de drop fica para a Fase 2 (o callback é no-op sem pedais).
- Zoom 200% e light theme: roteiro manual do owner (headless não troca OS theme).

---

## 📐 EXECUÇÃO — R-RESPONSIVO (30/09, Playwright · Chrome do sistema 1440/1280/1024)

> Pedido do owner: board + looper em telas menores (1280/1024) sem quebrar o
> alinhamento dos painéis. Verificação OBJETIVA via `analysis/responsivo_checks.js`
> no Chrome real (channel "chrome" — preview webview instável; mesmo método da
> rodada e2e). 9 asserts por viewport, **27/27 PASSARAM**; screenshots regenerados.

| Viewport | Overflow X | Board ↔ Biblioteca | Board (slots) | Looper (tracks) | Rack ≥ 240px |
|---|---|---|---|---|---|
| 1440×900 | 0 | lado a lado (gap 12; biblioteca 320px fixa) | 9×1, larguras idênticas, pitch uniforme | 4 tracks: 439.3 · 439.3 · 439.3 · **0** | 439.3px ✅ |
| 1280×800 | 0 | lado a lado | 3×3 (≤1340) | 3 tracks: 386 ×3 | 386px ✅ |
| 1024×768 | 0 | EMPILHA (≤1100): biblioteca abaixo do board, mesma largura 984px | 3×3 | 3 tracks: 300.7 ×3 | 300.7px ✅ |

**Achados desta rodada:**
1. Track "0px" no looper @1440 é ESPERADO: `repeat(auto-fit, minmax(300px,1fr))`
   colapsa a track vazia — sem buraco visual. Mantido (protege larguras
   intermediárias; alternativa seria fixar 3 colunas).
2. Nenhum texto clipado nos 3 painéis (board/biblioteca/looper) em nenhuma largura.
3. Blocos deck/VU/rack do looper sem sobreposição (área de interseção = 0); VU
   meters contidos no painel.
4. Recortes por painel gerados p/ inspeção visual do owner:
   `files/images/responsivo-{board,looper,lib}-{1440,1280,1024}.png`
   (+ fullPage `responsivo-{1440,1280,1024}.png`).
5. Ferramenta viva: os asserts desta rodada foram PROMOVIDOS a e2e permanente
   (`e2e/responsivo.spec.ts` + `e2e/_helpers.ts`, 30/09) — rodam na CI a cada
   mudança de `packages/app/ui/`. Screenshots/recortes: `pnpm dev` +
   `node analysis/responsivo_shots.js`.

---

## \U0001f9f0 EXECUÇÃO — #11 LIBRARYPANEL: a biblioteca comanda o pedalboard (02/10)

> Escopo fechado com o owner: (a) estreitar a biblioteca; (b) fazer o patch
> selecionado REFLETIR nos 9 pedais; (c) implementar a UI de User Patch (até
> então decorativa, bloqueada). Gates: tsc · eslint · vitest · coverage · build ·
> playwright. Resultado: **144 unit ✅ (12 arquivos) · 77 e2e ✅ · cobertura
> 89,9% stmts / 86,6% branch** (gate 85) · 18 baselines win32 regeneradas.

**Achado de fundo (o bug real):** o core Rust já montava a cadeia por preset;
o que estava congelado era o **mock web**, com uma cadeia FIXA para os 99. Dois
consequências que os testes revelaram ao deixar de mentir:
1. **20 dos 99 presets têm a cadeia TROCADA** (`@x` manda, não a família) — P06
   (ppID 5) tem DST antes do PRE. O `Stage` ordenava por família e mentia.
2. **O PRE do P01 no all.prst é C-Wah, não COMP.** Vários asserts (unit + R7)
   foram escritos contra o COMP que o mock inventava; foram refeitos para ler o
   artefato GERADO (`PRESET_CHAINS`) em vez de repetir a mentira.

| O que | Onde | Teste |
|---|---|---|
| Cadeia real dos 99 no mock | `presetChains.ts` (GERADO) + `localMockBoard` | unit `ipc.device` (12) |
| Ordem do palco pelo `slot` real | `Stage::baseOrder` | unit `stage.foundation` (25) |
| Patch de usuário (snapshot/U##/excluir) | `userPatches.ts` + `App` | unit `userPatches` (8) + 3 unit em `app.interactions` |
| Biblioteca estreita (300 → 248px) | `.shell-main` | e2e `responsivo` (assert 248.5) + baselines |
| Roteiro R2b (novo) | `shell.roteiros.spec.ts` | e2e R2b |

**Achados desta rodada:**
- `deleteUserPatch` tinha que devolver o palco à fábrica — deixar o patch apagado
  na tela seria a UI mentindo sobre o que existe.
- A aba da biblioteca passou a seguir o BANCO ABERTO (abrir um patch troca a aba;
  clicar na aba só navega a visão) — senão a biblioteca mente sobre o que toca.
- A lista de patches de usuário é um `list`/`listitem`, não `listbox`: com o
  botão de excluir dentro, o `option` órfão ficaria com filhos presentacionais
  para o leitor de tela.
- Flake: `erro-boot` (visual) deu timeout na suíte completa (8 min) e passou
  isolado — contenção de recurso, não regressão.

### R2b. Patch de usuário: salvar a cadeia corrente, abrir de volta e excluir
1. Abrir a aba User → estado vazio orienta o caminho + nota de prévia local.
2. Nomear e salvar → a lista mostra `U01` + o nome + de qual patch veio.
3. Abrir o patch → `U01` no LED e na navbar; os 9 pedais mostram o SNAPSHOT.
4. Excluir → volta ao preset de fábrica confirmado pelo device.

## \U0001f3ae EXECUÇÃO — #19 EFFECTS LIST: trocar o efeito dentro do pedal (02/10)

> Escopo definido com o owner: UI inteira da lista de efeitos por pedal, em
> prévia local, morando como painel lateral do `PedalModal` (o palco fica
> limpo). Só a escrita no device espera a captura do `0x47`
> (`docs/CAPTURE_PLAN.md`, CAPTURA 5).
>
> **Resultado: 155 unit ✅ (13 arquivos) · 78 e2e ✅ · cobertura 90,3% stmts /
> 86,5% branch** (gate 85). `effects.ts` e `PedalModal.tsx` em 100% de linhas.

**Regras que os testes travam (as que dão sentido ao editor):**
- A lista é do **MÓDULO**: PRE não oferece `Bog RedM` (que é AMP).
- As entradas de **nome vazio** do dicionário (variante `fx`, índice 1.048.576+,
  slots internos do firmware) nunca aparecem — a lista não mostra linha em branco.
- Trocar **preserva** posição, família e ligado/desligado.
- Trocar **zera os controles para o default do algoritmo novo** — herdar o
  `Range` do C-Wah como `Sustain` do COMP seria a UI mentindo sobre o som.
- O efeito trocado entra no **patch de usuário**: o patch é o retrato do palco,
  não do preset de fábrica.

| O que | Onde | Teste |
|---|---|---|
| Lista do módulo + filtro + `effectCode` | `effects.ts` | `tests/effects.test.ts` (9) |
| Troca refletindo no palco (estado único) | `App.onChangeEffect` | `app.interactions` (2) |
| Efeito trocado entra no patch de usuário | `userPatches` | `app.interactions` (1) |
| Roteiro R7b (novo) | `shell.roteiros.spec.ts` | e2e R7b |

**Achado desta rodada:** o nome acessível da opção é o NOME do efeito (o botão
tem texto); o `title` carrega a ação e só vira nome acessível quando não há
conteúdo. A e2e tentou casar pelo `title` e o Playwright não resolveu — o
seletor correto é `getByRole("option", { name: "COMP", exact: true })`.

## ⌨️ EXECUÇÃO — R-ATALHOS (30/09, Playwright · Chromium + vitest)

> Pedido do owner: atalhos globais — Espaço = drum play/stop, R = REC do looper,
> Esc fecha painéis — com documentação na aba Help do Settings. Suíte total após
> a rodada: **23 unit ✅ · 11 e2e ✅** (8 roteiros anteriores + 3 de atalhos).

| Teste (e2e/atalhos.spec.ts) | Resultado | Observações |
|---|---|---|
| Espaço liga/para o drum | ✅ | chip ⏵ → ⏹ → ⏵ (mesmo estado do botão do painel) |
| R = REC no looper | ✅ | REC→PLAY com o MESMO comportamento do botão ●; digitar "rock" na busca NÃO dispara (guarda de campo de texto) |
| Esc com precedência | ✅ | fecha Settings → Drum → pushes, do topo para a base; com o modal aberto, Espaço/R ficam inertes |

**Achado corrigido nesta rodada:**
1. O DrumPanel tinha listener PRÓPRIO de Esc — com o Esc global, uma tecla fechava
   Settings E drum juntos (precedência quebrada; detectado pelo teste unitário).
   Corrigido: `drumOpen` subiu do TopBar para o App e o Esc é 100% global (o
   popover virou controlado puro).
2. Guardas de a11y para não roubar teclado: Espaço sobre botão focado ativa o
   BOTÃO (não o drum); Ctrl/Alt/⌘ e auto-repeat ignorados; Help documenta tudo.

**Cobertura unit (tests/shortcuts.test.tsx):** helpers puros (`isTextEntryTarget`,
`isSpaceNativeTarget`, `shouldHandleShortcut`) + integração no App (toggle do drum,
REC→PLAY, digitação não dispara, precedência do Esc).

---

## 🧪 EXECUÇÃO — U-5: E2E NA CI + COBERTURA DO MANUAL (30/09, Playwright · Chromium)

> Pedido do owner: "tudo deve ser testado — atalhos, funcionalidades, settings,
> navegação, botões" — e responsividade + alinhamento sem depender de observação
> manual. Suíte total após a rodada: **14 e2e ✅ · 23 unit ✅**, com job `e2e`
> no pipeline (contra o `pnpm dev`) como GATE do release.

| Espec | Testes | O que trava |
|---|---|---|
| shell.roteiros.spec.ts | 8 ✅ | R1–R6 + drum + looper (casca completa; R1 agora cobre ◀ ▶ em ciclo P99↔P01) |
| atalhos.spec.ts | 3 ✅ | Espaço/R/Esc + precedência + **aba Help documenta os atalhos** |
| responsivo.spec.ts | 3 ✅ | alinhamento @1440/1280/1024: 0 overflow, 9→3×3, biblioteca lado a lado/empilhada, looper sem sobreposição, sem texto clipado |

**Achados corrigidos nesta rodada:**
1. **BPM do drum desalinhado** (owner): media 258×46 vs 240×32 dos selects — o
   user-agent só aplica `border-box` a `<select>`, não a `<input>`. Fix:
   `.gp-num { box-sizing: border-box }`; assert e2e permanente no teste do drum.
2. **Redundância do looper:** estado da fita aparecia 3× ("vazia", "fita: vazia",
   "máx 45s") — agora 1 linha no deck (`fita: 90s (PRE)`), com o tempo NO estado.
3. **◀ ▶ do manual §1.1 implementados** (ciclo P01–P99; navbar + LED + biblioteca
   em sincronia) — estava no plano da Fase 1 e faltava.
4. Strict mode: novo `role="status"` da fita colidiu com o do modo — asserts do
   looper escopados por `filter({ hasText: "●" })`.
5. Matriz de cobertura do manual: `docs/MANUAL_COVERAGE.md` (33 ✅ · 5 🟡 · 5 🔴 —
   nada silencioso; 🔴 = Fase 2/3 documentadas).

**CI (`_validate.yml`):** job `e2e` = setup pnpm → `playwright install --with-deps
chromium` → `playwright test` (webServer sobe o `pnpm dev` na :5173);
o publish (`release.yml`) só roda depois do `validate` completo — tag não
corta com casca quebrada.

---

## 🔍 EXECUÇÃO — Q-REVIEW (30/09, code review do owner: campos falsos + nome congelado + limpeza)

> Owner achou na UI: Noise Gates que "não existem de verdade", nome do preset
> não atualizando no visor/navbar, e perguntou por que os testes não pegaram.
> Resultado: Q-1…Q-9 no ROADMAP (FASE Q); corrigidos Q-2/Q-3/Q-4 nesta rodada.

| Achado | Correção | Teste que trava agora |
|---|---|---|
| Noise Gate (1)/(2) e Noise Mode NÃO existem no device | removidos do Settings | unit: `not.toContain("Noise Gate")` |
| APP Language ativo mas sem trocar idioma (mesmo engano) | desabilitado + "próxima versão" | R4 e2e (desabilitado) |
| Nome do preset congelado (fallback do board ignorava o pp) | fallback usa presetData; pp/ppType reais | e2e: NOME completo nos 3 locais (R1/R2) |
| Efeito colateral dentro do updater (StrictMode 2×) | updater puro + useEffect([pp]) | unit R1 (◀ ▶ com nome) |
| Vocabulário interno visível (fases/fios/capturas/§) | centralizado em `src/i18n/messages.ts` + limpeza | padrão §8 do UI_REFERENCE |

**Por que os testes não pegaram (lição registrada):** os asserts verificavam
NÚMERO (`/^P25 /`) e presença do nome INICIAL ("It's GP100") — nunca o nome do
preset SELECIONADO. Regra nova: todo elemento que exibe dado dinâmico tem
assert do valor TROCADO (não só do inicial). Cenários restantes ficaram como
issues Q-6/Q-7 (sliders/kill/error states).

---

## 🖼️ EXECUÇÃO — U-6/U-7: REGRESSÃO VISUAL + SMOKE DO SHELL TAURI (30/09)

**U-6 — visual.spec.ts (toHaveScreenshot):** 15 baselines (board/looper/biblioteca
+ erro-boot/erro-preset × 1440/1280/1024), animações congeladas, tolerância 1%,
**baseline por
plataforma** (`{arg}-{platform}`; fontes divergem Win/Linux). Estado atual:
baselines `win32` **e `linux`** commitadas e comparando no CI ✅ (issue #45);
baselines novas nascem pelo **input `update-snapshots`** do pipeline (job
`e2e-visual` sobe o artefato → commitar). Sem baseline no CI = job VERMELHO (o
skip silencioso não existe mais).

**U-7 — e2e-tauri (job na CI, Ubuntu):** build debug do `gp100-ui` com o
dist embutido (mock, sem hardware) → `tauri-driver` + `WebKitWebDriver` sob
`xvfb-run` → Selenium (`e2e/tauri.smoke.mjs`): banner no webview, os 3 painéis
no DOM e a biblioteca com os **99 presets reais dentro do webview**. Cobre
também o **DeviceGone ponta-a-ponta** (#48): o app sobe com
`GP100_DEBUG_FAULT=die-after:60`, o device morre no meio do boot e o smoke
clicla o ⟳ exigindo, após a 2ª falha: alerta amigável (sem vazar o detalhe
técnico), LED off, zero barra de progresso e retry vivo. Receita
oficial Tauri v2 (webkit2gtk-driver + xvfb + WEBKIT_DISABLE_DMABUF_RENDERER).

| Item | Estado |
|---|---|
| 9 baselines win32 geradas e estáveis | ✅ (rodada 30/09) |
| job e2e-visual (Ubuntu) + input update-snapshots | ✅ _validate.yml |
| baselines linux commitadas | ✅ 48 PNGs (issue #45; 1º run do input) |
| job e2e-tauri (smoke webview real + DeviceGone) | ✅ _validate.yml (gate de release via validate) |

---

## FASE 2 — Pedais (U-3; roteiro base, repetir POR EFEITO)

> Um efeito por vez, começando pelo COMP. A cada rodada: executar R7 com o efeito
> em TODAS as 9 posições antes de aprovar e partir para o próximo.
>
> **Entregue (fatia 1 da issue #19):** o palco renderiza o board REAL
> (`device_board`): COMP (PRE) no slot da família com os knobs do dicionário
> editáveis (arrasto/teclado/valor por textbox; knob numérico →
> `device_set_param` §13.11), LED verde/vermelho e footswitch por prévia LOCAL,
> trava ⇄ mover com reordenação local (arrastar o pedal para qualquer posição)
> e os 8 lugares restantes como placeholders.
>
> **Entregue (fatia 2):** liga/desliga do **modo engenheiro** na aba General do
> Settings (persistido local) — o tooltip do knob passa a mostrar
> `addr/code/ctrl`, sem a referência interna `§13.11` (passo 5 do R7).
>
> **Entregue (fatia 3):** **modal de edição** — clique/Enter no corpo do pedal
> (com a trava ⇄ em repouso) amplia 1.25× (knob 80px); Esc/✕/clique fora fecham
> e o valor ajustado no modal aparece no palco na hora (estado único).
>
> **Entregue (fatia 4):** **pedal compacto no palco** — enclosure no espaçamento
> do board (118–132px, knob 32px) com o VALOR de cada knob em texto e **knobs
> travados** (`role="img"`: sem foco/handlers); QUALQUER clique/Enter no pedal
> abre o modal (o footswitch conserva o clique próprio), a edição inteira mora
> lá e volta para o palco na hora. **DST entrou** como 2º efeito real (2 pedais +
> 7 placeholders). Falta a **validação manual do owner** (R7 em todas as 9
> posições) antes de partir para a próxima família.

### R7. Um pedal (ex.: COMP) em cada posição do board
1. O efeito aparece no slot da vez; os demais continuam placeholders.
2. Modelo/cores coerentes com a família (PRE = cinza-azulado; DST vermelho; …).
3. TODOS os knobs/switches/comboxes do algoritmo aparecem (contar vs fxData).
4. **No palco** o pedal é compacto (118–132px) e caberia 9 numa linha larga:
   nome + VALOR de cada knob em texto, nada sobrepõe em NENHUMA posição, e o
   pedal não é reescalado/estoura a coluna.
5. Knob do palco é SÓ LEITURA (não arrasta, não foca, não muda valor); o tooltip
   continua útil (com o modo engenheiro mostra `addr/code/ctrl`).
6. **Toda a edição acontece no modal**: clicar no pedal abre; arrastar o knob
   (dy/260), setas ±1% (Shift 5%), duplo-clique = default.
7. Valor EDITÁVEL (textbox do modal): digitar → Enter aplica (`device_set_param`
   no real); inválido é ignorado; Esc restaura; blur aplica; o palco reflete.
8. LED do pedal: verde quando ON, vermelho quando OFF; footswitch alterna.
9. Modal de edição: pedal 1.25× (knob 80px) SEM barra de rolagem em janela
   ≥640px, nada cortado/sobreposto, fecha com Esc/✕/clique fora.
10. `⇄ mover` OFF: tentar arrastar o pedal NÃO move (protege o knob).
11. `⇄ mover` ON: drag reordena (prévia local) e cabos seguem os jacks.
12. Cabo de sinal: entra no jack IN do próximo; pulso anima só entre pedais ON.

### R8. Cadeia completa (fim da Fase 2)
1. 9 pedais preenchem o board; 3 linhas 3+3+3 sem sobreposição.
2. Preset de fábrica carrega a cadeia real (nomes/efeitos do preset).
3. Tamanho/espaçamento revisados em cada uma das 9 posições.
4. Roteiro R6 (a11y) reexecutado com os pedais no ar.

### R9. Afinador do palco (issue #8 — refinamento profissional)
> Executado no e2e (`e2e/interacoes.spec.ts`, "Afinador: painel fixo, monitor
> visual, demo move agulha e nota, ref pitch"); o que sobra é julgamento visual.
1. O painel está sempre visível no cabeçalho, do tamanho do display do patch
   (nada de colapsar/saltar de altura ao ligar o monitor).
2. **Grade fixa**: monitor, modo, REF PITCH e demo na MESMA linha, com o
   monitor desligado também — zero buraco no meio, zero quebra de linha.
3. **Botão do monitor é visual**: ícone ♪ + LED VERDE ligado / VERMELHO
   desligado, sem palavra de estado (tooltip explica e diz o estado atual).
4. Ordem de fluxo: on/off junto do display (LED/escala) e a demo na ponta.
5. LED próprio do painel: cinza desligado, âmbar ouvindo, verde/vermelho pela
   banda — o LED do patch (pp/nome) nunca é tomado.
6. Monitor DESLIGADO = repouso honesto: nota “—”, agulha centrada, nenhuma demo
   rodando; ligar mostra a leitura e a cor da banda.
7. “▶ demo” com o monitor desligado liga o monitor junto (vira “■ demo”);
   desligar o monitor para a demo na hora.
8. REF PITCH 435–445 Hz persiste (`gp100.tuner.v1`), junto do monitor e do modo;
   modo em ciclo bypass→thru→mute.
9. Nada disso vai ao device (o tuner do GP-100 é gesto de hardware) — sem
   promessa de comando inexistente na UI.

---

## 🧪 EXECUÇÃO — Q-1/Q-6/Q-7: CATÁLOGO ÚNICO + INTERAÇÃO + ERROS (30/09, Vitest + Playwright · Chromium)

> Fechamento das três issues remanescentes da FASE Q. Suíte total: **32 e2e ✅
> (23→32) · 27 unit ✅ (23→27)**; gates `tsc -b` / `lint` / `build` 100% ✅.

**Q-1 — catálogo de mensagens + lint (`local/no-user-literals`):**
| O que | Resultado |
|---|---|
| Varredura exaustiva | 100% dos componentes migram p/ `MSG`; resíduos da 1ª passada migram também (aria do pedal e do rolo do looper, `title="capstan"`, placa "GP-100" do AmpHead, `0x` do pp na ConnectionBar, ternários "ok?"/"✕" e "Ligar/Desligar efeito") |
| Lint (eslint.config.js) | regra `error` em `src/components/**` + `App.tsx`: JSXText com letras, `title`/`placeholder`/`aria-label`/`label`/`alt` com literal, template sem interpolação e ternário com ramo literal (gap da 1ª rodada) |
| Estado final | `pnpm lint` = 0 violações; texto de usuário novo fora do catálogo quebra o build (padrão §8 do UI_REFERENCE) |

**Q-6 — interação (`e2e/interacoes.spec.ts`, 5 testes):**
- master VOL: display numérico acompanha o slider + reload restaura;
- drum volume/speed, looper Rec/Play/P-VOL + rota PRE/POST: reflexo na UI,
  JSON do localStorage conferido e reload restaura (persistência REAL, não só
  estado de sessão);
- kill switch: alcançável por teclado (focus + Enter), casca segue de pé;
- Settings campo a campo: input/normal level (display), USB Audio (switch),
  Hint Mode (select), Tap Tempo (3 checkboxes) — valor TROCADO assertado
  (regra Q-3) + persistência integral.

**Q-7 — error states (`e2e/estados.spec.ts`, 4 testes + `tests/device.fail.test.ts`):**
- ganho de teste `localStorage["gp100.debug.failDevice"]` (="info"|"boot"|
  "board"|"all") no fallback de `src/ipc/device.ts`, setado antes do load
  (`addInitScript`); fora do fallback (webview real) não tem efeito;
- contrato travado: info falha → "Device desconectado" e casca de pé; boot
  falha → `role="alert"` com MSG.connBootError (sem stack técnica); board
  falha → banner MSG.errOpenPreset; "all" → nenhum crash. Detalhe técnico
  vai só para o console (§8).
- Regressão visual dos ERROS (seguimento da rodada): 6 baselines novas em
  visual.spec.ts (`erro-boot` = alerta do boot; `erro-preset` = banner do
  App) × 3 viewports, usando o gancho failDevice — capturadas em win32 e
  comparadas limpas na 2ª execução (38 e2e ✅ no total; CI compara com as
  `linux` via update-snapshots).

---

## 🧹 EXECUÇÃO — CONSOLIDAÇÃO DA NAVBAR (30/09, Playwright · Chromium)

> A seção "Conexão" da página foi REMOVIDA (decisão do owner): status,
> badge de backend e boot já vivem na navbar — nada duplicado.
>
> - `ConnectionBar.tsx` removida; Boot é botão da navbar (desabilita com
>   `aria-busy` durante o boot); o progresso é uma faixa fina no banner e
>   o erro um `role="alert"` — só existem DURANTE o boot/falha;
> - `bootPp` saiu do useBoot (consumidor era a barra removida);
> - e2e: `erro-boot` do visual.spec aponta o alerta do banner; R5 e os
>   estados de erro clicam o Boot dentro do banner (`getByRole("banner")`).
>
> **Remanejamento harmônico (mesma rodada):** a div solta de ações da
> navbar foi desfeita — **⇄ mover** foi para o cabeçalho do PALCO
> (EmptyBoard: controla o arrastar dos slots), **⭘ kill** para o cluster
> do master (mute junto do volume) e **⚙** para o cluster do master
> (global/sempre clicável — no rodapé o drawer do drum o cobriria).
> Página em VIEWPORT ÚNICA (`main` navbar → meio com scroll próprio →
> rodapé): a lista de 99 presets ganhou altura delimitada (max 38vh)
> com rolagem interna e o rodapé virou o chassi da pedaleira
> (IN · GP · OUT). 48 baselines regeneradas; 71/71 e2e ✅.
>
> **Navbar em 1 linha (garantia do owner):** logo compacta "GP-100" em
> UMA linha (tagline migrada para o rodapé), status de conexão "on/off",
> info do drum EMPILHADA (BPM sobre compasso), Boot automático no mount
> (o botão da navbar vira re-escanear manual; flip do LED só no manual —
> auto-boot é silencioso), legendas "patch"/"master" e badge Mock
> escondidos em viewports menores. **Guard permanente no responsivo**:
> altura do banner < 70px em ≥1024px (2 fileiras = falha). Boot simulado
> emite beats em lotes (sem congelar a main thread).
>
> **Gate de coverage 85% (01/10, Vitest + @vitest/coverage-v8):** unit
> 27 → 77 (tsc/lint ✅); thresholds statements/functions/lines = 85 no
> `vite.config.ts` (branches 81.6% medido, sem gate — ganho por
> incremento). A CI trava pelo build-front (`pnpm test:coverage`),
> válido para front, ui-rust, e2e-tauri e releases. Furos fechados:
> ipc/device completo (boot em lotes com cap travado em unit,
> board/library/comandos, failDevice por operação, **branch de webview
> Tauri** simulada com mock parcial de @tauri-apps), useBoot (auto 1×
> sob StrictMode, erro→retry, throttle rAF), fundação do palco (fxModels,
> Pedalboard, Pedal, Knob — teclado/arrasto/ciclo) e interações do App
> (◀/▶ ciclo P01↔P99, master, faixas de boot/erro com recuperação pelo
> ⟳, erro do openPreset, FSM completa do looper, busca da biblioteca,
> 6 abas do Settings, persistência do General, push log com cap de 100,
> drum on/off/BPM/compasso, VU modo + drag do EmptyBoard). Edge cases
> de IPC nível 2 (failDevice em select/set_param, disconnect mid-boot,
> retry/backoff) ficam para a integração real — mapa completo no
> ROADMAP V-7.
>
> **POM nos e2e (01/10, refactor V-7):** seletores encapsulados em
> `e2e/pages/_pages.ts` (Shell/Brand/Library/Board/Looper/Drum/Settings/VU);
> os 5 specs reescritos **1:1** (mesmos asserts, zero mudança de
> comportamento) e `_helpers.ts` segue como módulo de MEDIDAS geométricas.
> Bônus: restaurado o `toHaveScreenshot` dos loops de PAINÉIS e TEMA CLARO
> (perdido em edição anterior — só topbar/erros comparavam); o guard
> retomado pegou drift real de 2px no palco (VU no cabeçalho) e as
> baselines de painéis foram regeneradas. 72/72 e2e ✅ com comparação
> limpa na 2ª execução.

### Rodada #12 — Looper deck + navbar em rack (03/10)

**Efeito:** painel do looper redesenhado (issue #12) + navbar com largura de
módulo igual.

**Achados:**
- **Cronômetro ao contrário (achado pelo owner em tela).** O `mm:ss` tem 5
  caracteres e só 4 dígitos; indexando a posição do dígito pelo índice do
  CARACTERE, a vírgula ocupava um slot da lista e o último dígito caía fora
  (`undefined` → `x=0`), se sobrepondo ao primeiro. Só era visível quando os
  segundos saíam do zero: "00:05" desenhava "50:00". O teste anterior contava
  28 polígonos e passava — contava a pele, não a posição. Agora há assert de
  4 posições distintas e crescentes.
- **Fita sumindo atrás do metal.** A fita era pintada antes dos cabeçotes: nos
  40px onde ela cruza o corpo do cabeçote, o traço âmbar desaparecia e o deck
  parecia ter um mecanismo solto. A fita vai **por cima** — é assim que ela
  passa de verdade, contra a face polida do cabeçote.
- **Alvos abaixo do piso.** A chave de rota (26px) e os faders (26px)
  reprovaram o gate R6 (≥32px). Ambos foram a 32px.
- **Chip do drum virando "R…" em 1024.** Com a navbar em colunas iguais o
  stepper de BPM passou a comer o módulo do drum. `.nb-step`/`.nb-bpm` foram
  para o CSS para o media query poder encolher o bloco.
- **Banner quebrando em 2 fileiras.** `flex: 1 1 auto` no rack faz a base do
  flex virar o max-content (~1360px) e o header quebra a linha. `flex: 1 1 0`
  resolve — banner volta a 54px.
- **Tema claro incoerente.** Os tokens de superfície invertem e o looper virava
  painel branco com deck preto. `.lp` remapeia os tokens para os valores
  escuros: a máquina é escura nos dois temas.
- **Espaço vertical.** A 1ª versão do deck ficou 489px (quase o dobro do
  looper antigo). ViewBox reencolhido para 560×170 (3,3:1) → 423px.

**Posições testadas:** 1–9 (9 slots do palco) + 7 controles do looper
(REW/STOP/PLAY/REC/CLEAR, PRE/POST, 3 faders) + 4 módulos da navbar.

**Status:** ✅ 141 unit · 53 e2e funcionais (R1–R6 + drum + looper + 3
responsivos + atalhos) · cobertura 90,1%/85,9% · baselines visuais
regeneradas pela CI.

## Registro de rodada (colar no fim de docs/UI_REFERENCE.md §3)

```
Rodada: <data> · Efeito: <nome> · Posições testadas: 1..9
Achados: <bullet por achado> · Status: ✅ aprovado / 🔁 volta p/ ajuste
```
