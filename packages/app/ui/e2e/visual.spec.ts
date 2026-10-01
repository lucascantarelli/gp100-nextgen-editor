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
 *  - SEM baseline: local CRIA (dev-friendly) e no CI o teste é SKIP — gerar
 *    no CI só pelo input `update-snapshots` do workflow, que liga
 *    `UPDATE_SNAPSHOTS=true` + `--update-snapshots=all` (baixe o artefato e
 *    commite os PNGs); ver skipIfBaselineMissing para a semântica completa.
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

/** Estamos no CI? (GitHub Actions define CI=true; toHaveScreenshot NÃO cria
 *  baseline nova automaticamente lá — o teste falha com "snapshot doesn't
 *  exist, writing actual"). */
const IS_CI = !!process.env.CI;

/** Atualização EXPLÍCITA de baselines (o workflow liga via input):
 *  env `UPDATE_SNAPSHOTS=true` (setada no step do pipeline) OU a flag de CLI
 *  `--update-snapshots`/`=all` (o override de CLI vence o default de CI).
 *  NUNCA depender do default de `config.updateSnapshots`: no CI o Playwright
 *  só aceita a flag explícita, e o default `missing` local é indistinguível
 *  de "quero gerar" (bug real da run 36935738610: 48 visuais falharam). */
function isUpdatingSnapshots(): boolean {
  if (process.env.UPDATE_SNAPSHOTS === "true") return true;
  const mode = test.info().config.updateSnapshots;
  return mode === "all" || mode === "changed";
}

/** skip quando NÃO há baseline, estamos no CI e ninguém pediu para gerar.
 *  Semântica: local cria (dev-friendly); CI compara quando existe e SKIPa
 *  quando não existe; o input `update-snapshots` do workflow gera as
 *  baselines linux que faltam (artefato → commit). */
async function skipIfBaselineMissing(name: string): Promise<void> {
  test.skip(
    IS_CI && !isUpdatingSnapshots() && !existsSync(baselinePath(name)),
    "baseline ausente no CI — gere com o input update-snapshots e commite o PNG",
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
      // Semântica em skipIfBaselineMissing:
      //  - local sem baseline: CRIA e passa (developer-friendly);
      //  - CI sem baseline: SKIP (não falha a primeira execução);
      //  - CI com input update-snapshots: escreve (=all) e passa;
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
