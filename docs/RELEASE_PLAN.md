# 🚀 RELEASE PLAN — Issues pré-lançamento (binários multiplataforma para download direto)

> **Status:** ✅ atual · rev. 2 · 01/10/2026 — plano de EPIC-01..05 + issues REL-*
> para o primeiro lançamento multiplataforma (macOS Intel/ARM, Windows x64/ARM,
> Ubuntu, Linux genéricas). Fonte do escopo: auditoria do pipeline + requisitos
> de distribuição. Criar issues via templates `epic` (5 épicos) e `feature`
> (issues REL-*), labels: `release`, `ci` + `security` no EPIC-03.

---

## ⛔ Decisão do owner (01/10/2026) — DISTRIBUIÇÃO = SÓ DOWNLOAD DIRETO

**NÃO publicar em nenhuma store ou repositório externo, por enquanto:** nada de
Mac App Store, Microsoft Store, winget, Snapcraft Store, Flathub, AUR ou
repositório apt. O ÚNICO canal de distribuição é a **GitHub Release** (download
direto dos binários, com checksum). Formatos de *arquivo* para download direto
(NSIS, MSI, DMG, PKG, AppImage, DEB) continuam válidos — o que está fora é a
*publicação* em canais de terceiros. Agentes futuros: não re-introduzir essas
plataformas sem nova decisão do owner registrada aqui.

## 0. Auditoria do estado atual (provada no repo)

