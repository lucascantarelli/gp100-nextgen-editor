---
name: docs-sync
description: Sincroniza a documentação após qualquer marco — INDEX.md, knowledge.md, status das issues do ROADMAP e banners de status dos docs.
metadata:
  category: documentation
---

# Sincronização de documentação (pós-marco)

Use ao concluir qualquer issue do ROADMAP, descoberta de protocolo ou mudança de
status de fase. A documentação é o produto coletivo — dessincronizada, ela vira
fonte de retrocesso (viola a regra R1).

## Checklist de sincronização
1. **ROADMAP.md** — marcar a issue ✅ + data; se a fase virou 100%, destacar no topo.
2. **knowledge.md** — atualizar "Estado vivo" (1–3 linhas por marco, sem prosa).
3. **docs/INDEX.md** — novo documento? Entrar no inventário com status; doc mudou
   de natureza (ex.: virou histórico)? Atualizar a tabela.
4. **README.md** — só se algo do panorama mudou (marco estrutural, não issue menor).
5. **Banners de status** — qualquer doc que ganhou obsolescência parcial ganha
   banner ✅/⚠️/📜 com ponteiro para o substituto.
6. **PROTOCOL.md** — descobertas de protocolo SEMPRE aqui (com evidência), nunca
   só no knowledge/ROADMAP.

## Regras
- **Commit da docs-sync é PRÓPRIO e SEPARADO** do commit da mudança (padrão
  vigente: `6422cd1` M0.4 → `d2fa925 docs-sync: M0.4 ✅ …`; `d3088c8` M0.3 →
  `fc0b6e6 docs-sync: …`) — nunca escondido dentro do commit de código nem
  misturado a código pendente (o push do marco pode preceder a docs).
- knowledge.md = estado vivo curto (agentes leem inteiro a cada sessão).
- INDEX.md = mapa de navegação (agentes escolhem rota por ele).
- ROADMAP.md = plano e progresso (owner acompanha por ele).
- Nunca deixar a mesma informação viva em 2 lugares: uma fonte de verdade por
  assunto (tabela do INDEX §2), os outros lugares apenas apontam para ela.
