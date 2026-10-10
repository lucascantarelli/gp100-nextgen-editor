# 🗺️ ROADMAP — Era Hardware (o app comandado pelo aparelho)

> **Documento-mestre da virada.** Substitui o roadmap da engenharia reversa, arquivado em
> [`docs/arquivo/ROADMAP_era-RE_2026-09.md`](arquivo/ROADMAP_era-RE_2026-09.md) — lá ficou o
> histórico do que foi entregue (P0–P5, M0, M1, M2, gates H1–H4, auditorias).
>
> **Épico no tracker:** [#171 — EPIC · Era Hardware](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/171)
> (milestone [`v1.1.0`](https://github.com/lucascantarelli/gp100-nextgen-editor/milestone/2)).
> Cada issue filha carrega o **prompt do agente pronto** no corpo.
>
> Revisão: **09/10/2026** · base `develop` `e133153` · todos os números deste documento foram
> **medidos nesta data** no repo (não herdados).

---

## 1. Por que a virada

O app foi construído na era de engenharia reversa: especificação, dicionário, offsets, nomes e
fixtures vieram de análise — e **isso está embutido no runtime até hoje**. Com o hardware na
mesa, a fonte da verdade muda para o aparelho, e o repo precisa refletir isso ponta a ponta.

### 1.1 O problema, medido

| Sintoma | Evidência no repo (09/10/2026) |
|---|---|
| Boot "trava"/mostra placeholder e hex; tela não interage | `useBoot.ts` roda o boot em background e `App.tsx` **renderiza a casca inteira sempre**; só existe faixa de progresso, não gate. Sem dado, painéis caem em fallback (`MSG.libPp(pp)`, `ipc/fallbackData.ts`) |
| Dados amarrados a análises | `core/src/model.rs` faz `include_str!("../../../analysis/parameters.json")`; `core/src/preset_pages.rs` embute `analysis/state_pages_offsets.json`; 5 artefatos de UI (`fxData`, `drumData`, `presetChains`, `presetData`, `fxModels`) são **gerados por scripts de `analysis/`**; fixtures de captura são a espinha de vários testes Rust |
| Sem tipos fábrica × usuário | Não existem `FactoryPatch`/`UserPatch` no core. Fábrica é semeada de `files/patches/all.prst` (`TOTAL_DE_FABRICA = 99`); o barramento tem 198 posições (99+99, nomes 198/198 medidos no decode 13xx). A identidade do preset é um `pp: number` sem tipo |
| Dev local sem aparelho | Feature `real-device` **default OFF**; browser usa fallback sintético; sem ela o app sobe em `Backend::Desligado` (#150) — honesto, mas o **default do dev não tem aparelho** |
| Suíte inflada | Rust **309 / 36 suítes**; front **513 / 38 arquivos**; e2e **90 / 12 arquivos / 96 baselines**; pytest **92**; gates **13**. Boa parte prova a RE (golden, replay de captura, hash de baseline), não o comportamento com o aparelho |
| Repo espalhado | `analysis/` (60+ itens), `scripts/` (22 itens), `files/` (~90 MB locais), `docs/` (30 arquivos sem categoria), `.agents/skills` (10 skills, 5 da era RE), sem `.agents/prompts/` |
| CI fora da ordem | 26 jobs num workflow. `Relatório · resumo da run` depende **só do `plan`** (roda primeiro — devia ser o último); `Fechamento · issues vinculadas` sem `needs` (devia ser workflow próprio `Issues · Fechamento de issues vinculadas`); `Segurança` roda depois da cobertura; `Distribuição` não espera os gates |

### 1.2 Princípios (não negociáveis)

- **P1 — O aparelho é a fonte.** O que dá para ler, lê-se do GP-100. O que é protocolo/documentação vira
  **dado canônico versionado no projeto** — nunca um `include_str!` de `analysis/`.
- **P2 — Fábrica × usuário são tipos.** `FactoryPatch` (imutável) e `UserPatch` (mutável), como o manual
  separa. Hex nunca chega à tela: o modelo tipado é a fronteira.
- **P3 — Boot é gate.** Navbar + loading apenas; conexão validada, estado lido e validado; só então a
  casca monta. Falha = erro legível com retry; nada de tela meio-carregada.
- **P4 — Escrita é privilégio.** Trava de faixa (ADR-10) + política de escrita; **nada escreve no boot**;
  anti-brick é critério de aceite, não nota de rodapé.
- **P5 — Teste só do comportamento real.** Contrato de protocolo, modelo, boot, sync e UI do que existe.
  Replay de RE/golden/análise sai.
- **P6 — CI na ordem do dono.** Validação → Lint → Compilação → Segurança → Testes → Cobertura →
  Distribuição → Release; **Relatório por último**; Fechamento em workflow próprio.
- **P7 — Doc e harness na mesma PR que muda comportamento.** Skill, prompt, INDEX e knowledge acompanham.

---

## 2. As issues da era (milestone `v1.1.0`)

| # | Issue | Fase | Depende de | Skill principal |
|---|---|---|---|---|
| [#171](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/171) | **EPIC — Era Hardware** (contrato e análise completos) | — | — | todas |
| [#161](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/161) | Boot gate: só navbar + loading até validar; erro legível; zero placeholder/hex | F1 | — | `ui-ux-practices` |
| [#162](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/162) | `FactoryPatch`/`UserPatch`: modelo tipado, fábrica imutável × usuário mutável | F1 | — | `core-dev` |
| [#163](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/163) | Dados canônicos fora das análises; dissolver `analysis/` com proveniência | F2 | — | `core-dev` |
| [#164](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/164) | Sincronização bilateral em tempo real (device ↔ app) com trava anti-brick | F2 | #162, #163 | `core-dev` |
| [#165](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/165) | Política de device: **real por default no dev**; `--mock-device` só em teste; limitação documentada | F1 | — | `core-dev` |
| [#172](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/172) | Gate de campo H5: roteiro assinado (boot, modelo tipado, sync) — [`docs/H5_CHECKLIST.md`](H5_CHECKLIST.md) | Campo | #161, #162, #164, #165 | `core-dev` |
| [#166](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/166) | Suíte de testes enxuta: triagem arquivo a arquivo, corte fundamentado, doc de testes | F3 | #163, #165 | `rust-practices` |
| [#167](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/167) | Documentação oficial: `docs/` categorizada, `files/` triado, `SECURITY.md`, `.vscode` | F4 | #163, #168 | `docs-sync` |
| [#168](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/168) | Dissolver `scripts/`: migrar o vivo, arquivar o RE, decidir onde os gates moram | F4 | — | `github-flow` |
| [#169](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/169) | CI: grafo na ordem do dono, Relatório por último, Fechamento em workflow próprio, cache | F5 | #166, #168 | `github-flow` |
| [#170](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/170) | Harness do agente: skills, `.agents/prompts/`, templates de issue, `knowledge.md` | F6 | — | `docs-sync` |

Cada corpo de issue segue o contrato da era: **contexto medido · escopo · fora de escopo · DoD
(checkbox) · plano de teste · referências (docs + skill) · 🤖 prompt do agente pronto**.

---

## 3. Fases, ordem de execução e dependências

```
F1  Núcleo do comportamento        #161 (boot gate) · #162 (modelo) · #165 (política real/mock)
F2  Dados do aparelho              #163 (proveniência/analysis out) · #164 (sync bilateral)
F3  Suíte de testes                #166 (triagem e corte)
F4  Repositório                    #167 (docs/files/.vscode) · #168 (scripts)
F5  Pipeline                       #169 (o CI na ordem do dono)
F6  Harness do agente              #170 (skills/prompts/templates/knowledge)
```

- **Começar por F1.** #161 e #162 não dependem de nada e destravam o núcleo visível; #165 é
  pré-requisito prático de medir qualquer coisa no dev local.
- **F2** prepara o terreno para os testes novos: #163 remove a dependência de `analysis/`
  (guard automatizado incluso) e #164 entrega a convergência que o resto do app assume.
- **F3** só depois de #163/#165: a triagem precisa da suíte substituta no lugar.
- **F4** e **F5** reorganizam o repositório; #168 antes de #169 (o CI aponta para scripts).
- **F6** roda em paralelo, mas fecha por último o ciclo (o harness descreve o mundo já migrado).

### 3.1 Candidatas pós-núcleo (não reabrir antes de F1–F5)

| Item | Origem | Condição para voltar |
|---|---|---|
| **Live Mode** (setlist, troca por atalho/MIDI, metrônomo visual) | #117 (fechada) | depois de #164 (sync) — setlist é decisão do app, troca é do aparelho |
| **Ponte DAW MIDI CC/OSC** | #118 (fechada) | depois de #164/#165 — depende de caminho de escrita real medido |
| **A/B em campo (medição real)** | #116 (entregue na parte sem aparelho, PR #129) | depois de #164 — repetir no formato da Era HW com veredito de campo |
| Release/distribuição (v1.0.0) | #17 (fechada) | republicar no formato novo quando a era estiver estável |

---

## 4. Checklist de validação por fase

Comandos canônicos (executar em `develop` salvo indicação da issue):

**F1**
- [ ] `pnpm --dir packages/app/ui exec vitest run` e `playwright test` verdes; teste do gate prova
      ausência de casca em `loading`/`error`.
- [ ] `cargo test -p gp100-core` + clippy `-D warnings` verdes com os tipos `FactoryPatch`/`UserPatch`.
- [ ] Com o GP-100 na USB: `tauri dev` (fluxo novo de #165) mostra `backend: "real"` e o boot conclui.
- [ ] Nenhuma escrita no caminho de boot (teste com transporte que pune write).

**F2**
- [ ] `git grep "analysis/" packages/` = **zero** (guard automatizado no testes).
- [ ] Convergência: escrita → divergência → releitura mostra o valor do aparelho (mock fiel, #157).
- [ ] Log de fio automático gerado no backend real, com o formato atual.

**F3**
- [ ] Tabela de triagem completa; 4 suítes verdes; contagem antes/depois no `docs/INDEX.md` §6.
- [ ] Gates de proteção intactos (round-trip `.prst`, trava ADR-10, `write_gate`).

**F4**
- [ ] `docs/INDEX.md` responde "onde mora X" para todo assunto; zero link quebrado; zero citação a `analysis/`.
- [ ] `scripts/` inexistente; `gates.py` (ou sucessor) roda pelo comando documentado.

**F5**
- [ ] Run real de PR: ordem do grafo correta; Relatório por último resumindo a run inteira.
- [ ] `Issues · Fechamento de issues vinculadas` fecha a issue de um PR mergeado (teste controlado).
- [ ] Tempos antes/depois do cache publicados no PR.

**F6**
- [ ] Template de issue renderiza todos os campos obrigatórios; um agente executa uma issue da era só
      com o prompt dela.

---

## 5. Riscos e anti-brick

| Risco | Mitigação (contrato da era) |
|---|---|
| Escrita fora de faixa derruba o firmware (assert até power-cycle — #110) | Trava ADR-10 + política `write-verified`; **nada escreve no boot**; teste com transporte que pune byte |
| Reescrita ampla quebrar o que funciona | Cada issue tem "Garantias de não-regressão"; cortes com triagem escrita; PRs pequenos e CI verde |
| Perder evidência histórica na limpeza | `docs/arquivo/` recebe o que tem valor; relatórios H permanecem citáveis com caminho novo |
| Gate removido virar buraco silencioso | Nenhum gate de proteção sai sem substituto na mesma PR (regra no DoD) |
| Doc/harness divergirem do código | P7: doc + skill + prompt na mesma PR; `docs-sync` no fechamento |

---

## 6. Governança do CI (a partir de #169)

```
Validação (primeiro, sem needs)
  → Lint (needs validação)
    → Compilação (needs validação, lint)
      → Segurança (needs validação, lint, compilação)
        → Testes unit/integração/e2e (needs segurança)
          → Cobertura (needs testes)
            → Distribuição (needs cobertura; condições de tag preservadas)
              → Release (needs distribuição; workflow_dispatch/tag)
Relatório · resumo da run  ← needs TODOS, if: always() — SEMPRE o último
Fechamento  → workflow próprio: .github/workflows/issues-fechamento.yml
```

---

## 7. Referências

- **Protocolo e decisões:** [`docs/PROTOCOL.md`](PROTOCOL.md) · [`docs/DECISIONS.md`](DECISIONS.md)
  (ADR-1..12) · [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) · [`docs/REAL_DEVICE_GAP.md`](REAL_DEVICE_GAP.md)
- **Estado e números:** [`docs/INDEX.md`](INDEX.md) §6 (fonte única) · [`knowledge.md`](../knowledge.md)
- **Gates de campo (histórico):** [`docs/H1_REPORT.md`](H1_REPORT.md) · [`docs/H2_REPORT.md`](H2_REPORT.md) ·
  [`docs/H3_REPORT.md`](H3_REPORT.md) · [`docs/H4_REPORT.md`](H4_REPORT.md)
- **Era anterior:** [`docs/arquivo/ROADMAP_era-RE_2026-09.md`](arquivo/ROADMAP_era-RE_2026-09.md)
- **Harness:** `.agents/skills/*` (serão auditadas em #170) · `.agents/prompts/` (nasce em #170)
- **Tracker:** [épico #171](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/171) ·
  [milestone v1.1.0](https://github.com/lucascantarelli/gp100-nextgen-editor/milestone/2)
