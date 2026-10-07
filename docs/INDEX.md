# 📇 INDEX — Mapa da documentação (GP-100 NextGen Editor)

> **Comece aqui.** Este índice diz o que cada documento é, quando ler e qual é a
> fonte de verdade de cada assunto. Mantenha-o atualizado a cada novo documento
> ou mudança de status — é o contrato de navegação entre agentes e humanos.

**Última revisão:** 2026-10-06 (**gate H em campo** — o H2 passou nos 3 fluxos de escrita contra o GP-100 V2.1 real (#22, PR #108) e o H3 congelou a baseline (#23, PR #96); **o gate H inteiro fechou em campo** (#21 H1 · #22 H2 · #23 H3). **a épica M2 (#15) fechou com o gate H** — conteúdo (#24/#25/#26) entregue, distribuição é a #17. Auditoria 03/10 fechada (#71–#83). **Trabalho aberto vive em ISSUES do GitHub** (milestone **v1.0.0**; abertas em 06/10: **#17** — release, bloqueada no aparelho — e as tres medições de campo **M3** (#116–#118); #113/#114/#115 entregues, #119/#120 saíram da v1.0.0, a #110 fechou com a trava de faixa do ADR-10). Estado atual e números: **§6** — todos remedidos nesta data, não herdados.)

---

## 1. Rotas rápidas ("quero…")

| Objetivo | Rota de leitura |
|---|---|
| **Entender o projeto** | `README.md` → `docs/VISION.md` |
| **Saber o estado atual (o que foi entregue, quantos testes, o que está aberto)** | **`docs/INDEX.md` §6** (fonte única dos números) |
| **Contribuir (issue → branch → PR → merge)** | `docs/CONTRIBUTING.md` (GitFlow, conventional commits, gates por área, fechamento automático da issue)
| **Executar o plano de desenvolvimento** | **Issues do GitHub** (milestone **v1.0.0**: epics #13–#18 + filhas #19–#30) — o `docs/ROADMAP.md` guarda o histórico entregue e as regras (R1–R4) e aponta para as issues |
| **Planejar o lançamento (release multiplataforma)** | `docs/RELEASE_PLAN.md` (épicos EPIC-01..05 + issues REL-*: binários para download direto na GitHub Release, smoke em ambiente limpo, assinatura, auto-update — sem stores por decisão do owner) |
| **Levar o gp100-core à pedaleira (gate H1)** | `docs/H1_CHECKLIST.md` (checklist) + `docs/H1_REPORT.md` (relatório/plano de backup) + `scripts/h1_field.sh` (runbook: rehearsal/field/refresh-reference) + `scripts/h1_compare.py` (juiz da Fase C: classifica a sessão nos 3 níveis e imprime a tabela do §5) |
| **Escrever no device real (gate H2)** | `docs/H2_CHECKLIST.md` (3 fluxos, um por vez) + `docs/H2_REPORT.md` (relatório) — build com `--features real-device,write-verified` |
| **Congelar a especificação depois do hardware (gate H3)** | `docs/H3_CHECKLIST.md` (4 sessões) + `docs/H3_REPORT.md` + [`PROTOCOL.md` §13.14](PROTOCOL.md) (a conta da baseline v1.1) + `analysis/baseline.py show` |
| **Implementar o protocolo (gp100-core)** | `docs/protocol_golden.json` (especificação executável) + `docs/PROTOCOL.md` §13 (narrativa) |
| **Decidir arquitetura/estrutura no core (M0)** | `docs/DECISIONS.md` (ADR-1..12 aceitos) |
| **Saber onde o código novo do front vai morar** | `docs/ARCHITECTURE.md` (mapa de módulos + o que **não** entra em cada camada + orçamento de tamanho) |
| **Saber onde a persistencia do app mora (M2)** | `docs/DECISIONS.md` ADR-9 (crate `gp100-library` no workspace gnu, nao no crate MSVC do Tauri) |
| **Escrever/revisar código Rust (M0)** | `.agents/skills/rust-practices/SKILL.md` (gates fmt/clippy/test + estilo de docs) |
| **Modificar o protocolo / analisar nova captura** | `.agents/skills/capture-analyze/SKILL.md` → decoders → `build_golden.py` → `validate_golden.py` |
| **Compilar o proxy / nova captura em campo** | `.agents/skills/proxy-build/SKILL.md` + `.agents/skills/new-session/SKILL.md` |
| **Consumir dados de preset/efeitos** | `analysis/parameters.json` (dicionário) + `docs/PROTOCOL.md` §13.9 |
| **Saber o que falta / riscos** | `docs/BLOCKERS.md` + `analysis/capture_gaps.md` |
| **Contexto de decisões de arquitetura/stack** | `docs/VISION.md` §6–§9 |
| **Armadilhas de ambiente (Windows/Git Bash)** | `knowledge.md` |

## 2. Fonte de verdade por assunto

| Assunto | Fonte única | Não consultar (obsoleto/supersedido) |
|---|---|---|
| Protocolo de fio (bytes) | `docs/protocol_golden.json` | reparsear `captures/*.jsonl` direto |
| Protocolo de fio (semântica/narrativa) | `docs/PROTOCOL.md` §13 | — |
| Formato de ARQUIVO (TLV/CRC-8/objetos) | `docs/PROTOCOL.md` §1–12 | §13 (é fio, não arquivo) |
| Dicionário de efeitos/parâmetros | `analysis/parameters.json` | `algorithm_dict.*` (intermediários) |
| Formato `.prst` | `docs/PROTOCOL.md` §13.9 | — |
| Envelope do knob / save / IR no fio | §13.11 / §13.12 / §13.7 | leituras antigas do §13.4 (marcadas) |
| Estado do projeto / próximos passos | `docs/BLOCKERS.md` + `knowledge.md` (estado vivo) | `docs/CAPTURE_PLAN.md` (histórico) |
| Números (testes, cobertura, datas dos marcos) | `docs/INDEX.md` §6 | repetir o número em README/ROADMAP/knowledge (#81) |
| Estrutura do pipeline (jobs, tipos, ordem) | `.github/workflows/ci.yml` + `docs/CONTRIBUTING.md` §5 | workflows pré-#68 (`_validate.yml`/`release.yml`/`container.yml`) — #74 |
| Mapa knob→fio | `analysis/knob_map.json` (regenerável) | — |
| Decisões de implementação do core | `docs/DECISIONS.md` (ADR-1..12) | reabrir debate ad-hoc |
| Armadilhas Windows/ambiente | `knowledge.md` | — |

## 3. Inventário de documentos

### `docs/` — referência do projeto
| Documento | Papel | Status |
|---|---|---|
| `VISION.md` | Visão de produto/arquitetura, stack, features, roadmap M0–M3 (rev. v1.1) | ✅ atual |
| `PROTOCOL.md` | Referência única do protocolo: §1–12 formato de arquivo, §13.1–13.12 fio confirmado em campo | ✅ atual |
| `protocol_golden.json` | 39 templates request→resposta das capturas 1–4; consumir DAQUI no gp100-core | ✅ atual · **v1.1** (baseline v1.0 tinha template e 63 mensagens de save **fabricados** — §13.14) |
| `BLOCKERS.md` | Matriz de 12 subsistemas; 11 resolvidos, firmware-update diferido | ✅ atual |
| `CAPTURE_PLAN.md` | Plano original das rotas de captura | 📜 histórico (cumprido) |
| `ROADMAP.md` | Plano executivo: preparação (P), gp100-core (M0), gate de hardware (H) com issues e critérios de aceite | ✅ atual |
| `RELEASE_PLAN.md` | Plano pré-lançamento: 5 épicos (binários multiplataforma para download direto na GitHub Release — sem stores, decisão do owner, smoke em CI limpo, assinatura/notariação, canal único coeso, auto-update/rollback) com issues REL-* prontas para abrir | ✅ atual rev. 2 (issues a criar) |
| `PACKAGING.md` | Empacotamento e distribuição (#27/#28/#29): o mapa um-build→três-formatos, cada campo do `tauri.conf.json` e por quê, o pipeline de ícones (fonte 1024² derivada dos tokens do tema), as duas vias do `.deb` e a receita do Arch, mais o gate `check_bundle.py` | ✅ atual |
| `CONTRIBUTING.md` | Fluxo de contribuição: GitFlow (develop como integração), conventional commits, gates locais por área (coverage 85% incluso), template de PR e fechamento automático de issue no merge em develop (job `Fechamento · issues` do ci.yml; a `main` só recebe o SHA de uma tag `v*`) | ✅ atual |
| `UI_PLAN.md` | Planejamento issue-a-issue da Fase M1 (Editor UI Tauri/React): escopo, arquitetura DeviceActor, superfície IPC, telas, política de hardware, testes, riscos | 🔨 M1.0 ✅ (ADR-7) · M1.1 ✅ (actor + boot com barra) · M1.2/M1.3 ✅ (palco real + afinador) · V-8 ✅ (#20) · i18n ✅ (#30); M2 planejada (#24–#26) |
| `UI_REFERENCE.md` | Referência da casca do front: papéis de tela, contratos de estado, **§8 = fonte de verdade do texto de usuário** (o lint de i18n aponta para cá) | ✅ atual (M1.3) |
| `UI_TEST_PLAN.md` | **Fonte dos roteiros e2e** (declarada no `playwright.config.ts`): R1–R6 + drum/looper, matriz de viewports, política de baselines | ✅ atual · ⚠️ seção de CI cita o pipeline pré-#68 (#75) |
| `MANUAL_COVERAGE.md` | Matriz de cobertura do manual oficial V1.8 → requisitos implementados (X1..Xn), com o gate que prova cada um | ✅ atual · ⚠️ seção de CI cita o pipeline pré-#68 (#75) |
| `UI_DESIGN.md` | Design system da UI: paleta palco Valeton (âmbar/preto/vermelho/lavanda com rácios WCAG medidos), escala de Fibonacci, tipografia, motion, identidade "pedalboard ao vivo", checklist de review | ✅ atual (M1.0) |
| `ARCHITECTURE.md` | **Estrutura do front** (#82): mapa de módulos (`ipc`/`hooks`/`components`/`design`/…), o que **não** entra em cada camada, árvore de decisão para código novo e o orçamento de tamanho cobrado pelo gate `check_module_size.py` | ✅ atual (#82) |
| `H1_CHECKLIST.md` | Checklist operacional do gate H1 (primeiro contato real, só leitura): pré-requisitos, procedimento de campo, níveis de comparação (framing × estado × estrutural), log de divergência, fluxo R3 | ⏳ aguardando pedaleira + owner — RealDevice ✅, kit de campo ✅ e Fase C automatizada (`h1_compare.py`) |
| `H1_REPORT.md` | Relatório do gate H1 (template): execução por etapa, log de divergência, fluxo R3 e PLANO DE BACKUP fixo decidido antes de ligar | 📝 template |
| `H2_CHECKLIST.md` | Checklist operacional do gate H2 (escrita real — **muda estado do aparelho**): por que a trava é feature de compilação, os 3 fluxos capturados (set-param/save/upload-ir) um por vez com verificação no display, watchlist e critérios de saída | ⏳ aguardando pedaleira + owner — trava ✅ testada (15 testes, mutation-provada) |
| `H2_REPORT.md` | Relatório do gate H2 (template): veredito por fluxo, divergência classificada em protocolo/comportamento, pistas para o H3 | 📝 template |
| `REAL_DEVICE_GAP.md` | **O que o app fala com o aparelho, e o que ainda é mock** (#17/#16): a barreira do `MockDevice` monomórfico no `DeviceActor`, o inventário core·CLI·app·aparelho real capacidade por capacidade, os 4 pontos onde o comportamento muda com o aparelho ligado, e o caminho de 8 passos até a release real | ✅ novo (05/10) |
| `packages/library/` | **Biblioteca persistente** (#26, ADR-9): SQLite com migracoes versionadas (`PRAGMA user_version`), busca por nome/nº/estilo, import/export JSON versionado e seed dos 99 presets de fabrica do `all.prst`. Crate do workspace gnu, sem Tauri | ✅ novo (#26) |
| `H3_CHECKLIST.md` | Checklist operacional do gate H3 (congelamento da especificação): as 4 sessões que tiram o golden do Suite e põem o golden do gp100-core, o que fazer quando o juiz acusa endereço fora da spec, e por que a prova de save foi de 77 para 14 | ⏳ aguardando pedaleira + owner — maquinaria ✅ (normalizador, juiz, baseline versionada, `--log` com relógio) |
| `skills_audit_2026-09-29.md` | Auditoria das skills: regras que eram prática implícita, agora escritas (5 achados em core-dev/docs-sync/spec-baseline) | ✅ atual |
| `INDEX.md` | Este índice | ✅ manter atualizado |
| `analysis/wirelog.py` · `analysis/baseline.py` · `analysis/validate_core_capture.py` | O trilho do H3: normalizador dos dois schemas de log de fio · baseline versionada (versão + hash + motivo + histórico) · juiz frame a frame de uma captura do gp100-core contra a spec | ✅ (`analysis/baseline.py show`) |

### `analysis/` — laboratório (scripts + produtos + achados)
| Grupo | Arquivos | Papel |
|---|---|---|
| **Dicionário** | `parameters.json` (+ `build_parameters.py`, `algorithm.xml`, `algorithm_dict.*`) | 185 algs/639 controles, validado 3 vias |
| **Golden/validação** | `build_golden.py`, `validate_golden.py` | gera e prova a especificação executável (5 provas) |
| **Fixtures replay (P4)** | `make_fixtures.py` + `fixtures/` | fatia as 4 capturas por fase (boot/knobs/save/ir) p/ replay do M0.6; paridade no `manifest.json` |
| **Mapa de knobs** | `knob_map.json`, `validate_knob_map.py`, `dump_edit_writes.py`, `map_params_wire.py` | envelope semântico do knob (§13.11) |
| **Log de fio (P4)** | `packages/core/src/wire_log.rs` | o `--log` como decorador de transporte: uma unica implementacao do schema P4, usada pelo `--log` do CLI e pela sessao do app (`device_log_session`), que e o que os juiz `h1_compare.py`/`h2_compare.py` leem |
| **Faixas do knob** (#110, ADR-10) | `param_ranges.py` + `param_ranges.json` (gate `param_ranges`) | 639 regras do dicionário + as 92 amostras reais que as corroboram, com sha256 das duas fontes; a trava que impede o assert `para <= GetParaMaxVal` (`analysis/param_ranges.py show`) |
| **Capturas** | `captures/session1–4.jsonl`, `ir_slot*.bin` | matéria-prima bruta (append-only!) |
| **Decoders** | `decode_wire.py`, `check_session2.py`, `extract_ir_upload.py`, `derive_save_ops.py`, `recon_session3.py`, `ctx_dump.py`, `tail_dump.py`, `flow_dump.py`, `scan_addr.py`, `probe_session4.py`, `map_state_pages.py`, `scan_prst.py`, `decode_capture.py` | análise dirigida das capturas |
| **RE estática** | `FINDINGS_PROTOCOL/FSM/OBJECTS/COMMANDMAP.md`, `rtti_disasm.py`, `disasm_*.py`, `xrefs_opcodes.py`, `opcode_extract.py`, `find_817_imm.py`, `README_GHIDRA.md` | achados do binário do Suite (corroboram §13) |
| **Proxy** | `build_proxy.py`, `midi_proxy.c`, `forwarders.def`, `winmm.def`, `winmm.dll` (+`suite_local/`) | instrumentação do Suite oficial |
| **Catálogo** | `effect_catalog.csv`, `build_catalog.py`, `parse_*.py` | 909 slots catalogados |
| **Strings/extração** | `extract_strings.py`, `exe_strings.txt`, `fw_strings.txt`, `rtf_text.py`, `pdf_text.py`, `manual_v18.pdf`, `manual_v18.txt`, `manual_*.py`, `manual_streams.txt`, `release_note.txt`, `screens.html` | matéria-prima textual (manual V1.8 extraído p/ o afinador) |
| **Notas de revisão** | `notes/revision/` (ex.: `manual_v18_vs_firmware_v21.md`) | comparações manual × firmware com evidência |
| **IR** | `gen_test_ir.py`, `test_ir_*.wav` | IRs sintéticos p/ teste |
| **Material extraído** | `nsis_app/` (read-only), `driver_ext/`, `suite_local/` | não indexar; ver `.codebuffignore` |

### `files/` — artefatos oficiais de entrada (não modificar)
Instaladores, firmware V2.1, manual, driver, DebugView, screenshots,
`patches/*.prst` (biblioteca) e `prompt_inicial.md` (briefing original).

### `scripts/` — gates executáveis (o que roda onde está em `docs/CONTRIBUTING.md` §4)
| Script | Papel | Roda em |
|---|---|---|
| `ci_plan.py` | Fonte de verdade da regra branch → escopos → matriz/stágios | `Validação · plano e escopo` |
| `validate_workflows.py` | Sintaxe + contratos do pipeline + trigger (proíbe branch de trabalho no `push`) | `Lint · contratos do pipeline` |
| `check_bundle.py` | Schema do Tauri, ícones (16/32/48/256 + integridade PNG), `.deb`/PKGBUILD/`.desktop`, `targets` vs job de dist, tarball de fonte | `Lint · contratos do pipeline` (#71) |
| `check_base_images.py` | Toda `FROM`/`COPY --from=` resolve o default de `ARG` e sonda o registro antes do build | `Infra · imagens de container` |
| `simulate_release.py` | Executa os blocos rc/promote do PRÓPRIO ci.yml num sandbox git | `Lint · contratos do pipeline` |
| `check_commits.py` | Conventional commits (`--no-merges`) | `Lint · mensagens de commit` |
| `ci_report.py` / `security_report.py` / `ci_timings.py` | Step summary, achado de segurança como issue, medição de tempos | `Relatório · resumo` / `Segurança · auditorias` |
| `make_icon.py` | Gera a fonte 1024² do ícone a partir dos tokens do tema | local (`tauri icon` deriva o set) |
| `gates.py` | **Roda todos os gates de script de uma vez**, na ordem do CI (`--list` mostra a lista) | local (`python3 scripts/gates.py`) |
| `make_sdist.py` | Tarball de fonte com o `ui/dist` embutido — insumo do PKGBUILD | `Distribuição · tarball de fonte` |
| `sync_version.py` | Os 5 manifests de versão em sincronia (`--check` é o gate) | `Lint · contratos do pipeline` |
| `check_deadcode.py` | Todo `export` de `src/` tem consumidor fora do arquivo — cobertura 100% não prova que o contrato é consumível (#78) | `Lint · UI` |
| `add_cargo_path.ps1` / `h1_field.sh` / `h1_compare.py` | Fix do PATH do cargo (HKLM) · runbook do gate H1 · juiz da Fase C do H1 (níveis 1/2/3 + tabela do §5; `--self-check` é gate na CI) | local / campo |

### Raiz do repo — governança
`LICENSE` (MIT — declarado no `Cargo.toml` e no `PKGBUILD`),
`SECURITY.md` (política de segurança/secrets/material proprietário),
`.github/ISSUE_TEMPLATE/` (bug, feature, descoberta de protocolo) +
`.github/PULL_REQUEST_TEMPLATE.md` (checklist do gate + R1–R4),
`.gitignore`/`.gitattributes` (o que nunca entra — README §7).

### Infra do agente
`knowledge.md` (estado vivo + armadilhas), `.codebuffignore`,
`.agents/skills/{proxy-build,capture-analyze,new-session,spec-baseline,protocol-validate,core-dev,docs-sync,rust-practices,ui-ux-practices,github-flow}/SKILL.md`
— 10 skills: **fluxo de trabalho** (`github-flow`, `new-session`) · **desenvolvimento**
(`core-dev`, `rust-practices`, `ui-ux-practices`) · **qualidade/infra**
(`protocol-validate`, `proxy-build`, `capture-analyze`) · **governança**
(`spec-baseline`, `docs-sync`).

⚠️ As skills `rust-practices`, `ui-ux-practices`, `core-dev` e `github-flow` ainda descrevem o pipeline **pré-#68** (`_validate.yml`, `release.yml`, `container.yml`, `front-gate`, `close-linked`, label `ci-lite`) — issue #74 aberta. Tratar a skill como fonte de regra **e** conferir o `ci.yml` antes de agir.

### `packaging/` — recipes de pacote fora do Tauri
`packaging/arch/PKGBUILD` + `packaging/arch/gp100-nextgen-editor.desktop` (#29 / REL-ARCH).
O Tauri não gera pacote Arch: a unidade de distribuição **é** a receita versionada.
⚠️ `sha256sums=('SKIP')` bloqueia o AUR e o `build()` não monta o `ui/dist` (#72).

## 4. Convenções de documentação

1. **Todo documento tem um banner de status** nas primeiras linhas: ✅ atual /
   📜 histórico / ⚠️ parcial — e a data da última revisão.
2. **Descoberta nova de protocolo** → entra no `PROTOCOL.md` (com evidência) +
   no `protocol_golden.json` (via `build_golden.py`) + valida com
   `validate_golden.py`. Nunca só conversa.
3. **Mudança de status de marco** → `BLOCKERS.md` (matriz) + `knowledge.md`
   (estado vivo) + este índice (se criar/mover documento).
4. **Documento obsoleto não é apagado**: ganha banner 📜 HISTÓRICO apontando
   para o substituto.
5. Nomenclatura: docs de referência em `docs/`; produtos de análise em
   `analysis/`; anything fora disso é material de origem (`files/`).

## 5. Pendências de documentação (pequenas, não bloqueiam)

- ~~Tabela de TIPOS (`12001002`)~~ ✅ **FECHADO 28/09**: era a tabela dos 20
  User IRs — layout decifrado em §13.12 (nome 32B + CRC32; vazio = ppIRCRC
  do .prst); pendência menor: CRC de slot ocupado e byte [32]
- ~~Schema do `11000007`~~ ✅ **FECHADO 28/09**: 50B sempre zeros nas 3 amostras
  (provável campo reservado ppAuthor/ppNotes) — documentado em §13.12
- Blob de IR no device (campo 0x00BC/0x00B4, alinhamento int24) → §13.7
- Semântica fina de ppEXP1/ppCtrl (expCode) → §13.9
- Capturas G3–G6 (globals/BPM, knob físico, footswitch) → `capture_gaps.md`

---

## 6. Estado atual (números são a fonte única — **06/10**)

> Regra do `docs-sync`: **contador que aparece em mais de um documento é dívida**.
> Este bloco é a fonte; qualquer outro lugar aponta para cá em vez de repetir
> o número (item novo da skill `docs-sync`, issue #81).
>
> **Todos os números abaixo foram MEDIDOS em 06/10/2026**, com o comando da
> própria linha. Nenhum foi herdado de uma revisão anterior — a revisão de 03/10
> trazia 189 testes de front onde havia 399, e 10 de spec onde havia 81. Se um
> número divergir do que você vê, ele é que está errado: remeça e corrija aqui.

### Entregue até 06/10
| Fase/área | Estado |
|---|---|
| **P0–P5** (preparação) | ✅ ADR-1..9 em `DECISIONS.md` |
| **M0** (gp100-core) | ✅ M0.1–M0.8 — round-trip `.prst` byte-idêntico, replay das 4 fixtures |
| **M1** (Editor UI) | ✅ M1.0–M1.3 + V-8 (#20) + i18n pt/en/es/zh (#30) |
| **M2** (#24/#25/#26) | ✅ IR lab, SnapTone/NAM e biblioteca SQLite |
| **M3** (#113) | ✅ Biblioteca versionada: historico append-only, diff por knob e restauracao (tela `HistoryPanel`) |
| **M3** (#114) | ✅ Export universal: JSON versionado com round-trip byte-identico, folha de timbre em PDF (sem dependencia nativa), o recorte de um preset do `all.prst`, os tres commands da casca e a **tela** (`PresetFilePanel`, atras da porta do conteudo) — o e2e exporta um preset de fabrica e reimporta com a cadeia preservada |
| **M3** (#115) | ✅ Assistente de gain staging: `gp100_core::gain` (funcao PURA sobre o `BoardView`), o command `preset_gain_report` e a **tela** (`GainPanel`, 4o destino da porta) — relatorio por modulo com a **origem de cada numero**, o metodo e a LIMITACAO impressos na tela, exclusoes declaradas, risco e ordem de ajuste. NENHUM write: nenhum botao no painel manda byte para o aparelho; o e2e abre, le e fecha |
| **ACHADOS** | ✅ A-1..A-5 |
| **CI** (#68) | ✅ **1 workflow, 26 jobs** (o `gh pr checks` mostra mais porque os jobs de matriz se desdobram) |
| **Empacotamento** (#27/#28/#29) | ✅ 5 targets Tauri (`nsis`/`msi`/`dmg`/`deb`/`appimage`) · 18 arquivos de ícone (16 PNG + `.ico` + `.icns`) |
| **Auditoria 03/10** (#71–#83) | ✅ as 13 issues fechadas |
| **H** (gate de hardware) | ✅ **os 3 gates fechados em campo**: H1 (#21, PR #109) · H2 (#22, PR #108) · H3 (#23, PR #96) |
| **#126** (release) | ✅ **face (A) de leitura**: o `dist-ui` constrói com `--features real-device` (caminho `../ui` corrigido — o step antigo morria antes de compilar, `libasound2-dev` no Linux), a **política de escrita está na tela** (`escritaLiberada()`: knob/IR/SnapTone desabilitados COM O MOTIVO, select liberado por ser `WireKind::Read`) e o **keepalive do boot é omitido com a trava fechada** (ADR-5 rev. 06/10) |

### Contagem de testes (medido 06/10)
| Suíte | Comando | Contagem |
|---|---|---|
| Unit do front | `pnpm exec vitest run` | **485** em **36** arquivos (485 passando) |
| Cobertura do front | `pnpm run test:coverage` | **87,99%** stmts · **81,50%** branch · **87,20%** funcs · **90,34%** linhas |
| E2E (Playwright) | `pnpm exec playwright test --list` | **87** testes em **11** arquivos · **96** baselines |
| Rust (core + cli + library) | `cargo test --workspace` | **309** testes em **36** suítes (34 binários de teste + 2 suites de doc-test) |
| Spec (pytest) | `.venv/Scripts/python.exe -m pytest` | **92** |
| Gates locais | `python scripts/gates.py` | **13** |

> ✅ **A cobertura voltou a imprimir número em 05/10.** Ela vinha sem valor desde
> a revisão anterior, por uma falha intermitente de `testTimeout` que trocava de
> arquivo a cada rodada — o sintoma de `maxWorkers: 4` dimensionado para uma
> suíte de 15 arquivos, contra uma que já passou de 30. Três rodadas seguidas de
> `test:coverage` fecharam limpas, e o número acima é de uma delas.

### Trabalho aberto (milestone `v1.0.0`)
| # | Título | Bloqueio |
|---|---|---|
| ~~[#21](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/21)~~ | H1: primeiro contato real — **FECHADA em 05/10**: sessão real executada e arquivada em `analysis/captures/sessionH1/` (PR #109); juiz: níveis 1 e 3 limpos, divergências só de nível 2 (conteúdo, esperado) | ✅ |
| ~~[#110](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/110)~~ | `set_param_payload` aceita valor fora da faixa e trava o firmware do GP-100 — **FECHADA**: a trava de conteúdo virou o ADR-10 | ✅ |
| ~~[#15](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/15)~~ | EPIC M2 — **FECHADA em 05/10**: conteúdo entregue (#24 IR lab · #25 SnapTone/NAM · #26 biblioteca); a distribuição é a #17 | ✅ |
| ~~[#16](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/16)~~ | EPIC M3 — **FECHADA**, e destrinchada nas oito temáticas **#113–#120** (biblioteca versionada, export universal, gain staging, A/B, live mode, DAW, tone match, cloud) | ✅ |
| ~~[#113](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/113)~~ | M3-1 Biblioteca versionada — **FECHADA**: histórico append-only no `gp100-library` (migration v6, trigger anti-reescrita, diff por knob), 5 commands Tauri e a tela `HistoryPanel` (par antes/depois visível, restauração total e pontual que **acrescenta** versão) | ✅ |
| ~~[#114](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/114)~~ | M3-2 Export universal — **FECHADA**: JSON versionado com round-trip byte-idêntico, folha de timbre em PDF sem dependência nativa, os 3 commands da casca e a tela `PresetFilePanel` (export + reimport, e2e com a cadeia preservada) | ✅ |
| ~~[#115](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/115)~~ | M3-3 Assistente de gain staging — **FECHADA**: `gp100_core::gain` (função pura), `preset_gain_report` e a tela `GainPanel` com a origem de cada número, o método e a limitação NA TELA; nenhuma escrita (prova com `ByteGuard` no core) | ✅ |
| ~~[#126](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/126)~~ | `dist-ui` na face (A) de leitura — **FECHADA em 06/10**: `tauri build --features real-device` com o caminho `../ui` corrigido (o step antigo morria antes de compilar: 0 runs para `v0.1.0` e `v0.2.0-rc.*`) + `libasound2-dev` no Linux; a política de escrita na tela (`writeLock.ui.test.tsx` + `writeLock.spec.ts`); o keepalive do boot omitido com a trava fechada (ADR-5) | ✅ |
| [#17](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/17) | EPIC Release v1.0.0 multiplataforma | — |
| ~~[#18](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/18)~~ | EPIC Gate H — **FECHADA**: #21 ✅ (H1 executado, PR #109) · #22 ✅ (H2 em campo, PR #108) · #23 ✅ (H3 congelado, PR #96) | ✅ |

> Fechadas desde a revisão de 03/10: **#13**, **#14** (épicas de UI), **#19**,
> **#20**, **#24**, **#25**, **#26**, **#27**, **#28**, **#29**, **#30**,
> **#22**, **#23**, **#113**, **#114**, **#115**, **#126** e as **13 da auditoria** (#71–#83).

### Datas dos marcos
`01/10` Issues + #20 · `02/10` #45 baselines win32 · `02/10` **#68 CI consolidada** ·
`03/10` #27/#28/#29 empacotamento · `03/10` **auditoria completa** ·
`05/10` **#23 H3 congelado** (PR #96) · `05/10` **#21 sessão H1 real** (PR #109) ·
`05/10` **#22 H2 em campo — os 3 fluxos de escrita verdes** (PR #108), com o achado
que virou a #110
`06/10` **#126 release na face (A) de leitura** — `dist-ui` com a feature ligada,
trava de escrita na tela e keepalive do boot omitido (ADR-5 rev. 06/10)
