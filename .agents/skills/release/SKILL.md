---
name: release
description: Cadeia de release do projeto — rc por workflow_dispatch (action=release), promote (rc → tag final → main), play (tag por semver), guarda da main, artefatos da tag v* (instaladores, tarball de fonte p/ PKGBUILD, CLI de campo) e sincronização dos 5 manifests de versão. Use ao cortar rc, promover release, publicar tag ou diagnosticar cadeia de distribuição.
metadata:
  category: workflow
---

# Release (rc → promote → tag)

O fluxo humano compõe: **rc → teste do prerelease → promote → tag final na
`main`**. Tudo por `workflow_dispatch` no `ci.yml` (nunca por push — versionar
no push é a receita de tag prematura); o CORPO de cada etapa vive em
`.github/workflows/release-{rc,promote,play}.yml` (`workflow_call`, #245) e o
`ci.yml` guarda o gatilho, o grafo e as permissões. Detalhe humano:
`docs/CONTRIBUTING.md` §6; empacotamento: `docs/tecnico/PACKAGING.md`.

## A cadeia

| Etapa | Input | O que faz |
|---|---|---|
| RC | `action=release` (+ `source=develop\|main`, `version` opcional) | calcula a próxima versão por semver (feat desde a última tag → minor; hotfix sobre `main` → patch), cria/atualiza `release/x.y.z`, bumpa os 5 manifests, commita, tagueia `vX.Y.Z-rc.N` e publica no MESMO run |
| Teste | — | o prerelease é testado à mão (checklist da release); correções entram na release branch e another dispatch de `release` avança a rc |
| Promote | `action=promote` | localiza a rc, bumpa para a versão final, commita, cria a tag `vX.Y`, tenta fazer `main` receber o **SHA da tag**, mergeia a release na `develop`, avança a develop para a próxima minor e apaga a release branch. ⚠️ o passo da `main` **falha** hoje (`GH006`, #234): o push é do OWNER — ver abaixo |
| Play | `action=play` | corta tag direto pelo semver dos commits (`version-bump`), sem rc — para correção já estabilizada |

## Regras que não se reabrem

- **Tag empurrada com `GITHUB_TOKEN` não dispara workflow.** Por isso
  rc/promote publicam os artefatos **no mesmo run**; o gatilho de tag `v*`
  cobre só tags criadas por HUMANO. Não "otimizar" separando isso.
- **`main` só recebe o SHA de uma tag `v*`** — o job `Release · guarda da main`
  é o alarme (roda em todo push na `main` e falha se o sha não for de tag).
  Em repo pessoal o GitHub não permite branch protection real; o guard é a
  regra.
- ⚠️ **Hoje a promote NÃO move a `main` sozinha.** A `main` exige histórico
  LINEAR e o SHA da tag traz o merge do ciclo — o push morre com `GH006`
  ([#234](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/234)) e
  o **owner** empurra a `main` à mão com o comando documentado na issue. O
  guard acima segue valendo (a `main` só recebe SHA de tag); o que não vale é
  assumir que o job faz o push.
- **Release branch não roda testes de propósito** (freeze da develop, já
  testada no merge); o gate da release é o **push da tag `v*`**, que roda a
  suíte inteira antes de gerar artefato.
- Bump de versão é **bloco único**: `uv run python .github/scripts/sync_version.py --set X`
  e confirme com `--check` — os 5 manifests (`Cargo.toml` da raiz,
  `packages/app/api/Cargo.toml`, `tauri.conf.json`, `packages/app/ui/package.json`,
  `version.json`) têm de casar; fora de sincronia o instalador sai com versão
  velha (#73).

## Artefatos da tag (`stage-dist`)

| Job | Saída |
|---|---|
| `Distribuição · instalador` | Windows (`nsis,msi`), macOS Intel (`app,dmg`), Ubuntu (`deb,appimage`) — anexados na Release com sha256 |
| `Distribuição · tarball de fonte` | `make_sdist.py` monta o tarball COM `ui/dist` e `packages/core` (insumo do PKGBUILD do Arch, #72) e prova que é consumível (`cargo metadata --locked` dentro) |
| `Distribuição · CLI de campo` | zip Windows com o binário + checklists H1/H2/H3 + scripts de campo (o binário é de LEITURA; escrita exige rebuild pós-H2) |

O empacotamento Arch vivo é `packaging/arch/PKGBUILD` (a unidade de distribuição
do AUR é a receita versionada; pendências #72).

## Checklist antes de disparar

1. Develop verde (última run de push), CHANGELOG/notes decididos.
2. `sync_version.py --check` limpo localmente.
3. Disparar `action=release`, acompanhar o run, publicar o prerelease.
4. Testado? `action=promote`; conferir tag, `main`, develop avançada, branch
   apagada e artefatos na Release.
5. Fechar o ciclo: issues da milestone saíram com a tag (comentário do
   fechamento diz "a issue sai oficialmente na release").
