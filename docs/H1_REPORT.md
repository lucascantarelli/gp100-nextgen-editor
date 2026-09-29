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

| Etapa | Rodou? | Framing (nível 1) | Estrutura (nível 3) | Observação |
|---|---|---|---|---|
| B1 `info` | | | | pp/nome conferidos com o display? |
| B2 `list-user-irs` | | | | 20 slots? eco da página? |
| B3 `dump-preset` pp corrente | | | | meta6/páginas 196B/fim 4B? |
| B4 dumps ×3 (meio/fim) | | | | |
| B5 boot completo (opcional) | | | | inventário do device anotado |

## 3. Resultado consolidado

- [ ] **H1 OK** — níveis 1 e 3 limpos em todas as etapas → ROADMAP: H1 ✅ com data
- [ ] **H1 com divergências de nível 2 (estado)** — protocolo OK; documentar §4
- [ ] **H1 BLOQUEADO por divergência de nível 1/3** — fluxo R3 executado (§5)

## 4. Log de divergência (1 linha por ocorrência — copiar do §5 do checklist)

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
