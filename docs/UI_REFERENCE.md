# 🖼️ UI_REFERENCE — Referencial de UI do editor (app oficial + achados + plano por etapas)

> **Status:** 🔨 em andamento (Fase 1 — casca/navegação) · **Criado:** 30/09/2026 ·
> **Fontes:** capturas do app oficial (`files/images/`, `analysis/screens.html`), manual oficial
> (`files/GP-100_Online Manual_EN_Firmware V2.0.pdf` → texto em `analysis/manual_streams.txt`),
> dicionário (`analysis/parameters.json` — 185 algs/639 controles), `docs/UI_DESIGN.md`,
> `docs/UI_PLAN.md`, decisões do owner (30/09, conversa de revisão de UI).
>
> **Por que este doc existe:** decisão do owner — "anote todos os achados que temos até
> agora de ajustes em uma doc de UI para não perdermos nada". Este é o inventário vivo
> de referências e ajustes pendentes. Nada daqui morre no chat.

---

## 1. O app oficial (Valeton GP-100 Suite v1.5.1) — layout completo (capturas)

Fonte: `files/images/Captura de tela 2026-09-25 174604.png` (tela principal) e
`...174654.png` (modal Settings). Cópia servida em `analysis/screens.html` (base64).

### 1.1 Tela principal (uma janela única, ~1502×687)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ [logo GP-100 MULTI-EFFECTS PROCESSOR | VALETON]  [⏻ ON  STOMP MODE]          │
│   [DRUM | Volume/Speed sliders | 120 BPM | 4/4 ▾]  [Master VOL 99][kill][⚙]  │
├───────────────────────────┬──────────────────────┬───────────────────────────┤
│ PAINEL DO MÓDULO          │ coluna do PATCH      │ BIBLIOTECA                │
│  PRE | COMP        [ⓘ]   │  ◀ P25 Mist ▶        │ [Factory Patch|User Patch]│
│  [knob Sustain 50]        │  [EXP Setting]       │  [Search…        ]        │
│  [knob Output  50]        │  Patch BPM [120]     │  NO. | Patch Name | Style │
│                           │  Stomp Mode:         │   P21 Dirty Funk  Funk    │
│ Effects List              │   FS1: MOD·DST·OFF   │   P22 Psychfuzz   Indie   │
│  [Search…        ]        │   FS2: OFF·DLY·RVB   │   P23 Trem OD     Indie   │
│  COMP ✓ / COMP4 / Boost / │                      │   P24 Alt. Clean  Indie   │
│  14 Boost / AC Sim / …    │                      │   P25 Mist ✓      Rock    │
│                           │                      │   P26 Dreamer     Indie   │
├───────────────────────────┴──────────────────────┴───────────────────────────┤
│ RODAPÉ DA CADEIA: [PRE][DST][AMP][NR][CAB][EQ][MOD][DLY][RVB]  Patch VOL 50 │
│   mini-pedais coloridos por família + nome do efeito de cada slot            │
│   (COMP, SM Dist, Tweedy, Gate 2, Foy Guy, Mess EQ, A-Chorus, P-Echo, Church)│
│   ⚠ avisos sob alguns slots       [Rename] [Save] [Import] [Export]          │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Observações de UX do oficial (o que copiar e o que melhorar):**
- **Nº de patch exibido é 1-based**: o oficial mostra "P25 Mist" para o `ppID=24`
  do all.prst (comparação captura × XML). Adotado na UI: display P01..P99; o FIO
  continua 0-based (pp 0x0000..0x0062 — a navbar é a única superfície de status:
  conexão, backend e boot num cluster só, sem seção de conexão duplicada).
- **Clicar no mini-pedal do rodapé** troca o módulo mostrado no painel grande à esquerda.
- **Effects List** = troca o EFEITO do slot (o dicionário do módulo, com busca).
- **Nome do preset NÃO fica no board** — fica na coluna do patch (◀ ▶ + nome) e na
  biblioteca. O owner aprovou: no nosso, display LED estilo hardware no topo do board
  e/ou navbar (decisão pendente — ver §6 Q2).
- **Stomp Mode com atribuições FS1/FS2** (MOD·DST·OFF etc.) — no hardware real, os
  footswitches assumem o efeito atribuído. No nosso: fase futura (precisa de captura G).
- **DRUM** (metrônomo: Volume/Speed/BPM/compasso) e **Master VOL** — settings globais;
  fios G3–G6 ainda NÃO capturados → UI mostra, escrita bloqueada até captura nova.
- **Warns ⚠** sob slots: do app oficial (provavelmente efeito sem atribuição FS/EXP).
  Mapeamento exato pendente — não copiar sem confirmar.
