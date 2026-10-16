---
name: docs-sync
description: Sincroniza a documentação após qualquer marco — INDEX.md, knowledge.md, status das fases no ROADMAP (as issues vivem no GitHub) e banners de status dos docs.
metadata:
  category: documentation
---

# Sincronização de documentação (pós-marco)

Use ao concluir qualquer issue do GitHub (milestone `v1.1.0` — Era Hardware), descoberta de
hardware ou mudança de status de fase. O trabalho aberto é gerido nas ISSUES —
aqui só se reflete o que já está fechado/provado. A documentação é o produto coletivo — dessincronizada, ela vira
fonte de retrocesso (viola a fonte única por assunto do §7 deste checklist).

## Checklist de sincronização
1. **ROADMAP.md** — marcar a fase/linha ✅ + data e apontar a issue do GitHub
   (não reescrever o detalhe da issue aqui); se a fase virou 100%, destacar no topo.
2. **knowledge.md** — atualizar "Estado vivo" (1–3 linhas por marco, sem prosa).
3. **docs/INDEX.md** — novo documento? Entrar no inventário com status; doc mudou
   de natureza (ex.: virou histórico)? Atualizar a tabela. **Falta skill/prompt
   novo na seção "Infra do agente"?** (skills ativas em `.agents/skills/`,
   templates em `.agents/prompts/`, histórico em `docs/arquivo/`).
4. **README.md** — só se algo do panorama mudou (marco estrutural, não issue menor).
5. **Banners de status** — qualquer doc que ganhou obsolescência parcial ganha
   banner ✅/⚠️/📜 com ponteiro para o substituto (a forma canonica — UMA linha,
   marcador + data — e do INDEX §4.1, e o `check_docs.py` a cobra).
6. **PROTOCOL.md** — descobertas de protocolo SEMPRE aqui (com evidência), nunca
   só no knowledge/ROADMAP.
7. **CONTADORES** — número de teste, cobertura, e2e, baselines ou contagem de
   arquivo de ícone **pertence a `docs/INDEX.md` §6** e a nenhum outro lugar. Se o
   número mudar: atualizar o §6 e **apagar** as ocorrências antigas em
   README/ROADMAP/knowledge/docs de UI, substituindo por ponteiro ("N testes:
   `INDEX.md` §6"). Contador repetido em 2 docs = dívida garantida.
8. **Mecanismo citado ≠ mecanismo atual** — quando um doc descreve *como* uma
   parte do repo funciona (workflow, job, action, label, script), conferir o arquivo
   real antes de dar o fato por bom. A lista de skills desta pasta é ela própria
   parte do que este checklist sincroniza.
   **Corolário:** dívida já paga não pode continuar anunciada — antes de manter um
   aviso "⚠️ #N pendente" num doc, abra a issue e confira o artefato que ela acusa;
   aviso de dívida quitada contradiz o §6 e manda o próximo agente procurar defeito
   que não existe.
9. **Números têm baseline (develop)** — o §6 é medido em `develop`, não "aqui".
   Numa branch de feature os números divergem **por construção** (a branch *adiciona*
   testes). Antes de "corrigir" o §6, confirme em que branch você mediu; divergência
   em branch não é erro do §6.
10. **O par checklist × relatório do mesmo gate conta UMA história** —
    `*_CHECKLIST.md` e `*_REPORT.md` do mesmo gate levam o **mesmo marcador de
    status** (os dois ✅, ou os dois ⚠️, ou os dois 📜) e nenhum dos dois se
    anuncia como *template* ou "aguardando a sessão" quando o §3 do INDEX já dá o
    gate como fechado. Isto é **conteúdo**, não forma: a forma (banner de UMA
    linha, marcador + data) o `check_docs.py` já cobra, e é o único arquivo que
    enuncia as duas regras — esta aqui não se repete no INDEX nem no gate
    (#287: o `H4_REPORT` dizia 📜 contra o checklist ⚠️ parcial, e o `H3_REPORT`
    ainda se dizia template com o gate fechado desde 04/10).
11. **Gerador que emite arquivo versionado escreve LF explícito** — em Windows o
    modo texto (`open(..., "w")`) traduz `\n`→`\r\n` e suja a árvore a cada suite,
    contra o `* text=auto eol=lf` do `.gitattributes`. Use `newline="\n"` e, como
    critério de aceite de qualquer gerador novo, rode a    suite e exija **`git status` limpo**.

> **Dívida registrada do item 10:** os pares H1 e H2 ainda divergem
> (`H1_CHECKLIST` sem banner próprio com o status ⏳ do transporte × `H1_REPORT` 📜 e
> "template"; `H2_CHECKLIST` 📜 × `H2_REPORT` ✅). Estão na
> [#289](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/289) —
> reescrever o conteúdo dos relatórios H1/H2 ficou fora do escopo da #287.

## Regras
- **Commit da docs-sync é PRÓPRIO e SEPARADO** do commit da mudança, na MESMA
  branch/PR (`feat(ui): …` + `docs: …` como commits distintos) — nunca escondido
  dentro do commit de código nem misturado a código pendente.
- **Issue fechada é a fonte do "feito"**: o ROADMAP aponta (link/`#N`), não
  duplica DoD/checklist; se o doc divergir da issue, a issue manda.
- knowledge.md = estado vivo curto (agentes leem inteiro a cada sessão).
- INDEX.md = mapa de navegação (agentes escolhem rota por ele).
- ROADMAP.md = plano e progresso (owner acompanha por ele).
- Nunca deixar a mesma informação viva em 2 lugares: uma fonte de verdade por
  assunto (tabela do INDEX §2), os outros lugares apenas apontam para ela.
- **Documento de histórico preserva o marco, não o mecanismo**: ao superseder um
  workflow/script, o ROADMAP continua dizendo "FEITO em 01/10 — `release.yml` com
  `action=rc`", mas ganha uma linha dizendo que o mecanismo **foi supersedido** e
  por quê. Não reescrever o marco; marcar o desvio.

## Apêndice — proveniência das regras

A proveniência de cada regra (issue/data que a originou) saiu daqui para o skill não
carregar histórico: está em
[`docs/arquivo/docs-sync-proveniencia-2026-10.md`](../../docs/arquivo/docs-sync-proveniencia-2026-10.md).
