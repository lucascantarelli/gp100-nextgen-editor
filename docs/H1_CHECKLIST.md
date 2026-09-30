# 🎛️ H1_CHECKLIST — Gate de hardware: primeiro contato real (somente leitura)

> **Status:** ⏳ aguardando o transporte real (software da Fase M0 pronto: M0.7 ✅ e M0.8 ✅) · **Última revisão:** 2026-09-29 · **Responsáveis:** owner no hardware + skill `capture-analyze` para divergências
>
> **Kit de campo PRONTO (29/09):** build release do CLI · referências do mock em
> `analysis/h1_reference/` · runbook `scripts/h1_field.sh rehearsal|field` (ensaio
> executado, 0 divergências de framing) · relatório `docs/H1_REPORT.md` com o
> **plano de backup fixo** (§7) — decidido ANTES de ligar a pedaleira.
> **Fontes:** `docs/ROADMAP.md` (issue H1, regra R3), `docs/VISION.md` §7 (brick-proof), `docs/BLOCKERS.md` §4, ADR-3/ADR-4/ADR-6 (`docs/DECISIONS.md`), `docs/PROTOCOL.md` §13, `knowledge.md` (armadilhas de captura).
> **Regra deste gate:** **LER é seguro; ESCREVER é H2.** Nenhum byte de escrita sai
> nesta sessão (`set-param`/`save`/`upload-ir` proibidos MESMO com dry-run) e
> firmware update continua fora de escopo (política V2+).

---

## 1. Objetivo

Provar que o `gp100-core` lê o device REAL igual ao mock: as leituras de campo
casam com o golden (Baseline v1.0) **byte-a-byte no FRAMING**, e as divergências
de **CONTEÚDO** (estado do device) são esperadas e classificadas — não são falha
de protocolo. DoD do ROADMAP: *"leitura real idêntica ao mock; divergências →
fluxo R3"*.

## 2. Pré-requisitos (TODOS obrigatórios antes de conectar a pedaleira)

**Software** (em 29/09, ainda pendentes — executar antes do dia de campo):
- [x] **M0.7 ✅** — `gp100-cli` com `info`, `list-user-irs`, `dump-preset <pp>`
      **+ `--log` já no schema P4** (entregue na própria M0.7 — requisito
      antecipado; issue M0.7)
- [x] **Transporte real feature-gated** ✅ (29/09) — `RealDevice` (midir/WinMM;
      porta por nome "gp-100") atrás da feature `real-device` (default OFF);
      binário de campo: `cargo build --release -p gp100-cli --features real-device`;
      `--real` exige `--i-know-what-im-doing` (testado nas duas camadas)
- [x] **Log de fio no CLI** ✅ (entregue na M0.7) — `--log <arquivo>` grava TODOS
      os frames (OUT e IN) no MESMO schema das fixtures P4
      (`{"s","dir","func","addr","data"}` em hex): reusa `decode_wire.py` e o
      replay sem adaptar nada
- [ ] **CI verde no commit usado em campo** (regra 6 do `core-dev`: marco com CI
      vermelha não é marco) + gates locais fmt/clippy/test

**Ambiente/hardware:**
- [ ] GP-100 na USB **direta** do PC (sem hub), firmware V2.1, driver instalado
- [ ] **Suite oficial FECHADO** — todas as instâncias (uma instância residual mata
      a nossa conexão; occupancy de MIDI — armadilha de captura do knowledge)
- [ ] Ensaio prévio do MESMO fluxo contra o **MOCK** na mesma build (comportamento
      de referência conhecido; saída do mock salva p/ diff)
- [ ] Anotar o que o **DISPLAY** da pedaleira mostra (nome do preset atual, slots
      de IR com conteúdo) — em campo, o display é a verdade de estado

## 3. Procedimento de campo (nesta ordem; ~30–45 min)

**Fase A — saneamento (5 min)**
- [ ] `gp100-cli` (mock): `info` → salvar saída de referência
- [ ] Conectar a pedaleira; aguardar o driver enumerar (Gerenciador de Dispositivos)
- [ ] Iniciar o log: `gp100-cli --real --i-know-what-im-doing --log analysis/captures/sessionH1.jsonl`

**Fase B — leitura incremental** (cada passo SÓ avança se o anterior não deu
`InvalidShape`/`Timeout`; timeout = repetir 1× antes de classificar — sem retry
automático, D6)
- [ ] **B1 `info`** — pp corrente, nome, tipo, nº de presets. Conferir com o DISPLAY
      (mock ≠ real aqui é ESPERADO se o device foi editado — nível 2, ver §4)
- [ ] **B2 `list-user-irs`** — tabela dos 20 slots (nomes; CRC de vazio = ppIRCRC).
      Slots 0/1 têm IRs reais no device (upload da S2) → conteúdo divergente do
      `all.prst` é ESPERADO (nível 2)
- [ ] **B3 `dump-preset` do pp CORRENTE** — 9 páginas; conferir FORMAS (meta6 6B,
      páginas 196×8 + 32B, fim 4B em `13010005` — nível 3)
- [ ] **B4 `dump-preset` de 2–3 pps** (1º, um do meio, um do fim)
- [ ] **B5 (opcional, prova de framing total)** — boot completo: valida a sequência
      T1→scan→T6→T2→T3 do §13.10 contra o device real. ⚠️ watch: o inventário de
      pps do scan é do DEVICE (S1: começa em 0x0100; default do core é 0..198) —
      conjunto/ordem divergentes = achado R3, não "bug do CLI"
