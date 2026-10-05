# 🧾 H1_REPORT — Relatório do gate H1 (preencher em campo)

> **Status:** 📝 template · **Preencher durante/imediatamente após a sessão** (Fase C do [H1_CHECKLIST.md](H1_CHECKLIST.md)) e arquivar como evidência da issue H1.
> **Complemento operacional:** `scripts/h1_field.sh` (ensaio `rehearsal` / roteiro `field`) + referências do mock em `analysis/h1_reference/`.

## 1. Identificação da sessão

| Campo | Valor |
|---|---|
| Data/hora | ____ |
| Build do CLI (sha256 do exe) | ____ |
| Commit do repo no binário | ____ |
| Device (firmware/display) | GP-100 V2.1 · pp no display: ____ |
| USB | porta direta? hub? driver enumerado? |
| Suite oficial fechado? | sim/não |
| Log bruto | `analysis/captures/sessionH1.jsonl` (sha256: ____) |

## 2. Execução (por etapa; repetição ×1 antes de classificar Timeout)

**Cole o veredito do juiz aqui** (`python3 scripts/h1_compare.py <dir> --markdown`).
As colunas de nível 1/3 abaixo sao as do juiz — não as recalcule na mão: se o
número divergir, o log mudou depois da comparação.

| Etapa | Rodou? | Framing (nível 1) | Estrutura (nível 3) | Observação |
|---|---|---|---|---|
| B1 `info` | | `sem-evidencia` | — | **não emite tráfego de fio**; pp/nome conferidos com o display? |
| B2 `list-user-irs` | | | | 20 slots? eco da página? |
| B3 `dump-preset` pp corrente | | | | meta6/páginas 196B/fim 4B? |
| B4 dumps ×3 (meio/fim) | | | | |
| B5 boot completo (opcional) | | | | inventário do device anotado |

Código de saída do juiz: `____` (0 = limpo · 1 = forma/framing → R3 · 3 = drift de
conteúdo, que em CAMPO é estado esperado).

## 3. Resultado consolidado

- [ ] **H1 OK** — níveis 1 e 3 limpos em todas as etapas → ROADMAP: H1 ✅ com data
- [ ] **H1 com divergências de nível 2 (estado)** — protocolo OK; documentar §4
- [ ] **H1 BLOQUEADO por divergência de nível 1/3** — fluxo R3 executado (§5)

## 4. Log de divergência (1 linha por ocorrência — colar a tabela que o juiz imprimiu)

> O juiz já separa nível 1/3 (`bloqueia-H1`, ação "fluxo R3") de nível 2
> (`estado`, ação "documentar"). Cole a tabela inteira; a coluna "Ação" já vem
> certaina, e um passo `sem-evidencia` **não** pode ser marcado como passou.

| # | Passo | Endpoint (func/addr) | Nível | Esperado (golden/mock) | Obtido (hex curto) | Hipótese | Severidade | Ação |
|---|---|---|---|---|---|---|---|---|
| 1 | | | | | | | | |

## 5. Fluxo R3 executado (só se houver `bloqueia-H1`)

1. Roteiro PARADO em: ____ (etapa)
2. Reprodução: nova leitura com `--log` / nova captura com o proxy winmm
   (arquivo: ____; anotado no `analysis/captures/`)
3. `build_golden.py` regenerado? ____ · `validate_golden.py` 100%? ____
4. Bump de baseline (5 lugares: §13, `GOLDEN_BASELINE_SHA`, `test_protocol.py`,
   DECISIONS/INDEX): ____ (novo sha: ____)
5. Código retomado (R1) e H1 re-executado: data ____

## 6. Estado divergente documentado (nível 2 — esperado por desenho)

| Item | Mock (all.prst) | Device real | Explicação |
|---|---|---|---|
| pp corrente/nome | It's GP100 @ 0x0000 | ____ | S4 gravou o estado ao vivo no device |
| IRs slots 0/1 | vazio (CRC de fábrica) | ____ | S2 subiu IRs reais |
| Outros | | | |

## 7. Plano de backup (executar SE a leitura divergir — decidido ANTES de ligar)

**Regra de ouro: divergência não vira improvisação em campo.** Em qualquer
`bloqueia-H1`, o plano é fixo:

1. **PARAR o roteiro** (nenhum passo adicional; nenhuma "tentativa rápida").
2. **Preservar a evidência:** copiar `sessionH1.jsonl` + saídas `.txt` + sha256
   do binário para `analysis/h1_field_<tag>/` (nunca apagar; log é append-only).
