# 🔎 AUDITORIA — harness do agente (`.agents/`) — 2026-10-14

> **Status:** ✅ atual (desta auditoria) · **Prompt usado:** [`.agents/prompts/auditoria.md`](../../.agents/prompts/auditoria.md)
> **Escopo:** 9 skills ativas (`.agents/skills/`) + 6 prompts (`.agents/prompts/`) + `.agents/README.md`.
> **Método:** *mechanism over doc* — cada regra/skill foi conferida contra o artefato real
> (ROADMAP, DECISIONS, `ci.yml`, issues do tracker). Auditoria de **leitura**: nenhuma
> skill foi alterada; a consolidação está proposta na §3.

**Medido em 14/10/2026:** 9 skills = **869 linhas**; 6 prompts = **187 linhas**.
`docs/dev/DECISIONS.md` tem **16 ADRs** (ADR-1..16). O ROADMAP vigente define os
princípios **P1–P7** (`docs/planejamento/ROADMAP.md` §1.2); as regras **R1–R4** são da
era RE, arquivada.

---

## Sumário executivo

| Severidade | Nº | Temas |
|---|---|---|
| 🔴 bloqueante | 0 | — |
| 🟠 dívida | 7 | regras de era RE ainda load-bearing, faixa de ADR errada, mecanismo de release contradito pela #234, duplicação em 6 regras, camada do app Rust sem skill |
| 🟡 nit | 5 | caminho arquivado malformado, descrição com issue-fantasma, refs cruzadas para doc obsoleto, citações datadas |

**Veredito honesto:** o harness é **bom e coeso** — as 9 skills cobrem as áreas certas e os
6 prompts são curtos e acionáveis (187 linhas no total é enxuto). O problema não é
*faltar* peça: é **a Era RE ainda estar embutida nas regras** (R1–R4, M0.x, P2/P4, ADR-1..13)
e **a mesma regra viver em até 6 arquivos** — o que garante que a próxima correção
sincronize 3 e esqueça 3. As duas maiores oportunidades são: **(a)** dar **uma casa só**
para cada regra transversal; **(b)** criar a skill que **falta** para o crate do app
(`packages/app/api` — o `actor.rs` de 2702 linhas não tem dono de skill).

---

## 1. Achados

### H-01 🟠 Regras `R1–R4` citadas como lei, mas o ROADMAP vigente é `P1–P7`
- `github-flow:11` ("o ROADMAP (fases/R1–R4)"), `rust-practices:11-12` ("Complementa as
  regras R1–R4 do ROADMAP … em conflito, valem R1–R4"), `docs-sync:13` ("viola a regra R1"),
  `ui-ux-practices:3,19` ("a regra R1 aplicada ao front" / "## R1 no front").
- **Artefato real:** `docs/planejamento/ROADMAP.md` §1.2 define **P1–P7**; R1–R4 são do
  `docs/arquivo/ROADMAP_era-RE_2026-09.md`.
- **Impacto:** "em conflito, valem R1–R4" manda o agente obedecer um conjunto de regras
  que **não existe mais** no ROADMAP vivo.
- **Ação:** trocar por P1–P7 (ou remover a numeração e apontar `ROADMAP §1.2`).

### H-02 🟠 Faixa de ADR desatualizada em `device-reference`
- `device-reference:50` diz `docs/dev/DECISIONS.md` (**ADR-1..13**).
- **Artefato real:** `DECISIONS.md` tem **16** ADRs.
- **Ação:** trocar por "ADR-1..16" ou, melhor, "`DECISIONS.md` (a fonte)" — sem faixa, para
  não apodrecer.

### H-03 🟠 Rótulos de fase da era RE em regras load-bearing
- `rust-practices:10` "issues **M0.x** do ROADMAP"; `core-dev:73-77` mapeia o workspace como
  `model (M0.1) … session (M0.6)`, `(M0.7)`, `rust-toolchain.toml (P2)`; `rust-practices`
  cita "Sem unsafe no **M0**", "fixtures **P4**", "exemplo vivo (**M0.4**)", "propriedade
  mínima (M0.3)".
- **Artefato real:** o milestone vigente é **v1.1.0 / Era Hardware**; M0/P2/P4 são fases do
  ROADMAP arquivado.
- **Impacto:** `rust-practices` é a skill **mais usada** (201 linhas) e abre mandando o
  agente para uma fase que não existe; os rótulos `P2/P4` são usados como **atalho de
  referência** ("fixtures P4") sem definição viva.
- **Ação:** substituir por nomes de artefato (`fixtures em packages/core/data/fixtures/`) e
  pelo milestone atual; preservar o histórico como nota 📜 se o valor didático importa.

### H-04 🟠 A skill `release` ensina um mecanismo que a #234 (aberta) prova que não funciona
- `release:21`: promote "faz `main` receber o **SHA da tag** (branch protegida não aceita
  merge)"; `prompts/release.md:22`: "confirme main == SHA da tag".
