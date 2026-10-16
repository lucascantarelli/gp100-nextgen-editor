# AGENTS.md — a entrada canônica do agente

> ✅ **atual** (15/10/2026) · **Este arquivo é ponteiro, não fonte.** Cada regra tem
> **uma casa** (a tabela de fontes únicas está em [`.agents/README.md`](.agents/README.md));
> aqui fica o índice que qualquer harness carrega sozinho — inclusive os que não sabem
> carregar agente definido no projeto (a limitação medida está em
> [`.agents/agents/README.md`](.agents/agents/README.md) §1).

## 1. A premissa (vale acima de tudo)

**O aparelho real é a fonte** (ADR-13): o que a GP-100 faz se **mede no aparelho**, não
se deduz do mock, do manual ou de engenharia reversa. Resposta ao dono **em PT-BR**.
A escrita no aparelho vive atrás das travas (ADR-5 + gate H2) — ver `hardware-dev`.

## 2. Ordem de leitura (não pule)

1. [`knowledge.md`](knowledge.md) — premissa, armadilhas de ambiente, o que não fazer.
2. [`docs/INDEX.md`](docs/INDEX.md) — qual doc responde qual pergunta.
3. **A issue** (`gh issue view #N`) — contexto medido, escopo, DoD e o `🤖 prompt` pronto.
4. **A skill da área** (`.agents/skills/*/SKILL.md`, disparada pela `description`).
5. **O papel da vez** — [`.agents/agents/README.md`](.agents/agents/README.md).

## 3. A equipe: quem faz o quê

Um **papel** é um contrato **lido**, não um agente carregado: funciona em qualquer harness
que saiba ler arquivo. O estado da demanda vive em **labels `flow:*` no GitHub** — é o
quadro de trabalho, e sobrevive à sessão.

| Papel | Contrato | Entra em | Sai em |
|---|---|---|---|
| triagem | [`.agents/agents/triagem.md`](.agents/agents/triagem.md) | demanda crua | `flow:pronto` (issue auto-suficiente) |
| implementador | [`.agents/agents/implementador.md`](.agents/agents/implementador.md) | `flow:pronto` | `flow:revisao` (PR + gates) |
| revisor | [`.agents/agents/revisor.md`](.agents/agents/revisor.md) | `flow:revisao` | `flow:verificacao` ou 1 correção |
| verificador | [`.agents/agents/verificador.md`](.agents/agents/verificador.md) | `flow:verificacao` | `flow:integrado` (squash) |

O ciclo, o formato do handoff, o anti-loop e a fronteira papel × skill estão em
[`.agents/agents/README.md`](.agents/agents/README.md) §4 — não os repito aqui.
Qualquer etapa pode terminar em `flow:bloqueado`, com o impasse escrito **na issue**.

## 4. Comandos canônicos (índice; a fonte é [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) §4)

```bash
# Rust (raiz)          cargo fmt --check · cargo clippy --workspace --all-targets -- -D warnings · cargo test
# Front (ui/)          npx tsc -b · pnpm lint · pnpm test:coverage · pnpm build · pnpm exec playwright test
# Spec + gates         uv run pytest · python3 .github/scripts/gates.py   (--list: os 14)
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

Quando o dono diz que estará fora, vale o **modo ausente** (`.agents/README.md`, #202):
executa as rotinas sem perguntar, comenta o crítico na issue e para. Todo o resto deste
arquivo continua valendo.

## 7. Ferramentas que o cliente injeta (medido, 15/10/2026)

- **Agentes**: só `researcher-web` — agente definido no projeto não é carregado (§1 de
  [`.agents/agents/README.md`](.agents/agents/README.md));
- **MCPs**: `github` (issue/PR/review/checks), `playwright` (o front no navegador — a janela
  Tauri nativa não é alcançável) e `filesystem`. O `gh` CLI é alternativa, não requisito.
