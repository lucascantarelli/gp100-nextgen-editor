# AGENTS.md — a entrada canônica do agente

> ✅ **atual** (16/10/2026) · **Este arquivo é ponteiro, não fonte.** Cada regra tem
> **uma casa** (a tabela de fontes únicas está em [`.agents/README.md`](.agents/README.md));
> aqui fica o índice que qualquer harness carrega sozinho — inclusive os que não sabem
> carregar agente definido no projeto (a limitação medida está em
> [`docs/analises/harness-v2-limite-medido-2026-10-15.md`](docs/analises/harness-v2-limite-medido-2026-10-15.md)).

## 1. A premissa (vale acima de tudo)

**O aparelho real é a fonte** (ADR-13): mede-se no aparelho, não se deduz do mock, do manual
nem de engenharia reversa. Resposta ao dono **em PT-BR**; a escrita no aparelho vive atrás das
travas (ADR-5 + gate H2) — ver `hardware-dev`.

## 2. Ordem de leitura (não pule)

1. [`knowledge.md`](knowledge.md) — premissa, armadilhas de ambiente, estado vivo.
2. [`docs/INDEX.md`](docs/INDEX.md) — qual doc responde qual pergunta.
3. **A issue** (`gh issue view #N`) — contexto medido, escopo, DoD e o `🤖 prompt` pronto.
4. **A skill da área** (`.agents/skills/*/SKILL.md`, disparada pela `description`).
5. **O papel da vez** — [`.agents/agents/README.md`](.agents/agents/README.md).

## 3. A equipe: quem faz o quê

Um **papel** é um contrato **lido** (`.agents/agents/`), não um agente carregado: funciona
em qualquer harness que saiba ler arquivo. O estado da demanda vive em **labels `flow:*`
no GitHub** — é o quadro de trabalho, e sobrevive à sessão.

| Papel | Contrato | Entra em | Sai em |
|---|---|---|---|
| triagem | [`.agents/agents/triagem.md`](.agents/agents/triagem.md) | demanda crua | `flow:pronto` (issue auto-suficiente) |
| implementador | [`.agents/agents/implementador.md`](.agents/agents/implementador.md) | `flow:pronto` | `flow:revisao` (PR + gates) |
| revisor | [`.agents/agents/revisor.md`](.agents/agents/revisor.md) | `flow:revisao` | `flow:verificacao` ou 1 correção |
| verificador | [`.agents/agents/verificador.md`](.agents/agents/verificador.md) | `flow:verificacao` | `flow:integrado` (squash) |

O ciclo, o handoff, o anti-loop e a fronteira papel × skill: `.agents/agents/README.md` §4 —
não os repito aqui. Qualquer etapa pode terminar em `flow:bloqueado`, com o impasse **na issue**.

## 4. Comandos canônicos (índice; a fonte é [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) §4)

```bash
# Rust (raiz)          cargo fmt --check · cargo clippy --workspace --all-targets -- -D warnings · cargo test
# Front (ui/)          npx tsc -b · pnpm lint · pnpm test:coverage · pnpm build · pnpm exec playwright test
# Spec + gates         uv run pytest · python3 .github/scripts/gates.py --list
# Só a área que você tocou — o CI filtra o resto por caminhos.
```

## 5. Não fazer (cada item aponta a casa da regra)

- **Relaxar gate** para o PR passar (asserção fraca, `skip`, `#[ignore]`, `--no-verify`)
  — a regra é do `github-flow` + o gate existe para reprovar.
- **Adivinhar protocolo/preset** — wire e `.prst` são carve-out: medir no aparelho.
- **Merge de carve-out** (`.github/**`, `.agents/**`, este arquivo, `DECISIONS.md`,
  release/tag/`main`, segredo, migração destrutiva) sem `guard:humano` e o dono.
- **Duplicar uma regra** em mais de um arquivo: regra que mora em dois lugares
  sincroniza em um e apodrece no outro.
- **Escrever contador à mão** (testes, cobertura, jobs) — a fonte é `docs/INDEX.md` §6.
- **Commit sem label convencional** ou movimento de código sem o trailer do
  `check_move_only` — os dois são gates, não estilo.

## 6. Dono ausente

Dono ausente: vale o **modo ausente** ([`.agents/README.md`](.agents/README.md), #202) — a
rotina executa, o crítico vira comentário **na issue**. Aqui é ponteiro, não cópia.

## 7. Ferramentas que o cliente injeta (medido em 15/10/2026)

Só `researcher-web` é spawnável; `github`, `playwright` e `filesystem` são MCPs injetados. A
medição, as sondas e os limites declarados estão em
[`docs/analises/harness-v2-limite-medido-2026-10-15.md`](docs/analises/harness-v2-limite-medido-2026-10-15.md).
