> 📜 **histórico** (arquivado 16/10/2026, #269) — os trechos que o enxugamento do harness
> **removeu** dos arquivos de entrada, e para onde cada regra foi. Nada aqui é regra
> vigente: o que vale está na casa apontada. Arquiva-se porque o corte **não apaga** —
> o texto removido fica registrado com a origem.

## O que saiu, e para onde foi

| Trecho | Estava em | A regra vive em |
|---|---|---|
| lista "O que NÃO fazer" (7 itens: RE, escrita, gate, issue, contador, palpite, skill morta) | `knowledge.md` | [`AGENTS.md`](../../AGENTS.md) §5 (a lista única) + a casa de cada item (§ abaixo) |
| "Dono ausente" (rotina sem perguntar, crítico na issue) | `AGENTS.md` §6 | [`.agents/README.md`](../../.agents/README.md) §"Modo ausente do dono" (#202) |
| medição "o que o cliente carrega" (4 sondas, MCPs, limites) | `.agents/agents/README.md` §1 | [`docs/analises/harness-v2-limite-medido-2026-10-15.md`](../analises/harness-v2-limite-medido-2026-10-15.md) |
| registro da forma do agente de projeto | `.agents/poc-pesquisador.ts` | [`harness-poc-agente-projeto.ts`](harness-poc-agente-projeto.ts) |
| proveniência das regras do checklist | `.agents/skills/docs-sync/SKILL.md` | [`docs-sync-proveniencia-2026-10.md`](docs-sync-proveniencia-2026-10.md) |
| marcos 14/10 → 09/10 do "Estado vivo" | `knowledge.md` | [`knowledge-estado-vivo-2026-10.md`](knowledge-estado-vivo-2026-10.md) |

A casa de **cada item** da lista que saiu do `knowledge.md`: engenharia reversa →
`hardware-dev` (premissa, ADR-13) e o `AGENTS.md` §5 · escrita no aparelho →
`hardware-dev` + `AGENTS.md` §1 (ADR-5 + gate H2) · contornar gate → `github-flow` e
`rust-practices` · implementar sem issue e o ciclo → `github-flow` · contador no §6 →
`docs-sync` · palpite de protocolo → `AGENTS.md` §5 + o fluxo `protocolo` · skill morta →
`.agents/README.md` ("nada de apagar").

## O texto removido (como estava)

### `knowledge.md` — seção "O que NÃO fazer"

```markdown
## O que NÃO fazer (regras de era, não sugestões)

- **Nada de engenharia reversa**: spec não é reconstruída de Suite/proxy/captura.
  `docs/arquivo/analysis/` é histórico/evidência; o que importa de lá migrou para
  `packages/core/data/` (#163). Skills da Era RE estão arquivadas — não
  ressuscite o processo delas.
- **Nada de escrever no aparelho fora das travas** (ADR-5): escrita exige a
  feature `write-verified` + gate H2 passado + roteiro assinado. Não crie
  terceira camada (env var, flag, subcomando) para contornar.
- **Nada de contornar gate vermelho** (CI ou local): sem relaxar `-D warnings`,
  sem pular job, sem `--no-verify`. CI vermelha não vira marco.
- **Nada de implementar sem issue**; uma issue = um PR; descoberta nova no
  caminho vira issue na hora — o ciclo é a skill `github-flow`.
- **Contadores**: a casa é o `docs/INDEX.md` §6 (skill `docs-sync`) — número em
  dois docs é dívida garantida.
- **Palpite de protocolo = para o fluxo**: divergência app × aparelho vira issue
  `protocolo` com evidência; nunca "consertar" o codec/UI no chute.
- Skill nova só se justificar (description dispara em tarefa recorrente); skill
  morta vai para `docs/arquivo/`, nunca é apagada.
```

### `AGENTS.md` — seção "Dono ausente" (duplicava o `.agents/README.md`)

```markdown
## 6. Dono ausente

Quando o dono diz que estará fora, vale o **modo ausente** (`.agents/README.md`, #202):
executa as rotinas sem perguntar, comenta o crítico na issue e para. Todo o resto deste
arquivo continua valendo.
```
