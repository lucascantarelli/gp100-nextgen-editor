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

## Regras da casa (ROADMAP §1.2 — princípios P1–P7)

<!-- O ROADMAP vigente é a Era Hardware: os princípios são P1–P7. As regras R1–R4
     são da era de engenharia reversa, arquivada em docs/arquivo/. -->

- [ ] **P1** — o aparelho é a fonte: nenhum byte "adivinhado" (golden/§13 são insumo) nem dado de análise embutido no runtime
- [ ] **P2** — fábrica × usuário são tipos; hex nunca chega à tela
- [ ] **P4** — escrita é privilégio: nada escreve no boot, faixa travada (ADR-10); anti-brick é critério de aceite
- [ ] **P5** — teste só do comportamento real (sem replay de RE); round-trip `.prst` byte-idêntico preservado se a mudança toca o modelo de preset
- [ ] **P7** — doc e harness acompanham: skill/prompt/INDEX/knowledge na MESMA PR que muda comportamento
- [ ] **ADR** — se mudou decisão de `docs/dev/DECISIONS.md`: novo ADR marcando o anterior como Superseded

## Política de conteúdo (README §7)

- [ ] Nenhum material proprietário incluído (firmware, instaladores, driver, strings do Suite)