- **Dores já registradas** (VISION §4.4/§8): sem undo/diff, busca fraca, sem profundidade.

### 1.2 Modal Settings (captura 2) — abas e controles

```
Settings                                                   [X]
[ General | Global EQ | About | Info Frame | Help | Release Note ]
General:
  Input Level .......... [slider 100]
  Noise Gate (1) ....... [toggle]     Noise Gate (2) ....... [toggle]
  USE Audio ............ [toggle]
  Normal Level ......... [slider 100]
  Hint Mode ............ [Left | Right]
  Noise Mode ........... [Aver | Pink]      (legibilidade da captura limitada)
  Tap Tempo Mode ....... [x] PRE Mode  [ ] MOD Mode  [ ] DLY Mode
      "This is to provide for simplification of a modeless editing
       process for effects related to the pedal tempo"
  APP Language ......... [English ▾]
```

- **Global EQ** (confirmado no texto do manual): "Global EQ 1/5 OFF / Footswitch
  Mode / USB Audio ON/OFF / L-CUT FREQ / H-CUT FREQ" — são **5 bandas de EQ global**
  + cortes L/H; a UI do oficial lista como abas/páginas 1/5…5/5.
- Nossos **placeholders equivalentes**: ver §5 (issues U-1/U-2).
- **Política nossa (UI_PLAN §5):** escritas globais só pós-captura (G3–G6); a aba
  General da fase 1 persiste LOCAL (localStorage) e marca "prévia local".

### 1.3 Achados do manual (texto extraído, `analysis/manual_grep.py`)

| Item | Evidência (fio/manual) | Impacto na UI |
|---|---|---|
| Global EQ 5 bandas | "Global EQ 1/5 OFF, L-CUT FREQ, H-CUT FREQ" | aba própria com 5 páginas |
| Footswitch Mode | texto do manual ao lado do Global EQ | Stomp Mode / FS1-FS2 (fase futura) |
| USB Audio ON/OFF | idem | aba General (toggle prévia) |
| Tap Tempo Mode | tooltip do manual no checkbox (PRE/MOD/DLY) | aba General (checkbox trio) |
| APP Language | combo English (firmware tem en/zh/CN/es) | i18n 4 línguas (UI_PLAN M1.5) |
| Página do pedal | painel grande = 1 módulo com knobs + ⓘ | modal de edição do pedal (fase 2) |

---

## 2. Nossos dados reais (o que a UI pode listar HOJE)

| Fonte | Conteúdo | Onde a UI usa |
|---|---|---|
| `analysis/parameters.json` | 185 algoritmos/639 controles (knob/switch/combox, pos/min/max/default/options) | Effects List por módulo, knobs, ranges |
| `analysis/fx_map.json` + `packages/app/ui/src/artifacts/fxData.ts` | catálogo gerado por módulo (variant/nibble/index/name/controles) | fallback dev + models |
| `.prst` XML da máquina (fábrica) | presets nomeados (ex.: "It's GP100", PP25 "Mist") | biblioteca de fábrica (99 presets) |
| `docs/PROTOCOL.md` §13.9/13.10 | formato XML + scan de presets | leitura REAL da biblioteca |
| Stats por família | PRE16 DST19 AMP51 NR2 CAB60 (IRs, 1 knob) EQ3 MOD12 DLY12 (max 9 ctrl) RVB10 | tamanhos dos pedais, contagem da Effects List |

