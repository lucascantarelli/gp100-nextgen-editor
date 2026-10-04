# ARQUITETURA — o front (gp100-ui)

> **O que este documento é.** O mapa de módulos do front e, principalmente, **o que
> não entra em cada módulo**. `VISION.md` §6–§9 responde "com que stack"; este
> responde "onde a linha mora". Nasceu da #82, que mediu `App.tsx` com 514 linhas
> e o teste que o cobre com 1027 — o sintoma de um componente que passou a hora de
> virar hook/container.
>
> **Escopo.** Só o `packages/app/ui` (Tauri + React + TypeScript). O core Rust
> tem decisão própria em `docs/DECISIONS.md` (ADR-1..8); a fronteira entre os dois
> é o comando Tauri, descrito em `docs/UI_PLAN.md`.

---

## 1. A regra que substitui a convenção

Cada camada é definida pelo que ela **proíbe** fazer. Um módulo que não pode
violar a própria fronteira é mais difícil de quebrar do que um que "deveria"
seguir o padrão.

| Camada | O que é | **O que NÃO entra** |
|---|---|---|
| `src/ipc/` | A **porta** para o device. Fala Tauri, aplica política de execução, traduz erro técnico em erro de domínio. | Nada de React. Nada de estado de tela. Nada de regra de produto (quem decide que preset 12 abre o slot 8 é o palco, não a porta). |
| `src/hooks/` | **Estado de sessão**. Compõe a porta com regras de UI; é onde mora a política ("não otimista", "falha tem ação"). | Nada de JSX. Nada de geometria. Nada de `localStorage` cru fora do hook que é dono da chave. |
| `src/components/` | **Apresentação.** Recebe dados prontos e callbacks. | Nada de chamada direta ao device — se um componente precisa do device, é sinal de que falta um hook. Nada de número mágico de layout (vai para `design/`). |
| `src/design/` | **Puro, sem React.** Geometria, tokens, cálculos de layout. Testável sem renderizar. | Nada de React — nem como tipo. Se `design/` importar React, a distinção deixou de ser caminho de import e virou sugestão. |
| `src/artifacts/` | **Dados gerados.** Dicionário de efeitos, presets, cadeias. Vem de `analysis/`. | Nada de lógica — é dado, e dado não decide nada. Regenerável: se alguém editar à mão, o próximo `build_*.py` sobrescreve. |
| `src/i18n/` | **Texto de usuário.** Toda string visível mora aqui. | Nada de lógica de tela. `facts.ts` é a exceção: números que a UI mostra sem vir do device. |
| `src/looper/`, `src/tuner/` | **Regras de domínio** dos dois painéis que têm máquina de estados. | Nada de React — mesma razão de `design/`. |

**Direção permitida:** `App → components → hooks → ipc → (Tauri)`, com `design/`
e os artefatos importáveis de qualquer camada abaixo. **Proibido:** `ipc → hooks`,
`design → components`, `artifacts → i18n` (o dicionário de efeitos não escolhe
idioma) e qualquer seta que faça o core Rust depender do front.

---

## 2. O mapa

```
src/
├── App.tsx (274)          casca: boot, atalhos, modais abertos e o JSX
├── main.tsx (11)          monta o React
│
├── ipc/                   ── a PORTA ──────────────────────────
│   ├── device.ts (505)    device_select_preset, device_board, … + retry declarativo
│   ├── push.ts (63)       log de pushes do device
│   └── types.ts (146)     BoardSlot/BoardView/DeviceInfo — o contrato do palco
│
├── hooks/                 ── ESTADO DE SESSÃO ───────────────────
│   ├── useStage.ts (248)  o que está NO PALCO: abrir/salvar/apagar preset e
│   │                      patch, knob, efeito, footswitch, o banner {msg,retry}
│   ├── usePrefs.ts (141)  o que SOBREVIVE à janela: master, drum, tuner,
│   │                      looper, settings/idioma
│   ├── useBoot.ts (113)   a sequência de boot e seu progresso
│   ├── usePushLog.ts (38) o ring buffer de pushes
│   └── useGlobalShortcuts.ts (96)
│
├── components/            ── APRESENTAÇÃO ───────────────────────
│   ├── Stage.tsx (359) · Pedalboard.tsx (337) · Pedal.tsx (504)
│   │   PedalModal.tsx (284) · Knob.tsx (207) · TopBar.tsx (385)
│   ├── LibraryPanel.tsx (269) · LooperPanel.tsx (610)
│   ├── DrumPanel.tsx (196) · TunerPanel.tsx (359) · PushLog.tsx (104)
│   └── BootProgressBar.tsx (63) · ErrorBanner.tsx (56)
│
├── design/                ── PURO, SEM REACT ────────────────────
│   ├── geometry.ts (131)  pedalDims/layoutFor: a geometria saiu do Pedal (#82)
│   └── tokens.ts (168)    paleta, escala de Fibonacci, motion
│
├── looper/fsm.ts (146) · tuner/pitch.ts (162) ── máquinas de estado puras
├── effects.ts (94)        troca de algoritmo dentro do slot (issue #19)
├── userPatches.ts (83)    snapshot de patch de usuário (prévia em localStorage)
│
├── artifacts/             ── DADOS GERADOS ──────────────────────
│   └── presetChains.ts · fxData.ts · fxModels.ts · presetData.ts · drumData.ts
│
└── i18n/                  messages.ts (MSG) · facts.ts · dictionaries/{pt-BR,en,es,zh}.ts
```

