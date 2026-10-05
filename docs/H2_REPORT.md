# 🧾 H2_REPORT — Relatório do gate H2 (escrita real dos 3 fluxos)

> **Status:** 📝 template · preencher durante/imediatamente após a sessão
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
| F1 set-param | set-param (knob, §13.11) | 1 OUT · 0 IN | ____ | ______ |
| F2 save | save (§13.12) | 9 OUT na ordem · 0 IN | ____ | ______ |
| F3 upload-ir | upload-ir (§13.7) | 1 BEGIN · N chunks · N ACKs · dup final | ____ | ______ |

> Saída `1` do juiz = **PARAR** (§7 do checklist). Preserve o log e a saída
> antes de desligar, e classifique a divergência: o device respondeu diferente
> do §13 é **protocolo** (fluxo R3); respondeu o §13 mas o efeito não foi o
> esperado é **comportamento** e vira issue nova (R1: não se conserta mexendo
> no codec).

## 1. Identificação da sessão

| Campo | Valor |
|---|---|
| Data/hora | ____ |
| Commit do repo no binário | ____ |
| sha256 do `gp100-cli.exe` | ____ |
| Comando de build | `cargo build --release -p gp100-cli --features real-device,write-verified` |
| H1 anterior | arquivado em ____ (data) |
| Device (firmware/display) | GP-100 V2.1 · pp no display: ____ |
| Slot de IR escolhido | ____ · **estava vazio?** ____ |
| Preset alvo do `save` | ____ · **é descartável?** ____ |
| Knob alvo do `set-param` | slot da cadeia ____ · code ____ · ctrl ____ |

## 2. Pré-requisitos verificados

- [ ] `gp100-cli --help` cita `write-verified` e mostra `upload-ir`
- [ ] `set-param` **sem** `--dry-run` foi recusado pela trava (e não enviou)
- [ ] H1 arquivado antes desta sessão
- [ ] Suite oficial fechado durante a sessão toda
- [ ] Inventário de IRs anotado **antes** de escrever

## 3. Verificação por fluxo

> Um bloco por fluxo. **Veredito é uma palavra** (`ok` · `não` · `anômalo`),
> não uma adjetivação. Se der para escrever "pareceu", não veredicou.

### 3.1 `set-param` (knob, §13.11)

| | Valor |
|---|---|
| Valor no display ANTES | ____ |
| Valor pedido | ____ |
| Valor no display DEPOIS | ____ |
| Veredito | ____ |
| Log | `f1_setparam.jsonl` · nº de frames OUT: ____ |

Observação: o fluxo não tem read-back no fio (D4). A conferência é visual.
Anote se o display reaguiu com latência perceptível.

> AVISO (Antes da #22): este fluxo NAO escrevia. O `set-param` imprimia o frame e
> anunciava "(nada enviado - dry-run)" mesmo SEM o `--dry-run`, porque nao chamava
> a `Session` -- o `--log` ficava com 0 bytes. Se a sua sessao foi rodada com um
> binario anterior, o log vazio NAO prova que o knob nao girou: prova que o comando
> nao mandou nada. Confira `wc -l f1_setparam.jsonl`.

### 3.2 `save` (§13.12)

| | Valor |
|---|---|
| Nome no display ANTES | ____ |
| Nome pedido (≤12 chars) | ____ |
| pp / pp-type | ____ / ____ |
| Nome no display DEPOIS | ____ |
| Persiste depois de sair e voltar? | ____ |
| Veredito | ____ |
| Log | `f2_save.jsonl` · frames OUT: ____ (esperado 9) · IN: ____ (esperado 0) |

Observação: burst IN tardio (`11000008`/`12000001`) depois da operação é o
quirk de fim de sessão (§13.7) — **não** é confirmação de save (D3).

> ⚠️ **Antes da #22 este fluxo também não escrevia** (mesmo defeito do §3.1, e
> é o pior dos dois: o `save` é a escrita que PERSISTE). O esperado era 9
> frames OUT; um log com menos é o defeito, não um preset que não gravou.

### 3.3 `upload-ir` (§13.7)

| | Valor |
|---|---|
| Slot alvo (vazio?) | ____ |
| Blob: caminho / bytes | ____ / ____ (múltiplo de 15? ____) |
| Chunks / ACKs reportados | ____ / ____ |
| Nome do slot no display | ____ |
| `list-user-irs` depois: nome do slot | ____ |
| Veredito | ____ |
| Log | `f3_upload.jsonl` · frames OUT: ____ |

## 4. Divergências

> Se algo não bateu: **qual** dos dois tipos, porque o caminho é outro.

| # | Fluxo | Endereço (func/addr) | Tipo | Esperado (§13) | Obtido | Severidade |
|---|---|---|---|---|---|---|
| 1 | | | protocolo / comportamento | | | bloqueia-H2 / info |

- **protocolo** (o device respondeu diferente do §13) → fluxo R3 do
  [H1_CHECKLIST.md](H1_CHECKLIST.md) §6
- **comportamento** (respondeu o §13, o efeito não foi o esperado) → nova
  issue; **não** se corrige mexendo no codec (R1)

Anomalia do device durante a sessão (travamento, reboot, display incoerente)?
____

## 5. Fechamento

- [ ] **H2 OK** — os 3 fluxos com veredito `ok` e display confirmando
- [ ] **H2 parcial** — registre quais fluxos passaram; o que passou NÃO
      habilita o que falhou
- [ ] **H2 BLOQUEADO** — nenhuma escrita adicional até a causa ser tratada

Decisão do owner sobre o padrão do binário de campo: manter `write-verified`
fora do padrão (H1 segue seguro por construção) ou promovê-lo a padrão?
____

## 6. Pistas que valem para o H3

O H3 (baseline v1.1) usa os logs desta sessão. Anote aqui o que não é
byte-a-byte:

- o pacing do `save` (os 9 frames saem sem espera; a S4 mediu op0 em +578 ms
  e op1 em +593 ms — se o device aceitar sem erro, isso é achado)
- burst IN de fim de sessão presente? ____
- algum `ACK` de chunk do upload fora do padrão? ____
- a leitura de página depois do `set-param` mostra o valor novo? ____
  (o §13.11 é fire-and-forget, mas o **estado** deveria refletir)