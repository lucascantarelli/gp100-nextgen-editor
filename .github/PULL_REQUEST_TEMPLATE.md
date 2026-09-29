## O quê e por quê

<!-- 1–3 frases: a mudança e a motivação. Referencie a issue do ROADMAP (ex.: M0.1). -->

## Como foi validado

<!-- Comandos executados e resultados. O gate é obrigatório: -->

- [ ] `uv run pytest` — **10/10** (golden 5 provas + hash da baseline + knob_map + fixtures)
- [ ] `cargo fmt --check` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo test`
- [ ] Teste novo acompanha código novo (padrão de DoD das issues)

## Regras da casa (ROADMAP R1–R4)

- [ ] **R1** — nenhum byte de protocolo "adivinhado": golden/§13 consultados como insumo
- [ ] **R2/R3** — se tocou spec/capturas: captura nova → `build_golden` → `validate_golden` 100% → bump de hash no §13
- [ ] **R4** — round-trip `.prst` byte-idêntico preservado (se a mudança toca o modelo de preset)
- [ ] **ADR** — se mudou decisão de `docs/DECISIONS.md`: novo ADR marcando o anterior como Superseded

## Política de conteúdo (README §7)

- [ ] Nenhum material proprietário incluído (firmware, instaladores, driver, strings do Suite)
