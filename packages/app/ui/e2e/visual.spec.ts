/**
 * Visual (R-VISUAL, docs/UI_TEST_PLAN.md) — regressão ESTÉTICA dos painéis
 * com `toHaveScreenshot` (além do alinhamento geométrico do responsivo.spec).
 *
 *  - 3 painéis (board, looper, biblioteca) × 3 viewports = 9 baselines;
 *  - + estados de ERRO (alerta do boot e banner de preset do App) × 3
 *    viewports = 6 baselines, via gancho gp100.debug.failDevice;
 *  - baseline POR PLATAFORMA (fontes divergem entre Windows/Linux — o CI
 *    compara com as baselines `linux`, o dev local com as `win32`);
 *  - animações congeladas + tolerância 1% (config do playwright.config);
 *  - SEM baseline: o teste é SKIP (não quebra a primeira execução); gere com
 *    `pnpm exec playwright test visual.spec.ts --update-snapshots` e commite
 *    os PNGs — no CI há o input `update-snapshots` do workflow.
 *
 * Refactor POM (V-7): seletores via pages/_pages.ts e a falha simulada pelo
 * shell.failDevice(). RESTAURADO (comportamento documentado no cabeçalho,
 * perdido em edição anterior): toHaveScreenshot dos loops de PAINÉIS e
 * TEMA CLARO — hoje só topbar/erros comparavam.
 */
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "@playwright/test";
import type { Locator } from "@playwright/test";
import { ShellPage } from "./pages/_pages";

const here = path.dirname(fileURLToPath(import.meta.url));
const baselinePath = (name: string) =>
  path.join(here, "__screenshots__", `${name}-${process.platform}.png`);

const PANELS = [
  { key: "board", page: "board" },
  { key: "looper", page: "looper" },
  { key: "lib", page: "lib" },
] as const;
type PanelKey = (typeof PANELS)[number]["page"];

/** root do painel pelo page object (mesmos seletores das versões anteriores). */
function panelLocators(shell: ShellPage): Record<PanelKey, Locator> {
  return {
    board: shell.board.root,
    looper: shell.looper.root,
    lib: shell.library.root,
  };
}

const VIEWPORTS = [
  [1440, 900],
  [1280, 800],
  [1024, 768],
  [800, 600],
  [1920, 1080],
  [2560, 1440],
  [2560, 1080],
] as const;

/** Baselines dedicadas ao TEMA CLARO (emulateMedia colorScheme light) —
 *  sufixo `-light` no nome do arquivo, mesma mecânica de skip/comparação. */
const LIGHT_VIEWPORTS = [
  [1440, 900],
  [1920, 1080],
] as const;

/** Estados de erro da casca (mesma mecânica dos e2e de error state):
 *  a falha simulada é setada ANTES do load e o banner aparece sozinho —
 *  no boot, após clicar o botão (o boot não é automático). */
const ERROR_STATES = [
  {
    key: "erro-boot",
    op: "boot",
    section: '[role="alert"]',
    wait: '[role="alert"]',
    clickBoot: true,
  },
  {
    key: "erro-preset",
    op: "board",
    section: '[role="alert"]',
    wait: '[role="alert"]',
    clickBoot: false,
  },
] as const;

/** skip quando NÃO há baseline e a execução não está atualizando.
 *  O sinal correto é `config.updateSnapshots` (o Playwright força `none` no
 *  CI e `missing` localmente) — depender de `process.env.CI` deixava os
 *  baselines ausentes falharem no job principal `e2e`, que roda TODOS os
 *  specs. Semântica: local cria (missing), CI compara ou SKIPa, e o input
 *  `update-snapshots` do workflow gera as baselines linux. */
async function skipIfBaselineMissing(name: string): Promise<void> {
  const updating = test.info().config.updateSnapshots !== "none";
  test.skip(
    !updating && !existsSync(baselinePath(name)),
    "baseline ausente — gere com --update-snapshots (input do workflow) e commite",
  );
}

/** espera o fim do auto-boot (faixa some do frame antes da captura). */
async function waitBootSettled(shell: ShellPage): Promise<void> {
  await shell.bootProgress
    .waitFor({ state: "detached", timeout: 5000 })
    .catch(() => {});
}

for (const [w, h] of VIEWPORTS) {
  for (const p of PANELS) {
    test(`R-VISUAL ${p.key} ${w}×${h}`, async ({ page }) => {
      const name = `${p.key}-${w}x${h}`;
      // Semântica (updateSnapshots default = "missing"; CI trava a escrita):
      //  - local sem baseline: CRIA e passa (developer-friendly);
      //  - CI sem baseline: SKIP (não falha a primeira execução);
      //  - CI com --update-snapshots (input do workflow): escreve e passa;
      //  - com baseline: COMPARA (é a regressão estética de fato).
      await skipIfBaselineMissing(name);

      const shell = new ShellPage(page);
      await page.setViewportSize({ width: w, height: h });
      await shell.goto();
      const panel = panelLocators(shell)[p.page];
      await panel.waitFor();
      await page.waitForTimeout(600); // settle de layout pós-boot (led-flip é congelado)
      await waitBootSettled(shell);

      await expect(panel).toHaveScreenshot(`${name}.png`);
    });
  }
}

/* Topbar isolado (navbar consolidada) — 1 baseline por viewport dark. */
for (const [w, h] of VIEWPORTS) {
  test(`R-VISUAL topbar ${w}×${h}`, async ({ page }) => {
    const name = `topbar-${w}x${h}`;
    await skipIfBaselineMissing(name);

    const shell = new ShellPage(page);
    await page.setViewportSize({ width: w, height: h });
    await shell.goto();
    await shell.banner.waitFor();
    await page.waitForTimeout(600);

    await expect(shell.banner).toHaveScreenshot(`${name}.png`);
  });
}

/* Tema CLARO: painéis principais (sufixo -light no baseline). */
for (const [w, h] of LIGHT_VIEWPORTS) {
  for (const p of PANELS) {
    test(`R-VISUAL ${p.key} light ${w}×${h}`, async ({ page }) => {
      const name = `${p.key}-${w}x${h}-light`;
      await skipIfBaselineMissing(name);

      const shell = new ShellPage(page);
      await page.setViewportSize({ width: w, height: h });
      await page.emulateMedia({ colorScheme: "light" });
      await shell.goto();
      const panel = panelLocators(shell)[p.page];
      await panel.waitFor();
      await page.waitForTimeout(600);
      await waitBootSettled(shell);

      await expect(panel).toHaveScreenshot(`${name}-light.png`);
    });
  }
}

for (const [w, h] of VIEWPORTS) {
  for (const s of ERROR_STATES) {
    test(`R-VISUAL ${s.key} ${w}×${h}`, async ({ page }) => {
      const name = `${s.key}-${w}x${h}`;
      await skipIfBaselineMissing(name);

      const shell = new ShellPage(page);
      await page.setViewportSize({ width: w, height: h });
      // falha simulada ANTES do load (localStorage disponível no mount)
      await shell.failDevice(s.op);
      await shell.goto();
      if (s.clickBoot) {
        await shell.rescanButton.click();
      }
      await page.waitForSelector(s.wait);
      await page.waitForTimeout(300); // settle do banner (sem animação própria)

      await expect(page.locator(s.section).first()).toHaveScreenshot(`${name}.png`);
    });
  }
}
