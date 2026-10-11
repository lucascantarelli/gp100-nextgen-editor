# 🧭 H4_CHECKLIST — Gate de hardware: as páginas 13xx em campo (nome, palco, knob)

> **Status:** ⏳ aguardando a pedaleira + owner · **Criado:** 2026-10-09 · **Issues:** [#155](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/155) (esta sessão) · [#152](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/152) (o negativo que ela confirma em campo)
> · **Responsáveis:** owner no hardware
>
> ⚠️ **Este gate é IRMÃO do H1, não do H2.** Tudo aqui é **leitura**: o app
> observa o que o aparelho tem. A única escrita da sessão é **feita no próprio
> pedal, pela mão do owner** (renomear um patch, girar um knob) — o app só lê
> depois. Por isso este gate **não** exige `--features write-verified`, e não
> deve ser rodado com ela.
>
> O que este gate fecha: os critérios de #155 que **só** o aparelho prova
> (`§10 — edição no hardware reflete no app`) e a prova em campo do negativo de
> #152 (a meta6 **não** guarda nomes).

---

## 0. O que já está provado sem hardware — e o que sobra

O decode das 9 páginas, o nome em pg0/offset 2 (198/198 contra o `all.prst`), o
`BoardView` reconstruído dos offsets e o cache das páginas no boot foram
entregues e estão em `develop` (`d131b5a`, PR #158). O `validate_state_pages.py`
sai **0**. **Nada disso precisa ser refeito aqui.**

O que sobra é a parte que a captura S1 **não pode** responder, porque ela é do
Suite escaneando um aparelho intocado:

| # | O que só o campo mede | Por que a captura não responde |
|---|---|---|
| 1 | **O nome no app é o nome GRAVADO no pedal** (não o do `all.prst`) | a S1 varre os 198 pps de fábrica: os nomes das páginas **coincidem** com o `all.prst` — coincidir não prova ser a mesma fonte |
| 2 | **O palco desenha o conteúdo vivo** do patch editado | o aparelho da S1 não tinha edição pendente; com o default de fábrica o cache e o artefato são indistinguíveis |
| 3 | **O mapa knob ↔ offset** (§2 do plano da #155) | exigiria adivinhar **qual** knob foi girado; a S1 só mostra os valores de fábrica |
| 4 | **A meta6 é constante mesmo com o nome mudado** (#152) | idem: sem renomear, "constante em 198/198" ainda admite "constante porque nada mudou" |

O item 4 é o que dá a este gate o status de **prova em campo de um resultado
negativo** — não de uma funcionalidade nova.

---

## 1. Pré-requisitos (TODOS antes de ligar a pedaleira)

**Software (na máquina do field):**
- [ ] Build de **leitura**: `tauri build --features real-device` — **sem**
      `write-verified`. O badge do `FieldDiagPanel` tem de dizer `real`.
- [ ] `uv run pytest -q` verde · `cargo test --workspace` verde
- [ ] `uv run python analysis/validate_state_pages.py` → **exit 0**
- [ ] `uv run python analysis/validate_golden.py` → **2299/2299** (baseline
      intocada — se este número mudou, **parar**: o gate não é a causa)

**Com o device ligado:**
- [ ] Suite oficial **fechado** (occupancy de MIDI — armadilha do `knowledge.md`)
- [ ] O log de fio automático ligou: o `stderr` mostra
      `[log] wire log de campo: <caminho>` e o painel de diagnóstico mostra
      **o mesmo caminho** (`device_log_path`). **Anote o caminho agora**, antes
      de qualquer coisa — é a evidência da sessão.
- [ ] Um patch **descartável** disponível para renomear. A renomeação é feita
      **no pedal**, pelo owner; nada aqui escreve pelo app.

---

## 2. O roteiro, em 4 sessões

> Uma por vez, com o **log preservado** entre elas — o log é um arquivo **por
> execução** do app, então cada sessão nova gera um `.jsonl` novo. Não reinicie
> o app no meio de uma sessão: o arquivo daquela execução para de crescer.

### Sessão 1 — o palco e os nomes vêm do aparelho (leitura pura)

O critério é a **discordância**: para o app provar que lê o pedal, ele tem de
mostrar **a mesma coisa que o display do pedal**, inclusive quando isso **difere**
do `all.prst`.

- [ ] Boot pelo painel de diagnóstico. Anote o total de transações do relatório:
      no aparelho tem de ser **2299** (inventário da captura, ADR-12) — **2297**
      significa que o inventário do mock vazou para o build de campo.
- [ ] Abra um preset e compare, lado a lado, **o display do pedal** e **o nome no
      navbar/biblioteca**. Têm de bater. Anote 3 presets de fábrica (ex.: `P01`,
      `P50`, `P99`) com os dois nomes.
- [ ] **A prova que importa:** o palco desenha a cadeia. Escolha um preset cuja
      cadeia **difere** da do `all.prst` (20 dos 99 têm a cadeia trocada —
      `presetChains.ts` lista quais) e confirme que o palco mostra a do **pedal**.
- [ ] Se algum nome vier **vazio**: é o comportamento honesto (sem cache = vazio,
      nunca o dicionário de fábrica emprestado) e significa que **aquele** pp não
      entrou no scan. Anote o pp.
      **Cenário variante (#177, medido no H4):** o device real pode ter slot
      **sem nome** (pg0 zerada) ou com **rabo stale após o NUL** (o device não
      zera o resto do nome antigo ao renomear). O decode aceita os dois: nome
      vazio = slot honesto em branco; rabo stale = nome legítimo até o primeiro
      NUL (não é erro). Só recusa byte não-imprimível ANTES do NUL. Se vir esses
      casos, ANOTE o pp — são o teste real de que o app lê o hardware, não o
      `all.prst`.

### Sessão 2 — o nome vive nas páginas, não na meta6 (a prova em campo)

> ⚠️ **A renomeação é feita no pedal**, pela mão do owner. O app é só leitura.

- [ ] No pedal, **renomeie** o patch descartável para um nome que **não existe**
      em nenhum lugar do `all.prst` (evita coincidência). Anote o antes e o depois.
      Se o nome novo for MAIS CURTO que o antigo, o display/app pode mostrar um
      **rabo stale** (byte do nome antigo após o NUL) — é comportamento medido do
      device (#177), não bug; o nome legítimo é o prefixo até o primeiro NUL.
- [ ] Confirme no display do pedal que o nome gravou (SAVE persistiu).
- [ ] **Reinicie o app** (novo boot = novo scan = novo cache) e abra o pp.
      - [ ] O app mostra o **nome novo**? → critério §10 da #155 **fechado**.
      - [ ] O app mostra o **nome antigo**? → o cache não está sendo usado:
            **issue nova** com o log anexado (não se ajusta o decode às cegas).
- [ ] **A contraprova do #152, no mesmo log:** filtre as respostas de
      `13/13010001` do `.jsonl` desta sessão, para o pp renomeado.
      - [ ] O payload segue `<pp u16BE> 0c 1c 01 40`? → a meta6 **não** carrega o
            nome, confirmado por manipulação (o negativo deixa de ser estático).
      - [ ] O payload mudou? → a issue #152 **reabre** como folha de campo: a
            hipótese original volta a ser candidata e o corpo dela diz como.

### Sessão 3 — o mapa knob ↔ offset

- [ ] Com o palco de um pp aberto, **gire UM knob** no pedal (um só, e anote
      qual: slot + parâmetro + de/para).
- [ ] Sem reiniciar o app, refaça a leitura do pp (novo boot, mesmo pp).
- [ ] O `.jsonl` desta sessão tem as páginas antes e depois. O trabalho é
      **diff**: quais offsets de qual página (`0..8`) mudaram.
      - [ ] Exatamente **um** campo mudou e ele é **estável** entre os pps? →
            candidato a offset do knob; entra em `state_pages_offsets.json` com
            **prova-negativa** (guarda do #110), não só por ter casado uma vez.
      - [ ] Vários campos mudaram (o instrumento recalcula o preset)? → anote a
            lista; **não** promova nenhum a offset.
- [ ] Não mexer em `state_pages_offsets.json` na hora. O registro é a
      observação; o offset entra por PR, com o teste que o rejeita fora de lugar.

### Sessão 4 — entregar a evidência

- [ ] No painel: **copiar o caminho** do log e **abrir a pasta** (os dois botões
      só aparecem com o log ativo). Anexe o `.jsonl` das 3 sessões.
- [ ] `uv run python analysis/validate_state_pages.py` de novo → **exit 0**
- [ ] `uv run python analysis/validate_golden.py` → **2299/2299**
- [ ] `uv run pytest -q` · `cargo test --workspace` (o golden **não** muda nesta
      sessão: nenhuma captura nova entra no baseline — ADR R2/R3)
- [ ] Preencher `docs/H4_REPORT.md` e arquivar junto dos `.jsonl`

---

## 3. Critérios de saída (DoD desta sessão)

- [ ] Os nomes no app **batem com o display do pedal**, inclusive onde diferem do
      `all.prst` — e o pp renomeado **sem re-seed** aparece com o nome novo
      (o §10 da #155)
- [ ] O palco, com backend `real`, desenha a cadeia **do pedal** (não a do
      artefato) e **não** a recusa de cache ausente
- [ ] A meta6 medida **depois** da renomeação: constante → #152 fica fechada como
      negativa confirmada em campo; mudou → #152 reabre (e o registro vai na
      própria issue)
- [ ] Mapa knob ↔ offset: **um** campo estável com prova-negativa **ou** o
      registro honesto de que o diff foi ambíguo (e o knob segue sem offset)
- [ ] `.jsonl` do app anexado, juízes verdes, `H4_REPORT.md` arquivado
- [ ] Nada foi escrito no aparelho **pelo app** (o gate é leitura; a escrita do
      menu GLOBAL segue atrás do ADR-5)

### Se algum fluxo falhar

- [ ] **PARAR.** Sem repetir às cegas.
- [ ] Guardar o `.jsonl` **antes** de desligar o pedal — é a única evidência.
- [ ] Classificar, como no H3:
  - **protocolo** (o pedal respondeu diferente do §13) → fluxo R3 do
    [H1_CHECKLIST.md](H1_CHECKLIST.md) §6
  - **comportamento** (respondeu o §13 e o app mostrou outra coisa) → **issue
    nova**; não se corrige o decode mexendo no artefato de offsets (R1)
  - **especificação incompleta** (endereço/página sem template) → ADR novo +
    captura; template nasce de bytes observados, nunca de dedução

### Se o aparelho assertar

`sanitized` — o firmware cai num assert e para de responder até um power-cycle
físico (foi o que a [#110](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/110)
fechou para o knob). Neste gate **não se escreve pelo app**, então o risco
esperado é baixo; se acontecer, o `.jsonl` da execução **é** o diagnóstico.

---

## 4. Fontes

- [#155](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/155) — o plano, item §2 (captura dirigida) e os critérios de aceite
- [#152](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/152) — o negativo da meta6 e a condição de reabertura
- `docs/PROTOCOL.md` §13.10 — o layout das páginas 13xx, entregue no PR #158
- `packages/core/src/preset_pages.rs` · `packages/core/tests/preset_pages.rs`
- `analysis/state_pages_offsets.json` · `analysis/map_state_pages.py` · `analysis/validate_state_pages.py`
- [`docs/REAL_DEVICE_GAP.md`](REAL_DEVICE_GAP.md) §6 passo 7 (a sessão de campo) e §4b.1 (o log automático)
- [`docs/H1_CHECKLIST.md`](H1_CHECKLIST.md) (leitura, fluxo R3) ·
  [`docs/H3_CHECKLIST.md`](H3_CHECKLIST.md) (o irmão de baseline)