- **Artefato real:** issue **[#234](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/234)** (OPEN) —
  o `promote` **não** move a `main` (`GH006`: histórico linear + o SHA da tag traz o merge do
  ciclo); o owner **empurra a `main` à mão**.
- **Impacto:** é uma **skill de workflow de release** afirmando como resolvido um passo que
  falha; quem seguir ao pé da letra para no meio da promote.
- **Ação:** registrar a limitação real na skill (o que funciona hoje: push do owner na
  `main` com o comando documentado) e linkar a #234; o prompt aponta, não repete.

### H-05 🟠 Duplicação: a mesma regra vive em até 6 arquivos
Medido por `grep` (arquivos que contêm a regra):

| Regra transversal | Arquivos (nº) | Onde deveria viver |
|---|---|---|
| "o aparelho é a fonte / medir, não deduzir" (ADR-13) | 6 (`core-dev`, `device-reference`, `hardware-dev`, `README`, `implementar-issue`, `code-review`) | **`hardware-dev`** (a skill do aparelho); os demais apontam |
| Ciclo issue→branch→PR→CI→merge | 5 (`github-flow`, `core-dev`, `implementar-issue`, `executar-issues-em-sequencia`, `corrigir-bug`) | **`github-flow`** (skill) + `implementar-issue` (prompt canônico) |
| Conventional commits + footer Codebuff | 4 (`github-flow`, `core-dev`, `implementar-issue`, `code-review`) | **`github-flow`** |
| Unitário `src/` × contrato `tests/` | 3 (`rust-practices`, `testing`, `core-dev`) | **`rust-practices`** |
| Contadores só no `INDEX §6` | 2 (`docs-sync`, `TESTING.md`) | **`docs-sync`** |
| Cargo no PATH / toolchain pinada | 3 (`core-dev`, `rust-practices`, `knowledge.md`) | **`knowledge.md`** |

- **Impacto:** a regra que existe em 6 lugares sincroniza em 3 e apodrece nos outros 3 —
  exatamente o modo de falha que a própria `docs-sync` regra 8 combate ("mecanismo citado ≠
  mecanismo atual").
- **Ação:** aplicar o princípio de **fonte única** às regras transversais (§3.2).

### H-06 🟠 Gap: a camada Rust do app (`packages/app/api`) não tem skill
- **Artefato real:** `packages/app/api/src/actor.rs` = **2702 linhas** (issue **#243**);
  `rust-practices` cobre `gp100-core`/`gp100-cli` (a `description` diz "packages/core,
  packages/cli"); `ui-ux-practices` cobre o front e cita o shell Tauri de passagem;
  `docs/dev/ARCHITECTURE.md` documenta só o front.
- **Impacto:** o maior arquivo do projeto não tem skill dona nem orçamento — `actor.rs`
  chegou a 2702 linhas sem freio. Nenhuma skill descreve `Request`/`DeviceActor`/read-back.
- **Ação:** criar skill **`app-dev`** (Tauri Rust: actor, commands, sync/read-back, política
  de escrita na porta) e registrá-la no `README` e no INDEX; ela é o par de `core-dev`.

### H-07 🟡 Caminho arquivado malformado (3×)
- `core-dev:91` e `rust-practices:17,199`: `docs/arquivo/scripts-era-RE/add_cargo_path.ps1
  (arquivado na #168)` — o parêntese **dentro** do caminho entre backticks (o arquivo existe,
  mas o texto copiado não resolve).
- **Ação:** `` `docs/arquivo/scripts-era-RE/add_cargo_path.ps1` `` (e "(arquivado na #168)"
  fora do code span).

### H-08 🟡 `testing` — a `description` cita um gate que morre com issue já fechada
- `testing:3`: "o **pytest de spec que morre com HW-3**". **HW-3 = #163, CLOSED**.
- **Artefato real:** o pytest de spec segue **vivo** (47 testes, `uv run pytest` verde hoje);
  o contrato da #166 diz que o espelho da RE é substituído por MockDevice fiel / modelo
  tipado (HW-1/2/4), não por HW-3.
- **Impacto:** a `description` é o **gatilho** da skill — um agente que a lê conclui que a
  suíte está morta.
- **Ação:** reescrever a `description` sem a referência a issue fechada.

### H-09 🟡 `device-reference` aponta para docs que a auditoria do repo marcou como obsoletas
- `device-reference` "Onde cada coisa mora" → `docs/referencias/MANUAL_COVERAGE.md §8` e
  `docs/analises/audit-era-real-2026-10-07.md`; a §4 ainda trata "Setlist" como pendência.
- **Artefato real:** `MANUAL_COVERAGE.md` está na Fase 1, com caminho morto (`EmptyBoard.tsx`)
  e premissa de captura já realizada — **achado F-03** da auditoria do repo (issue **#242**).
- **Ação:** trocar o ponteiro para `REAL_DEVICE_GAP.md` quando a #242 fundir o doc.

### H-10 🟡 Gap menor: nem a limpeza/higiene nem os gates de CI têm skill/prompt
- Não há guia para **faxina de repositório** (docs redundantes, artefatos locais) nem para os
  **gates de script** (`gates.py`, `check_*`) e imagens de container — o que existe está
  espalhado em `github-flow` e no `CONTRIBUTING`.
- **Ação:** ou um prompt curto `limpeza.md`, ou uma seção na `auditoria.md` (é atividade de
  auditoria, não de desenvolvimento). Preferir a segunda (evita prompt novo sem demanda).

### H-11 🟡 Citações datadas na `docs-sync` (ruído histórico)
- `docs-sync` carrega F-11, F-12, #68, "auditoria 03/10", "conferido 11/10 na #170" em regras
  de uso diário. São **proveniência legítima**, mas inflam a skill com histórico.
- **Ação:** manter o Porquê de 1 linha e mover o resto para um apêndice "histórico" no fim.

### H-12 🟡 `README.md` duplica o catálogo que as `description` já carregam
- `.agents/README.md` lista as 9 skills com 1 linha cada — útil como mapa, mas é a **3ª
  cópia** (exists também no `INDEX.md` "Infra do agente" e nas próprias `description`).
- **Ação:** o README mantém só a **ordem de entrada** e a **tabela de fontes únicas**; o
  catálogo detalhado fica no INDEX (que já é a fonte de navegação).

---

## 2. O que está certo (preservar)

- **9 skills = as 9 áreas certas**, sem skill órfã e sem sobreposição estrutural (cada uma
  tem `description` com o "quando usar").
- **Prompts curtos** (187 linhas nas 6) — `implementar-issue` canônico e os outros apontando
  para ele, em vez de reescrever o ciclo.
- **`testing` e `rust-practices`** são as fontes de verdade de organização de teste e já
  mandam "não mover unitário para tests/" explicitamente (a dúvida que sempre volta).
- **`README.md` "Modo ausente do dono"** com a linha divisória rotineiro × crítico — modelo
  claro e alinhado ao que foi usado nesta sessão.
- **`docs-sync` regra 8/10** (mecanismo ≠ atual; LF explícito) — regras que pegam bug real.

---

## 3. Proposta de consolidação

### 3.1 Princípio — uma casa por regra ("fontes únicas")

| Assunto | Skill/prompt dono | Quem aponta (não repete) |
|---|---|---|
| O aparelho é a fonte; medir, não deduzir (ADR-13) | `hardware-dev` | `core-dev`, `device-reference`, `README`, prompts |
| Ciclo issue→PR→CI→merge | `github-flow` + `implementar-issue` | `core-dev`, `corrigir-bug`, `executar-issues-em-sequencia` |
| Conventional commits + footer | `github-flow` | `core-dev`, prompts |
| Onde cada teste vive | `rust-practices` (Rust) · `testing` (visão por camada) | `core-dev` |
| Contadores só no `INDEX §6` | `docs-sync` | `TESTING.md`, `code-review` |
| Armadilhas de host (cargo PATH, toolchain) | `knowledge.md` | `core-dev`, `rust-practices` |
| Cadeia de release rc→promote | `release` (skill) | `prompts/release.md` (entrada fina) |

### 3.2 Moves concretos (por arquivo)

1. **`rust-practices`** — remover `M0.x`/`P2`/`P4`/`M0.3-6`; apontar fixtures/artefatos por
   caminho; trocar "R1–R4" por "P1–P7 (`ROADMAP §1.2`)". É a maior alavanca (201 linhas).
2. **`core-dev`** — tirar o ciclo de PR detalhado (deixar 1 linha → `github-flow`), tirar a
   regra 1a duplicada (→ `hardware-dev`), corrigir o caminho do `add_cargo_path` e o mapa do
   workspace (sem `M0.x`).
3. **`device-reference`** — corrigir `ADR-1..13` → `DECISIONS.md` (sem faixa); trocar o
   ponteiro `MANUAL_COVERAGE` por `REAL_DEVICE_GAP` (após #242).
4. **`testing`** — reescrever a `description` (sem "morre com HW-3"); manter a §"onde cada
   teste vive" e **apontar** `rust-practices` para a regra de organização.
5. **`release`** — adicionar a limitação real (`main` via push do owner; #234) com o comando
   documentado; o prompt só confirma.
6. **`docs-sync`** — mover F-11/F-12/#68/auditorias para apêndice "histórico"; manter os
   Porquês.
7. **`github-flow`** — trocar "R1–R4" por "P1–P7"; manter o resto (é a fonte do ciclo).
8. **Novo `app-dev/SKILL.md`** — dono de `packages/app/api`: `DeviceActor`, `Request`,
   read-back/sync (#164/ADR-15), política de escrita na porta (`escritaLiberada()`), e o
   orçamento de tamanho (liga com a issue **#243**). Registrar no `README` e no INDEX.
9. **`auditoria.md`** — acrescentar um passo de **higiene** (docs redundantes, artefatos
   locais, referências mortas) — vira a casa da "faxina" sem prompt novo.
10. **`.agents/README.md`** — deixar só a ordem de entrada + a tabela de fontes únicas (§3.1);
    o catálogo detalhado vive no INDEX.

### 3.3 Resultado esperado

- Nenhuma regra transversal em >2 arquivos (hoje até 6).
- Zero referência a R1–R4/M0.x/P2/P4 em regra de uso diário.
- Faixas de ADR e mecanismos de release **verificáveis** (sem número que apodrece).
- Uma skill dona para o crate do app — o maior arquivo do repo deixa de ser órfão de harness.

### 3.4 Sequência sugerida — **issue aberta**

**[#249](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/249)** (milestone `v1.1.0`, `refactor, area:docs`) — consolidação do harness com PR em 3 commits:
**(1)** correções de era/ADR/mecanismo (H-01..H-04, H-07..H-09); **(2)** desduplicação e
tabela de fontes únicas (H-05, H-11, H-12); **(3)** skill `app-dev` + passo de higiene
(H-06, H-10). A skill nova merece revisão à parte por ser a única **adição**.

> **Não executado nesta etapa:** nenhuma skill foi editada. É auditoria de leitura —
> a consolidação é a §3 (vira issue → prompt `implementar-issue`).

---

## 4. Desfecho (#249 — executado em 14/10/2026)

A consolidação da §3 foi executada em PR único de 4 commits (branch
`docs/249-harness-consolidacao`). O que ficou como está e o que mudou:

| Achado | Desfecho |
|---|---|
| H-01 era `R1–R4` | Todas as skills/prompts passam a citar **P1–P7** (`ROADMAP.md` §1.2); zero `M0.x` em regra viva |
| H-02 faixa de ADR | `device-reference` cita `docs/dev/DECISIONS.md` **sem faixa** (o número apodrecia) |
| H-03 mecanismo da release | `release` documenta a limitação real (`main` pelo SHA da tag, `GH006` — #234) |
| H-04 ponteiro obsoleto | `device-reference` → `REAL_DEVICE_GAP.md` (vivo); `MANUAL_COVERAGE` marcada histórica (#242) |
| H-05 duplicação | Tabela **fontes únicas** no `README.md` + regra "uma casa por regra"; ciclo de PR/commits e regra 1a saem de `core-dev` |
| H-06 camada do app sem skill | **criada** `.agents/skills/app-dev/SKILL.md` (10 skills); registrada no INDEX |
| H-07 caminho arquivado | `docs/arquivo/scripts-era-RE/add_cargo_path.ps1` em todas as citações |
| H-08 issue-fantasma | `description` da `testing` reescrita (sem issue morta) |
| H-09 citações por fase | Rótulos trocados por **P1–P7**/issue |
| H-10 higiene | Passo 8 do prompt `auditoria.md`: o próprio `.agents/` é artefato auditável |
| H-11 histórico | Proveniência (`F-11/F-12/#68`, "auditoria 03/10") movida para apêndice da `docs-sync` |
| H-12 3ª cópia | `README.md` fica com **ordem de entrada + fontes únicas**; o catálogo vive no INDEX |

**Provas rodadas na branch:** zero `R1[–-]R4`/`M0.x` em `.agents/`; as **10** skills com
front-matter YAML parseável, `name` == pasta e `description` que dispara; nenhuma faixa
`ADR-1..13`; `add_cargo_path` resolvendo no caminho novo.

**Nota de histórico:** esta auditoria mediu **9** skills (14/10, antes do #249); a 10ª
(`app-dev`) é o produto da H-06. Os números de linhas do cabeçalho são daquele momento.
