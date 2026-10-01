## O quê e por quê

<!-- 1–3 frases: a mudança e a motivação. Referencie a issue: `Closes #N` (fecha na integração em develop), ou apenas `#N` para vínculo simples. -->

## Antes → Depois

| | Antes | Depois |
|---|---|---|
| **Comportamento** | | |
| **Prova** (teste/baseline/doc) | | |

## Gates executados

<!-- Marque o que rodou localmente; apague as linhas que não se aplicam à área tocada. -->

| Gate | Resultado |
|---|---|
| `uv run pytest` (spec — golden 5 provas + hash + knob_map + fixtures) | ☐ 10/10 |
| `cargo fmt --check` + `clippy --workspace --all-targets -- -D warnings` + `cargo test` | ☐ ✅ |
| `packages/app/ui`: `tsc -b` + `pnpm lint` + `pnpm test` | ☐ ✅ |
| `packages/app/ui`: `pnpm build` | ☐ ✅ |
| `packages/app/ui`: e2e Playwright (funcional + visual) | ☐ ✅ / **N** baselines regeneradas |

## Ambiente verificado

<!-- SO, toolchains, device/mode (mock vs real). Ex.: Windows 11 · rustc 1.98 (gnu) · Node 22 · mock -->

## Teste acompanha código novo

- [ ] Teste/coverage novo para o comportamento (padrão de DoD das issues) — ou justifique a exceção

## Regras da casa (ROADMAP R1–R4)

- [ ] **R1** — nenhum byte de protocolo "adivinhado": golden/§13 consultados como insumo
- [ ] **R2/R3** — se tocou spec/capturas: captura nova → `build_golden` → `validate_golden` 100% → bump de hash no §13
- [ ] **R4** — round-trip `.prst` byte-idêntico preservado (se a mudança toca o modelo de preset)
- [ ] **ADR** — se mudou decisão de `docs/DECISIONS.md`: novo ADR marcando o anterior como Superseded

## Política de conteúdo (README §7)

- [ ] Nenhum material proprietário incluído (firmware, instaladores, driver, strings do Suite)
