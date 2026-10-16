> 📜 **histórico** — Era de RE, arquivado na #168 · última revisão 2026-10-13 · casas vivas: `docs/INDEX.md` §3.

# Skills da Era de Engenharia Reversa (arquivadas em 11/10/2026 — #170)

Estas **5 skills** descreviam o processo da Era de Engenharia Reversa, em que a
especificação do GP-100 era reconstruída de tráfego capturado do **Suite
oficial** (proxy `winmm.dll`, logs em `%TEMP%`, `analysis/` como fonte). A Era
Hardware (#171) aposentou essa premissa: **o aparelho real é a fonte**
(ADR-13), o Suite instrumentado saiu do fluxo e a captura virou **evidência de
campo** (wire log do CLI de campo), não matéria-prima de spec.

Foram movidas para cá (não apagadas) porque ainda ensinam **fatos históricos
citáveis** — por que o golden tem a forma que tem, por que o `F7` final é
regra de ouro, por que a baseline era byte-a-byte. O que era **processo vivo**
foi para as skills da Era HW:

| Skill arquivada | O que fazia | Substituto hoje |
|---|---|---|
| `new-session` | roteiro de sessão de captura com o Suite instrumentado | skill `hardware-dev` (gate de campo; o CLI de campo consulta o aparelho sem Suite) |
| `capture-analyze` | copiava o log do `%TEMP%` e rodava os decoders de captura | skill `hardware-dev` (wire log + evidência) e `docs/tecnico/PROTOCOL.md` §13 (formas) |
| `proxy-build` | rebuildava o proxy `winmm.dll` (Rota 1 de captura) | **sem substituto** — era ferramenta de RE; o binário ficou na história do repo |
| `protocol-validate` | suíte `validate_golden` (5 provas) como gate de spec | `uv run pytest` (spec) até HW-3/HW-6; a regência passa para o aparelho + `guard_dados_canonicos` |
| `spec-baseline` | processo formal de bump da baseline (`protocol_golden.json`) | ADR-13: cada espaço de VALOR ganha modelo próprio; baseline de FORMA só muda com evidência de campo (skill `hardware-dev`) |

## Regra de leitura

Quem encontrar referência a estas skills em doc **histórico** (relatórios H1–H4,
`docs/analises/skills-audit-2026-09-29.md`, ADRs, `knowledge.md` em git) não deve
"restaurá-las": o conteúdo é registro da época. Referência em doc **vivo**
(`docs/*.md` atuais, `.agents/skills/*` ativos, README, templates) é bug de
sincronização — corrija apontando para o substituto desta tabela
(skill `docs-sync`, regra "mecanismo citado ≠ mecanismo atual").
