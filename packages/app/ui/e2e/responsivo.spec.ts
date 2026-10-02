/**
 * R-RESPONSIVO (docs/UI_TEST_PLAN.md) — alinhamento da casca em 1440/1280/1024
 * no Chromium (CI e local). Asserts portados de analysis/responsivo_checks.js
 * (27/27 validados no Chrome do sistema): overflow, cadeia do board numa
 * ÚNICA fileira de 9 pedais, biblioteca lado a lado/empilhada, looper sem
 * sobreposição e sem texto clipado.
 *
 * Refactor POM (V-7): _helpers.ts segue como módulo de MEDIDAS (não é page
 * object — é geometria de página inteira); o spec usa o ShellPage só para
 * o contrato de goto/banner.
 */
import { expect, test } from "@playwright/test";
import { expectShellAligned, measureShell } from "./_helpers";
import { ShellPage } from "./pages/_pages";

let shell: ShellPage;

test.beforeEach(async ({ page }) => {
  shell = new ShellPage(page);
  await shell.beforeStandard();
});

for (const [w, h] of [
  [1440, 900],
  [1280, 800],
  [1024, 768],
] as const) {
  test(`R-RESPONSIVO ${w}×${h}: painéis alinhados, 0 overflow`, async ({ page }) => {
    await page.setViewportSize({ width: w, height: h });
    await page.waitForSelector('[aria-label="Looper (máquina de fita)"]');
    await page.waitForTimeout(600); // settle de layout pós-boot

    await expectShellAligned(page, w);

    // looper: nº de tracks por largura (auto-fit; 4ª colapsa a 0px @1440 — ok)
    const m = await measureShell(page);
    const wantTracks = w <= 1100 ? 3 : w <= 1340 ? 3 : 4;
    expect(m.looper.tracks).toBe(wantTracks);
  });
}
