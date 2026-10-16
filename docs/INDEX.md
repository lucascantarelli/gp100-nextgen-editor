> ✅ **atual** · última revisão 2026-10-14.

# 📇 INDEX — Mapa da documentação (GP-100 NextGen Editor)

> **Comece aqui.** Este índice diz o que cada documento é, quando ler e qual é a
> fonte de verdade de cada assunto. Mantenha-o atualizado a cada novo documento
> ou mudança de status — é o contrato de navegação entre agentes e humanos.

**Última revisão:** 2026-10-13 (**docs categorizada**, #167 — árvore nova em `dev/` `tecnico/` `planejamento/` `analises/` `referencias/` `arquivo/`; todos os caminhos do repo atualizados; golden bump v1.3 só de proveniência; `SECURITY.md` virou doc oficial em `tecnico/seguranca.md` com o root apontando; `files/` triado no disco. Revisão anterior: 2026-10-09 (**Era Hardware aberta** — roadmap reescrito (`docs/planejamento/ROADMAP.md`, histórico em `docs/arquivo/ROADMAP_era-RE_2026-09.md`); trabalho aberto no épico **#171** + filhas **#161–#170** (milestone **v1.1.0**): boot gate, `FactoryPatch`/`UserPatch`, dados canônicos fora de `docs/arquivo/analysis/`, sync bilateral, real-por-default no dev, suíte enxuta, docs oficial, scripts dissolvidos, CI na ordem do dono e harness do agente; issues antigas fechadas com veredito. Números: **§6**. Última revisão anterior: 2026-10-06 — **gate H em campo** — o H2 passou nos 3 fluxos de escrita contra o GP-100 V2.1 real (#22, PR #108) e o H3 congelou a baseline (#23, PR #96); **o gate H inteiro fechou em campo** (#21 H1 · #22 H2 · #23 H3). **a épica M2 (#15) fechou com o gate H** — conteúdo (#24/#25/#26) entregue, distribuição é a #17. Auditoria 03/10 fechada (#71–#83). **Trabalho aberto vive em ISSUES do GitHub** (milestone **v1.0.0**; abertas em 06/10: **#17** — release, bloqueada no aparelho — e as tres medições de campo **M3** (#116–#118); #113/#114/#115 entregues, #119/#120 saíram da v1.0.0, a #110 fechou com a trava de faixa do ADR-10). Estado atual e números: **§6** — todos remedidos nesta data, não herdados.)

---

## 1. Rotas rápidas ("quero…")

| Objetivo | Rota de leitura |
|---|---|
| **Entender o projeto** | `README.md` → `docs/planejamento/VISION.md` |
| **Saber o estado atual (o que foi entregue, quantos testes, o que está aberto)** | **`docs/INDEX.md` §6** (fonte única dos números) |
| **Contribuir (issue → branch → PR → merge)** | `docs/CONTRIBUTING.md` (GitFlow, conventional commits, gates por área, fechamento automático da issue) |
| **Saber o que a suíte de testes cobre / adicionar um teste / o que NÃO entra / proteções que não caem** | **`docs/tecnico/TESTING.md`** (contrato da suíte: rodar, critérios de admissão de teste novo, regras de exclusão, DoD #5) | **Executar o plano de desenvolvimento** | **Issues do GitHub** — milestone **v1.1.0**: épico [#171](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/171) + filhas **#161–#170** (cada uma com prompt de agente pronto) — e o **`docs/planejamento/ROADMAP.md`** (Era Hardware) como documento-mestre; o histórico entregue (v1.0.0) está em `docs/arquivo/ROADMAP_era-RE_2026-09.md` |
| **Planejar o lançamento (release multiplataforma)** | `docs/planejamento/RELEASE_PLAN.md` (épicos EPIC-01..05 + issues REL-*: binários para download direto na GitHub Release, smoke em ambiente limpo, assinatura, auto-update — sem stores por decisão do owner) |
| **Levar o gp100-core à pedaleira (gate H1)** | `docs/arquivo/H1_CHECKLIST.md` (checklist) + `docs/arquivo/H1_REPORT.md` (relatório/plano de backup) + `docs/arquivo/scripts-era-RE/h1_field.sh` (runbook: rehearsal/field/refresh-reference) + `docs/arquivo/scripts-era-RE/h1_compare.py` (juiz da Fase C: classifica a sessão nos 3 níveis e imprime a tabela do §5) |
| **Escrever no device real (gate H2)** | `docs/arquivo/H2_CHECKLIST.md` (3 fluxos, um por vez) + `docs/arquivo/H2_REPORT.md` (relatório) — build com `--features real-device,write-verified` |
| **Congelar a especificação depois do hardware (gate H3)** | `docs/arquivo/H3_CHECKLIST.md` (4 sessões) + `docs/arquivo/H3_REPORT.md` + [`PROTOCOL.md` §13.14](tecnico/PROTOCOL.md) (a conta da baseline v1.1) + `tools/baseline.py show` |
| **Medir as páginas 13xx em campo (gate H4)** | `docs/arquivo/H4_CHECKLIST.md` (é leitura — nada escreve no aparelho pelo app: as edições do roteiro são feitas no pedal) + `docs/arquivo/H4_REPORT.md` (relatório) + `tools/validate_state_pages.py` (gate do artefato de offsets) |
| **Implementar o protocolo (gp100-core)** | `docs/referencias/protocol_golden.json` (especificação executável) + `docs/tecnico/PROTOCOL.md` §13 (narrativa) |
| **Decidir arquitetura/estrutura no core (M0)** | `docs/dev/DECISIONS.md` (ADR-1..14 aceitos) |
| **Saber onde o código novo do front vai morar** | `docs/dev/ARCHITECTURE.md` (mapa de módulos + o que **não** entra em cada camada + orçamento de tamanho) |
| **Saber onde a persistencia do app mora (M2)** | `docs/dev/DECISIONS.md` ADR-9 (crate `gp100-library` no workspace gnu, nao no crate MSVC do Tauri) |
| **Escrever/revisar código Rust (M0)** | `.agents/skills/rust-practices/SKILL.md` (gates fmt/clippy/test + estilo de docs) |
| **Modificar o protocolo / analisar nova captura** | `tools/wirelog.py` (normaliza o log) → `tools/build_golden.py` → `tools/validate_golden.py` → `tools/baseline.py bump` (fluxo R2/R3; processo detalado no histórico `docs/arquivo/skills-era-RE/capture-analyze.md`) |
| **Compilar o proxy / nova captura em campo** | 📜 histórico: `docs/arquivo/skills-era-RE/{proxy-build,new-session}.md` + `docs/arquivo/analysis/build_proxy.py` — a captura de hoje sai do `--log` do `gp100-cli` |
| **Consumir dados de preset/efeitos** | `packages/core/data/parameters.json` (dicionário) + `docs/tecnico/PROTOCOL.md` §13.9 |
| **Saber o que falta / riscos** | `docs/planejamento/BLOCKERS.md` + `docs/arquivo/analysis/capture_gaps.md` |
| **Contexto de decisões de arquitetura/stack** | `docs/planejamento/VISION.md` §6–§9 |
| **Armadilhas de ambiente (Windows/Git Bash)** | `knowledge.md` |

## 2. Fonte de verdade por assunto

| Assunto | Fonte única | Não consultar (obsoleto/supersedido) |
|---|---|---|
| Protocolo de fio (bytes) | `docs/referencias/protocol_golden.json` | reparsear `captures/*.jsonl` direto |
| Protocolo de fio (semântica/narrativa) | `docs/tecnico/PROTOCOL.md` §13 | — |
| Formato de ARQUIVO (TLV/CRC-8/objetos) | `docs/tecnico/PROTOCOL.md` §1–12 | §13 (é fio, não arquivo) |
| Dicionário de efeitos/parâmetros | `packages/core/data/parameters.json` | `algorithm_dict.*` (intermediários) |
| Formato `.prst` | `docs/tecnico/PROTOCOL.md` §13.9 | — |
| Envelope do knob / save / IR no fio | §13.11 / §13.12 / §13.7 | leituras antigas do §13.4 (marcadas) |
| Estado do projeto / próximos passos | `docs/planejamento/BLOCKERS.md` + `knowledge.md` (estado vivo) | `docs/arquivo/CAPTURE_PLAN.md` (histórico) |
| Números (testes, cobertura, datas dos marcos) | `docs/INDEX.md` §6 | repetir o número em README/ROADMAP/knowledge (#81) |
| Contrato da suíte de testes (o que testa, como rodar, o que NÃO entra, proteções) | `docs/tecnico/TESTING.md` | repetir gates por área (é do `CONTRIBUTING` §4) nem roteiros e2e (é do `UI_TEST_PLAN`) |
| Estrutura do pipeline (jobs, tipos, ordem) | `.github/workflows/ci.yml` + `docs/CONTRIBUTING.md` §5 | workflow pré-#68 (`_validate.yml`/`release.yml`/`container.yml`) — **consolidados no #68; não existem mais** |
| Mapa knob→fio | `tools/knob_map.json` (regenerável) | — |
| Decisões de implementação do core | `docs/dev/DECISIONS.md` (ADR-1..14) | reabrir debate ad-hoc |
| Armadilhas Windows/ambiente | `knowledge.md` | — |

## 3. Inventário de documentos

### `docs/` — a árvore (#167)

```
docs/
├── INDEX.md              # este mapa — fonte única dos números (§6)
├── CONTRIBUTING.md       # exceção documentada: o GitHub auto-detecta CONTRIBUTING.md em docs/
├── dev/                  # quem desenvolve: arquitetura, decisões (ADR), UI
├── tecnico/              # doc técnica: protocolo, testes, distribuição, segurança, gate H5
├── planejamento/         # roadmap, release, visão, bloqueios
├── analises/             # auditorias e achados (histórico de revisões)
├── referencias/          # golden, cobertura do manual, device, UI reference
└── arquivo/              # era RE + gates H fechados + cápsula analysis/ + skills-era-RE
```

Convenção (#167): **docs movidos mantêm o nome original** (identidade e grep estáveis
nas 100+ referências); **docs novos nascem em kebab-case minúsculo** (ex.:
`tecnico/seguranca.md`). `CONTRIBUTING.md` fica na raiz de `docs/` por detecção
automática do GitHub.

### `docs/dev/` — desenvolvimento
| Documento | Papel | Status |
|---|---|---|
| `ARCHITECTURE.md` | **Estrutura do front** (#82) e o **orçamento do Rust** (#248, §4.1): mapa de módulos (`ipc`/`hooks`/`components`/`design`/…), o que **não** entra em cada camada, árvore de decisão para código novo e os tetos de tamanho cobrados pelo gate `check_module_size.py` (front e `packages/*/src`) | ✅ atual (#82, #243, #248) |
| `DECISIONS.md` | ADR-1..15: decisões de arquitetura/stack do core e do app (cada uma com contexto, decisão e consequência) | ✅ atual |
| `UI_DESIGN.md` | Design system da UI: paleta palco Valeton (âmbar/preto/vermelho/lavanda com rácios WCAG medidos), escala de Fibonacci, tipografia, motion, identidade "pedalboard ao vivo", checklist de review | ✅ atual (M1.0) |
| `UI_PLAN.md` | Planejamento issue-a-issue da Fase M1 (Editor UI Tauri/React): escopo, arquitetura DeviceActor, superfície IPC, telas, política de hardware, testes, riscos | 🔨 M1.0 ✅ (ADR-7) · M1.1 ✅ (actor + boot com barra) · M1.2/M1.3 ✅ (palco real + afinador) · V-8 ✅ (#20) · i18n ✅ (#30); M2 planejada (#24–#26) |
| `UI_TEST_PLAN.md` | **Fonte dos roteiros e2e** (declarada no `playwright.config.ts`): R1–R6 + drum/looper, matriz de viewports, política de baselines | ✅ atual |

### `docs/tecnico/` — documentação técnica
| Documento | Papel | Status |
|---|---|---|
| `PROTOCOL.md` | Referência única do protocolo: §1–12 formato de arquivo, §13.1–13.15 fio confirmado em campo (inclui o sync por poll + read-back da #164) | ✅ atual |
| `TESTING.md` | **Contrato da suíte de testes** (#166): as 4 suítes + gates, como rodar, o que um teste novo precisa provar (5 portas), o que **não** entra (anti-espelho da RE, dado de `docs/arquivo/analysis/`, duplicata) e as 6 proteções DoD #5 — aponta CONTRIBUTING/UI_TEST_PLAN/rust-practices em vez de duplicar | ✅ atual (11/10) |
| `PACKAGING.md` | Empacotamento e distribuição (#27/#28/#29): o mapa um-build→três-formatos, cada campo do `tauri.conf.json` e por quê, o pipeline de ícones (fonte 1024² derivada dos tokens do tema), as duas vias do `.deb` e a receita do Arch, mais o gate `check_bundle.py` | ✅ atual |
| `seguranca.md` | **Doc oficial de segurança** (#167): política de reporte, dados locais do app, wire log, credenciais e material proprietário — o `SECURITY.md` da raiz aponta para cá | ✅ novo (13/10) |
| `H5_CHECKLIST.md` | Gate de campo H5 (épico #174, HW-11): roteiro assinado do boot, modelo tipado e sincronização | 🔨 gate em curso |

### `docs/planejamento/` — roadmap e planos
| Documento | Papel | Status |
|---|---|---|
| `ROADMAP.md` | Plano executivo da Era Hardware: preparação (P), gp100-core (M0), gate de hardware (H) com issues e critérios de aceite | ✅ atual |
| `RELEASE_PLAN.md` | Plano pré-lançamento: 5 épicos (binários multiplataforma para download direto na GitHub Release — sem stores, decisão do owner, smoke em CI limpo, assinatura/notariação, canal único coeso, auto-update/rollback) com issues REL-* prontas para abrir | ✅ atual rev. 2 (issues a criar) |
| `VISION.md` | Visão de produto/arquitetura, stack, features, roadmap M0–M3 (rev. v1.1) | ✅ atual |
| `BLOCKERS.md` | Matriz de 12 subsistemas; 11 resolvidos, firmware-update diferido | ✅ atual |

### `docs/analises/` — auditorias e achados (histórico)
| Documento | Papel | Status |
|---|---|---|
| `audit-2026-10-14.md` | **Auditoria ampla (14/10)**: docs, Rust, front, CI/DevOps, latência MIDI e cobertura do manual — 18 achados (F-01…F-18) com severidade e backlog priorizado; faxina dos artefatos locais não rastreados (`re_tmp`, `superpowers`, `test-results`) | ✅ atual (14/10) |
| `audit-harness-2026-10-14.md` | **Auditoria do harness (14/10)**: as 9 skills e os 6 prompts de `.agents/` — 12 achados (H-01…H-12), plano de consolidação em 3 commits e a tabela de fontes únicas (issue [#249](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/249)) | ✅ atual (14/10) |
| `medicoes-datadas-2026-10-15.md` | **Registro das medições datadas (15/10)**: os contadores que saíram dos documentos vivos na [#270](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/270) — diagnóstico da virada (09/10), rodadas de teste manual da UI (set/out) e números de prosa — cada um com a data e a origem; o doc vivo aponta para cá **ou** para o §6 | 📜 histórico (registro da #270) |
| `audit-2026-10-07.md` | **Auditoria completa (07/10)**: code review + governança + arquitetura + limpeza em todo o repo, com números remediados, achados F-01…F-12 e backlog A-1…A-10 (desfecho na §10) | 📜 histórico (desfecho 08/10) |
| `audit-era-real-2026-10-07.md` | Auditoria da Era Real (07/10): estado do app real vs mock | 📜 histórico |
| `skills-audit-2026-09-29.md` | Auditoria das skills: regras que eram prática implícita, agora escritas (5 achados em core-dev/docs-sync/spec-baseline) | 📜 histórico |

### `docs/referencias/` — referências de dado e contexto
| Documento | Papel | Status |
|---|---|---|
| `protocol_golden.json` | 39 templates request→resposta das capturas 1–4; **spec executável consumida pelo gp100-core** (`include_str!` em `golden.rs`); baseline v1.3 (`tools/baseline.json`) | ✅ atual · **v1.3** (§13.14) |
| `MANUAL_COVERAGE.md` | Matriz de cobertura do manual oficial V1.8 → requisitos implementados (X1..Xn), com o gate que prova cada um | ✅ atual |
| `GP100_DEVICE.md` | **Contexto canônico da pedaleira real**: funcionalidades, comportamentos, vocabulário (User Patch × User IR × Setlist-inexistente), menu GLOBAL, lacunas de leitura do hardware e divergências expostas pela pesquisa de 10/10. Skill `device-reference` aponta pra cá | ✅ atual (10/10) |
| `REAL_DEVICE_GAP.md` | **O que o app fala com o aparelho, e o que ainda é mock** (#17/#16): o inventário core·CLI·app·aparelho real capacidade por capacidade, os pontos onde o comportamento muda com o aparelho ligado e o caminho até a release real | ✅ atual (vivo — atualizado a cada marco HW) |
| `UI_REFERENCE.md` | Referência da casca do front: papéis de tela, contratos de estado, **§8 = fonte de verdade do texto de usuário** (o lint de i18n aponta para cá) | ✅ atual (M1.3) |
| `packages/library/` | **Biblioteca persistente** (#26, ADR-9): SQLite com migracoes versionadas (`PRAGMA user_version`), busca por nome/nº/estilo, import/export JSON versionado e seed dos 99 presets de fabrica do `all.prst`. Crate do workspace gnu, sem Tauri | ✅ atual (#26) |

### `docs/arquivo/` — histórico (era RE e gates fechados)
| Documento | Papel | Status |
|---|---|---|
| `H1_CHECKLIST.md` · `H1_REPORT.md` | Gate H1 (primeiro contato real, só leitura) — roteiro + relatório; a sessão de campo está em `analysis/captures/sessionH1/` | ✅ gate fechado em campo (§6) |
| `H2_CHECKLIST.md` · `H2_REPORT.md` | Gate H2 (escrita real — muda estado do aparelho): os 3 fluxos e o veredito | ✅ gate fechado em campo (§6) |
| `H3_CHECKLIST.md` · `H3_REPORT.md` | Gate H3 (congelamento da especificação): as 4 sessões e o veredito — a baseline saiu daqui | ✅ gate fechado em campo (§6) |
| `H4_CHECKLIST.md` · `H4_REPORT.md` | Gate H4 (páginas 13xx em campo — leitura) | ✅ gate fechado (§6) |
| `CAPTURE_PLAN.md` | Plano original das rotas de captura | 📜 histórico (cumprido) |
| `ROADMAP_era-RE_2026-09.md` · `knowledge-era-RE_2026-10.md` | Roadmap e knowledge da Era de Engenharia Reversa (supersedidos) | 📜 histórico |
| `skills-era-RE/` | As 5 skills da era RE (`proxy-build`, `capture-analyze`, `new-session`, `spec-baseline`, `protocol-validate`) | 📜 histórico |
| `analysis/` | Cápsula da Era de RE (abaixo) — proveniência, não processo | 📜 histórico |

### `docs/` (raiz) — governança de navegação
| Documento | Papel | Status |
|---|---|---|
| `INDEX.md` | Este índice | ✅ manter atualizado |
| `CONTRIBUTING.md` | Fluxo de contribuição: GitFlow (develop como integração), conventional commits, gates locais por área (coverage 85% incluso), template de PR e fechamento automático de issue no merge em develop (workflow próprio `issues-fechamento.yml` desde a #169; a `main` só recebe o SHA de uma tag `v*`) | ✅ atual |
| `tools/wirelog.py` · `tools/baseline.py` · `tools/validate_core_capture.py` | O trilho do H3: normalizador dos dois schemas de log de fio · baseline versionada (versão + hash + motivo + histórico) · juiz frame a frame de uma captura do gp100-core contra a spec | ✅ (`tools/baseline.py show`) |


### `docs/arquivo/analysis/` — cápsula da Era de RE (histórico, não processo)

O laboratório inteiro foi dissolvido na #163. O que ainda **roda** saiu daqui —
ferramentas de spec/dados para `tools/`, dado canônico do runtime para
`packages/core/data/` — e o resto ficou como **proveniência**: de onde veio cada
número, para sempre. Nada aqui é insumo do runtime: o `guard_dados_canonicos.rs`
reprova qualquer `analysis/` dentro de `packages/`.

| Grupo | Onde (sob `docs/arquivo/analysis/`) | Papel |
|---|---|---|
| **Capturas** (proveniência do golden) | `captures/session1–4.jsonl`, `sessionMap.jsonl`, `sessionG3-G6.jsonl`, `captures/sessionH1/`, `field/` | matéria-prima bruta das provas A–E (append-only) — insumo de `tools/build_golden.py` e `tools/make_fixtures.py` |
| **Origem do dicionário** | `algorithm.xml` (payload oficial da Suite V1.5.1), `effect_catalog.csv`, `algorithm_dict.*`, `build_parameters.py` | de onde saíram os 185 algs / 639 controles de `packages/core/data/parameters.json`; o gerador é histórico (exige `fw_strings.txt` local, que não versiona) |
| **Evidência dos gates H** | `h1_reference/`, `h2/`, `h2_reference/`, `h4/` (sessão 10/10, não rastreada) | o que `docs/arquivo/H1_REPORT.md`/`H2_REPORT.md`/`H4_*` citam; os juízes **arquivados** `docs/arquivo/scripts-era-RE/h{1,2}_compare.py` leem daqui |
| **Decoders e RE estática** | `decode_*.py`, `disasm_*.py`, `rtti_disasm.py`, `xrefs_opcodes.py`, `opcode_extract.py`, `FINDINGS_{PROTOCOL,FSM,OBJECTS,COMMANDMAP}.md`, `README_GHIDRA.md` | análise dirigida do binário do Suite; achados que corroboram o §13 |
| **Proxy de captura** | `build_proxy.py`, `midi_proxy.c`, `forwarders.def`, `winmm.*`, `suite_local/` (não rastreado) | como o Suite oficial era instrumentado para gravar sessão |
| **Catálogo, strings e manual** | `effect_catalog.csv`, `build_catalog.py`, `extract_strings.py`, `manual_v18.*`, `manual_*.py`, `manual_streams.txt`, `notes/`, `release_note.txt`, `screens.html` | matéria-prima textual (manual V1.8 extraído para o afinador) |
| **Mapas intermediários** | `dump_edit_writes.py`, `map_params_wire.py`, `scan_prst.py`, `check_orphans.py`, `check_session2.py` | derivados da RE que perderam função com o dado canônico |
| **IRs de teste** | `gen_test_ir.py`, `test_ir_*.wav` | IRs sintéticos usados no gate H2 |


### `files/` — artefatos de origem (não modificar; triados no #167)

| Item | Status | Motivo |
|---|---|---|
| `patches/*.prst` (3) + `prompt_inicial.md` | **versionados** | insumo do round-trip (R4) e do parse; briefing original (histórico) |
| manual PDF oficial | local | referência legal de protocolo/uso; não distribuído (README §8) |
| `firmware/` (V2.1) | local | recuperação de campo (reflash) — operação de risco, documentada no gate H |
| `driver/` | local | instalação do driver em máquina nova |
| `images/` | local | **proveniência do design**: capturas do Suite oficial citadas por `docs/referencias/UI_REFERENCE.md` + shots `responsivo-*.png` regeneráveis (`docs/arquivo/analysis/responsivo_shots.js`) |
| ~~`app/` (instalador 77 MB) · `DebugView/` · `logs/`~~ | **saiu do disco (13/10, #167)** | não são fonte: o aparelho é a fonte; logs são evidência temporária |

### `.github/scripts/` — gates executáveis (o que roda onde está em `docs/CONTRIBUTING.md` §4; era `scripts/`, dissolvida na #168)
| Script | Papel | Roda em |
|---|---|---|
| `ci_plan.py` | Fonte de verdade da regra branch → escopos → matriz/stágios | `Validação · plano e escopo` |
| `validate_workflows.py` | Sintaxe + contratos do pipeline + trigger (proíbe branch de trabalho no `push`) + a ligação **gate → job** (`GATES_LIGADOS`, contrato 16 — nenhum gate de script fica desligado, #273) | `Lint · contratos do pipeline` |
| `check_bundle.py` | Schema do Tauri, ícones (16/32/48/256 + integridade PNG), `.deb`/PKGBUILD/`.desktop`, `targets` vs job de dist, tarball de fonte | `Lint · contratos do pipeline` (#71) |
| `check_base_images.py` | Toda `FROM`/`COPY --from=` resolve o default de `ARG` e sonda o registro antes do build | `Infra · imagens de container` |
| `simulate_release.py` | Executa os blocos rc/promote dos reutilizáveis `release-*.yml` (conferindo o `uses:`/`with:` do caller no `ci.yml`) num sandbox git — no CI só por escopo: schedule, tag/dispatch e PR de release (#245/#246) | `Lint · contratos do pipeline` |
| `check_commits.py` | Conventional commits (`--no-merges`) | `Lint · mensagens de commit` |
| `ci_report.py` / `security_report.py` / `ci_timings.py` | Step summary, achado de segurança como issue, medição de tempos | `Relatório · resumo` / `Segurança · auditorias` |
| `packaging/make_icon.py` | Gera a fonte 1024² do ícone a partir dos tokens do tema | local (`tauri icon` deriva o set) |
| `gates.py` | **Roda todos os gates de script de uma vez**, na ordem do CI (`--list` mostra a lista) | local (`uv run python .github/scripts/gates.py`) |
| `packaging/make_sdist.py` | Tarball de fonte com o `ui/dist` embutido — insumo do PKGBUILD | `Distribuição · tarball de fonte` |
| `sync_version.py` | Os 5 manifests de versão em sincronia (`--check` é o gate) | `Lint · contratos do pipeline` |
| `check_deadcode.py` | Todo `export` de `src/` tem consumidor fora do arquivo — cobertura 100% não prova que o contrato é consumível (#78) | `Lint · UI` |
| `check_docs.py` | **Higiene da documentação**: link/âncora resolvem, todo doc tem banner de status com data e contador vive só neste §6 (#270) — teste em `tools/tests/test_check_docs.py` | gate local (`gates.py check_docs`) · `Lint · contratos do pipeline` |
| `check_module_size.py` | Orçamento de tamanho: front (`App.tsx` < 300 linhas) **e** os `.rs` de `packages/*/src` (#248) — tetos no §4/§4.1 do `docs/dev/ARCHITECTURE.md`; teste em `tools/tests/test_check_module_size.py` | gate local (`gates.py`) · `Lint · UI` · `Lint · Rust (fmt)` |
| `check_docs_modulos.py` | **Obsolescência do doc × módulo** (#275): compara a última revisão do documento com a **superfície** dos módulos que ele cita (ancestralidade de commit, não relógio; refatoração interna não cobra) e **aponta o documento**; isenção declarada com motivo, isenção órfã reprova — teste em `tools/tests/test_check_docs_modulos.py` | gate `check_docs_modulos` · CI `Lint · contratos do pipeline` |
| ~~`add_cargo_path.ps1` / `h1_field.sh` / `h1_compare.py`~~ (**arquivados** na #168 em `docs/arquivo/scripts-era-RE/`) | Fix do PATH do cargo (HKLM) · runbook do gate H1 · juiz da Fase C do H1 (níveis 1/2/3 + tabela do §5) | histórico / campo |
| ~~`h2_field.sh` / `h2_compare.py`~~ (**arquivados** na #168 em `docs/arquivo/scripts-era-RE/`) | Runbook do gate H2 (escrita) · juiz da transcrição do H2 contra a referência de escrita (#22) | histórico / campo |
| ~~`wirelog_compare.py`~~ (**arquivado** na #168 em `docs/arquivo/scripts-era-RE/`) | **Núcleo comum dos juízes H1/H2** (#141): o `Frame` do schema P4, o parser e os helpers de tabela | histórico |


#### Ferramentas de dados e da especificação (casa desde #163)

O que saiu de `analysis/` e **ainda roda** (CI, `gates.py`, pytest ou o fluxo
R2/R3). Tabela de proveniência completa no PR da #163.

| Ferramenta | Papel | Roda em |
|---|---|---|
| `build_golden.py` · `validate_golden.py` | Extrai os templates das 4 capturas → `docs/referencias/protocol_golden.json` e prova as 5 provas A–E | pytest · fluxo R2/R3 |
| `baseline.py` · `baseline.json` | Registro da baseline (versão + hash + **motivo** + histórico); `--check` é gate, `bump` exige justificativa | gate `baseline` · CI `Testes · Especificação` |
| `wirelog.py` · `validate_core_capture.py` | Normaliza os 2 schemas de log (Suite e gp100-core) · juiz frame a frame de uma captura do core | CI `Testes · Especificação` |
| `make_fixtures.py` | Regenera `packages/core/data/fixtures/` e trava a paridade com o golden | pytest |
| `param_ranges.py` · `param_ranges.json` | 639 faixas do knob + as 92 amostras reais que as corroboram (ADR-10) | gate `param_ranges` · CI `Testes · Especificação` |
| `validate_state_pages.py` | Layout das páginas 13xx contra o artefato embutido no binário (#155) | gate `state_pages` · CI `Testes · Especificação` |
| `map_state_pages.py` | Gera `packages/core/data/state_pages_offsets.json` a partir de S1 + `all.prst` | manual (gate de campo H4) |
| `check_byte_order.py` | effectCode é u32 **LE** no fio real — prova com o par do H2 (#23) | gate `byte_order` · CI `Testes · Especificação` |
| `validate_knob_map.py` · `knob_map.json` | Mapa knob→fio (§13.11): 13/14 no range + exceção CAB/Mic | pytest |
| `dump_fx_map.py` · `dump_drum_kit.py` · `dump_preset_chains.py` · `dump_preset_list.py` | Geram `packages/core/data/fx_map.json` e os artefatos de `packages/app/ui/src/artifacts/` (não editar à mão) | manual |
| `tests/` | A suíte **Spec** (pytest) — o que ela cobre está em `docs/tecnico/TESTING.md` §1 | CI `Testes · Especificação` |

### Raiz do repo — governança
`LICENSE` (MIT — declarado no `Cargo.toml` e no `PKGBUILD`),
`SECURITY.md` (política de segurança/secrets/material proprietário),
`.github/ISSUE_TEMPLATE/` (epic, feature, bug, refactor, protocolo — contrato
com seções obrigatórias + 🤖 prompt do agente, validado pelo gate
`.github/scripts/check_issue_templates.py`) +
`.github/PULL_REQUEST_TEMPLATE.md` (checklist do gate + princípios P1–P7),
`.gitignore`/`.gitattributes` (o que nunca entra — README §7).

### Infra do agente (#170)
`knowledge.md` (estado vivo + armadilhas), `.codebuffignore`, `.agents/README.md`
(mapa do harness), `.agents/prompts/` (6 templates: implementar-issue canônico,
executar-issues-em-sequencia, corrigir-bug, code-review-completo, auditoria,
release),
`.agents/skills/{github-flow,core-dev,rust-practices,app-dev,ui-ux-practices,testing,hardware-dev,device-reference,release,docs-sync}/SKILL.md`
— 10 skills da Era HW: **fluxo de trabalho** (`github-flow`, `release`) ·
**desenvolvimento** (`core-dev`, `rust-practices`, `app-dev` — o crate do app/Tauri,
`ui-ux-practices`, `hardware-dev`)
· **qualidade** (`testing`) · **governança** (`docs-sync`) · **conhecimento do
device** (`device-reference` → `docs/referencias/GP100_DEVICE.md`).

As 5 skills da Era de Engenharia Reversa (`proxy-build`, `capture-analyze`,
`new-session`, `spec-baseline`, `protocol-validate`) estão **arquivadas** em
`docs/arquivo/skills-era-RE/` — histórico, não processo.

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
5. Nomenclatura (**casa desde #167**, continuação da #163): docs categorizados
   em `docs/{dev,tecnico,planejamento,analises,referencias}/`; **dado canônico
   do runtime** em `packages/core/data/`; **ferramentas que rodam** (spec,
   dados, gates) em `.github/scripts/` (CI e gates) e `tools/` (spec/dados); histórico da Era de RE e gates H fechados em
   `docs/arquivo/`; material de origem (`files/`) fora de tudo. Docs movidos
   mantêm o nome; docs novos em kebab-case minúsculo.

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
> **Baseline:** os números abaixo são **medidos em `develop`**. Numa branch de feature
> eles divergem por construção (a branch *adiciona* testes) — divergência em branch **não**
> é erro deste §6; re-meça em `develop` antes de corrigir (auditoria 07/10, F-11, #146).
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
| **CI** (#68) | ✅ **1 workflow, 25 jobs** (o `gh pr checks` mostra mais porque os jobs de matriz se desdobram) — + **3 workflows desde a #169**: o pipeline (`ci.yml`, grafo na ordem do dono, Relatório por último, archlinux na matriz de OS), o `Issues · Fechamento de issues vinculadas` (`issues-fechamento.yml`) e a regeneração de baselines visuais (`visual-baselines.yml`, dispatch sem inputs e `contents: read` — #204) — e **+ 6 workflows reutilizáveis desde a #245** (`dist-instalador`/`dist-sdist`/`dist-cli`/`release-rc`/`release-promote`/`release-play`, `workflow_call` e um job cada): o corpo de Distribuição/Release saiu do `ci.yml`, que ficou com o grafo, o nome dos jobs e a matriz de OS |
| **Empacotamento** (#27/#28/#29) | ✅ 5 targets Tauri (`nsis`/`msi`/`dmg`/`deb`/`appimage`) · 18 arquivos de ícone (16 PNG + `.ico` + `.icns`) |
| **Auditoria 03/10** (#71–#83) | ✅ as 13 issues fechadas |
| **H** (gate de hardware) | ✅ **os 3 gates fechados em campo**: H1 (#21, PR #109) · H2 (#22, PR #108) · H3 (#23, PR #96) |
| **#126** (release) | ✅ **face (A) de leitura**: o `dist-ui` constrói com `--features real-device` (caminho `../ui` corrigido — o step antigo morria antes de compilar, `libasound2-dev` no Linux), a **política de escrita está na tela** (`escritaLiberada()`: knob/IR/SnapTone desabilitados COM O MOTIVO, select liberado por ser `WireKind::Read`) e o **keepalive do boot é omitido com a trava fechada** (ADR-5 rev. 06/10) |
| **M3** (#116) | ✅ **A/B com blind test** (parte sem aparelho): o par vem do histórico versionado (#113), a troca desenha no palco e manda só os knobs graváveis de slots de mesmo algoritmo (o `0x47` não tem formato capturado — BLOCKERS 10b, e a tela diz), o **nível é POSIÇÃO** com método e limitação impressos ao lado do número, a **calibração** iguala o lado alto ao baixo (medido: delta 40.0 → 0.0) e o **blind** esconde rótulo/versão/nível/relatório até o palpite (o teste de UI prova a AUSÊNCIA). **Medição em campo pendente de aparelho** |

### Contagem de testes (front re-medido em `develop` 08/10; gates e Spec re-medidos em 13/10 pós-#168; Rust/e2e em 06/10)
| Suíte | Comando | Contagem |
|---|---|---|
| Unit do front | `pnpm exec vitest run` | **513** em **38** arquivos (513 passando) · *08/10* |
| Cobertura do front | `pnpm run test:coverage` | **87,62%** stmts · **81,25%** branch · **86,78%** funcs · **90,04%** linhas · *08/10* |
| E2E (Playwright) | `pnpm exec playwright test --list` | **90** testes em **12** arquivos · **96** baselines · *08/10* |
| Rust (core + cli + library) | `cargo test --workspace` | **349** testes em **42** suítes (40 binários + 2 doc-tests) · *13/10* |
| Spec (pytest) | `uv run pytest` | **81** em **7** arquivos · *14/10, re-medido com `--collect-only` pós-#248 (o teste do gate de tamanho entrou; os juízes H arquivados seguem fora desde a #168)* |
| Gates locais | `uv run python .github/scripts/gates.py` | **15** · *15 em 15/10 — entraram a higiene da documentação (#270) e a obsolescência doc × módulo (#275); `--list` mostra a lista* |

> ✅ **Tempo da suíte do front (medido 08/10 na mesma base `develop`):**
> `pnpm exec vitest run` caiu de **84,7s → 52,3s** com `pool: 'vmThreads'` (#145, F-10):
> o jsdom deixou de ser criado **38× (113,4s = 39% do tempo rastreado)** e passou a ser
> reaproveitado por worker, mantendo o isolamento por arquivo. Antes e depois com os
> MESMOS **513 testes em 38 arquivos** (todos verdes) e a cobertura do gate intacta
> (87,62% ≥ 85%).

> ✅ **A cobertura voltou a imprimir número em 05/10.** Ela vinha sem valor desde
> a revisão anterior, por uma falha intermitente de `testTimeout` que trocava de
> arquivo a cada rodada — o sintoma de `maxWorkers: 4` dimensionado para uma
> suíte de 15 arquivos, contra uma que já passou de 30. Três rodadas seguidas de
> `test:coverage` fecharam limpas, e o número acima é de uma delas.

### Trabalho aberto (milestone `v1.0.0`)
| # | Título | Bloqueio |
|---|---|---|
| ~~[#21](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/21)~~ | H1: primeiro contato real — **FECHADA em 05/10**: sessão real executada e arquivada em `docs/arquivo/analysis/captures/sessionH1/` (PR #109); juiz: níveis 1 e 3 limpos, divergências só de nível 2 (conteúdo, esperado) | ✅ |
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
