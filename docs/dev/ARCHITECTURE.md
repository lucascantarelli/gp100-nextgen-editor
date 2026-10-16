> ✅ **atual** · última revisão 2026-10-14.

# ARQUITETURA — o front (gp100-ui)

> **O que este documento é.** O mapa de módulos do front e, principalmente, **o que
> não entra em cada módulo**. `VISION.md` §6–§9 responde "com que stack"; este
> responde "onde a linha mora". Nasceu da #82, que mediu `App.tsx` com 514 linhas
> e o teste que o cobre com 1027 — o sintoma de um componente que passou a hora de
> virar hook/container.
>
> **Escopo.** O `packages/app/ui` (Tauri + React + TypeScript) e — desde a #243 —
> o orçamento de tamanho e o mapa de módulos do crate do app, `packages/app/api`
> (§4.1). O core Rust tem decisão própria em `docs/dev/DECISIONS.md` (ADR-1..8); a
> fronteira entre os dois é o comando Tauri, descrito em `docs/dev/UI_PLAN.md`.

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
| `src/artifacts/` | **Dados gerados.** Dicionário de efeitos, presets, cadeias. Vem de `docs/arquivo/analysis/`. | Nada de lógica — é dado, e dado não decide nada. Regenerável: se alguém editar à mão, o próximo `build_*.py` sobrescreve. |
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
├── App.tsx (300)          casca (gate de boot #161): navbar + gate, atalhos,
│                          modais abertos e o JSX — a casca só monta em `ready`
├── main.tsx (11)          monta o React
│
├── ipc/                   ── a PORTA ──────────────────────────
│   ├── device.ts (534)    device_select_preset, device_board, … + retry declarativo
│   │                      e a POLÍTICA de escrita da tela: `escritaLiberada()`
│   │                      é a ÚNICA leitura de `writeVerified` para decidir
│   │                      botão (#126 — knob/IR/SnapTone nascem travados)
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
│   └── types.ts (241)     BoardSlot/BoardView/DeviceInfo — o contrato do palco
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
│   ├── useBoot.ts (192)   a sequência de boot, seu progresso e a VALIDAÇÃO
│   │                      do relatório (#161: inventário/nomes coerentes
│   │                      com o catálogo — falhou, o gate vira erro)
│   ├── usePushLog.ts (38) o ring buffer de pushes
│   ├── useFieldDiag.ts (197) o alvo (patch/nome) e o que voltou do diagnóstico
│   ├── useGlobalShortcuts.ts (96)
│   ├── useGain.ts (59)   o relatório do assistente em andamento, com falha
│   │                      COM ação (#20) — o retry relê o MESMO pp
│   ├── useAb.ts (295)  o A/B entre duas versões do MESMO patch (#116): o par
│   │                      do histórico, a troca (palco + set_param), a
│   │                      calibração de nível e o BLIND — estado da tela, lido
│   │                      só na abertura da porta (não no mount)
│   ├── useContentMenu.tsx (136)  a PORTA do conteúdo (#25/#24/#114/#115/#116):
│   │                      escolhe entre tons, IRs, o preset em arquivo, o
│   │                      assistente de gain e o A/B — as três últimas telas
│   │                      nascem AQUI, porque a casca está no teto (300/300)
│
├── components/            ── APRESENTAÇÃO ───────────────────────
│   ├── Stage.tsx (359) · Pedalboard.tsx (337) · Pedal.tsx (518)
│   │   PedalModal.tsx (293) · Knob.tsx (218) · TopBar.tsx (385)
│   ├── LibraryPanel.tsx (516) · LooperPanel.tsx (637)
│   ├── HistoryPanel.tsx (298) o histórico do patch (#113): par antes/depois
│   │                      visível, o diff por knob e as duas restaurações
│   ├── IrLabPanel.tsx (440) · SnapTonePanel.tsx (434)  os laboratórios
│   ├── SettingsModal.tsx (409) · TunerPanel.tsx (359)
│   ├── PresetFilePanel.tsx (244) o preset em arquivo (#114): os 3 caminhos
│   │                      (JSON, folha, import) e o relato do que entrou
│   ├── GainPanel.tsx (271) o assistente de gain staging (#115): risco, menor
│   │                      folga, a ORIGEM de cada número, a ordem de ajuste e o
│   │                      MÉTODO impresso na tela — sem nenhum botão de escrita
│   ├── AbPanel.tsx (246) o A/B com blind (#116): os dois lados, o nível em
│   │                      POSIÇÃO com o método ao lado, a calibração e o modo
│   │                      blind — armado, rótulo/versão/nível/relatório somem
│   │                      até o palpite (o teste de UI prova a ausência)
│   ├── DrumPanel.tsx (196) · PushLog.tsx (104) · ContentMenu.tsx (105)
│   ├── FieldDiagPanel.tsx (351) o diagnóstico de campo em tela (carrega o
│   │                      próprio <details>, para o App ganhar 1 linha)
│   ├── BootGate.tsx (122) o GATE de boot (#161): fora do ready a página é
│   │                      navbar + este componente — progresso (loading) ou
│   │                      motivo + ações "Refazer o boot"/"Reconectar" (erro,
│   │                      role=alert com foco)
│   └── BootProgressBar.tsx (64) · ErrorBanner.tsx (57)
│
├── design/                ── PURO, SEM REACT ────────────────────
│   ├── geometry.ts (131)  pedalDims/layoutFor: a geometria saiu do Pedal (#82)
│   └── tokens.ts (168)    paleta, escala de Fibonacci, motion
│
├── looper/fsm.ts (146) · tuner/pitch.ts (162) ── máquinas de estado puras
├── abLevel.ts (150)     o NÍVEL e a TROCA do A/B (#116): posição média dos
│                         controles de saída, a calibração (só compartilhados)
│                         e o que é gravável — puro, sem IPC nem React
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

O que ficou no `App` é o que só ele tem: o gate do boot (#161 — fora do
`ready` a página é navbar + `BootGate`), log de pushes, atalhos globais, quais
modais estão abertos e o JSX.

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

O gate é `.github/scripts/check_module_size.py`, rodado em `.github/scripts/gates.py` e no
`ci.yml` (no job do escopo do **front** e no do escopo **Rust** — `lint-ui` sozinho
não bastava, porque ele só existe quando o front muda). Ele falha com o nome do
arquivo e a contagem, e a lista de exceções fica no próprio script — exceção nova
é decisão consciente, não esquecimento. O teste do gate (com árvores sintéticas,
porque inflar arquivo de verdade para provar que ele reprova é teste que ninguém
roda duas vezes) vive em `tools/tests/test_check_module_size.py`.

### 4.1 Rust: `packages/*/src` e o crate do app

A **#248** estendeu o MESMO gate ao Rust, e a razão é medida: sem freio nenhum,
`packages/app/api/src/actor.rs` chegou a **2 702 linhas** (F-07 da auditoria de
14/10). A régua é **700 linhas** por `.rs` não declarado — acima disso o módulo
tem mais de um assunto — e quem já estava acima quando o gate passou a medir tem
teto **congelado** na própria medida. Os **cinco** congelados (`session`,
`transport/mock`, `golden`, `preset` e o `main.rs` do CLI) foram cortados na
#257: as entradas de hoje são o **handle medido** daquele corte, com o motivo
escrito na própria linha.

| Arquivo | Teto | Por que |
|---|---|---|
| qualquer `.rs` de `packages/*/src` | **700** | teto genérico do Rust: mesmo critério do `components/` — dois assuntos no mesmo arquivo |
| `packages/core/src/session.rs` | **200** | **handle da FSM** desde a #257: só a `Session` (o que ela POSSUI) e o ciclo de vida (`new`/`into_transport`); método novo vai para o módulo do assunto |
| `packages/core/src/session/<módulo>.rs` | **400** | coorte do `session` (#262): cada módulo é UM assunto (a transação, o boot, a escrita…); o número sai da medida do corte — ver a nota do mapa, abaixo |
| `packages/core/src/transport/mock.rs` | **350** | **handle** desde a #257: o que o `MockDevice` POSSUI (estado, dicionário, inbox, plano de falha) e `new`/`with_slot_sem_nome`/`state` — mais o `mod tests` inline, que não pode virar arquivo (um `mod tests` em `mock/tests.rs` é o `clippy::module_inception`, que a CI reprova) |
| `packages/cli/src/main.rs` | **800** | **handle** desde a #257: o USO, os CÓDIGOS de saída (`EXIT_*`), os TIPOS (`Command`/`Args`/`UsageError`) e o `main` — mais o `mod tests` inline (o corpo re-indentado o `check_move_only` lê como perdido, e o bloco intacto em `tests.rs` é o `clippy::module_inception`, o mesmo caso do `mock`) |
| `packages/core/src/preset.rs` | **175** | **handle** desde a #257: o MODELO (os TIPOS e o que cada um POSSUI, com os campos fechados: `Document`, `Element` e o `LineBreak`), a EMISSÃO (`to_bytes`, a projeção dos próprios campos) e os acessores (`root`/`root_mut`/`preset_info`/`presets`); o `shape`, o helper de erro que todos os módulos usam |
| `packages/core/src/golden.rs` | **250** | **handle** desde a #257: o que o `GoldenFile` POSSUI (`meta`, `templates`, `by_out`, `by_in`) e a carga/validação (`from_json`/`embedded`); o `GOLDEN_JSON` e o `shape`, o helper de erro que todos os módulos usam |
| `tests.rs`, `tests/` | — | **fora do orçamento**: cresce com a COBERTURA, não com funcionalidade — medir isso só ensinaria a escrever menos teste |

> **Teto declarado** é decisão, não alvo: o número é a medida daquele dia — no
> `congelado` da #248 era exatamente o tamanho do arquivo que já passava do
> genérico, e depois dos cortes (#243 no crate do app, #257 no core e no CLI) é o
> HANDLE medido. Quem precisa de linha nova corta — ou traz a decisão de subir o
> número, com a issue que assume o corte.

Mapa do `session` (**#257**, primeiro corte dos congelados) — `session.rs` é o
handle, não o todo:

| Módulo | Teto | Dono de |
|---|---|---|
| `session.rs` | 200 | a `Session` (o que a FSM possui: `transport`, `current_pp`, `pps`, `backlog`, `pages`) e `new`/`set_inventory`/`inventory`/`current_pp`/`into_transport` |
| `session/escrita.rs` | **400** | o que MANDA byte: select (+trava de faixa), knob (D4), save (D3) + `save_frames`, IR (§13.7), SnapTone (§5) |
| `session/boot.rs` | **400** | o script §13.10 (boot + scan) e o `progress` observacional |
| `session/transacao.rs` | **400** | D1/D5/D6/D7: montar o pedido, esperar no endpoint certo, validar a FORMA (`tx_req*`, `wait_for*`, `Forma`) |
| `session/tipos.rs` | **400** | os DTOs e constantes que atravessam a fronteira |
| `session/paginas.rs` | **400** | a família 13xx: ler do fio e cachear (`state_page`, `scan_state`, `preset_state`) |
| `session/inventario.rs` | **400** | o espaço de pps do aparelho (§13.4) |
| `session/globais.rs` | **400** | os registradores da §13.13 (a família fora do golden) |
| `session/leitura.rs` | **400** | o que só PERGUNTA: User IRs, `read_state5`, backlog de pushes |

O **400** é medida, não escolha (#262): no corte, o maior assunto legítimo
respondia por 347 linhas (as cinco operações de escrita), e a coorte do actor é
**300** sobre o maior módulo dela — a mesma folga relativa. Os números de hoje não
mudam a regra: passou de 400, o módulo virou dois (o primeiro candidato é separar
IR/SnapTone da `escrita`).

> O caminho público não mudou: tudo continua `crate::session::<Item>` (os
> `pub use` do handle são o contrato), então o corte é invisível para `cli`,
> `library` e o crate do app. A prova é a bateria do workspace — **349 passed /
> 0 failed / 1 ignorado** antes e depois do corte, com a lista de nomes
> `diff`-idêntica (a medição é da #257; o número do crate sozinho é menor).

Mapa do `transport/mock` (**#257**, segundo corte dos congelados) — `mock.rs` é o
handle, não o todo:

| Módulo | Dono de |
|---|---|
| `mock.rs` | o `MockDevice` (os CAMPOS que ele possui: `state`, `dict`, `inbox`, `fault`, `latency`, contadores) e `new`/`with_slot_sem_nome`/`state`; `embedded_preset`, `shape_err` e as constantes embedadas |
| `mock/despacho.rs` | a porta do FIO (`impl DeviceTransport`: `open`/`close`/`send_raw`/`recv_raw`, com a latência de ida/volta e a morte que precede o parse) e o despacho do envelope §13 (`ingest`, D1/D3/D5). Os dois no mesmo módulo porque `ingest` é privado dele — de fora só se entra pela porta |
| `mock/estado.rs` | o ESTADO (`MockState`, derivado de `all.prst`) e os defaults globais |
| `mock/falha.rs` | o plano de falha do test double (`MockFault::{DieAfter,UsbFlaky}`) e o jitter determinístico (LCG de semente fixa) |
| `mock/fila.rs` | a fila IN por endpoint (D1/D7): `drain_inbox` e os `queue_push*` |
| `mock/familia.rs` | o segundo framing do mock: a família GP-50/SnapTone-NAM (§5) e o seu ACK por bloco |
| `mock/respostas.rs` | do template do golden para a RESPOSTA no fio (o fill indexado, as vars e o envelope) |

> **O que a `clippy` decide na estrutura.** O `mod tests` fica INLINE no handle, e
> não em `mock/tests.rs`: um módulo `tests` dentro de um arquivo `tests.rs` é o
> `clippy::module_inception` (o `-D warnings` da CI reprova), e mover o bloco para
> um arquivo de nome diferente quebraria o `use super::*` dos testes. O teto de
> 350 conta esse bloco, que a doutrina do gate preferiria fora do orçamento.
> A `impl DeviceTransport` fica no `despacho.rs` pelo motivo simétrico: `ingest`
> nasceu método privado do `impl MockDevice` do `mock.rs`, e o corte não pode
> reescrever a assinatura — `pub(super) fn ingest(...)` passaria dos 100 colunas e
> o `rustfmt` teria de REEXPRESSAR a linha movida, o que a cláusula (b) do
> `check_move_only` recusa (a linha reescrita sai da origem e aparece como
> PERDIDA). Com a porta no mesmo módulo, `ingest` continua privado e a linha
> assinada sai byte a byte.

> O caminho público não mudou: tudo continua `crate::transport::mock::<Item>`
> (`MockDevice`, `MockState`, `MockFault` — os `pub use` do handle são o
> contrato), então o corte é invisível para `cli`, `library` e o crate do app. A
> prova é a bateria do workspace — **349 passed / 0 failed / 1 ignorado** antes e
> depois do corte, com as contagens por binário `diff`-idênticas, mais o `fmt
> --check` e o `clippy -D warnings` do workspace.

Mapa do `golden` (**#257**, terceiro corte dos congelados) — `golden.rs` é o
handle, não o todo:

| Módulo | Dono de |
|---|---|
| `golden.rs` | o `GoldenFile` (os CAMPOS que ele possui: `meta`, `templates` e os dois índices por endpoint) e a CARGA/validação (`from_json`/`embedded`) mais `meta`/`templates`; o `GOLDEN_JSON` e o `shape` (o helper de erro que todos os módulos usam) |
| `golden/fio.rs` | os BYTES do fio: `hex_decode` (hex ASCII → bytes) e `decode_envelope` (o SysEx completo em `(func, addr, data)`) |
| `golden/padrao.rs` | o PADRÃO de payload (`Pattern`/`PatternKind`/`Segment`/`SegmentKind`): casar, extrair as vars e construir |
| `golden/template.rs` | a TRANSAÇÃO (`Template`/`Endpoint`/`Example`): o par OUT/IN e a materialização dele no fio (`build_request`, `matches_response`, `build_response`) |
| `golden/despacho.rs` | o DESPACHO por endpoint: o índice `(func, addr)` e a escolha do template (`request_template`, `for_response`/`for_request`, `match_*`) |

> **O que a privacidade decide na estrutura.** Este corte não precisou de um único
> `pub(super)`: o `shape` (o helper de erro) é privado do handle e os módulos o
> alcançam por `super::shape`, e o único privado que cruzaria fronteira —
> `addr_hex`, usado só nas mensagens de erro do despacho — foi para o
> `despacho.rs`, junto do seu único chamador. A linha da origem
> `use crate::{ProtocolError, SYSEX_EOX, SYSEX_HEADER};` saiu **intacta** para o
> `template.rs` (o único módulo que usa os três); o handle fica com a versão curta,
> que é andaime — e `use` é andaime por definição do gate, não linha de item.

> **O preço declarado do byte-a-byte.** Três links intra-doc cruzavam módulo e
> deixam de resolver: `[GoldenFile::request_template]` e
> `[Template::matches_shape]` (no `padrao.rs`) e `[decode_envelope]` (no
> `despacho.rs`). O corte não pode REEXPRESSAR a linha do doc (a cláusula (b) do
> `check_move_only` a recusa como conteúdo novo) nem acrescentar o `use` que a
> faria resolver (import usado só por doc-link é `unused_imports` no `-D warnings`
> da CI). Re-apontar um link é conteúdo, não andaime: fica para um PR fora do
> `Move-only` — `cargo doc` não é gate da CI, e o crate já gerava 51 avisos
> desses antes deste corte.

> O caminho público não mudou: tudo continua `crate::golden::<Item>` (`GoldenFile`,
> `Template`, `Pattern`, `PatternKind`, `Segment`, `SegmentKind`, `Endpoint`,
> `Example`, `hex_decode`, `decode_envelope`, `GOLDEN_JSON` — os `pub use` do
> handle são o contrato), então o corte é invisível para `codec`, `session`,
> `transport::mock`, `wire_log`, `cli` e os testes. A prova é a bateria do
> workspace — **349 passed / 0 failed / 1 ignorado** antes e depois do corte, com
> as linhas de `test result` `diff`-idênticas contagem a contagem, mais o `fmt
> --check` e o `clippy --workspace --all-targets -- -D warnings` limpos.

Mapa do `preset` (**#257**, quarto corte dos congelados) — `preset.rs` é o
handle, não o todo:

| Módulo | Dono de |
|---|---|
| `preset.rs` | o MODELO (os TIPOS e o que cada um POSSUI, com os campos FECHADOS: `Document` com `decl`/`root`/`trailing`, `Element` com os campos de layout e o `LineBreak`), a EMISSÃO (`to_bytes`, a projeção dos próprios campos) e os acessores (`root`/`root_mut`/`preset_info`/`presets`); o `shape` (o helper de erro que todos os módulos usam) |
| `preset/elemento.rs` | o COMPORTAMENTO do `Element` — os acessores, o writer e o `contains_raw_ampersand` — e o `escape_value`; o `Attr` |
| `preset/leitura.rs` | a LEITURA estrita do dialecto: o `Parser` (o cursor) e o `Document::parse`, que o dirige |
| `preset/pp.rs` | o espaço de `pp`: o DECIMAL do documento (`ppID`) × o banco/slot do FIO |
| `preset/view.rs` | as VIEWS tipadas do vocabulário `pp*`/`Effect`/`params_0..14` (§13.9) |
| `preset/instantaneo.rs` | o INSTANTÂNEO do layout (`Quebra`/`ElementSnapshot`/`DocumentSnapshot`, o `snapshot` e o `from_snapshot`) e o `element_from`, que o reconstrói |
| `preset/recorte.rs` | o RECORTE de um preset de fábrica do arquivo inteiro (`apenas_preset`) |

> **O que a tolerância de visibilidade do gate decide na estrutura.** As
> DECLARAÇÕES ficam no handle, e é isso que mantém este corte em **três**
> `pub(super)` em vez de treze. Um campo privado de `preset` já é visível para o
> `preset` e toda a sua subárvore — que é exatamente quem o toca (`elemento` lê e
> escreve, `leitura` constrói o `Element`, `instantaneo` o reconstrói e o projeta,
> e o handle lê `root.pre_ws` e chama `root.write`) —, então nenhum campo abre, e o
> "Campos privados de propósito" que o doc do `Element` promete continua valendo.
> O que sobra qualificar são os dois casos que cruzam a fronteira DE VERDADE: o
> `fn write` (chamado pelo `to_bytes`, que fica no módulo PAI — privado de um filho
> não é visível para o pai) e o `el` das duas views (construído pelo `presets()` do
> handle). A alternativa foi MEDIDA e reprova no gate: com o `Element` declarado no
> `elemento` e os nove campos `pub(super)`, a tolerância de visibilidade do
> `check_move_only` é GULOSA — `    name: String,` casa com a variante
> `    pub name: String,`, que no destino é a linha do `ElementSnapshot` — e o "não
> perdeu" acusa quatro linhas perdidas (`name`, `self_closing`, `at_attr`,
> `cont_indent`).

> O caminho público não mudou: tudo continua `crate::preset::<Item>` (`Document`,
> `Element`, `Attr`, `PresetView`, `EffectView`, `Quebra`, `ElementSnapshot`,
> `DocumentSnapshot`, `escape_value`, `pp_id_decimal`, `indice_do_documento` — os
> `pub use` do handle são o contrato), então o corte é invisível para `preset_json`
> (que toca os campos públicos dos dois instantâneos), `pedalboard`, `tone_sheet`,
> `transport::mock`, `library::seed`, o crate do app e os testes. A prova é a
> bateria do workspace — **349 passed / 0 failed / 1 ignorado** antes e depois do
> corte, com as 42 linhas de `test result` formando o mesmo multiconjunto (a ordem
> troca porque o cargo roda os binários em paralelo), mais o `fmt --check` e o
> `clippy --workspace --all-targets -- -D warnings` limpos. E, ao contrário do
> `golden`, **nenhum link intra-doc quebra** neste corte: o `cargo doc` dá o mesmo
> conjunto de 54 avisos, linha a linha, antes e depois.

Mapa do `cli` (**#257**, quinto e ÚLTIMO dos congelados) — `main.rs` é o handle,
não o todo (no bin, um `mod x;` do crate root procura `src/x.rs`, e não
`src/main/`: os dois módulos são IRMÃOS do handle em `packages/cli/src/`):

| Módulo | Dono de |
|---|---|
| `main.rs` | o USO (`USAGE`), os CÓDIGOS de saída (`EXIT_POLICY_BLOCKED`/`EXIT_PROTOCOL_ERROR`), os TIPOS que o parser constrói e a execução lê (`Command`, `Args`, `UsageError`) e o ciclo de vida do bin (`main`); o `mod tests` inline |
| `args.rs` | o PARSER: a superfície (subcomando e flags) e a POLÍTICA do `--real` — o `parse_args` e os três `parse_u*` |
| `execucao.rs` | a EXECUÇÃO: o despacho do `run` e os três fluxos de escrita (`set-param`, `save`, `upload-ir`) com a trava do ADR-5, mais o `fail` (erro de protocolo → exit 3) e o `escrita_verificada` (a trava de compilação) |

> **O que a `clippy` decide na estrutura — e o teto que ela força a declarar.** O
> `mod tests` fica INLINE, como no `mock`, e as duas saídas foram MEDIDAS: mover o
> corpo para `src/tests.rs` obriga a re-indentar as ~460 linhas do bloco (o
> `rustfmt` reindenta, `fmt --check` é gate da CI — e a linha re-indentada o
> `check_move_only` lê como PERDIDA), e o bloco intacto dentro de `src/tests.rs` é
> o `clippy::module_inception` (um `mod tests` declarado num arquivo `tests.rs`; o
> `-D warnings` da CI reprova). É por isso que o teto deste arquivo (**800**) fica
> acima do genérico — depois do `commands.rs` (900, dívida declarada da #243) é o
> único caso: 759 linhas são o handle mais o bloco de testes que a doutrina do
> gate preferiria fora do orçamento.

> **As DECLARAÇÕES ficam no handle** (`Command`, `Args`, `UsageError`, `USAGE` e os
> `EXIT_*`), como no corte do `preset`: um campo privado do crate root já é visível
> para o crate root e toda a sua subárvore — que é exatamente quem o toca (`args`
> constrói o `Args`, `execucao` lê os campos) —, então nenhum campo abre, e o corte
> fica em CINCO `pub(super)`: o `parse_args` e o `run` (chamados pelo `main`) e os
> três que só os TESTES do handle chamam (`run_set_param`, `run_save`,
> `escrita_verificada`).

> O corte é invisível — e, sendo o CLI um **binário**, não há caminho público a
> preservar: nem o nome dos testes mudou. A prova é a bateria do workspace, no
> `diff` mais forte da série: **349 passed / 0 failed / 1 ignorado** antes (medido
> num `git worktree` em `HEAD`) e depois, com a lista de **350 nomes de teste
> idêntica** — incluindo o caminho do módulo: os 23 do CLI continuam
> `tests::<nome>` — e as 42 linhas de `test result` iguais contagem a contagem, na
> MESMA ordem; mais o `fmt --check` e o `clippy --workspace --all-targets --
> -D warnings` limpos.

#### No crate do app (`packages/app/api`) — desde a #243

| Módulo | Teto | Por que |
|---|---|---|
| `src/actor.rs` | **600** | é o HANDLE (a API que a thread do Tauri usa) + o spawn: coorte que couber aqui é responsabilidade no lugar errado |
| `src/actor/<coorte>.rs` | **300** | cada coorte é `Request` → `Session` de UM assunto (leitura/escrita/boot/diagnóstico) ou uma peça do relógio; acima disso, o assunto virou dois |
| `src/commands.rs` | 900 | porta do IPC: cresce por operação exposta, e o corte por coorte é a mesma receita da #243 (**dívida declarada**) |
| `src/lib.rs` | 400 | `run()`, `AppState` e o registro do `generate_handler!` |
| `src/actor/tests.rs` | — | **fora do orçamento**: cresce com a cobertura (um teste por contrato do actor), não com funcionalidade |

Mapa do actor (#243) — `actor.rs` é a fachada, não o todo:

| Módulo | Dono de |
|---|---|
| `actor.rs` | handle clonável + spawn (`spawn`, `desligado`, `desligado_procurando`) e os métodos que os commands chamam |
| `actor/estado.rs` | o que a thread POSSUI (`Estado`, caches do scan, relógio da convergência) e os auxiliares `device_snapshot`/`emite`/`marca` |
| `actor/relogio.rs` | o loop com prazos (debounce + refresh), o read-back e o roteamento das `Request` |
| `actor/fila.rs` | `Request` — o contrato da fila |
| `actor/tipos.rs` | os DTOs do IPC (snapshot, sync, biblioteca) |
| `actor/backends.rs` | a trait do transporte e real/mock/`Desligado`/`ProcuraAparelho` |
| `actor/{leitura,escrita,boot,diagnostico}.rs` | as coortes de `Request` → `Session` |
| `actor/tests.rs` | os contratos do actor (unitário em `src/` — regra da `rust-practices`) |

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