3. **Tentativa única de diagnóstico sem hardware** (no PC, com o log capturado):
   rodar os decoders (`decode_wire.py`) no log e comparar o FRAMING com as
   referências de `analysis/h1_reference/` — isola se o desvio é de ordem,
   endereço ou conteúdo (nível 1 vs 2 vs 3).
4. **Reprodução controlada (só se o owner autorizar nova conexão):** nova
   sessão `field` com o MESMO binário, UM passo (o que divergiu), log novo.
   Se precisar do comportamento do SUITE para comparar: captura com o proxy
   winmm (skill `capture-analyze`) — nunca testes ad-hoc de escrita.
5. **Encerrar a sessão de campo** e tratar via fluxo R3 no escritório:
   `build_golden` → `validate_golden` 100% → bump de baseline nos 5 lugares →
   código retomado (R1) → H1 re-executado do zero em outra oportunidade.
6. **Se o device não enumerar / OpenFailed persistente:** Suite fechado?,
   outro cabo/porta, Gerenciador de Dispositivos, reiniciar a pedaleira —
   3 tentativas e encerrar (não é divergência de protocolo, é logística).

**Critério de abortar a sessão inteira:** 2 etapas consecutivas com divergência
de nível 1/3, ou qualquer sintoma anômalo do device (travamento, reboot, display
incoerente) → desligar, preservar evidência, R3.

---

## 6. SESSÃO REAL EXECUTADA — 05/10/2026 (hardware ligado)

> **O que muda nesta seção:** o relatório acima era template. Esta é a
> **execução de verdade** contra um GP-100 ligado, e ela **não fecha a #21
> sozinha** — falta a conferência do display (§1.2) e o B5 opcional.

| | Valor |
|---|---|
| Data/hora | 2026-10-05 11:43 BRT |
| Binário | `gp100-cli.exe` compilado com `--features real-device` (H1) |
| Sessão | `analysis/h1_field_20261005_114300` |
| Frames de fio | **120** (20+20 no B2; 20 em cada um dos 3 dumps) |
| Relógio | `t` de 0.3 ms a 99.4 ms (o `--log` grava relógio desde o #23) |
| Suite oficial | fechado (só `ValetonUsbAudioCpl.exe`, que não ocupa MIDI) |

### Veredito do juiz

```
H1 com niveis 1 e 3 limpos em todos os passos com evidencia.
4 divergencia(s) de CONTENDO (nivel 2) e 1 passo(s) sem evidencia.
```

| Passo | Veredito | O que significa |
|---|---|---|
| B1 `info` | **SEM EVIDÊNCIA** | esperado por construção: `info --real` não emite tráfego de fio |
| B2 `list-user-irs` | **ESTADO** (nível 2) | framing e forma iguais; os bytes são o estado do aparelho |
| B3 `dump-preset 0x0000` | **ESTADO** (nível 2) | idem |
| B4 `dump-preset 0x0031` | **ESTADO** (nível 2) | idem |
| B4 `dump-preset 0x0062` | **ESTADO** (nível 2) | idem |

**Nenhuma divergência de nível 1 (framing) ou nível 3 (forma).** As 9 páginas
do `dump-preset` vieram com as formas do §13: `meta6` 6B, páginas 196B ×8,
a última 32B e o fim 4B. É a leitura real idêntica ao mock no que o protocolo
afirma — o resto é conteúdo de estado, que **é esperado** por desenho (o
aparelho não é o `all.prst`).

### O que o aparelho revelou (nível 2, para documentar)

- **Slots 0 e 1 têm IRs reais:** `test_ir_mono` e `test_ir_stereo` — é o
  upload da S2, e o §2 do checklist previa exatamente isso.
- **Slots 2–19 vazios.** Relevante para o H2: a #22 precisa de um slot vazio.

### Um achado operacional

A **primeira** tentativa de `list-user-irs` deu `timeout de 3000 ms`. A
segunda, sem mudar nada, respondeu. É o D6 do checklist (timeout se repete 1×
antes de classificar) e o runbook já faz esse retry sozinho — mas vale
registrar: **a primeira leitura depois de conectar o cabo é fácil de
confundir com falha de protocolo**, e o operador sem o retry teria aberto o
fluxo R3 à toa.

### O que ainda falta para fechar a #21

- [ ] Ler o **display** e comparar com o §1.2 (pp corrente, nome, tipo)
- [ ] Conferir o **B5** (boot completo) — opcional, e exige a feature do H2
      porque o keepalive `00020001` é escrita (ADR-5 rev. 04/10)
- [ ] Assinar o §1.1 (firma de quem estava com a pedaleira na mão)
