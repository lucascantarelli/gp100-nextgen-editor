# 🧾 H2_REPORT — Relatório do gate H2 (escrita real dos 3 fluxos)

> **Status:** ✅ **H2 OK — sessão real de 05/10/2026.** Os 3 fluxos passaram no fio **e** no aparelho; o preset `0x0000` foi sobrescrito com o nome `H2 TESTE` e persiste. Evidência versionada em [`analysis/h2/20261005_120024/`](../analysis/h2/20261005_120024/).
> ([H2_CHECKLIST.md](H2_CHECKLIST.md)) e arquivar como evidência da issue H2.
>
> ⚠️ Este relatório é sobre **alteração de estado de um aparelho do usuário**.
> Preencher "pareceu que funcionou" como se fosse "funcionou" é o modo de
> falha mais caro deste projeto: o preset gravado errado não volta sozinho.
>
> **A Fase C é automatizada (issue #22).** O juiz
> [`scripts/h2_compare.py`](../scripts/h2_compare.py) confere os invariantes do
> §13 nos 3 logs e imprime a tabela do §3.0 — inclusive o **valor que o display
> DEVERIA mostrar**, para o veredito abaixo ser uma comparação e não uma
> impressão. O que o fio não preenche é a última coluna, e é a única que
> decide o H2.

## 3.0 Fase C — o que o fio provou (cola a saída do juiz)

```bash
python3 scripts/h2_compare.py analysis/h2/<sessão> --markdown
```

| # | Fluxo (§13) | Invariante do fio | Obtido | Veredito do operador |
|---|---|---|---|---|
| F1 set-param | set-param (knob, §13.11) | 1 OUT · 0 IN | **conforme** (1 OUT) | **ok** |
| F2 save | save (§13.12) | 9 OUT na ordem · 0 IN | **conforme** (9 OUT) | **ok** |
| F3 upload-ir | upload-ir (§13.7) | 1 BEGIN · N chunks · N ACKs · dup final | **conforme** (297 OUT / 296 IN) | **ok** |

> Saída `1` do juiz = **PARAR** (§7 do checklist). Preserve o log e a saída
> antes de desligar, e classifique a divergência: o device respondeu diferente
> do §13 é **protocolo** (fluxo R3); respondeu o §13 mas o efeito não foi o
> esperado é **comportamento** e vira issue nova (R1: não se conserta mexendo
> no codec).

## 1. Identificação da sessão

| Campo | Valor |
|---|---|
| Data/hora | 05/10/2026, 12:00–12:25 |
| Commit do repo no binário | `ef3a1a3` (branch `feat/22-h2-kit`) |
| sha256 do `gp100-cli.exe` | `cab0d23c0ebd7169…` |
| Comando de build | `cargo build --release -p gp100-cli --features real-device,write-verified` |
| H1 anterior | [`analysis/captures/sessionH1/`](../analysis/captures/sessionH1/) (05/10/2026, entrada pelo PR #109) |
| Device (firmware/display) | GP-100 **V2.1** (o assert do firmware é dessa versão) · pp 0 |
| Slot de IR escolhido | **2** · **estava vazio?** sim (`(vazio)`) |
| Preset alvo do `save` | `0x0000` · **é descartável?** sim — o owner autorizou a sobrescrita na sessão |
| Knob alvo do `set-param` | slot da cadeia **3** (AMP) · code **0x0700006e** (Bog RedM) · ctrl **0** |

## 2. Pré-requisitos verificados

- [x] `gp100-cli --help` cita `write-verified` e mostra `upload-ir`
- [x] `set-param` **sem** `--dry-run` foi recusado pela trava (e não enviou)
- [x] H1 arquivado antes desta sessão
- [x] Suite oficial fechado durante a sessão toda
- [x] Inventário de IRs anotado **antes** de escrever — slots 0 e 1 ocupados (`test_ir_mono` / `test_ir_stereo`), 2–19 livres

## 3. Verificação por fluxo

> Um bloco por fluxo. **Veredito é uma palavra** (`ok` · `não` · `anômalo`),
> não uma adjetivação. Se der para escrever "pareceu", não veredicou.

### 3.1 `set-param` (knob, §13.11)

| | Valor |
|---|---|
| Valor no display ANTES | 99.0 |
| Valor pedido | **15.0** |
| Valor no display DEPOIS | **15.0** |
| Veredito | **ok** |
| Log | `verde/f1_setparam.jsonl` · nº de frames OUT: **1** (IN: 0, D4) |

Observação: o fluxo não tem read-back no fio (D4). A conferência é visual —
**e foi duplicada por máquina**: o `dump-preset` antes/depois mostra o `f32`
do knob mudando de **99.0 → 15.0** na página 0, com o CRC da página
recalculado (`ec 36 75 d0` → `98 17 3d 35`). Ver
[`decode_dump_diff.py`](../analysis/h2/20261005_120024/) —
reproduce com `python analysis/h2/20261005_120024/decode_dump_diff.py`.

> 🛑 **O PRIMEIRO F1 DESTA SESSÃO TRAVOU O APARELHO, E O CULPADO FOI O RUNBOOK.**
> A primeira versão mandava `set-param 3 0x0700006e 0 99.5`. O GP-100 **V2.1**
> caiu no assert do próprio firmware:
>
> ```
> CODE:para <= GetParaMaxVal(
> line 1828
> file: ..\..\Drivers\audio\audio.c
> Version: V2.1 page 1
> ```
>
> e parou de responder a **toda** transação (timeout de 3000 ms em
> `12/12001002` e `12/13010001`, 6+ tentativas, USB e MIDI enumerationados e
> `OK` no Windows) até um **power-cycle físico**. O preset ficou byte-a-byte
> igual ao anterior — o device não chegou a gravar.
>
> A causa: **99.5 é um valor inventado**, e estava acima do teto real. As 6
> amostras desse par exato (slot 3 / `0x0700006e` / ctrl 0) na captura da
> Suite vão de **15.0 a 99.0**. O runbook agora manda **15.0**, que é um valor
> que a Suite realmente enviou — e o aparelho continuou respondendo normal.

> AVISO (Antes da #22): este fluxo NAO escrevia. O `set-param` imprimia o frame e
> anunciava "(nada enviado - dry-run)" mesmo SEM o `--dry-run`, porque nao chamava
> a `Session` -- o `--log` ficava com 0 bytes. Se a sua sessao foi rodada com um
> binario anterior, o log vazio NAO prova que o knob nao girou: prova que o comando
> nao mandou nada. Confira `wc -l f1_setparam.jsonl`.

### 3.2 `save` (§13.12)

| | Valor |
|---|---|
| Nome no display ANTES | (nome anterior do pp 0) |
| Nome pedido (≤12 chars) | `H2 TESTE` |
| pp / pp-type | `0x0000` / **4** |
| Nome no display DEPOIS | **`H2 TESTE`** |
| Persiste depois de sair e voltar? | **sim** |
| Veredito | **ok** |
| Log | `verde/f2_save.jsonl` · frames OUT: **9** (esperado 9) · IN: **0** (esperado 0) |

Observação: burst IN tardio (`11000008`/`12000001`) depois da operação é o
quirk de fim de sessão (§13.7) — **não** é confirmação de save (D3).

> ⚠️ **Antes da #22 este fluxo também não escrevia** (mesmo defeito do §3.1, e
> é o pior dos dois: o `save` é a escrita que PERSISTE). O esperado era 9
> frames OUT; um log com menos é o defeito, não um preset que não gravou.

### 3.3 `upload-ir` (§13.7)

| | Valor |
|---|---|
| Slot alvo (vazio?) | **2** · **sim**, `(vazio)` no inventário antes da escrita |
| Blob: caminho / bytes | `analysis/captures/ir_slot0.bin` / **4425** (múltiplo de 15? **sim**, 295 × 15) |
| Chunks / ACKs reportados | **295** / **296** (o +1 é a duplicação do último chunk, §13.7) |
| Nome do slot no display | `test_ir_mono` |
| `list-user-irs` depois: nome do slot | **`test_ir_mono`** (era `(vazio)`) |
| Veredito | **ok** |
| Log | `verde/f3_upload.jsonl` · frames OUT: **297** · IN: **296** |

> Este é o único dos 3 fluxos com read-back **de verdade**: o ACK por chunk
> (§13.7) prova que o device aceitou cada pedaço, e a tabela de nomes confirma
> no level de usuário. O F3 é também o único dos 3 que **não** derrubou o
> aparelho — 297 frames de escrita e o GP-100 continuou atendendo leitura na hora seguinte.

## 4. Divergências

> Se algo não bateu: **qual** dos dois tipos, porque o caminho é outro.

| # | Fluxo | Endereço (func/addr) | Tipo | Esperado (§13) | Obtido | Severidade |
|---|---|---|---|---|---|---|
| 1 | **F1 com valor `99.5`** | `12/10030002` | **comportamento** | knob do AMP Gan para o valor pedido | **assert de firmware** `para <= GetParaMaxVal(` em `Drivers/audio/audio.c:1828`; device mudo para toda transação até power-cycle | **bloqueia-H2** — resolvido nos dois fronts: o runbook passou a `15.0` (o valor real da captura) **e** a [#110](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/110) agora recusa valor fora da faixa **antes do fio** (ADR-10) |
| 2 | F3 com o device já travado | `12/12001002` | protocolo | ACK por chunk | timeout de 3000 ms; **slot 2 continuou `(vazio)`**, ou seja, o chunk 0 nem chegou a ser gravado | info (efeito do #1) |
| 3 | 1ª `list-user-irs` após power-cycle | `12/12001002` | comportamento | resposta na 1ª tentativa | timeout; a **2ª tentativa, sem mudar nada, responde** | info — é o D6 já documentado. Vale como alerta: um único timeout **não** prova device travado |

- **protocolo** (o device respondeu diferente do §13) → fluxo R3 do
  [H1_CHECKLIST.md](H1_CHECKLIST.md) §6
- **comportamento** (respondeu o §13, o efeito não foi o esperado) → nova
  issue; **não** se corrige mexendo no codec (R1)

Anomalia do device durante a sessão (travamento, reboot, display incoerente)?

**Sim, e grave: o GP-100 travou duas vezes**, ambas por conta do F1 com o
valor `99.5`. Sintomas, na ordem:

1. a tela do aparelho passou a mostrar a tela de **assert do firmware**
   (texto transcrito na §3.1) e o `Version: V2.1` no rodapé;
2. toda transação seguinte tomou **timeout de 3000 ms**, inclusive leituras
   puras (`list-user-irs` em `12/12001002`, `dump-preset` em `12/13010001`);
3. o Windows continuava vendo o device normalmente — `VID_84EF&PID_0021`
   presente e `OK`, sem processo pendurado. **Não era cabo, driver nem porta**:
   era o firmware;
4. a **única** recuperação foi o **power-cycle físico** (desligar/religar o
   pedal ou tirar o USB). Um restart de software não é garantido —
   `pnputil /restart-device` exige elevação e não foi tentado com o device já
   travado;
5. depois do power-cycle o device voltou **sem perda**: os slots 0 e 1
   continuaram com seus nomes e o slot 2, que tinha sido grabado, manteve-se.

**Lição para o H3 e para qualquer escrita futura:** o valor de um `set-param`
não é um campo livre. Ele tem de sair de uma **captura real** do mesmo par
(cadeia + `effectCode` + ctrl). O codec hoje só recusa `slot` fora de 1..=9 e
passa qualquer `f32` — e o firmware responde a isso com um assert que **derruba
o aparelho inteiro**, não com um erro de protocolo recuperável.

> ⚠️ **Recomendação (issue nova, não corrigida aqui):** o `set_param_payload`
> deveria validar o valor contra uma tabela de faixas por `effectCode`, derivada
> de `analysis/fixtures/knobs.jsonl`. Hoje um `--help` que não avisa disso
> transforma um erro de digitação em tijolo no pedal de alguém.

## 5. Fechamento

- [x] **H2 OK** — os 3 fluxos com veredito `ok` e display confirmando
- [ ] **H2 parcial** — registre quais fluxos passaram; o que passou NÃO
      habilita o que falhou
- [ ] **H2 BLOQUEADO** — nenhuma escrita adicional até a causa ser tratada

> Ressalva honesta: o `H2 OK` acima é para os **3 fluxos** com os valores
> **corrigidos**. O gate, na primeira tentativa, derrubou o aparelho duas
> vezes. A correção está no mesmo PR (#22) e é ela que torna o kit seguro
> para o próximo operador — mas o achado precisa ser lido junto do verde.

Decisão do owner sobre o padrão do binário de campo: manter `write-verified`
fora do padrão (H1 segue seguro por construção) ou promovê-lo a padrão?

**Recomendação: manter fora do padrão.** O H2 mostrou que `write-verified`
destrava corretamente a escrita — mas que o **conteúdo** do que destrava ainda
não é seguro o bastante para ser o padrão (o `set-param` aceitou um valor fora
da faixa e derrubou o aparelho). Promover a feature antes de validar os valores
trocaria um gate que funciona por um que não funciona.

## 6. Pistas que valem para o H3

O H3 (baseline v1.1) usa os logs desta sessão. Anote aqui o que não é
byte-a-byte:

- o pacing do `save` (os 9 frames saem sem espera; a S4 mediu op0 em +578 ms
  e op1 em +593 ms — se o device aceitar sem erro, isso é achado)
- burst IN de fim de sessão presente? não — os IN do `save` foram 0, e a
  leitura seguinte respondeu normal
- algum `ACK` de chunk do upload fora do padrão? não — **296** ACKs para 295
  chunks + a duplicação do último, exatamente o §13.7, e nenhum `UnexpectedAck`
- a leitura de página depois do `set-param` mostra o valor novo? **sim, e este
  é o achado mais útil da sessão**: o §13.11 é fire-and-forget (D4), mas o
  **estado** reflete — `f32` 99.0 → 15.0 na página 0 com o CRC recalculado.
  Isso dá ao H3 um read-back de knob que o §13 não previa.
- `list-user-irs` é o único read-back de nome que existe hoje. O nome do
  **preset** (`11/11000000`) continua sem leitura no kit — o §3.2 teve de
  depender do display. Um `list-presets` seria o próximo fôlego do projeto.