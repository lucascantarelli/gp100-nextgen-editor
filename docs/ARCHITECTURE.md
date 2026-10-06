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
├── App.tsx (299)          casca: boot, atalhos, modais abertos e o JSX
├── main.tsx (11)          monta o React
│
├── ipc/                   ── a PORTA ──────────────────────────
│   ├── device.ts (535)    device_select_preset, device_board, … + retry declarativo
│   ├── diag.ts (132)      a COORTE de diagnóstico: gravar no aparelho, ler o
│   │                      dump, a sessão de log e a prévia do envio
│   ├── push.ts (63)       log de pushes do device
│   ├── library.ts (517)   a lista e o CRUD de patch de usuário + o store do
│   │                      histórico em memória (o fallback); `registraVersao`
│   │                      mora AQUI, dentro do `librarySave` — ver `history.ts`
│   ├── history.ts (312)   o HISTÓRICO por patch (#113): as 5 leituras/restaurações
│   │                      e a cópia do diff do crate para o fallback
│   ├── preset.ts (190)    o PRESET COMO ARQUIVO (#114): exportar em JSON
│   │                      versionado, a folha de timbre em PDF e o import de
│   │                      volta para o palco. O fallback fora do shell exporta
│   │                      a CADEIA, com formato próprio — o front não tem o
│   │                      `.prst`, e o core recusa as duas formas com a
│   │                      mensagem certa (ver `preset_json.rs`)
│   ├── gain.ts (263)     o ASSISTENTE DE GAIN STAGING (#115): o relatório
│   │                      módulo a módulo, com a origem de cada número, o risco
│   │                      e a ordem de ajuste. Porta de LEITURA — sem
│   │                      `set_param`; o fallback espelha a regra de
│   │                      `packages/core/src/gain.rs` (a mesma regra, duas
│   │                      cópias, e o preço de divergirem está no cabeçalho)
│   ├── ir.ts (394) · tones.ts (388)   os laboratórios de IR e de tones (#24/#25)
│   └── types.ts (218)     BoardSlot/BoardView/DeviceInfo — o contrato do palco
│
├── hooks/                 ── ESTADO DE SESSÃO ───────────────────
│   ├── useStage.ts (310)  o que está NO PALCO: abrir/salvar/apagar preset e
│   │                      patch, o IMPORTADO de arquivo (#114), knob, efeito,
│   │                      footswitch, o banner {msg,retry}
│   ├── useLibrary.ts (378) a lista da biblioteca, a busca e o CRUD de usuário
│   ├── useHistory.ts (253) o par comparado, o diff e as restaurações (#113);
│   │                      a política fica no hook porque ela sobrevive ao reload
│   │                      da lista (o 1º `restaura` recarrega — o par não pode
│   │                      morar no componente)
│   ├── useIrs.ts (248) · useTones.ts (267)  os laboratórios (#24/#25)
│   ├── usePrefs.ts (142)  o que SOBREVIVE à janela: master, drum, tuner,
│   │                      looper, settings/idioma
│   ├── useBoot.ts (113)   a sequência de boot e seu progresso
│   ├── usePushLog.ts (38) o ring buffer de pushes
│   ├── useFieldDiag.ts (197) o alvo (patch/nome) e o que voltou do diagnóstico
│   ├── useGlobalShortcuts.ts (96)
│   ├── useGain.ts (59)   o relatório do assistente em andamento, com falha
│   │                      COM ação (#20) — o retry relê o MESMO pp
│   ├── useContentMenu.tsx (102)  a PORTA do conteúdo (#25/#24/#114/#115):
│   │                      escolhe entre tons, IRs, o preset em arquivo e o
│   │                      assistente de gain — as duas últimas telas nascem AQUI,
│   │                      porque não têm estado a preservar entre aberturas
│
├── components/            ── APRESENTAÇÃO ───────────────────────
│   ├── Stage.tsx (359) · Pedalboard.tsx (337) · Pedal.tsx (504)
│   │   PedalModal.tsx (284) · Knob.tsx (207) · TopBar.tsx (385)
│   ├── LibraryPanel.tsx (516) · LooperPanel.tsx (637)
│   ├── HistoryPanel.tsx (298) o histórico do patch (#113): par antes/depois
│   │                      visível, o diff por knob e as duas restaurações
│   ├── IrLabPanel.tsx (417) · SnapTonePanel.tsx (417)  os laboratórios
│   ├── SettingsModal.tsx (409) · TunerPanel.tsx (359)
│   ├── PresetFilePanel.tsx (244) o preset em arquivo (#114): os 3 caminhos
│   │                      (JSON, folha, import) e o relato do que entrou
│   ├── GainPanel.tsx (271) o assistente de gain staging (#115): risco, menor
│   │                      folga, a ORIGEM de cada número, a ordem de ajuste e o
│   │                      MÉTODO impresso na tela — sem nenhum botão de escrita
│   ├── DrumPanel.tsx (196) · PushLog.tsx (104) · ContentMenu.tsx (98)
│   ├── FieldDiagPanel.tsx (350) o diagnóstico de campo em tela (carrega o
│   │                      próprio <details>, para o App ganhar 1 linha)
│   └── BootProgressBar.tsx (64) · ErrorBanner.tsx (57)
│
├── design/                ── PURO, SEM REACT ────────────────────
│   ├── geometry.ts (131)  pedalDims/layoutFor: a geometria saiu do Pedal (#82)
│   └── tokens.ts (168)    paleta, escala de Fibonacci, motion
│
├── looper/fsm.ts (146) · tuner/pitch.ts (162) ── máquinas de estado puras
├── effects.ts (94)        troca de algoritmo dentro do slot (issue #19)
├── userPatches.ts (118)   snapshot de patch de usuário (prévia em localStorage)
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

**A porta do histórico (#113) é o contraexemplo que fixa a regra.** `library.ts`
e `history.ts` parecem o mesmo assunto e viraram dois arquivos assim mesmo: o
primeiro é o CRUD da lista e o registro corrente (mutável), o segundo é a
leitura de um histórico **imutável** mais as restaurações. Duas
responsabilidades, duas coortes de contrato. E a seta de dependência ficou de um
lado só de propósito: o store de versões do fallback mora em `library.ts`,
porque a invariante que garante a feature — "**toda** gravação de patch
versiona" — é do `librarySave`. Se o store viesse para `history.ts`, o
`librarySave` teria que importar de lá e o par viraria um ciclo. O que impede
isso de se perder é o teste do `librarySave` que conta a versão a cada gravação.

---

## 3. Onde vai o código novo (árvore de decisão)

1. **É um número de layout ou um cálculo puro?** → `design/`. Não precisa de React
   para testar, e não deve poder importar.
2. **Precisa falar com o device?** → passa por `ipc/device.ts`, **declarando a
   política de execução na chamada** (`IDEMPOTENTE` ou `semRetry("motivo")`). Não
   existe chamada que herda retry — o padrão está na assinatura justamente para o
   compilador cobrar a decisão. Se a operação é de **diagnóstico de campo**
   (gravar/ler/dump/log/prévia), ela vai para `ipc/diag.ts`: são outra coorte
   de contrato — hexadecimal em vez de estado, uma delas não idempotente, uma
   delas sessão de arquivo — e misturar as duas fez a porta do dia a dia passar
   do teto sem nenhuma decisão nova.
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
| `ipc/diag.ts` | 600 | herda a regra: cresce por operação de diagnóstico, não por feature de UI |

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
