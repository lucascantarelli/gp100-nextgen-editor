# 🔍 Auditoria da era real — do dado inventado ao dado do aparelho (07/10/2026)

> **Status:** 🔨 primeira edição viva, feita ÁREA POR ÁREA à medida que o campo responde.
> **Gatilho:** a primeira sessão da UI contra o GP-100 V2.1 real (07/10) desmentiu uma
> premissa que nenhum gate pegou (→ #148) e expôs a frente de trabalho da transição:
> **o app foi construído na era mock e agora o hardware está disponível.** Este documento
> organiza essa transição — não a executa sozinho.
> **Método:** cada afirmação abaixo tem fonte (arquivo + linha, captura, manual ou sessão
> de campo). Nada aqui é suposição sem ponteiro. Padrão das auditorias anteriores
> (`audit_2026-10-07.md`, achados F-* / issues #138–#147), mas com o critério novo:
> **o dado é do aparelho (real) ou é suposição que o aparelho já desmente?**

---

## 0. O princípio (a regra que a #148 institucionalizou)

**Teste só trava o que foi modelado. Valor não modelado é valor não travado.**

O pipeline que funcionou na #148 (e antes na #110, para valores de knob):

```
captura real → extrai o MODELO (o espaço de valores, não só a forma)
             → guarda no CORE (função pura + guarda antes do fio)
             → teste que congela a lista contra a captura
             → campo confirma
```

O golden H continua sendo a spec de FORMA (framing/estrutura §13); cada **espaço de
valores** ganha o seu próprio modelo. O mock deixa de ser fonte de verdade e passa a
ser exercício da tela fora do hardware.

---

## 1. O dado inventado que o campo JÁ desmentiu (resolvido nesta data)

| # | O que era | Evidência pré-campo | O que o campo desmentiu | Estado |
|---|---|---|---|---|
| 1 | Inventário do scan = intervalo linear `0..198` | `0u16..198` em `session.rs`; mock valida SHAPE (796 selects "batem" com qualquer pp) | assert `PresetNum < TOTAL_PA` (`audio.c:912`), reproduzido 2×; espaço real = banco/slot `0000..=0062` + `0100..=0162` (796 selects, 198 payloads distintos em S1–S4) | ✅ **fix em develop (#132/#137, a742ccf)** |
| 2 | Palco força `openPreset(0)` no arranque (assumia fábrica) | `useStage.ts:285` | owner com preset débil tocando; boot já descobre o corrente (`current_pp`) | ✅ **fix em develop (#132/#137, a742ccf)**: `mount` abre o `current_pp` via `device_preset_library` |
| 3 | A numeração de preset era suposição de label | LED real do aparelho | **P01–P99 = USER, F01–F99 = FÁBRICA (1-based)** — manual V1.8, "two patch banks" | 📋 **#150** (medir em campo o mapeamento banco↔P/F antes de trocar o label) |

---

## 2. Inventário da era mock — o que precisa substituir (por área)

### 2a. Biblioteca da UI (a fonte da lista)

- **Hoje:** aba Factory = artefato estático (`all.prst` semeado em
  `artifacts/presetData`); aba User = **SQLite do editor** (#113). O aparelho nunca é
  a fonte. Detalhe em **#150**.
- **Caminho:** nomes dos 198 do device (a tabela do boot T3 já circula no fio),
  abas Modelo/Usuário refletindo o aparelho, artefato virando fallback fora do
  Tauri, trust chain `device → cache pós-scan → artefato`.

### 2b. Board do fallback (`FACTORY_PRESETS` → `PRESET_CHAINS`)

- **Hoje:** `localMockBoard(pp)` escolhe a cadeia desenhada de um índice da lista;
  os knobs vêm do dicionário. No shell real o board vem do `device_board` do core
  (dicionário + páginas do fio). **O caminho real já é o certo** — é o fallback
  que semeia o dado estático.
- **Caminho:** nenhum dado na UI fora do fallback deve nascer de índice inventado;
  o Artifacts-estático fica titularmente de fallback (fora do webview).

### 2c. Documentação que ainda descreve a era mock

- `packages/app/README.md` — "O backend default é o **mock** — o modo real entra
  como build de campo pós-gate H1 (feature `real-device` espelhada do core)":
  **defasado** — a #126/#128 entregaram o shell `real-device` e a forma canônica é
  `--features real-device[,write-verified]` (BLOCKERS §; CONTRIBUTING).
- `docs/REAL_DEVICE_GAP.md` — bom contéudo histórico/auditorável; precisa do banner
  ⚠️/✅ do docs-sync (item 8/corrolário F-03/F-04) apontando que a barreira de código
  caiu (d26e28a) e o que sobra é a integração #150/#116.
- **Caminho:** um commit docs-sync dedicado (a skill manda commit PRÓPRIO e SEPARADO)
  atualizando README/banner/knowledge em cima dos artefatos reais atuais.

### 2d. Premissas de teste que herdaram a era mock

- **Mock/golden:** valida FORMA, não valor (o buraco da #148). Já fechado PARA PP;
  o princípio §0 vale para o próximo espaço de valores que aparecer.
- **`failDevice` / ganchos de debug do e2e** (`gp100.debug.failDevice`): exercitam o
  fallback (navegador) — legítimos como teste de tela; NUNCA como prova do device.
- **Conferência de valor:** onde o teste hoje CONFERE um número contra captura?
  Apenas duas: knobs (#110) e pp (#148). O inventário do que falta está na §3.

---

## 3. A fila do dado real (as próximas áreas, com fonte do fio)

1. **Nomes dos 198 do aparelho** — a tabela T3 (nomes `11000008`) circula no boot;
   hoje o core só valida o que volta? vira porta pro front (#150).
2. **User patches do aparelho** (banco do device) — ler o board de cada slot do
   banco user, listar/editar/gravar pelo Suite-path (SAVE).
   O versionamento #113 vira camada ABAIXO (snapshots do que foi editado no editor).
3. **Plug do A/B no caminho real** — a sessão de campo da #116 (intervalo
   pedido→último frame) com o build `write-verified` e o boot corrigido.
4. **Estilos/PP types do aparelho** — `ppType` hoje vem do artefato?

---

## 4. O que a era real CONFIRMA (o campo positivou, não negativou)

- **A FSM da Sessão (D1–D8)** resistiu: o boot completo passou no aparelho após o
  fix de endereçamento — o desenho de transação/backlog/quirk §13.4 é real.
- **O golden H como spec de forma** resistiu (framing/estrutura/quirk).
- **O dicionário de knobs (105 nomes, ranges)** é dado real e virou guarda (#110)
  — hoje o aparelho não derruba com valores fora (assert `audio.c:1828`).
- **A guarda ADR-10** funcionou em campo pela entrega da #110; o mesmo padrão
  acaba de cobrir pp (#132/#137).
- **O scan completo (2299 transações)** é o roteiro do Suite real, e agora é o
  boot do app.

---

## 5. Respostas diretas às perguntas do owner (07/10)

| Pergunta do owner | Resposta |
|---|---|
| "app/api tem modelos/regras organizadas?" | O app/api é INTENCIONALMENTE fino (ADR-7/9: orquestra, não modela); regra nenhuma mora no shell. A modelagem mora no core (preset, dicionário, faixa de knob #110, agora o espaço de pp #148). O que faltava era exatamente um modelo: o espaço de endereçamento nunca foi modelado — era intervalo linear. |
| "Nossos testes são baseados em dados reais?" | Forma: sim (goldens/fixtures = capturas reais). Valor: NÃO (golden compara forma; valor divergindo é "esperado por desenho"). Os espaços de valor só viram modelo quando o campo grita (#110, #148) — a fila da §3 antecipa os próximos. |
| "O app é agnóstico de hardware?" | Não, por desenho: protocolo GP-100. A fronteira existe (`DeviceTransport`/`Session`), mas outro aparelho = outro protocolo. O caminho agnóstico DE HOJE: os bancos do device virarem cidadãos de primeira classe na UI (#150). |
| "Muito dado hardcoded que diverge?" | 2 confirmados em campo e mortos na segunda (inventário linear; o `openPreset(0)`). A confirmar: rótulos P/F (base 1) da UI vs LED. Defasados: `app/README` (era mock), `REAL_DEVICE_GAP` (precisa banner). |
| "Goldens: agnósticos ou de fábrica?" | São **capturas reais do Suite** (S1–S4) usadas como spec de FORMA. Como spec de valor, cada espaço precisa do próprio modelo extraído (hoje: knobs #110, pp #148). |
| "Por que os testes não barravam parâmetro inexistente?" | Teste trava o que foi MODELADO. O espaço de pp não era modelo — era suposição. O princípio §0 (+ a fila §3) é a correção sistêmica. |
| "Colapsar os pacotes no app?" | Não recomendado: hardeniza protocolo ao Tauri (MSVC) e mata os gates H (CLI) e a matriz gnu de 3 SOs (ADR-7), e separação library/core=SW do storage compartilhado. Complexidade REAL a cortar está na auditoria 07/10 (F-02/F-07/F-09/F-10). |
| "Decisões da era pré-hardware podem ser atualizadas?" | Sim, por este documento: cada ADR tocada por campo ter a nota (ex.: READ SELECT = WireKind::Read mas MUDA estado no aparelho; leitura de CODE = leitura vírgula o ADR-5 ela fala a ideia). |

---

## 6. O próximo passo (a agenda da transição)

1. **Campo fecha o label P/F** (sessão curta: toca, olha LED, lê pp corrente no app) → `#150`.
2. **#150 entrega a biblioteca do aparelho** (nomes T3 → porta → abas F/P exemplo).
3. **#116 em campo** (a medição do intervalo pedido→último frame, com o boot corrigido).
4. **docs-sync único** (commit próprio): README do app, banner do REAL_DEVICE_GAP, knowledge.md"estado vivo", §6 índice — em cima do estado REAL atual.
5. Cada novo espaço de valores que o campo expor entra na §3 da fila — e sai como
   modelo + guarda + teste, **antes** de qualquer outra sessão tocar o fio.

---

## 7. Fontes citadas

- Sessões de campo 07/10: asserts do device (2) + boot limpo pós-fix + biblioteca.
  - [ ] arquivar os logs de campo desta data em `analysis/field/2026-10-07/`
    (mesmo padrão do H1/H2_REPORT).
- `analysis/captures/session{1..4}.jsonl` (796 selects; payloads = 0000..0062, 0100..0162).
- `analysis/manual_v18.txt` ("two patch banks: User P01–P99 / Factory F01–F99").
- Valeton oficial (valeton.net/product/gp-100): "198 presets (99 user + 99 factory)".
- Issues: #148 (duplicata consolidada em #132/#137), #150 (biblioteca do aparelho), #116 (A/B), #110 (precedente knob).
- `packages/core/src/session.rs` (inventory/select_preset/pp_e_valido), `packages/app/ui/src/hooks/useStage.ts` (carregaCorrente).
- Regras: `knowledge.md`, `docs/CONTRIBUTING.md`, `.agents/skills/docs-sync/SKILL.md`.