- [ ] Encerrar o log e SÓ ENTÃO desconectar (flush do arquivo)

**Fase C — comparação (no PC, logo após a sessão)**
- [ ] Rodar os decoders na `sessionH1.jsonl` (mesmos scripts das capturas do Suite)
- [ ] Comparar o FRAMING de cada log vs `analysis/h1_reference/` (runbook:
      `scripts/h1_field.sh` — nível 1; conteúdo divergente = nível 2, esperado)
- [ ] Classificar cada divergência nos níveis do §4 e lançar na tabela do §5
- [ ] Preencher `docs/H1_REPORT.md` e arquivar (§7; plano de backup no §7 dele)

**PROIBIDO nesta sessão:** qualquer escrita (mesmo dry-run — é H2), update de
firmware, e "testinhas" fora do roteiro. Curiosidade custa pedaleira.

## 4. Níveis de comparação (o que é falha de protocolo e o que é estado)

| Nível | O quê | Fonte do esperado | Se divergir |
|---|---|---|---|
| **1. FRAMING** | func/addr de cada IN, na ordem; casa com template do golden (`match_response`, igual ao mock) | golden (Baseline v1.0) | **FALHA DE PROTOCOLO → R3 (§6)** |
| **2. CONTEÚDO de estado** | pp/nome/tipo corrente, nomes de IR, CRCs de slots ocupados | DISPLAY + histórico (S2 subiu IRs; S4 gravou preset no device) | ESPERADO — documentar no relatório |
| **3. ESTRUTURAL** | formas/tamanhos: meta6 6B, páginas 196/32B, fim 4B, ACK 4B | golden §13 | **FALHA → R3 (§6)** |

Desempate "protocolo × estado": repetir a MESMA leitura 2× — protocolo é
determinístico (diverge igual nas duas), estado não muda entre leituras.

## 5. Log de divergência (1 linha por ocorrência; vai para `docs/H1_REPORT.md`)

| # | Passo | Endpoint (func/addr) | Nível | Esperado (golden/mock) | Obtido (hex curto) | Hipótese | Severidade | Ação |
|---|---|---|---|---|---|---|---|---|
| 1 | B2 | 12/12001002 | 2 | CRC vazio = ppIRCRC do all.prst | outro CRC | slot 0 tem IR da S2 | estado | documentar |

- **Severidade:** `bloqueia-H1` (nível 1/3), `estado` (nível 2, esperado), `info`.
- Bruto da sessão: `analysis/captures/sessionH1.jsonl` (append-only; **`-text` no
  .gitattributes antes do 1º commit** — regime de bytes da skill rust-practices).

## 6. Fluxo R3 (do ROADMAP) — o que fazer com cada `bloqueia-H1`

1. **PARAR o roteiro** (não seguir "com ressalvas") e logar a divergência com hex completo.
2. **Reproduzir por captura:** repetindo a leitura com o CLI (log novo) ou — se
   precisar do comportamento do SUITE — nova sessão de captura com o proxy winmm
   (skill `capture-analyze`: fechar instâncias, log append-only, segmentar por gaps >30s).
3. **Re-derivar + validar:** `build_golden.py` → `validate_golden.py` 100% nos
   critérios da skill `protocol-validate` (A–E).
4. **Bump de baseline (skill `spec-baseline`):** novo sha256 nos **5 lugares** —
   §13 do PROTOCOL.md, `GOLDEN_BASELINE_SHA` no `make_fixtures.py`, assert no
   `test_protocol.py`, exemplos em DECISIONS.md/INDEX. Golden é `-text` (EOL conta!).
5. **Só então** retomar o código (R1: golden é insumo) e re-executar o H1 do zero.

## 7. Critérios de saída (DoD da H1)

- [ ] B1–B4 sem `InvalidShape`/`Timeout` — níveis 1 e 3 limpos ("leitura real
      idêntica ao mock no framing")
- [ ] Toda divergência classificada: `estado` documentada no H1_REPORT;
      `bloqueia-H1` tratada via R3 e H1 re-executado
- [ ] `docs/H1_REPORT.md` arquivado + linha no `knowledge.md` (estado vivo)
- [ ] ROADMAP: H1 ✅ com data (ou itens R3 abertos como issues)
- [ ] `sessionH1.jsonl` preservado (matéria-prima para o H3/golden v1.1)

## 8. Watchlist específica do H1 (coisas que ESPERAMOS encontrar)

- **Estado ≠ `all.prst`:** S4 gravou "It's GP100" no device (persistência
  confirmada no display); S2 subiu IRs nos slots 0/1 → o device real difere do
  mock, que DERIVA de `all.prst` (nível 2 — esperado por desenho).
- **Inventário do scan:** como o Suite descobre a lista de pps (S1: 0x0100…)?
  Device real com pps fora de `0..198` → boot() com inventário default diverge
  (abrir R3 se confirmar; a `set_inventory` já existe na FSM).
- **Flush no close:** S2/S4 tiveram bursts tardios (32 ACKs `12001002`,
  `11000008`+`12000001`) no fim de sessão — se o log H1 mostrar burst no
  disconnect, é o mesmo fenômeno (D7 não modela; documentar no relatório).
- **Timing pós-boot:** device recém-ligado pode responder lento no 1º `info`
  (janela ADR-3 = 3s) — repetir 1× antes de classificar Timeout.
- **Occupancy:** Suite/qualquer app MIDI aberto → `OpenFailed` no `open()` —
  fechar tudo e reconectar (o mesmo objeto reconecta, ADR-4).