### Onde o estado mora (e por que o corte foi coesão, não tamanho)

O critério de extração **não foi "arquivo pequeno"**. Foi: *um preset de fábrica
e um patch de usuário são a mesma coisa vista de dois lugares* — ambos são um
`BoardView`. Abrir, salvar, excluir e mexer no palco são então **operações sobre
um único estado**, e foram para o `useStage` juntos. Preferências de menu, drum,
tuner e looper não têm nada a ver com o palco e foram para o `usePrefs`.

O que ficou no `App` é o que só ele tem: boot, log de pushes, atalhos globais,
quais modais estão abertos e o JSX.

---

## 3. Onde vai o código novo (árvore de decisão)

1. **É um número de layout ou um cálculo puro?** → `design/`. Não precisa de React
   para testar, e não deve poder importar.
2. **Precisa falar com o device?** → passa por `ipc/device.ts`, **declarando a
   política de execução na chamada** (`IDEMPOTENTE` ou `semRetry("motivo")`). Não
   existe chamada que herda retry — o padrão está na assinatura justamente para o
   compilador cobrar a decisão.
3. **É estado que sobrevive a um rerender?** → `hooks/`. Se alguém fecha a janela,
   ele vai para `usePrefs` com a chave de `localStorage` exportada pelo dono
   (`LooperPanel.KEY`, `SettingsModal.KEY`) — nunca com a string repetida no
   chamador.
4. **Desenha alguma coisa?** → `components/`, recebendo props prontas.
5. **Não é nenhum dos acima?** → provavelmente é regra de negócio: para o módulo
   de domínio (`looper/`, `tuner/`), não para o componente.

---

## 4. Orçamento de tamanho (e por que é gate)

A #82 é, na verdade, **prevenção**: cada marco novo adiciona estado ao App, e o
App volta a inchar. Um critério que só vale no dia do PR não impede nada.

| Arquivo | Teto | Por que |
|---|---|---|
| `App.tsx` | **300** | casca: se passar disso, estado de sessão está voltando para a casca |
| `LooperPanel.tsx` | 700 | apresentação grande; a FSM já saiu para `looper/fsm.ts` |
| qualquer outro `components/` | 700 | acima disso, ou falta extrair regra ou falta extrair subcomponente |
| `ipc/device.ts` | 600 | a porta cresce por **operação de device**, não por feature de UI |

O gate é `scripts/check_module_size.py`, rodado em `scripts/gates.py` e no
`ci.yml`. Ele falha com o nome do arquivo e a contagem, e a lista de exceções fica
no próprio script — exceção nova é decisão consciente, não esquecimento.

---

## 5. O que este documento **não** é

- **Não é a spec do produto.** `VISION.md` e `UI_REFERENCE.md` (que é a fonte de
  verdade do texto de usuário).
- **Não é o mapa do core Rust.** `DECISIONS.md`.
- **Não é o roteiro de testes.** `UI_TEST_PLAN.md`.
- **Não é o estilo do pipeline.** `CONTRIBUTING.md` §5.

Se algo aqui ficar errado, **este arquivo é o que se corrige** — não o
comentário do arquivo de código. Comentário que duplica documento apodrece na
primeira alteração.
