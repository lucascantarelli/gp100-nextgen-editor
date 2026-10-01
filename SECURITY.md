# Política de segurança

> **Status:** ✅ atual · **Última revisão:** 2026-10-01

## Reportando uma vulnerabilidade

**Não abra issue pública** para vulnerabilidades. Use o recurso de
[relato privado do GitHub](https://github.com/lucascantarelli/gp100-nextgen-editor/security/advisories/new)
("Report a vulnerability") ou contate o owner diretamente. Resposta esperada:
5 dias úteis.

## Escopo

- `gp100-core` / `gp100-cli` (Rust): panics/overflows, erros de framing que
  corrompam presets do device, violação da política de escrita (ADR-5).
- Proxy `winmm.dll`: gravação fora do esperado, crash do Suite.
- O que **não** é escopo: segurança do firmware da pedaleira (fechado, fora do
  nosso controle) e do Valeton Suite oficial.

## Política de secrets e material proprietário

1. **Nenhum token/secret no repositório** — o `GH_TOKEN` de automação vive no
   ambiente, nunca em arquivos; se vazar, revogar em
   github.com/settings/tokens e regenerar com **escopo mínimo** (`repo` + `workflow`).
2. **Material de origem da Valeton não é distribuído** (instaladores, firmware,
   driver, extrações) — bloqueado pelo `.gitignore` (README §7). O repo contém
   apenas análise, protocolo validado e código original.
3. **Escrita no device** só com captura validada e `WRITE_VERIFIED=true`
   (ADR-5; gate H2 do ROADMAP). Update de firmware: fora de escopo, sempre.
4. Secret scanning/push protection do GitHub: manter habilitados quando
   disponíveis para o repo; bloqueios de push por secret **não** devem ser
   contornados com `--no-verify`/bypass.

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

Projeto em desenvolvimento ativo (pré-M1): somente `main`.
