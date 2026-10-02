---
name: github-flow
description: Fluxo obrigatório de trabalho no GitHub (GitFlow) — toda mudança nasce de uma issue, vira branch tipo/N-slug, PR com Closes #N para develop, CI verde, merge (squash) e fechamento automático; define o papel de cada branch e o que roda de CI em cada uma.
metadata:
  category: workflow
---

# Fluxo de trabalho no GitHub (GitFlow) — obrigatório

Use ANTES de qualquer implementação e ao fechar qualquer trabalho. Complementa
`docs/CONTRIBUTING.md` (detalhe humano) e o ROADMAP (fases/R1–R4). Em conflito,
vale o que estiver em `docs/CONTRIBUTING.md` + `.github/workflows/`.

## Regra zero — trabalho aberto é uma ISSUE

1. **Nunca implementar sem issue.** Procure a issue no GitHub (milestone
   `v1.0.0`); se não existir, crie ANTES (template do repo: `bug`, `feature`,
   `refactor`, `epic`, `protocolo` — ver `.github/ISSUE_TEMPLATE/`).
2. **Não duplicar plano em doc**: o ROADMAP aponta para as issues; DoD e
   checklist vivem na issue (epic usa sub-issues/filhas).
3. **Descoberta nova no caminho** (bug/protocolo/decisão): vira issue na hora;
   não "resolve de passagem" sem registro.

## Papéis das branches e o que o CI roda

| Branch | Papel | CI no push | PR de destino |
|---|---|---|---|
| `feature/<N>-slug` | desenvolvimento (fase/issue) | **nada** (valida no PR) | `develop` |
| `fix/<N>-slug` / `bugfix/` | correção de bug | **nada** (valida no PR) | `develop` |
| `chore/`, `docs/`, `refactor/` | manutenção | **nada** (valida no PR) | `develop` |
| `develop` | integração (origem das releases) | validação completa | — |
| `release/x.y.z` | freeze de rc (rc→promote) | validação completa | `main` (via promote) |
| `hotfix/<N>-slug` | produção urgente | validação completa | `main` **e** `develop` |
| `main` | releases publicadas | validação completa | — (só recebe release/hotfix) |

- Nomeie sempre com o número da issue: `feature/42-knob-drag`.
- **Push em branch de desenvolvimento NÃO dispara CI** (decisão de custo):
  valide localmente os gates da área e abra o PR — o CI roda no PR e na
  integração.
- Tags `v*` disparam o **release** (publish CLI+instalador); a cadeia
  rc→promote é manual (`release.yml`, input `action`).
- A **imagem de CI** (`container.yml`, ghcr.io) é artefato versionado: mudou o
  Dockerfile/lockfile em `develop`/`main` → a imagem `:1` é republicada; PR que
  mexe no Dockerfile só BUILDAA (sem push). Jobs de container referenciam `:1`
  (nunca `sha-<curto>`); bump de tag é manual e só quando incompatível.

## Ciclo operacional (agente)

1. **Issue** — confirme/abra; anote o número (`#N`).
2. **Branch** — `git checkout develop && git pull && git checkout -b feature/N-slug`.
3. **Implementação** — gates LOCAIS da área antes de tudo (ver skills
   `rust-practices`, `ui-ux-practices`, `core-dev`); teste acompanha código novo.
4. **Commit** — conventional commits (`tipo(escopo): assunto`) — o gate da CI
   REJEITA fora do padrão; footer Codebuff quando você (agente) escreveu.
5. **Push da branch + PR para `develop`** com o template preenchido; tabela de
   gates executados; **`Closes #N` NO CORPO** (nunca no título/comentário).
6. **CI do PR** — acompanhe até verde (`gh run watch <id> --exit-status`); PR
   com label `ci-lite` roda só Windows (WIP barato); tire a label na revisão.
7. **Review** — releia o diff (`git diff develop...HEAD`) e o PR antes do merge;
   achou algo? corrige na branch (novo commit) — nunca em commit "fix" solto
   na develop.
8. **Merge (squash) em `develop`** — o job `close-linked` do `ci.yml` fecha a
   issue via API com comentário de rastreabilidade (PR + sha + run).
9. **Backport/limpeza**: hotfix mergeia também em `develop`; release branch é
   apagada pelo `promote` (não apague à mão).

## Proibições explícitas

- **Nunca** `git push` direto em `main`/`develop` para trabalho normal (as
  branches de integração recebem PR; exceção: o próprio fluxo de release
  automatizado do `release.yml`).
- **Nunca** commitar sem issue correspondente (nem "arruma isso rapidinho").
- **Nunca** marcar ✅ em doc antes do CI verde (marco com CI vermelha não é marco).
- **Nunca** usar `Closes #N` em PR que não conclui a issue (só o PR final).
- **Nunca** fechar issue de `achados-security` por PR: fechamento é manual, com a
  correção provada.
- **Nunca** contornar gate vermelho (relaxar `-D warnings`, skip de job, `--no-verify`).

## Comandos de apoio

```bash
gh issue list --milestone v1.0.0 --state open          # trabalho aberto
gh issue develop <N> --branch-prefix feature/ -b develop # branch ligada à issue (opcional)
gh pr create --base develop --title "…" --body "Closes #N"
gh pr checks --watch                                   # estado do CI do PR
gh run watch <run-id> --exit-status                    # uma run específica
gh issue view <N> --json state,title,labels            # estado da issue
```