**Decisão de dados da fase 1 (owner pediu "todos os de fábrica, navegação
completamente funcional"):** importar/gerar a lista REAL dos 99 presets de fábrica
(nomes+tipos do `all.prst`/XML) para a biblioteca; abrir preset = REAL via §13.10
(mock pode sintetizar boards, mas a LISTA deve ser a de fábrica).

---

## 3. Achados acumulados das sessões de UI (checklist vivo — fase dos pedais)

> Tudo abaixo foi aprendido/decidido nas rodadas anteriores do pedalboard artístico.
> NADA disso se perde: vira o checklist da fase 2 (issue U-3+).

### 3.1 Comportamento (aprovado pelo owner)
- [x] **Trava do drag-and-drop** — toggle no CABEÇALHO DO PALCO ("⇄ mover",
      `aria-pressed`): drag só com ela ativa; no palco real (fatia 1) o pedal
      arrasta para QUALQUER posição com a trava destravada (`draggable`).
- [x] **Um efeito por vez primeiro** — validar CADA efeito isolado (o COMP primeiro)
  antes de devolver os 9 slots ao board. Owner: "primeiro vc deve ajustar todos os
  efeitos de comp, apenas com esse efeito visível". *Fatia 1 (PR da #19):* o
  Stage renderiza só as famílias de `PEDAL_FAMILIES_READY` e mantém os demais
  lugares como placeholders. *Fatia 4:* entraram **PRE/COMP e DST** (2 pedais
  reais + 7 placeholders); cada rodada do R7 acrescenta a próxima família.
- [x] **Modal de edição do pedal** — clique (ou Enter) em QUALQUER ponto do pedal
  amplia para edição dos parâmetros (estilo painel grande do oficial): **1.25×**
  do layout de edição (knob 64 → 80px), mesmos handlers (estado ÚNICO com o
  palco), fecha com **Esc / ✕ / clique fora** e o transporte global fica inerte
  enquanto aberto. Com a trava ⇄ ativa o clique pertence ao drag (não amplia).
  **Board = leitura, modal = edição.**
- [x] **Knobs do palco são SÓ LEITURA (pedido do owner, fatia 4)** — o pedal do
  board é compacto e mostra o VALOR de cada knob em texto; não há textbox nem
  gesto de ajuste no palco ("a edição pode ficar ruim com ele pequeno"):
  clica → abre o modal → ajusta → o palco reflete. O knob travado é
  `role="img"` (sem foco/handlers) e mantém o tooltip (modo engenheiro incluso);
  só o **footswitch** conserva clique próprio (liga/desliga).
- [x] **Nome do patch fora do board** — display LED (estilo hardware) no cabeçalho
  do palco (e na navbar); o board é só a cadeia.
- [x] **Toggle/LED** — LED VERDE = ON, VERMELHO = OFF. *Fatia 1:* footswitch
  alterna por prévia LOCAL (o protocolo capturado não tem comando de toggle).
- [x] **Valor editável via textbox** sob o knob (Enter aplica, Esc cancela,
  duplo-clique no knob = default; setas ±1%, Shift ±5%). A textbox vive no
  **modal** (no palco o valor é texto de leitura). Aplicar é LOCAL +
  `device_set_param` quando o knob é numérico (switch/combox só local — o
  protocolo capturado não tem encoding de opção).

### 3.2 Geometria/tamanhos (aprendidos com os testes)
- [x] **Pedal do PALCO: compacto, dentro do espaçamento do board** — largura
  `clamp(catálogo fxModels, 118, 132)` (os dois mínimos do `.board-slots`),
  knob 32px, passo 46px (encolhe p/ caber), rótulo + VALOR em texto, sem placa;
  altura cresce por linha (174 / 228 / 282px). `flexShrink: 0` no svg: o flex do
  slot nunca reescala o desenho (encolher o svg desalinhava tudo).
- [x] **Escala de EDIÇÃO (modal)**: layout grande próprio (420px de largura
  mínima, knob 64, pitch 116, placa GP-100) × `MODAL_SCALE = 1.25` → 525×470px
  com knob 80px. O bloco interno do modal PRECISA de `width/height` explícitos
  (sem eles ele herda a largura do wrapper e o 1.25× vira 1.5625×).
- [ ] Formas por variante: mini/box/widebox/treadle/filter/tscream/fuzz/rotary/
  amphead/echo (catálogo `fxModels.ts`). AMP = cabeçote NA CADEIA (Bog RedM: 6 knobs,
  PRES/MASTER/BASS/MIDDLE/TREBLE/GAIN); CAB = gabinete IR (1 knob de mic; 60 IRs);
  DLY pode ter 9 controles (6 knobs + 3 switches); wah = treadle com rocker.
- [ ] Testar CADA pedal em TODAS as 9 posições do board (tamanho/espaçamento/linhas).
- [ ] Knobs bidirecionais (min>max = centro físico) normalizados; EQ usa sliders.
- [x] Tooltip do modo engenheiro — **liga/desliga na aba General do Settings**
      (persistido local): `Sustain: 42 · SET · addr 10 01 00 02 · code 0x… · ctrl 0 ·
      payload [code u32 LE][ctrl][00][f32 LE]`. Sem referência à doc interna
      (`§13.11`) — lição Q-8 travada por teste (unit + e2e).

### 3.3 Lições de engenharia (não repetir)
- ⚠️ **`transform: scale()` + wrapper dimensionado = escala aplicada 2×** se o
  bloco interno não tiver `width/height` próprios: o bloco herda a largura já
  escalada do wrapper e o scale multiplica de novo. Sintomas: corpo menor que a
  área (espaço vazio), conteúdo descentralizado e barra de rolagem. O
  `PedalModal` ficou assim desde a fatia 3 e só apareceu no `Modal` real
  (medir `scrollWidth` do container é o teste rápido).
- ⚠️ **Item de flex encolhe o SVG** (padrão `flex-shrink: 1`): um pedal mais largo
  que a coluna do board era reduzido com o `viewBox` “meet” (desenho menor e
  centralizado, com letterbox). Ou o enclosure cabe no espaçamento (é o caso do
  palco: 118–132px) ou o svg precisa de `flexShrink: 0`.
- ⚠️ **`place-items: center` + overflow = conteúdo cortado à esquerda e
  inalcançável** (não dá scroll para o lado negativo). Use `justifyItems:
  safe center` quando o conteúdo pode ser maior que o container (modal).
- ⚠️ `str_replace` com texto parcial já quebrou o `Pedal.tsx` 2× — sempre reler o
  arquivo após edições grandes; preferir reescrever o bloco inteiro.
- ⚠️ Fallback dev exige nomes EXATOS do `fxData.ts` (era "4x12 Green", correto é
  "UK-GN 4x12").
- ⚠️ `fxData.ts` é GERADO (não editar mão) — fonte é `parameters.json` via
  `analysis/dump_fx_map.py`.
- ⚠️ `control_count` no fxData quebrou o tsc (removido).
- ⚠️ Crate `gp100-ui` NÃO compila no host (MSVC ausente — ADR-7); CI é a prova.

### 3.4 Afinador do palco (issue #8 — refinamento profissional)
- **Botão do monitor é VISUAL**: ícone ♪ + LED verde (ligado) / vermelho
  (desligado), sem palavra de estado; o rótulo acessível mora no
  `aria-label`/`title` (que já diz o estado). `data-tuner-power` para teste.
- **Grade FIXA de 4 colunas** (`auto auto 1fr auto` = monitor · modo · ref ·
  demo) que **nunca colapsa**: antes, com o monitor desligado, modo e REF
  PITCH sumiam e sobrava um buraco na linha (o painel ainda mudava de altura
  78→82px ao ligar). O slider do REF absorve a sobra — zero espaço vazio.
- **Ordem = fluxo**: on/off junto do display (LED/escala) e a demo — que é
  ferramenta de teste — na ponta direita.
- **O monitor MANDA na leitura**: desligado = não ouve (nota “—”, agulha
  centrada, LED próprio cinza) e nenhuma demo roda em background. Clicar em
  “▶ demo” com o monitor desligado **liga o monitor junto** (demonstrar exige
  ouvir) e desligar o monitor para a demo.
- **Nada vai para o device**: o tuner do GP-100 entra por gesto de hardware
  (segurar os 2 footswitches) e o protocolo capturado ainda não tem comando —
  os ajustes são prévia LOCAL persistida (`gp100.tuner.v1`). A leitura real
  entra pela prop `reading` quando o canal de áudio existir.

### 3.5 Referências externas (VISION §8.1)
- Neural DSP (cadeia visual drag-and-drop), Line 6 Helix Native (editores grandes
  por bloco, snapping), Boss Tone Studio (biblioteca+editor numa tela).
- Padrão vencedor: **cadeia horizontal + painel contextual + biblioteca lateral** —
  o oficial já segue; nossa modernização = trava de drag, undo de sessão, diff,
  busca rica, modais de edição, acessibilidade (UI_DESIGN §6-7).

---

## 4. Plano por etapas (decisão do owner, 30/09)

```
FASE 1 (AGORA) — CASCA COMPLETA, SEM PEDais
  layout geral, navegação, biblioteca de fábrica funcional (99), settings modal,
  board VAZIO com os 9 lugares marcados, trava do drag no lugar certo, logos,
  roteiros de teste manual → APROVAÇÃO DO OWNER
  (+ ajuste R-responsivo 30/09: board/looper em 1280/1024 — ver §7.3)
FASE 2 — PEDAL POR PEDAL (1 cadeia por entrega)
  PRE (COMP) → validar em todas as posições → DST → AMP → NR → CAB → EQ → MOD
  → DLY → RVB; modelagem SVG, knobs do dicionário, valor editável, modal;
  a cada rodada: teste manual (roteiro) → ajuste → entrega da cadeia completa
FASE 3 — CONTO E POLIMENTO
  footswitch/stomp, DRUM, Global EQ real, i18n, undo/diff, IR lab (issues M1.4+)
```

**O que a FASE 1 deve conter (definição de pronto):**
1. Topbar estilo oficial: logo GP-100·VALETON (nosso, sem copiar marca), status de
   conexão (LED de palco), Stomp Mode/DRUM/Master VOL visíveis (prévia local),
   botão ⚙ abrindo o modal Settings.
2. **Navegação completa de presets**: biblioteca com os 99 de fábrica (nome/tipo),
   busca, ◀ ▶, abrir = REAL §13.10 (mock quando sem device); display do corrente.
3. **Board vazio**: os 9 slots com placeholders nomeados (PRE/DST/AMP/NR/CAB/EQ/
   MOD/DLY/RVB), sem pedais desenhados; travinha "⇄ mover" presente (sem efeito
   visível ainda, mas já integrada).
4. **Modal Settings** com as 6 abas do oficial: General funcional-local (os itens
   §1.2), Global EQ placeholder (5 páginas), About/Info Frame/Help/Release Note.
5. Roteiros de teste manual (docs/UI_TEST_PLAN.md) e gate de aprovação do owner.

---

## 5. Issues registradas no ROADMAP (FASE U)

| Issue | Título | Doc |
|---|---|---|
| **U-1** | Casca da UI + navegação de presets de fábrica (99) funcional — ✅ FEITO 30/09 (aguarda aprovação) | ROADMAP §FASE U |
| **U-2** | Modal Settings (6 abas) + trava do drag no header — ✅ FEITO 30/09 (aguarda aprovação) | ROADMAP §FASE U |
| **U-3** | Pedais fase 2: 1 efeito por vez, checklist §3, teste por posição — 🔜 após aprovação | ROADMAP §FASE U |
| **U-4** | Roteiros de teste manual por rodada — ✅ FEITO 30/09 | docs/UI_TEST_PLAN.md |
| **U-5** | E2e da casca na CI (gate de release) + cobertura do manual — ✅ FEITO 30/09 | docs/MANUAL_COVERAGE.md |

---

## 6. Perguntas ao owner — ✅ RESPONDIDAS 30/09 (decisões da Fase 1)

- **Q1. Estilo da casca:** ✅ **próprio tokenizado** (organização do oficial +
  identidade palco/Valeton do UI_DESIGN).
- **Q2. Nome do patch:** ✅ **ambos** — navbar (`P25 Mist`) + display LED no topo
  do board (nº âmbar + nome + tipo).
- **Q3. Settings fase 1:** ✅ **persistência local + badge "prévia local"**
  (localStorage; escrita no device só pós captura G3–G6).

---

## 7. Drum e Looper — dados extraídos do firmware (30/09)

### 7.1 Bateria (drum) — 87 ritmos REAIS do firmware V2.1
- **Fonte:** tabela de estilos no `GP-100 Firmware V2.1.bin` (entre as divisões
  de nota `1/8T` e os compassos) — gerador `analysis/dump_drum_kit.py` →
  `packages/app/ui/src/artifacts/drumData.ts` (GERADO).
- **Estilos (87) em 5 grupos** na ordem do firmware: Electronic (Metro, Electro1,
  Electro2, Techno, TripHop, H-Hop1-4, D&B, Break, E-Pop) · Rock (Surfin, Hard 1-3,
  Rock 1-3, P Punk 1-2, Punk 1-5, Nu 1-2, P Rock1-3, Metal1-2, SF3/4, SF4/4,
  Rock5/4, EMO, R'n'R, Classic, Ballad, Shuffle, Core, NWave, Garag, Prog) ·
  Pop (Funk 1-4, Pop 1-3, Pub, Blues 1-4, Folk, B-grass) · World (March 1-2,
  Latin 1-3, RAG1-2, Bossa1-2, NuAge1-2, Samba, Ska, Polka, Mazuke, Beguine,
  Musette, Tango, Army, Waltz) · Jazz (Jazz 1-4, Funk1-3, Fusion).
- **Rótulos de gênero são INFERIDOS** (o firmware só grava "Electronic" como
  label; os cortes de grupo são evidentes na ordem).
- **Compassos reais:** 2/4 · 3/4 · 4/4 · 6/4 · 7/4 · 6/8 · 7/8 · 9/8 (a UI antiga
  tinha só 2/4…7/4 — corrigido).
- Marketing Valeton diz "100 patterns incl. metrônomos"; a tabela extraída tem
  87 estilos nomeados (inclui Metro). Mantemos os 87 reais.
- **UI:** o chip no topbar tem play/stop e stepper de BPM DIRETO na navbar
  (sem abrir o painel); o chip abre o `DrumPanel` como MODAL no padrão do
  Settings (gênero → estilo dependente, BPM, compasso, volume, speed;
  prévia local persistente — o drum NUNCA volta tocando).

### 7.2 Looper — máquina de fita reel-to-reel (skeuomórfico)
- **Dados reais (firmware menu LOOPER):** Rec VOL (0–99) · Play VOL (0–99) ·
  Pre/Post · P-VOL.
- **Specs oficiais (valeton.net + manual v1.8):** loop stereo com efeitos,
  **90 s em PRE / 45 s em POST** (24-bit · 44.1 kHz · SNR 110 dB).
- **Referências visuais (pesquisa):** Studer A807 / Nagra / Tandberg — dois rolos
  (supply/take-up) girando quando roda, VU meters com face âmbar e agulha animada,
  contador mecânico estilo fita, transporte com símbolos clássicos
  (⏪ ■ ▶ ●) e cabeçotes REC/PB + capstan no caminho da fita.
- **UI (`LooperPanel`):** deck de fitas + VU L/R + contador mm:ss + transporte
  (REW/STOP/PLAY/REC/CLEAR com confirmação; PLAY desabilitado com fita vazia;
  REC→PLAY→DUB→PLAY como looper de pedaleira) + rack com os 4 parâmetros do
  firmware + seletor PRE·90s/POST·45s + nota de specs. Prévia local (fio aguarda
  captura G3–G6).
- **Ajuste de linguagem visual:** painel "madeira/metal" escuro compatível com a
  paleta palco; animações desligam com `prefers-reduced-motion`.

### 7.3 Responsividade da casca (R-responsivo, 30/09)

> Pedido do owner: board + looper em 1280/1024 sem quebrar o alinhamento dos
> painéis. Implementado em CSS (sem estilos inline de grid) e verificado por
> MEDIÇÃO no Chrome (`analysis/responsivo_checks.js` — 9 asserts/viewport,
> 27/27 ✅; registro em docs/UI_TEST_PLAN.md, rodada R-RESPONSIVO).

| Regra | Onde vive | Comportamento |
|---|---|---|
| `.shell-main` | `design.css` (usada em `App.tsx`) | `minmax(0,1fr) 248px` (era 320px; estreitada na #11 para devolver largura ao pedalboard — 9 slots ganham ~50px de palco); ≤1100px EMPILHA (biblioteca vira linha, mesma largura do board) |
| `.board-slots` | `design.css` (usada em `Stage.tsx`) | colunas por conteúdo: `repeat(3, minmax(132px, max-content))`; ≥1700px `repeat(9, minmax(118px, max-content))`; `overflow-x: auto` |
| Looper `card` | `LooperPanel.tsx` | `repeat(auto-fit, minmax(300px,1fr))`: 4 tracks @1440 (a 4ª colapsa a 0px — sem buraco), 3 tracks @1280/1024 |
| `.gp-num` | `design.css` | number input sem spinners (visual idêntico aos selects) |

- **Medidas canônicas (Chrome):** overflow X = 0 nos 3 tamanhos; slots medidos
  POR LINHA com colunas por conteúdo (o lugar vazio tem 142px; o pedal do palco
  fica em 118–132px dentro dele) e **zero sobreposição**; looper tracks
  439.3×3+0 @1440, 386×3 @1280, 300.7×3 @1024; nenhum texto clipado nos painéis.
- **Ferramenta:** os asserts de alinhamento viraram TESTE e2e
  (`e2e/responsivo.spec.ts` + `e2e/_helpers.ts::expectShellAligned` — rodam na
  CI e localmente, 3 viewports). O script `analysis/responsivo_shots.js`
  continua útil só para gerar screenshots/recortes de inspeção do owner.

### 7.4 Atalhos globais de teclado (R-ATALHOS, 30/09)

> Pedido do owner: Espaço = drum play/stop, R = REC do looper, Esc fecha painéis.
> **Doc de usuário = aba Help do Settings** (SettingsModal); esta seção é a doc
> técnica. Registro de execução: docs/UI_TEST_PLAN.md (R-ATALHOS).

| Tecla | Ação | Detalhe |
|---|---|---|
| `Espaço` | Drum play/stop | alterna `drum.on` (mesmo efeito do toggle ⏵/⏹ da navbar e do modal de gestão) |
| `R` | Looper REC | dispara o MESMO `onRec` do botão ● (rec → play → dub → play) |
| `Esc` | Fecha o painel do TOPO | precedência: Settings → Drum → drawer de pushes |

- **Implementação:** hook `src/hooks/useGlobalShortcuts.ts` (listener único em
  `window`; handlers relidos via ref a cada render; helpers PUROS exportados:
  `isTextEntryTarget`, `isSpaceNativeTarget`, `shouldHandleShortcut`).
- **Guardas de a11y:** campos de texto/busca/select nunca disparam transporte;
  Espaço sobre botão/switch/option mantém a ativação NATIVA (o controle focado
  ganha, não o drum); Ctrl/Alt/⌘ ignorados; auto-repeat ignorado; com o modal
  Settings aberto, Espaço/R ficam inertes (Esc fecha o modal primeiro).
- **Precedência do Esc:** exigiu subir `drumOpen` do TopBar para o App (estado
  controlado) e REMOVER o listener próprio de Esc do DrumPanel — com os dois,
  Esc fechava Settings E drum de uma vez (achado dos testes). O `Esc` do diálogo
  do Settings (foco dentro) permanece e é idempotente com o global.
- **R no looper:** o `mode` de transporte vive DENTRO do LooperPanel (sessão);
  o App só incrementa `recRequest` e o painel converte o pulso em `onRec` via
  ref (sem re-registrar efeito, sem estado global extra).
- **Testes:** 6 unit novos em `tests/shortcuts.test.tsx` (helpers + integração:
  toggle do drum, REC→PLAY, guarda de digitação, precedência do Esc) e 3 e2e em
  `e2e/atalhos.spec.ts` no Chromium — suíte total: **23 unit · 11 e2e** ✅.

### 7.5 BANCOS DE PATCH: o patch aberto É o que o pedalboard mostra (#11)

> Antes desta rodada a biblioteca era decorativa: trocar de preset mudava o nome
> no LED/navbar e os 9 pedais seguiam IGUAIS. A causa era o **mock web**
> (`localMockBoard`), que tinha uma cadeia FIXA (COMP/Green OD/Bog RedM/…) para
> os 99 presets. O core Rust já montava a cadeia por preset
> (`pedalboard::board_view_for`: `slot` = `@x`, `family` = `effectModuleName`,
> `code` = `effectCode`, knob = `params_N`) — o gap era só a fonte do mock.

| Peça | Onde | O que faz |
|---|---|---|
| Cadeia real dos 99 | `src/artifacts/presetChains.ts` (**GERADO** por `analysis/dump_preset_chains.py` do `all.prst`) | 99 × 9 slots com `family`, `name`, `state`, `code`, `params` crus |
| Leitura no mock | `src/ipc/device.ts::localMockBoard` | `algFor` pelo `effectCode`; knob = `params[pos]` se plausível (dentro do range, nunca o sentinel `0xFFFF`); algoritmo fora do dicionário → pedal SEM knobs (regra do core) |
| Ordem do palco | `Stage.tsx::baseOrder` | ordena pelo `slot` REAL, não pela família: **20 dos 99 presets têm a cadeia trocada** (ex.: P06 = DST antes de PRE) |
| Banco + rótulo | `BoardView.bank` / `BoardView.ppLabel` | `"factory"`/`"user"` + `"P25"` (1-based) ou `"U01"` — o LED e a navbar leem o rótulo do board |
| Patch de usuário | `src/userPatches.ts` | snapshot PROFUNDO dos 9 slots (retrato imutável), `localStorage` `gp100.userpatch.v1`, `boardOfUserPatch` devolve um `BoardView` no mesmo formato do device |

**Contrato de honestidade da UI:** `pp` só muda DEPOIS do device confirmar
(`openPreset`); abrir patch de usuário não pode falhar (nada é escrito); excluir
o patch que está no palco devolve o preset de fábrica confirmado; a aba segue o
BANCO ABERTO (abrir um patch troca a aba, clicar na aba só navega a visão).

**Por enquanto o patch de usuário é PRÉVIA LOCAL.** O `save_preset` do device
precisa do canal de escrita USB (BLOCKERS §4: fluxos novos ficam atrás de
`WRITE_VERIFIED=false` até captura própria). O formato salvo é o do `.prst`
(`PP ppID` + `Effect`), então a migração é escrever o arquivo — não refazer a UI.

### 7.6 PLANEJADO — lista de efeitos por pedal (issue #19, U-3)

> O manual do firmware (EDIT mode, pág. 7-8) confirma o modelo: 9 módulos
> selecionados por um knob, parâmetros paginados (3 por página), e
> pressionar-e-seguir o PARA muda a POSIÇÃO do módulo. O wireframe do §1.1 já
> prevê a coluna "Effects List" (busca + lista de algoritmos do módulo, com o
> atual marcado).

**O que já existe no repo:** o dicionário completo (`analysis/parameters.json`,
`algorithm.xml` do Suite e o artefato `src/artifacts/fxData.ts`) com as 9
famílias e TODOS os algoritmos, cada um com knobs/switches/comboxes, ranges e
defaults reais. Trocar de efeito é, no `.prst`, mudar `effectName` + `effectCode`
do `<Effect>` do slot e reiniciar `params_N`.

**O que falta no fio:** a família `0x4X` do PROTOCOL §4 é `0x43` select,
**`0x47` change-effect**, `0x48` change-param, `0x49` toggle-block, `0x4F` bulk
write. Só `0x48` (envelope semântico `10 <slot> 00 02`, §13.11) e `0x4F`
(`0x1D`, bulk do `.prst`, §4) têm formato validado. **O `0x47` ainda não foi
caracterizado** — logo, a troca de efeito é UI primeiro (prévia local, a mesma
política do toggle de footswitch e do switch/combox) e só vira escrita depois da
captura própria.

### 7.7 CI: e2e da casca contra o `pnpm dev` + cobertura do manual (30/09)

- **Job `e2e` do `_validate.yml`** (ubuntu, Chromium via `playwright install
  --with-deps`): roda a suíte completa contra o dev server que o próprio
  `playwright.config.ts` sobe (`webServer` + strictPort + reuseExistingServer).
  Mesmo filtro de caminho do job front (mudança em `packages/app/ui/`).
- **Gate de release:** o publish (`release.yml` → `_publish.yml`) só roda após
  o reusable `_validate.yml` COMPLETO (gate+spec+matrizes+e2e+visual+smoke) —
  **tag só sai com a casca verde**.
- **Suíte total:** 14 e2e ✅ (R1–R6 + drum + looper + 3 atalhos + 3 responsivos)
  · 23 unit ✅. Asserts de alinhamento portados p/ `e2e/_helpers.ts`
  (`measureShell`/`expectShellAligned`) — alinhamento de componentes é testado
  AUTOMATICAMENTE (não depende de observação manual).
- **Cobertura funcional:** `docs/MANUAL_COVERAGE.md` — matriz item-por-item do
  manual/app oficial com implementação + teste (33 ✅ · 5 🟡 · 5 🔴; nenhum
  estado silencioso: o que aguarda captura está EXPLICITADO na UI).
- **Achados corrigidos na rodada:** BPM do drum desalinhado (258×46 vs 240×32 —
  o UA só aplica `border-box` a `<select>`; `.gp-num` ganhou `box-sizing`, com
  assert e2e), redundância do looper removida (estado da fita num lugar só),
  navegação ◀ ▶ do manual implementada e testada, Help verificado por e2e.

---

## 8. Padrão de mensagens de UI (criado 30/09 — review Q-1/Q-2)

> **Fonte única:** `src/i18n/messages.ts` (`MSG`). Nenhum texto de usuário novo
> fica inline no JSX. Motivo: o Settings já prevê APP Language (pt/en/es/zh) —
> texto espalhado inviabiliza o i18n; e o review de 30/09 encontrou
> vocabulário interno (fases, fios, capturas, issues) vazando para o usuário.

**Regras (o lint/review cobra):**
1. **Sem vocabulário interno** — o usuário não conhece nosso roadmap: nada de
   "fase 2", "U-3", "captura G3–G6", "fio", "mock do protocolo", "§". O que
   ainda não funciona aparece como "disponível em uma próxima versão" (e o
   controle fica DESABILITADO — nunca um controle ativo que não faz nada).
2. **Honestidade de estado** — "prévia local" quando a escrita ainda não chega
   ao hardware; controles de escrita pendente são desabilitados com a razão.
3. **Números e dados reais** — 87 ritmos, 99 presets, 90/45 s vêm dos artefatos
   gerados; nada de "100 patterns" de marketing.
4. **Módulo pronto p/ i18n** — `MSG` é um dict plano tipado (strings e
   funções puras `t(q) => string`); trocar por dicionário por idioma é
   mecânico quando o i18n entrar (M1.5 do UI_PLAN).

**Já removidos por violação:** "pedal na fase 2" → "vazio"; "(D7)" no PushLog;
"captura G3–G6" (drum/looper/settings); "Fase M"/"H1" na ConnectionBar;
"§13.12" na biblioteca; "docs/MANUAL_COVERAGE.md" no Release Note.

**Guard-rail automático (30/09, Q-1):** o lint custom `local/no-user-literals`
(eslint.config.js, plugin local) roda como `error` em `src/components/**` +
`src/App.tsx` e bloqueia: JSXText com letras, literais em `title`/`placeholder`/
`aria-label`/`label`/`alt`, template sem interpolação e ternários com ramo
literal (`{cond ? "ok?" : "✕"}`) nos mesmos pontos de texto. A mensagem manda
mover para MSG. Texto novo de usuário = chave nova em `messages.ts` — se o lint
deixou passar, é buraco da regra (fechar, não contornar); só sobrevivem
literais técnicos (className, data-*, role, state).

---

*Manutenção: novo achado → adicionar em §3 com checkbox; decisão do owner → §4/§6;
issue nova → §5 + ROADMAP.*
