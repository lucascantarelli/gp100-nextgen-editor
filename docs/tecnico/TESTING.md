# 🧪 TESTING — Contrato da suíte de testes

> **Status:** ✅ atual · **Revisão:** 2026-10-11 · **Nascido da #166** (suíte enxuta, Era Hardware)
>
> Este é o **mapa e o contrato** da suíte: o que ela cobre, como rodar, **o que um
> teste novo precisa provar**, **o que não entra** e **as proteções que não podem
> cair**. Ele não substitui — **aponta**: gates por área em
> [`CONTRIBUTING.md` §4](../CONTRIBUTING.md), roteiros e2e em
> [`UI_TEST_PLAN.md`](../dev/UI_TEST_PLAN.md), como escrever Rust/UI em
> `.agents/skills/{rust-practices,ui-ux-practices}/SKILL.md`.
>
> **Contadores** (nº de testes, cobertura, baselines) vivem em
> [`INDEX.md` §6](../INDEX.md) — fonte única (regra do `docs-sync` #81). Não os repita
> aqui nem em outro doc: um número em dois lugares é dívida garantida.

---

## 1. As quatro suítes — o que é testado

| Suíte | Onde | O que ela prova | Fonte da verdade |
|---|---|---|---|
| **Rust** (`core`·`cli`·`library`·`shell`) | `packages/*/tests/*.rs` + `#[cfg(test)]` dentro de `src/` | protocolo/codec byte-a-byte, FSM de sessão, round-trip `.prst`, travas de valor e de escrita, boot, MockDevice | golden §13 · fixtures `packages/core/data/fixtures/` · ADRs |
| **Unit do front** (vitest/jsdom) | `packages/app/ui/tests/` | componentes, hooks, contratos IPC, i18n (4 idiomas), acessibilidade | [`UI_REFERENCE.md`](../referencias/UI_REFERENCE.md) · tokens (`design/tokens.ts`) |
| **Spec** (pytest) | `tools/tests/` | travas da especificação (golden/baseline), gates de script, faixas ADR-10, juízes de campo H1/H2/H3 | `docs/referencias/protocol_golden.json` · `tools/baseline.json` |
| **E2E** (Playwright) | `packages/app/ui/e2e/` | fluxos vivos da casca (roteiros R1–R7), regressão visual, falhas de IPC | [`UI_TEST_PLAN.md`](../dev/UI_TEST_PLAN.md) (fonte dos roteiros) |

Cada suíte tem um **juiz independente** — passar numa não cobre outra. A CI roda as
quatro em jobs separados (`.github/workflows/ci.yml`), filtradas por caminho
(`.github/scripts/ci_plan.py`): doc puro não sobe Rust nem front. Os workflows
reutilizáveis (`dist-*.yml`/`release-*.yml`, #245) carregam o CORPO da
distribuição e da release; o `ci.yml` fica com o grafo e o nome dos jobs.

---

## 2. Como rodar

Comandos de nível de **suíte** (a régua por **área** é a
[§4 do CONTRIBUTING](../CONTRIBUTING.md); o CI espelha essa ordem):

```bash
# Rust — unit + contratos + DOC-TESTS (exemplos de /// compilam e rodam)
cargo test --workspace
cargo test --test codec_wire            # um contrato específico
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings

# Unit do front
cd packages/app/ui && pnpm exec vitest run
pnpm run test:coverage                  # GATE 85% (statements/functions/lines, vite.config.ts)
pnpm lint && pnpm tsc --noEmit && pnpm build

# E2E (Playwright)
cd packages/app/ui && pnpm exec playwright test

# Spec (pytest) — travas da especificação, inclui a suite do H3
uv run pytest

# Gates de script — todos de uma vez, na ordem da CI
python .github/scripts/gates.py                 # --list mostra a lista; nomes de gate filtram
```

- **Mudou só uma área?** A CI filtra por caminho; rode só os gates dessa área
  (tabela do [CONTRIBUTING §4](../CONTRIBUTING.md)).
- **`simulate_release` tem regime próprio no CI (#246).** É o gate mais caro
  (~2 min, executa a cadeia rc→promote de verdade) e roda no passo
  `simulação do GitFlow` só por **escopo**: `schedule`, tag/dispatch (`CHANGED=ALL`)
  e PR que toca a maquinaria de release (`.github/workflows/`,
  `.github/actions/version-bump/`, `simulate_release.py`, `ci_plan.py`,
  `sync_version.py`). Os demais gates de script continuam em todo PR. Local,
  `python .github/scripts/gates.py simulate_release` roda quando você pedir — o
  custo local é seu, não de cada PR.
- **Mexeu na cadeia de release?** O `simulate_release` é o gate que prova; ele lê
  os blocos dos workflows reutilizáveis e confere o `uses:`/`with:` do caller no
  `ci.yml` — a falha mais comum (input obrigatório sem valor) só aparece no LOAD
  da run, ou aqui.
- **Cobertura do front caiu de 85%?** O gate falha e **pede teste, não exceção**.
- **`gates.py`** é o atalho para os gates de script (pipeline, empacotamento,
  versão, spec, release).

---

## 3. O que um teste novo precisa provar

Um teste entra na suíte só se passa por estas cinco portas. Se hesitar, é porque
não é teste — é ruído.

1. **Protege comportamento que quebraria em silêncio.** O teste existe para que,
   se alguém tocar no código, a falha APAREÇA. "Documentar que a função existe"
   não é proteção; `assert!(true)` disfarçado também não.
2. **Justifica a própria existência num doc-comment.** É a cultura da casa: todo
   `#[test]` / `it()` abre com o *porquê* (o bug que evita, a regra que trava, o
   ADR que materializa). Teste sem narrativa é teste que ninguém entende quando
   falha.
3. **Asserta comportamento, não fixture.** O valor vindo de captura/golden é o
   *insumo*; o que se prova é o **comportamento do app** diante dele. Repetir o
   byte do arquivo não prova nada que o arquivo já não prova.
4. **Morada certa** (regra de decisão da `rust-practices`): usa só API `pub` →
   `tests/<assunto>.rs` (caixa-preta); precisa de internals → `#[cfg(test)]`
   dentro de `src/`. Não mover unitário para `tests/` "por organização" — perde
   acesso sem ganhar nada. Front: `tests/` junto do código, sem `invoke`
   espalhado (R1 do front).
5. **Não duplica prova já existente.** Antes de escrever, procure o par mais
   próximo e leia os *asserts* (não só os nomes). Se o outro já assertiona o mesmo
   — ou algo **mais forte** — o novo não nasce. (Foi assim que a #166 achou e
   cortou `embedded_golden_smoke`: subsumido por
   `embedded_golden_loads_39_templates`, que assertiona 39 templates × "não-vazio".)

**Acompanha código novo** é padrão de DoD: mudança de comportamento sem teste que
a proteja não passa no gate de cobertura nem no review.

---

## 4. O que NÃO entra na suíte

Regra de **exclusão** — o coração da #166. Isto aqui não nasce teste novo:

1. **Espelho da RE que só re-afirma bytes capturados.** Um teste cujo único papel
   é provar que "o replay da captura S1 reproduz a captura S1" prova o *fio
   gravado*, não o aparelho nem o app. Exemplos vivos que a era HW vai
   substituir (`replay_fixtures`, `preset_pages`, `golden_*`) — o substituto
   (MockDevice fiel #157, modelo tipado HW-2) nasce nas issues HW-1/2/4. Enquanto
   não existir, o arquivo **fica** (a issue manda: aponta o substituto, não abre
   buraco); mas **novo** teste nesse espelho não entra.
2. **Dado da RE como insumo.** Fixtures moram em
   `packages/core/data/fixtures/` (migração #163); `guard_dados_canonicos.rs`
   garante `git grep "analysis/" packages/` = 0 (o literal inclui o caminho novo
   `docs/arquivo/analysis/`, que contém a mesma sequência). Teste que lê da RE para dado de
   entrada viola o DoD #3 e o guard.
3. **Protocolo adivinhado (R1).** Assert sem respaldo em golden/§13 não entra —
   "o byte é assim porque a captura mostrou" exige a captura na fonte, não
   hardcoded no teste (`rust-practices`: magic number de protocolo nunca inline).
4. **Duplicata.** Prova já coberta por outro teste (porta 5 da §3).
5. **Relaxamento de proteção para ficar verde.** Nunca: afrouxar `value_gate`,
   `write_gate`, R4 ou o guard HW-3 para passar é trocar a trava pelo abajur.

---

## 5. Proteções que não podem cair (DoD #5)

Estas travas **nunca** podem ser removidas, relaxadas ou "simplificadas". São o
piso de segurança do projeto; se uma falhar, é incidente, não refator:

| Proteção | Teste que a materializa | O que ela impede |
|---|---|---|
| **Round-trip `.prst` (R4)** | `roundtrip_prst.rs` | byte-idêntico ao exportar/reimportar — o formato de arquivo não pode corromper |
| **Trava de valor (ADR-10)** | `value_gate.rs` + `tools/tests/test_param_ranges.py` | valor fora da faixa não vai para o aparelho (firmware trava) |
| **Trava de escrita (ADR-5)** | `write_gate.rs` | escrita só em binário `write-verified` — sem flag/ambient que contorne |
| **Segurança do boot** | `boot_mock.rs` | o boot não emite escrita nenhuma |
| **Guard HW-3 (dados canônicos)** | `guard_dados_canonicos.rs` | `packages/` não volta a depender de `analysis/` (nem do arquivo novo `docs/arquivo/analysis/`) |
| **Inventário do aparelho (ADR-12)** | `inventario_banco_slot.rs` | 99 presets/bancos lidos e enumerados sem saltos |

Proteções do DoD #5 foram verificadas verdes na baseline de 11/10 (triagem da
#166). Rodar `cargo test --workspace` + `uv run pytest` reconfirma as seis.

---

## 6. Onde cada peça vive (rota rápida)

| Preciso de… | Vá para |
|---|---|
| Gates por área antes do PR | [`CONTRIBUTING.md` §4](../CONTRIBUTING.md) |
| Roteiros e2e (R1–R7) e baselines visuais | [`UI_TEST_PLAN.md`](../dev/UI_TEST_PLAN.md) |
| Como organizar teste Rust (unit × contrato) | `.agents/skills/rust-practices/SKILL.md` |
| Gates/a11y do front | `.agents/skills/ui-ux-practices/SKILL.md` |
| Número de testes / cobertura (fonte única) | [`INDEX.md` §6](../INDEX.md) |
| Decisões de arquitetura (ADR-1..13) | [`DECISIONS.md`](../dev/DECISIONS.md) |
| Triagem detalhada da #166 (veredito arquivo a arquivo) | issue [#166](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/166) |
