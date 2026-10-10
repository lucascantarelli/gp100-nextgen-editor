# 🧭 H5_CHECKLIST — Gate de hardware da Era Hardware: boot, modelo tipado e sincronização

> **Status:** 📝 **rascunho** — este roteiro só roda depois de **#161** (boot gate),
> **#162** (FactoryPatch/UserPatch), **#164** (sync bilateral) e **#165** (aparelho por
> default) mergeadas em `develop`.
> **Criado:** 2026-10-09 · **Issue da sessão:** _a abrir (HW-11)_ · **Épico:** [#171](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/171)
> **Responsáveis:** owner no hardware · **Formato:** o mesmo dos relatórios H1–H4 (relatório
> assinado + evidência preservada).
>
> ⚠️ **Este gate é do APP, não do CLI, e não contorna a ADR-5.** Com o build de leitura
> (default), a escrita do app aparece travada na tela com o motivo; a Sessão 3 só executa
> escrita se o owner autorizar o binário `write-verified` **e** o roteiro for adaptado na
> hora com esse registro. Nada aqui escreve no boot.

---

## 0. O que este gate prova — e por que só o campo fecha

| # | Prova | Por que a suíte/mock não fecha |
|---|---|---|
| 1 | **Boot gate:** a casca só monta depois da leitura validada; falha vira erro legível (sem placeholder/hex na tela) | o mock sempre responde; o aparelho pode faltar, e o relatório pode vir incoerente |
| 2 | **Modelo tipado:** `FactoryPatch` × `UserPatch`, com nomes/efeitos/knobs **dinâmicos** (cada aparelho tem o SEU estado) | fixture não prova o estado do dono; renomear no pedal é o único vetor |
| 3 | **Default real por compilação:** o shell nativo sobe com `backend: "real"` sem flag | só o binário nativo com USB prova; navegador é UI-only por construção |
| 4 | **Convergência (sync):** escrita → releitura mostra o valor do APARELHO; queda/assert vira estado legível | o fio não tem read-back espontâneo (D4); o número é do campo |
| 5 | **Anti-brick:** nenhuma escrita no boot; valor fora de faixa recusado ANTES do fio | o assert do firmware (que exige power-cycle) só se evita no aparelho |

---

## 1. Pré-requisitos (TODOS antes de ligar a pedaleira)

**Software (na máquina do field):**
- [ ] `develop` com #161, #162, #164 e #165 mergeadas; `cargo test --workspace`,
      `pnpm exec vitest run` e `pnpm exec playwright test` verdes
- [ ] Build do app **de leitura**: `tauri build` (o aparelho já é default — #165). O badge da
      navbar **não** pode dizer `Mock Device`.
- [ ] `gp100-cli` de campo disponível com `--mock-device` (o log de fio é o MESMO schema P4
      das capturas — é ele que vai ao relatório)

**Com o device ligado:**
- [ ] Suite oficial **fechado** (occupancy de MIDI — armadilha do `knowledge.md`)
- [ ] O log de fio automático ligou: `stderr` mostra `[log] wire log de campo: <caminho>` e o
      painel de diagnóstico mostra **o mesmo caminho**. **Anote agora** — é a evidência da sessão.
- [ ] Um patch de **usuário** descartável (para renomear **no pedal**) e, se a escrita for
      autorizada, um segundo para salvar pelo app

---

## 2. O roteiro, em 4 sessões

> Uma sessão por vez, com o **log preservado** entre elas (o log é um arquivo por execução do
> app — não reinicie no meio de uma sessão).

### Sessão 1 — o boot é gate (leitura pura)

- [ ] Abra o app. A tela mostra **somente** navbar + loading durante o boot; nenhum painel do
      palco/biblioteca monta antes disso (o [#161](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/161) é o contrato).
- [ ] Ao concluir, o boot report mostra o inventário completo. Anote o número de transações —
      no aparelho é o inventário da captura (2299), nunca o do mock.
- [ ] Navegue por 3 presets de fábrica (`P01`, `P50`, `P99`): o **nome no app é o do display
      do pedal**, lado a lado.
- [ ] **Prova do estado real:** edite um knob **no pedal** (não no app), feche e reabra o
      preset no app — o valor mostrado é o do APARELHO.
- [ ] **Falha legível:** com o app aberto, desconecte o USB. O estado vira `off`/erro com
      motivo e ação; nada de tela mentindo conexão. Reconecte e use a ação de reconectar.
- [ ] **Sem escrita no boot:** o passo `boot` do log **não contém** frame mutante (conferir o
      `.jsonl`; é o negativo que a Sessão 4 fecha).

### Sessão 2 — fábrica × usuário e o dado DINÂMICO (leitura pura, edição no pedal)

- [ ] A biblioteca separa **fábrica** (`P##`, imutável) de **usuário** (`U##`, mutável) e o
      app **não** oferece escrita em fábrica.
- [ ] **A prova que importa (#162):** no pedal, **renomeie** o patch de usuário descartável
      para um nome que não existe em nenhum fixture. Reabra no app: o nome exibido é **o
      novo**, vindo do aparelho — nada de nome de exemplo/dicionário.
- [ ] Gire um knob desse patch **no pedal**; reabra: knobs/efeitos exibidos refletem o estado
      lido (dinâmico, por dono).
- [ ] Nenhuma string hexadecimal aparece na UI de patch (nome, categoria, knobs) — o modelo
      tipado é a fronteira.
- [ ] Se a leitura de um pp vier vazia, é o comportamento honesto (sem cache = vazio); anote
      qual e por quê.

### Sessão 3 — convergência e a política real (escrita só se autorizada)

> Sem autorização de escrita, esta sessão prova o **negativo**: botões desabilitados COM O
> MOTIVO na tela (build de leitura) e nenhum frame mutante no log.

- [ ] Com o build de leitura: knob/save/IR aparecem travados na tela com o motivo, e o log
      confirma **zero** frame mutante em qualquer interação.
- [ ] Se (e somente se) o owner autorizar o binário `write-verified`: gire um knob **pelo
      app**; o valor aplicado é relido do aparelho e a tela mostra o valor do aparelho quando
      os dois divergirem (reconciliação — #164). Anote o intervalo entre "pediu" e "aplicou".
- [ ] Desconecte no meio de uma operação: o estado cai para offline sem travar a UI e sem
      continuar escrevendo em vazio.
- [ ] O log de fio da sessão tem os frames da escrita e da releitura no MESMO schema P4.

### Sessão 4 — anti-brick e os negativos (a prova de que nada vaza)

- [ ] **Faixa:** tente (na UI ou no harness) um valor fora da faixa (ex.: `99.5` no ganho que
      já derrubou o aparelho — #110). O comando recusa ANTES do fio: log sem frame, erro
      legível; o aparelho **não** asserta.
- [ ] **Boot:** o log da abertura do app **não** contém `set-param`/`save`/`upload-ir`.
- [ ] **Fábrica:** nenhum caminho de escrita aceitou `FactoryPatch` durante a sessão.
- [ ] Ao fim: aparelho funcional (sem power-cycle necessário) — o veredito anti-brick da sessão.

---

## 3. Evidências da sessão (o que fica no relatório)

| Evidência | Onde |
|---|---|
| Log de fio por execução (boot, sessões, negativos) | caminho anotado no pré-requisito; cópia para o relatório |
| Veredito por item (1–5 da §0) | tabela do relatório, com o número medido (transações, intervalo) |
| Nomes/knobs observados no pedal × no app | tabela lado a lado, com data/hora |
| Build exato | `tauri build` (default) e features; commit/hash |

## 4. Assinatura

| Campo | Valor |
|---|---|
| Sessão | data · build (commit) |
| Itens 1–5 | ✅/❌ por item, com a evidência apontada |
| Veredito | fechado / aberto (o que falta e por quê) |
| Assinatura | owner (hardware) · agente (relatório) |

---

## 5. Referências

- [#161](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/161) boot gate ·
  [#162](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/162) modelo ·
  [#164](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/164) sync ·
  [#165](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/165) default real
- `docs/H1_REPORT.md` … `docs/H4_REPORT.md` (formato dos relatórios H) ·
  `docs/DECISIONS.md` ADR-5/ADR-10 · `docs/BLOCKERS.md` (#110)
