---
name: docs-sync
description: Sincroniza a documentação após qualquer marco — INDEX.md, knowledge.md, status das fases no ROADMAP (as issues vivem no GitHub) e banners de status dos docs.
metadata:
  category: documentation
---

# Sincronização de documentação (pós-marco)

Use ao concluir qualquer issue do GitHub (milestone `v1.0.0`), descoberta de
protocolo ou mudança de status de fase. O trabalho aberto é gerido nas ISSUES —
aqui só se reflete o que já está fechado/provado. A documentação é o produto coletivo — dessincronizada, ela vira
fonte de retrocesso (viola a regra R1).

## Checklist de sincronização
1. **ROADMAP.md** — marcar a fase/linha ✅ + data e apontar a issue do GitHub
   (não reescrever o detalhe da issue aqui); se a fase virou 100%, destacar no topo.
2. **knowledge.md** — atualizar "Estado vivo" (1–3 linhas por marco, sem prosa).
3. **docs/INDEX.md** — novo documento? Entrar no inventário com status; doc mudou
   de natureza (ex.: virou histórico)? Atualizar a tabela.
4. **README.md** — só se algo do panorama mudou (marco estrutural, não issue menor).
5. **Banners de status** — qualquer doc que ganhou obsolescência parcial ganha
   banner ✅/⚠️/📜 com ponteiro para o substituto.
6. **PROTOCOL.md** — descobertas de protocolo SEMPRE aqui (com evidência), nunca
   só no knowledge/ROADMAP.

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