| Área | Estado hoje | Prova |
|---|---|---|
| Empacotamento | **Só Windows x64**: instalador NSIS (app) + `gp100-cli.exe` (gnu) | `packages/app/api/tauri.conf.json` → `bundle.targets: ["nsis"]`; jobs `release-cli`/`release-installer` em `windows-latest` |
| Versão nos bundlers | ✅ **RESOLVIDO (#73)** — `scripts/sync_version.py` governa os 5 manifests (`Cargo.toml`×2, `tauri.conf.json`, `package.json`, `version.json`) e tem `--check` no gate do Lint | `sync_version.py --check` |
| Assinatura | Nenhuma (Windows nem macOS) — nada de cert/entitlements/notarization | ausente no pipeline |
| Smoke de instalação | Só smoke de **hardware** (`hardware-smoke`) e testes de cargo — nenhum teste de instalador em ambiente limpo | `Distribuição · instalador` (build da release) |
| Auto-update | Inexistente (`tauri-plugin-updater` ausente, sem endpoint/keys) | `tauri.conf.json` sem `plugins` |
| Rollback | Manual e sem runbook | — |
| Base que AJUDA | Matriz 3-OS já existe (windows-latest, ubuntu-24.04, macos-15-intel); tags `v*` disparam release; guard de prerelease p/ `-rc`; cadeia rc→promote **simulada no gate** (`scripts/simulate_release.py`) | `ci.yml` |

**Lacunas → 5 épicos, 22 issues.** Dependências críticas: assinatura (EPIC-03)
é o que torna o download direto viável (sem ela, SmartScreen/Gatekeeper
bloqueiam — e agora é o ÚNICO canal); a fonte única de versão (REL-VER-01)
bloqueia TODOS os bundlers; o smoke (EPIC-02) consome os artefatos do EPIC-01.

Fluxo de release que os épicos devem preservar (GitFlow já implementado):
`rc` → tag `vX.Y.Z-rc.N` → **prerelease** (pipeline publica com `-rc`) →
`promote` → tag `vX.Y.Z` → release final. Tudo o que vier a partir daqui tem de
encaixar nessa cadeia (rc publica binários de teste; promote publica os finais).

---

## EPIC-01 — Build & Empacotamento Multiplataforma Nativo

> Objetivo: para CADA SO/arquitetura do escopo, o push de tag `v*` produz
> binário/instalador nativo(s), versionado(s), anexados à GitHub Release para
> download direto. Pré-requisito transversal: REL-VER-01.

### REL-VER-01 · [Transversal] Fonte única de versão para os bundlers
- **Epic:** EPIC-01 · **Plataformas:** todas · **Prioridade:** Blocker
- **Descrição:** hoje `tauri.conf.json` carrega `version: "0.1.0"` fixo enquanto
  o release-gitflow bumpa `Cargo.toml`×2 + `version.json`. O bundler do Tauri
  prioriza a versão do conf → qualquer instalador sairia `0.1.0` numa rc `0.2.0-rc.1`.
  Definir fonte única: remover `version` do conf (o Tauri cai no `Cargo.toml` do
  crate `gp100-ui`) **ou** sincronizar o conf no passo de release. Provar nos
  3 SOs que o número aparece certo no binário e nos instaladores.
- **ci_cd_workflow_spec:**
  - `trigger_event`: push de tag `v*` + simulação local
  - `runner_matrix`: windows-latest, ubuntu-24.04, macos-26 (prova única)
  - `steps_to_execute`: ["Remover `version` do tauri.conf.json (ou step que escreve do version.json)", "Estender `simulate_release.py` p/ aplicar a mesma regra do conf", "Build de prova nos 3 SOs lendo `pnpm tauri build --version` e metadados do instalador"]
  - `expected_artifacts`: ["instaladores com versão = tag"]
- **definition_of_done:**
  - [ ] rc de simulação `0.2.0-rc.1` produz instalador `0.2.0-rc.1` (não `0.1.0`)
  - [ ] `simulate_release.py` valida também `tauri.conf.json`/`package.json`
  - [ ] Sem duplicação: UM lugar define a versão (version.json)
- **edge_cases_and_risks:** ["bundler usa a version do conf — resolvido por `sync_version.py` (#73)", "CLI não usa tauri — Cargo.toml da raiz continua a fonte dele"]

### REL-WIN-01 · [Windows] NSIS x64 — binário de referência (formalizar naming)
- **Epic:** EPIC-01 · **Plataformas:** Windows x64 · **Prioridade:** Blocker
- **Descrição:** o NSIS x64 já existe no pipeline — esta issue FORMALIZA o que
  falta para virar binário de download direto: convenção de nome
  (`gp100-nextgen-editor_<versão>_x64-setup.exe`), versão vinda do REL-VER-01,
  licença/ícones no instalador e SHA256 na release. É o artefato que TODO
  usuário Windows baixa.
- **ci_cd_workflow_spec:**
  - `trigger_event`: push de tag `v*` (prerelease em `-rc`)
  - `runner_matrix`: windows-latest
  - `steps_to_execute`: ["Padronizar nome do artefato (sem productName com espaços)", "Versão do instalador = version.json (REL-VER-01)", "SHA256 + tabela na release notes"]
  - `expected_artifacts`: ["gp100-nextgen-editor_<versão>_x64-setup.exe", "*.sha256"]
- **definition_of_done:**
  - [ ] Tag de teste publica o setup com nome/versão canônicos
  - [ ] `git describe`/`--version` do app instalado casa com a tag
- **edge_cases_and_risks:** ["productName tem espaços — manter SLUG fixo desde o 1º release (mudar depois quebra atualização por cima)", "GUI não muda — só empacotamento"]

### REL-WIN-02 · [Windows] MSI x64 (deploy corporativo offline)
- **Epic:** EPIC-01 · **Plataformas:** Windows x64 · **Prioridade:** High
- **Descrição:** adicionar `msi` a `bundle.targets` (WiX via Tauri) para quem
  distribui por GPO/Intune/SCCM a partir do download direto (instalação
  silenciosa `msiexec /qn` sem store). Mesmo build do NSIS, artefato separado.
- **ci_cd_workflow_spec:**
  - `trigger_event`: push de tag `v*`
  - `runner_matrix`: windows-latest
  - `steps_to_execute`: ["Adicionar `msi` aos targets do tauri.conf.json", "Estender job release-installer p/ publicar MSI + SHA256", "Ícones/licença consistentes com o NSIS"]
  - `expected_artifacts`: ["gp100-nextgen-editor_<versão>_x64.msi", "*.sha256"]
- **definition_of_done:**
  - [ ] Tag de teste produz exe + msi com mesma versão na release
  - [ ] `msiexec /i *.msi /qn` instala e o app abre (ver REL-CI-01)
- **edge_cases_and_risks:** ["WiX não suporta paths com espaços em algumas versões — testar", "manter só NSIS é aceitável se o MSI atrasar — não bloqueia o lançamento"]

### REL-WIN-03 · [Windows] ARM64 (NSIS/MSI `aarch64-pc-windows-msvc`)
- **Epic:** EPIC-01 · **Plataformas:** Windows ARM64 · **Prioridade:** High
- **Descrição:** cross-compile do app p/ ARM64 (Surface/ThinkPad ARM). Rust
  target `aarch64-pc-windows-msvc` + bundler com `--target`; frontend é
  agnóstico. CLI: avaliar (WinMM funciona em ARM64 via emulação — provar ou
  marcar CLI como x64-only na release notes).
- **ci_cd_workflow_spec:**
  - `trigger_event`: push de tag `v*`
  - `runner_matrix`: windows-latest (cross p/ aarch64)
  - `steps_to_execute`: ["`rustup target add aarch64-pc-windows-msvc`", "Build `--target aarch64-pc-windows-msvc --bundles nsis,msi`", "Suffix `-arm64` nos artefatos + SHA256"]
  - `expected_artifacts`: ["*_arm64-setup.exe", "*_arm64.msi"]
- **definition_of_done:**
  - [ ] Artefatos arm64 na release com checksum
  - [ ] Instalação testada (VM/real ou documentada como best-effort)
- **edge_cases_and_risks:** ["NSIS em ARM64 tem histórico instável no Tauri — MSI é o caminho primário", "webview é do SO — risco baixo"]

### REL-MAC-01 · [macOS] DMG universal (aarch64 + x86_64)
- **Epic:** EPIC-01 · **Plataformas:** macOS · **Prioridade:** Blocker
- **Descrição:** primeiro artefato macOS: `universal-apple-darwin` (fat binary),
  DMG assinado+notarizado (assinatura no EPIC-03; aqui o bundle) anexado à
  release. Ícones `.icns` ainda não existem — gerar via `pnpm tauri icon`.
- **ci_cd_workflow_spec:**
  - `trigger_event`: push de tag `v*`
  - `runner_matrix`: macos-26 (arm) cross p/ universal
  - `steps_to_execute`: ["`rustup target add x86_64-apple-darwin`", "`pnpm tauri icon` p/ gerar icns (comitar resultado)", "Build `--target universal-apple-darwin --bundles app,dmg`", "Upload DMG + SHA256"]
  - `expected_artifacts`: ["gp100-nextgen-editor_<versão>_universal.dmg", "*.sha256"]
- **definition_of_done:**
  - [ ] DMG na release (prerelease nas `-rc`, final no promote)
  - [ ] Binário universal confirmado (`lipo -info` mostra arm64+x86_64)
  - [ ] App abre no runner macOS (sem assinatura: clicar-direito/Gatekeeper documentado)
- **edge_cases_and_risks:** ["sem assinatura o Gatekeeper bloqueia o download direto — EPIC-03 é obrigatório ANTES do anúncio público", "risco principal é openssl/deps do cargo"]

### REL-MAC-02 · [macOS] PKG para download (deploy corporativo via `pkgbuild`)
- **Epic:** EPIC-01 · **Plataformas:** macOS · **Prioridade:** Medium
- **Descrição:** o Tauri não tem target `pkg` nativo — wrapping com `pkgbuild`
  do `.app` universal (REL-MAC-01) + `productbuild` do distribution.xml.
  Distribuído como download direto p/ quem usa MDM/Jamf.
- **ci_cd_workflow_spec:**
  - `trigger_event`: push de tag `v*` (atrás de flag/condição)
  - `runner_matrix`: macos-26
  - `steps_to_execute`: ["script `scripts/mac_pkg.sh` (pkgbuild --component app --scripts preinstall/postinstall)", "productbuild com title/version/identifier", "Assinar com cert Installer (REL-SIGN-02) e publicar"]
  - `expected_artifacts`: ["gp100-nextgen-editor_<versão>.pkg"]
- **definition_of_done:**
  - [ ] `installer -pkg *.pkg -target /` funciona no runner limpo
  - [ ] postinstall não copia por cima de /Applications sem checar versão
- **edge_cases_and_risks:** ["PKG precisa de cert SEPARADO (Developer ID Installer) do app (Application)", "versioning do pkg tem de casar com o app"]

### REL-LIN-01 · [Linux] AppImage x86_64 (distribuições genéricas)
- **Epic:** EPIC-01 · **Plataformas:** Linux genéricas · **Prioridade:** Blocker
- **Descrição:** AppImage é o "download direto" do Linux: um binário que roda
  em quase qualquer distro, anexado à release. Tauri bundler `appimage` no
  ubuntu-22.04 (floor de glibc DECIDIDO aqui: 22.04 = compatibilidade máxima;
  24.04 = libs novas).
- **ci_cd_workflow_spec:**
  - `trigger_event`: push de tag `v*`
  - `runner_matrix`: ubuntu-22.04
  - `steps_to_execute`: ["Deps do Tauri (webkit2gtk-4.1 etc.)", "`tauri build --bundles appimage` com `APPIMAGE_EXTRACT_AND_RUN=1` (CI sem FUSE)", "Publicar com sufixo `-amd64.AppImage` + SHA256"]
  - `expected_artifacts`: ["gp100-nextgen-editor_<versão>_amd64.AppImage", "*.sha256"]
- **definition_of_done:**
  - [ ] AppImage roda no runner (extract-and-run) e num container Ubuntu 22.04 limpo
  - [ ] `--appimage-extract-and-run --version` reporta a versão da tag
- **edge_cases_and_risks:** ["linuxdeploy precisa de FUSE — sempre usar env extract-and-run no CI", "floor glibc: build em 24.04 não abre em 22.04 — FICAR no 22.04 e documentar"]

### REL-LIN-02 · [Linux] DEB para download direto (Ubuntu 22.04/24.04 e derivadas)
- **Epic:** EPIC-01 · **Plataformas:** Ubuntu · **Prioridade:** Blocker
- **Descrição:** target nativo `deb` do Tauri, distribuído como ARQUIVO na
  release (instalação local `apt install ./arquivo.deb` — NÃO é publicação em
  repositório apt). Definir nome de pacote slug `gp100-nextgen-editor`
  (productName tem espaços — sanitize) e `mainBinaryName`.
- **ci_cd_workflow_spec:**
  - `trigger_event`: push de tag `v*`
  - `runner_matrix`: ubuntu-22.04
  - `steps_to_execute`: ["Configurar `bundle.linux.deb` (depends: libwebkit2gtk-4.1-0, libgtk-3-0…)", "`tauri build --bundles deb`", "Publicar `_amd64.deb` + SHA256"]
  - `expected_artifacts`: ["gp100-nextgen-editor_<versão>_amd64.deb", "*.sha256"]
- **definition_of_done:**
  - [ ] `dpkg -I *.deb` mostra versão/deps corretos; instala em container 22.04 E 24.04
  - [ ] `apt install ./file.deb` resolve deps sozinho num container limpo
- **edge_cases_and_risks:** ["nome com espaços/maiúsculas quebra ferramentas locais — slug obrigatório", "sem repositório apt: o usuário gerencia atualização manual (updater cobre AppImage/NSIS/DMG, não DEB)"]

### REL-GEN-01 · [Transversal] Checksums, SBOM e manifest do release
- **Epic:** EPIC-01 · **Plataformas:** todas · **Prioridade:** High
- **Descrição:** um único `SHA256SUMS` assinado (REL-SIGN-04) + SBOM (cargo
  `cyclonedx`/`syft`) anexados à release; release notes por plataforma com
  seção "o que baixar" por SO — é a "loja" do projeto (a própria página da
  release).
- **ci_cd_workflow_spec:**
  - `trigger_event`: push de tag `v*` (job final, `needs` todos os builds)
  - `runner_matrix`: ubuntu-24.04
  - `steps_to_execute`: ["baixar todos artefatos do run", "gerar SHA256SUMS + SBOM", "anexar na release + tabela nas notes"]
  - `expected_artifacts`: ["SHA256SUMS", "sbom.json", "RELEASE_NOTES.md"]
- **definition_of_done:**
  - [ ] Todos os binários da release cobertos pelo checksum (nada órfão)
  - [ ] SBOM listando deps Rust+npm da versão
- **edge_cases_and_risks:** ["job final pode falhar por 1 artefato faltando — fail fast com lista explícita esperada"]

---

## EPIC-02 — Validação Automatizada de Instalação em Ambientes Limpos (CI)

> Objetivo: ciclo completo **Instalar → Executar smoke → Desinstalar** para
> cada formato, em runner/contêiner efêmero (GitHub-hosted já é efêmero por
> job; contêineres Linux dão isolamento real). Gate da RELEASE, não do push.

### REL-CI-01 · [Transversal] Framework de smoke por formato (lifecycle completo)
- **Epic:** EPIC-02 · **Plataformas:** todas · **Prioridade:** Blocker
- **Descrição:** scripts idempotentes `scripts/smoke/{win,mac,linux}` que
  recebem o artefato e executam: instalação silenciosa → abre o app (processo
  vivo + janela/critério de saída) → desinstala → assert de resíduo. Um spec
  por formato: NSIS (`/S`), MSI (`msiexec /qn`), DMG (hdiutil attach/cp/open),
  PKG (`installer`), DEB (apt container), AppImage (container + extract-and-run).
- **ci_cd_workflow_spec:**
  - `trigger_event`: chamado pelo job REL-CI-02 (workflow_call)
  - `runner_matrix`: windows-latest, macos-26, ubuntu-22.04/24.04 + containers distro
  - `steps_to_execute`: ["instalar", "executar com timeout e capturar exit/log", "desinstalar", "checar resíduos (registry/~/Library/xdg)"]
  - `expected_artifacts`: ["log do ciclo por formato (upload on failure)"]
- **definition_of_done:**
  - [ ] Cada formato tem um spec que roda verde no CI com artefato real de rc
  - [ ] Falha de qualquer fase falha o job com log capturado
- **edge_cases_and_risks:** ["runner GitHub JÁ TEM VC++/webkit pré-instalados — smoke valida o instalador, não a máquina do usuário final; riscos residuais documentados", "GUI sem display no Linux/macOS: xvfb-run / login auto no runner macOS"]

### REL-CI-02 · [Transversal] Job `release-smoke` no pipeline (gate da tag)
- **Epic:** EPIC-02 · **Plataformas:** todas · **Prioridade:** Blocker
- **Descrição:** job no pipeline que roda APÓS os builds de release (`needs`:
  release-cli, release-installer, matriz nova do EPIC-01), baixa os artefatos
  e executa o framework REL-CI-01 antes da publicação final — em rc roda e
  publica mesmo assim (prerelease é o lugar de risco); na tag FINAL, smoke
  falho = release não sai (split: publish draft → smoke → flip to published).
- **ci_cd_workflow_spec:**
  - `trigger_event`: push de tag `v*`
  - `runner_matrix`: matriz por formato (≤10 min cada)
  - `steps_to_execute`: ["download artifacts", "smoke por formato", "sumário na GITHUB_STEP_SUMMARY com matriz pass/fail"]
  - `expected_artifacts`: ["logs", "summary"]
- **definition_of_done:**
  - [ ] rc publicada com smoke verde; smoke vermelho bloqueia promote real (checklist do §6)
  - [ ] Summary da run mostra a tabela de formatos × resultados
- **edge_cases_and_risks:** ["smoke consome ~10-20 min — paralelizar por SO", "flaky de GUI: retries explícitos e timeout por passo"]

### REL-CI-03 · [Transversal] Desinstalação limpa e resíduos
- **Epic:** EPIC-02 · **Plataformas:** todas · **Prioridade:** Medium
- **Descrição:** assert de "nada sobrou" (ou resíduo DOCUMENTADO): Windows
  (registry uninstall keys, %APPDATA%), macOS (~/Library/Application Support,
  LaunchAgents), Linux (xdg dirs, .desktop). Desinstalar DEVE preservar a
  biblioteca do usuário (dados ≠ app) — testar e documentar.
- **ci_cd_workflow_spec:**
  - `trigger_event`: junto do REL-CI-02
  - `runner_matrix`: idem
  - `steps_to_execute`: ["snapshot antes/depois da instalação", "diff pós-desinstalação", "lista de resíduos aceitáveis no doc"]
  - `expected_artifacts`: ["residue-report"]
- **definition_of_done:**
  - [ ] Relatório de resíduos por plataforma anexado na release (seção docs)
  - [ ] Biblioteca de presets NÃO é apagada na desinstalação (prova)
- **edge_cases_and_risks:** ["atualização por cima vs fresh install — smoke cobre os dois caminhos", "NSIS deixa dados de usuário por design — documento, não bug"]

---

## EPIC-03 — Assinatura Digital, Notariação e Segurança de Binários

> Objetivo: zero SmartScreen/Gatekeeper no download direto — que agora é o
> ÚNICO canal, então assinatura deixa de ser diferencial e vira requisito de
> adoção. Requer contas/certificados PAGOS — orçamento no roadmap (decisão do
> owner antes de implementar).

### REL-SIGN-01 · [Windows] Azure Trusted Signing (EV-equivalente) p/ exe/msi/cli
- **Epic:** EPIC-03 · **Plataformas:** Windows · **Prioridade:** Blocker
- **Descrição:** Trusted Signing (Azure) elimina cert físico EV e dá
  reputação SmartScreen imediata. Assinar os 3 binários (NSIS, MSI, CLI) no
  job de release. Autenticação por OIDC federado (sem segredo estático).
- **ci_cd_workflow_spec:**
  - `trigger_event`: push de tag `v*` (APÓS build, antes do upload)
  - `runner_matrix`: windows-latest
  - `steps_to_execute`: ["azure/login (federated) + trusted-signing-action", "sign dos artefatos", "verificação `Get-AuthenticodeSignature` como assert"]
  - `expected_artifacts`: ["binários assinados (mesmos nomes + SHA256 novos)"]
- **definition_of_done:**
  - [ ] `Get-AuthenticodeSignature` = Valid em todos os artefatos Windows
  - [ ] Smoke REL-CI-01 roda com binário ASSINADO (double-check do instalador)
  - [ ] Segredos via OIDC — nenhum segredo estático no repo
- **edge_cases_and_risks:** ["conta Azure + validação de identidade leva ~3 dias — iniciar cedo", "assinatura de MSI muda o hash — gerar SHA256SUMS DEPOIS de assinar (ordem no REL-GEN-01)"]

### REL-SIGN-02 · [macOS] Developer ID + notariação (notarytool/stapler)
- **Epic:** EPIC-03 · **Plataformas:** macOS · **Prioridade:** Blocker
- **Descrição:** conta Apple Developer, `Developer ID Application` (e `Installer`
  p/ PKG), hardend runtime + entitlements mínimos, notariação via App Store
  Connect API key e `stapler`. Tauri aceita env `APPLE_*` nativo no build.
  (A conta da Apple é usada só p/ certificar o binário — o app NÃO vai à Mac
  App Store.)
- **ci_cd_workflow_spec:**
  - `trigger_event`: push de tag `v*`
  - `runner_matrix`: macos-26
  - `steps_to_execute`: ["importar cert via secret P12 (base64)", "build com APPLE_API_ISSUER/KEY/KEY_PATH", "`spctl -a -v` e `stapler validate` como asserts", "DMG notarizado também (dmg assinado pós-build)"]
  - `expected_artifacts`: ["*.dmg/*.pkg notarizados (Gatekeeper ok)"]
- **definition_of_done:**
  - [ ] `spctl --assess` aceita o app; stapler ticket embutido
  - [ ] Download+abrir num macOS limpo sem override (prova manual documentada)
- **edge_cases_and_risks:** ["P12 expira — agenda de renovação + alerta", "entitlements demais = rejeição da notariação — começar mínimo"]

### REL-SIGN-03 · [Transversal] Keypair do updater (minisign) + cerimônia de chaves
- **Epic:** EPIC-03 · **Plataformas:** todas · **Prioridade:** Blocker
- **Descrição:** gerar par minisign do updater Tauri (`tauri signer generate`),
  pubkey COMMITADA no conf, private key como secret CI (`TAURI_SIGNING_PRIVATE_KEY`)
  — é o que assina os artefatos `.sig` que o auto-update valida (EPIC-05).
  Documentar rotação e backup (recuperação de desastre).
- **ci_cd_workflow_spec:**
  - `trigger_event`: one-off + rotação
  - `runner_matrix`: local (owner) — NUNCA gerar no CI
  - `steps_to_execute`: ["gerar par offline", "pubkey no tauri.conf.json (plugins.updater.pubkey)", "private key + password como secrets"]
  - `expected_artifacts`: ["pubkey no repo", "secrets configurados"]
- **definition_of_done:**
  - [ ] Build de tag gera `.sig` válido contra a pubkey commitada
  - [ ] Runbook de rotação/compromisso escrito (SECURITY.md link)
- **edge_cases_and_risks:** ["perda da private key = morte do updater — backup offline em 2 locais", "pubkey no binário: rotacionar = nova release full"]

### REL-SIGN-04 · [Transversal] SHA256SUMS assinado (minisign/GPG)
- **Epic:** EPIC-03 · **Plataformas:** todas · **Prioridade:** Medium
- **Descrição:** o checksum do REL-GEN-01 assinado com key dedicada de release;
  página/README ensina a verificar (`minisign -Vm SHA256SUMS`). Como o download
  direto é o único canal, é a linha de defesa contra espelho/S3 adulterado no
  futuro.
- **ci_cd_workflow_spec:**
  - `trigger_event`: job final da tag
  - `runner_matrix`: ubuntu-24.04
  - `steps_to_execute`: ["minisign -S com secret", "upload .minisig na release", "seção 'verificação' nas release notes"]
  - `expected_artifacts`: ["SHA256SUMS.minisig"]
- **definition_of_done:**
  - [ ] Verificação documentada e testada num container limpo
- **edge_cases_and_risks:** ["mais uma key p/ guardar — reutilizar a do updater se a política permitir separação menor"]

---

## EPIC-04 — GitHub Releases como canal único (download direto coeso)

> Objetivo: a página da release É a distribuição do projeto (decisão do owner:
> nenhuma store). Tudo aqui é sobre deixar ESSE canal coeso: uma release com
> todos os binários, notes claras, draft→smoke→published.

### REL-PUB-01 · [Transversal] Matriz única de release multiplataforma
- **Epic:** EPIC-04 · **Plataformas:** todas · **Prioridade:** Blocker
- **Descrição:** consolidar `release-cli` + `release-installer` (hoje 2 jobs
  Windows) numa matriz por SO/formato com `if` por booleano do plan (padrão
  do monorepo: matriz fixa + if, nunca 0 itens). Guard de prerelease `-rc`
  herdados por TODOS os novos SOs.
- **ci_cd_workflow_spec:**
  - `trigger_event`: push de tag `v*`
  - `runner_matrix`: ubuntu-22.04, ubuntu-24.04, macos-26, windows-latest
  - `steps_to_execute`: ["consolidar os workflows (FEITO na #68)", "smoke gates antes de publicar", "summary com artefatos por plataforma"]
  - `expected_artifacts`: ["todos os binários na MESMA release"]
- **definition_of_done:**
  - [ ] Tag de rc gera release com todos os formatos marcados prerelease
  - [ ] docs-only não dispara matriz (plan gate)
- **edge_cases_and_risks:** ["run longa — paralelismo por job, timeout em cada um", "reuso: composite actions existentes (setup-rust etc.)"]

### REL-PUB-02 · [Transversal] Release notes por plataforma + draft flow
- **Epic:** EPIC-04 · **Plataformas:** todas · **Prioridade:** High
- **Descrição:** notes com seção por SO (o que baixar, requisitos, verificação
  de assinatura/checksum), promoção draft→published DEPOIS do smoke
  (REL-CI-02). rc mantém nota de "o que testar".
- **ci_cd_workflow_spec:**
  - `trigger_event`: tag
  - `runner_matrix`: ubuntu-24.04
  - `steps_to_execute`: ["template de notes gerado do CHANGELOG/commits", "upload como draft", "job final publica"]
  - `expected_artifacts`: ["RELEASE_NOTES por tag"]
- **definition_of_done:**
  - [ ] Notes da release final listam todos artefatos + checksums + how-to-verify
- **edge_cases_and_risks:** ["automatizar sem chumbo: gerar dos conventional commits (semver já existe)"]

---

## EPIC-05 — Infraestrutura de Atualizações Automáticas (Auto-Update) e Rollback

> Objetivo: atualizar sem reinstalar, com rollback controlado — SEM store e
> SEM servidor próprio: o endpoint do updater é a própria GitHub Release
> (`latest.json` como asset). Nada aqui publica em canal externo. DEB não
> passa pelo updater (atualiza manualmente baixando o novo .deb) — documentar.

### REL-UPD-01 · [Transversal] Updater Tauri (plugin + config + artefatos .sig)
- **Epic:** EPIC-05 · **Plataformas:** Windows, macOS, Linux(AppImage) · **Prioridade:** Blocker
- **Descrição:** `tauri-plugin-updater` + `plugins.updater` no conf (pubkey do
  REL-SIGN-03, endpoints → `latest.json` da release). Builds de tag passam a
  gerar artefatos de update (NSIS setup, app.tar.gz mac, AppImage + `.sig`).
- **ci_cd_workflow_spec:**
  - `trigger_event`: push de tag `v*`
  - `runner_matrix`: windows-latest, macos-26, ubuntu-22.04
  - `steps_to_execute`: ["plugin + conf", "TAURI_SIGNING_PRIVATE_KEY no build", "upload dos pares artefato+.sig"]
  - `expected_artifacts`: ["setup.exe/.sig", "app.tar.gz/.sig", "AppImage/.sig"]
- **definition_of_done:**
  - [ ] App 0.1.0 instalado detecta e aplica update p/ 0.2.0 em cada SO suportado (smoke)
  - [ ] Assinatura inválida = update recusado (teste negativo)
- **edge_cases_and_risks:** ["updater no Windows substitui o exe em execução — usar o flow NSIS do plugin", "Linux: updater só AppImage — documentar no README"]

### REL-UPD-02 · [Transversal] latest.json + canais rc/stable
- **Epic:** EPIC-05 · **Plataformas:** todas (updater) · **Prioridade:** High
- **Descrição:** job que gera `latest.json` (versão, plataformas, URLs, assinaturas)
  da release atual; rc publica latest.json de PRÉ (endpoint `?prerelease` ou
  channel separado) só detectável por builds `-rc`; promote publica o estável.
  Instaladores apontam endpoints fixos por canal.
- **ci_cd_workflow_spec:**
  - `trigger_event`: tag (rc e final)
  - `runner_matrix`: ubuntu-24.04
  - `steps_to_execute`: ["gerar json do release", "validar contra schema do plugin", "upload como asset"]
  - `expected_artifacts`: ["latest.json (e latest-rc.json)"]
- **definition_of_done:**
  - [ ] rc não vaza update estável e vice-versa (prova nos dois caminhos)
  - [ ] Endpoint documentado p/ futuro CDN (URL é o único contrato)
- **edge_cases_and_risks:** ["ordem: latest.json só DEPOIS de todos os assets prontos (needs) — update parcial quebra install"]

### REL-UPD-03 · [Transversal] Rollback/revogação de release (runbook automatizável)
- **Epic:** EPIC-05 · **Plataformas:** todas · **Prioridade:** High
- **Descrição:** runbook de incidente pós-release: reverter `latest.json` p/ versão
  anterior (updater para de oferecer), marcar release como prerelease/retirar
  assets, comunicar (issue ACHADOS). Dry-run testado em tag de ensaio. Como não
  há stores, NÃO há passo de revert em canal externo — o rollback é 100%
  GitHub Releases.
- **ci_cd_workflow_spec:**
  - `trigger_event`: workflow_dispatch (crisis) — NUNCA automático
  - `runner_matrix`: ubuntu-24.04
  - `steps_to_execute`: ["workflow de rollback com input `version-alvo`", "permissões restritas (environments)"]
  - `expected_artifacts`: ["rollback executado + log"]
- **definition_of_done:**
  - [ ] Dry-run completo numa tag dummy (mesmo espírito do simulate_release.py)
  - [ ] Runbook em docs/ com checklist de 10 min
- **edge_cases_and_risks:** ["GitHub Release já baixada não é revogável — rollback protege o FUTURO updater, não os downloads passados", "TTL de caches do CDN futuro — header no endpoint"]

### REL-UPD-04 · [Transversal] Telemetria de lançamento (downloads + contagem por canal)
- **Epic:** EPIC-05 · **Plataformas:** todas · **Prioridade:** Medium
- **Descrição:** job pós-release que coleta download counts por asset (GitHub API)
  e publica summary/issue de acompanhamento. SEM tracking do usuário
  (SECURITY.md) — métrica é de distribuição, não uso. Como o GitHub Releases é
  o canal único, essa métrica cobre 100% da distribuição.
- **ci_cd_workflow_spec:**
  - `trigger_event`: schedule semanal
  - `runner_matrix`: ubuntu-24.04
  - `steps_to_execute`: ["gh api releases", "tabela por plataforma/tag", "comentário na issue do marco de release"]
  - `expected_artifacts`: ["relatório de adoção"]
- **definition_of_done:**
  - [ ] Primeira release com relatório gerado sozinho
- **edge_cases_and_risks:** ["rate limit da API — token do app, chamadas poucas", "números por asset ≠ usuários (updater baixa por máquina) — interpretar com caveat"]

---

## Ordem sugerida (caminho crítico)

1. **REL-VER-01** (versão certa em tudo) → 2. **EPIC-01** por plataforma
   (WIN-01/MAC-01/LIN-01/LIN-02 primeiro) → 3. **REL-SIGN-03** (keys) +
   **EPIC-03** (paraleliza: contas/cert demoram — e é o que desbloqueia o
   download direto) → 4. **REL-CI-01/02** (smoke começa com o que existir) →
   5. **REL-PUB-01/02** (release única coesa) → 6. **REL-UPD-01/02** (update) →
   7. o resto (MSI/PKG/ARM64/UPD-03/04) por demanda. rc do release-gitflow é o
   CAVALO DE TROIA: cada formato novo entra publicando numa rc de teste antes
   do promote real.

## Como transformar em issues

- 5 issues **epic** (template `epic`, checklist das filhas = medidor).
- 22 issues **feature** (template `feature`, título com `[ÉPICO-ID]`).
- Label `release` em todas + `security` no EPIC-03.
- Marcar ROADMAP (FASE release) na criação — e atualizar este doc quando
  cada issue abrir (número da issue no ID: `REL-WIN-01 (#42)`).
- **Não abrir** issues de stores/gerenciadores (decisão do owner no topo).
