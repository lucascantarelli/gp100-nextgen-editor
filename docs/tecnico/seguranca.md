# Segurança — doc oficial

> **Status:** ✅ atual · **Última revisão:** 2026-10-13 (#167 — virou a doc
> oficial de segurança; o `SECURITY.md` da raiz é o ponteiro curto que o
> GitHub usa para a política de reporte).
>
> **Fonte única deste assunto.** O root `SECURITY.md` aponta para cá e não
> duplica conteúdo (regra do INDEX §4: mesmo assunto, um caminho só).

## Reportando uma vulnerabilidade

**Não abra issue pública** para vulnerabilidades. Use o recurso de
[relato privado do GitHub](https://github.com/lucascantarelli/gp100-nextgen-editor/security/advisories/new)
("Report a vulnerability") ou contate o owner diretamente. Resposta esperada:
5 dias úteis.

## Escopo

- `gp100-core` / `gp100-cli` (Rust): panics/overflows, erros de framing que
  corrompam presets do device, violação da política de escrita (ADR-5).
- `gp100-app` (Tauri): a superfície IPC é a única via para o aparelho
  (`packages/app/ui/src/ipc/` — nenhum `invoke` espalhado); um command novo
  que exponha escrita sem a trava do ADR-5 é achado de segurança.
- Proxy `winmm.dll` (histórico da era RE): gravação fora do esperado, crash
  do Suite.
- O que **não** é escopo: segurança do firmware da pedaleira (fechado, fora do
  nosso controle) e do Valeton Suite oficial.

## Política de secrets e material proprietário

1. **Nenhum token/secret no repositório** — o `GH_TOKEN` de automação vive no
   ambiente, nunca em arquivos; se vazar, revogar em
   github.com/settings/tokens e regenerar com **escopo mínimo** (`repo` +
   `workflow`).
2. **Material de origem da Valeton não é distribuído** (instaladores, firmware,
   driver, extrações) — bloqueado pelo `.gitignore` (README §7). O repo contém
   apenas análise, protocolo validado e código original.
3. **Escrita no device** só com captura validada e `WRITE_VERIFIED=true`
   (ADR-5; gate H2 do ROADMAP). Update de firmware: fora de escopo, sempre.
4. Secret scanning/push protection do GitHub: manter habilitados quando
   disponíveis para o repo; bloqueios de push por secret **não** devem ser
   contornados com `--no-verify`/bypass.

## Dados locais do app (o que fica na máquina do usuário)

- **Preferências** em `localStorage` do webview (chaves `gp100.*`: idioma,
  painéis, ajustes de drum etc.) — nada disso sai da máquina; o app não tem
  telemetria nem rede.
- **Biblioteca persistente** em SQLite no `app_data_dir` do Tauri (crate
  `gp100-library`, ADR-9): presets de usuário, histórico append-only e
  import/export JSON — arquivos locais, apagáveis pela UI ou pelo explorador.
- **Logs de sessão** (comando `device_log_session`): transcrição do fio para
  diagnóstico de campo. Contêm tráfego MIDI com o aparelho (inclui nomes de
  preset do device) — são **evidência local**, não versionados
  (`.gitignore` cobre `files/logs/`) e não devem ser anexados em issue
  pública sem triagem (nomes de preset podem identificar o usuário).

## Wire log e capturas

- O `--log` do `gp100-cli` (modo de campo) grava o fio em JSONL. As capturas
  que viraram spec (`docs/arquivo/analysis/captures/`) foram feitas com o
  Suite oficial instrumentado e contêm apenas tráfego do preset default.
- Regra R1 do ROADMAP: **nenhum byte de protocolo "adivinhado"** — evidência
  nova entra pelo fluxo R2/R3 (`build_golden` → `validate_golden` → bump de
  baseline), nunca por conversa.

## Advisories tolerados (risco aceito, com gatilho de reavaliação)

A varredura noturna (`cargo audit` ×2 + `pnpm audit --prod`) trata como
**achado** qualquer exit ≠ 0. Advisories de categoria `unsound`/`unmaintained`
não falham o `cargo audit` por padrão, mas continuam visíveis — a lista abaixo
é o registro explícito do que foi avaliado e aceito:

| Advisory | Crate | Motivo da tolerância | Reavaliar quando |
|---|---|---|---|
| RUSTSEC-2024-0429 (unsound, alerta Dependabot #1) | `glib` 0.18.x | o Tauri 2 (gtk-rs) trava a série 0.18; o fix só existe em 0.20+ — **não há upgrade possível** (Dependabot falha com `security_update_not_possible`) | o Tauri migrar de gtk-rs (bump ≥ 0.20) |
| RUSTSEC-2024-0370 (unmaintained) | `proc-macro-error` | dependência transitiva de macros de terceiros; sem substituto no lockfile | o crate upstream trocar de macro |

Regra: **novo** advisory (mesmo `unsound`/`unmaintained`) que apareça na
varredura abre a issue ACHADOS e é avaliado — tolerar é decisão registrada
aqui, nunca silêncio.

## Versões suportadas

Projeto em desenvolvimento ativo: o fluxo de release é o `docs/planejamento/RELEASE_PLAN.md`
(sem stores por decisão do owner); somente a `main` recebe o SHA de uma tag `v*`.
