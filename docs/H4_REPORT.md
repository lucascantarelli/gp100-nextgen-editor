# 🧾 H4_REPORT — Relatório do gate H4 (as páginas 13xx em campo: nome, palco, knob)

> **Status:** ⏳ **aguardando a sessão de campo.** Este arquivo nasce junto do
> [H4_CHECKLIST.md](H4_CHECKLIST.md) para que a sessão tenha onde ser registrada
> — e para que "não medido" não possa ser confundido com "medido e ok".
>
> ⚠️ O modo de falha mais caro deste projeto é preencher "pareceu que funcionou"
> onde a coluna do fio diz "não medido". Enquanto o campo não rodar, **cada
> veredito abaixo fica `⏳`** — e isso é a resposta honesta, não uma pendência
> esquecida.

**Gate de leitura.** Nada nesta sessão escreve no aparelho pelo app: as duas
edições (renomear o patch, girar o knob) são feitas **no pedal, pela mão do
owner**, e o app só lê depois. Por isso a build é
`tauri build --features real-device`, **sem** `write-verified`.

---

## 1. Identificação da sessão

| Campo | Valor |
|---|---|
| Data/hora | ⏳ |
| Commit do repo no binário | ⏳ |
| Build | `tauri build --features real-device` (face de leitura) |
| Backend no painel | ⏳ (`real` é o esperado) |
| Device (firmware) | ⏳ (o assert do firmware conhecido é da **V2.1**) |
| pp renomeado na sessão 2 | ⏳ (descartável) |
| Knob girado na sessão 3 | ⏳ (slot + parâmetro + de/para) |
| Log de fio do app | ⏳ caminho do `.jsonl` de cada sessão (um arquivo por execução) |

## 2. Sessão 1 — o palco e os nomes vêm do aparelho

| Medida | Esperado | Obtido | Veredito |
|---|---|---|---|
| Total de transações do boot | **2299** (inventário da captura, ADR-12) | ⏳ | ⏳ |
| Nome no app == display do pedal (3 presets de fábrica) | P01/P50/P99 iguais | ⏳ | ⏳ |
| Palco de um preset com a cadeia **trocada** (20 dos 99) | a cadeia do **pedal**, não a do `all.prst` | ⏳ | ⏳ |
| Nome vazio (sem cache para aquele pp) | possível e honesto — anotar o pp | ⏳ | ⏳ |

> **2297 em vez de 2299** significa que o inventário do mock vazou para o build
> de campo. Parar e abrir issue — não é o gate que está errado.

## 3. Sessão 2 — o nome vive nas páginas, não na meta6

| Medida | Esperado | Obtido | Veredito |
|---|---|---|---|
| Nome gravado no pedal (display) | confirma o SAVE | ⏳ | ⏳ |
| App, após **novo boot**, mostra o nome **novo** | sim (critério §10 da #155) | ⏳ | ⏳ |
| Payload de `13/13010001` do pp renomeado | `<pp u16BE> 0c 1c 01 40` **constante** | ⏳ | ⏳ |

**Leitura do resultado:**

- payload **constante depois da renomeação** → o negativo de #152 deixa de ser
  estático: a meta6 **não** carrega o nome, provado por manipulação. #152 fica
  fechada como negativa confirmada em campo.
- payload **mudou** → a #152 **reabre** como folha de campo (a hipótese original
  volta a ser candidata; o corpo da issue diz como).

## 4. Sessão 3 — o mapa knob ↔ offset

| Medida | Esperado | Obtido | Veredito |
|---|---|---|---|
| Campos que mudaram no diff das páginas | **um** campo estável = candidato | ⏳ | ⏳ |
| O candidato é rejeitado fora de lugar (prova-negativa, guarda do #110) | sim, ou não entra em `state_pages_offsets.json` | ⏳ | ⏳ |
| Se o diff foi ambíguo | registrar a lista e **não** promover offset | ⏳ | ⏳ |

> Nenhum offset entra em `state_pages_offsets.json` nesta sessão: o registro aqui
> é a **observação**; o offset entra por PR, com o teste que o rejeita onde não
> deve casar.

## 5. Juízes e gates (rodar com a sessão ainda no disco)

| Gate | Esperado | Obtido |
|---|---|---|
| `analysis/validate_state_pages.py` | exit **0** | ⏳ |
| `analysis/validate_golden.py` | **2299/2299** | ⏳ |
| `uv run pytest -q` | verde | ⏳ |
| `cargo test --workspace` | verde | ⏳ |
| Escrita pelo app | **nenhuma** (o gate é leitura) | ⏳ |

O golden **não** muda nesta sessão: nenhuma captura nova entra no baseline
(R2/R3). Se `validate_golden` mudou de número, parar — o gate não é a causa.

## 6. Veredito do operador (assinado)

- [ ] Critério §10 da #155 (edição no hardware reflete no app, sem re-seed): ⏳
- [ ] Palco com backend `real` desenhando o conteúdo do pedal, sem a recusa de
      cache: ⏳
- [ ] Negativo da #152 confirmado em campo (ou reabertura registrada): ⏳
- [ ] Mapa knob ↔ offset: um campo provado **ou** ambiguidade declarada: ⏳
- [ ] `.jsonl` anexado · `H4_REPORT.md` arquivado: ⏳

**Assinatura / data:** ⏳

---

## 7. Fontes

- [H4_CHECKLIST.md](H4_CHECKLIST.md) — o roteiro desta sessão
- [#155](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/155) · [#152](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/152)
- [REAL_DEVICE_GAP.md](REAL_DEVICE_GAP.md) §6 passo 7 (a sessão de campo) e §4b.1 (o log automático)
- [H1_REPORT.md](H1_REPORT.md) · [H2_REPORT.md](H2_REPORT.md) · [H3_REPORT.md](H3_REPORT.md)
