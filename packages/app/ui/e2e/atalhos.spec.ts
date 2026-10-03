/**
 * R-ATALHOS (docs/UI_TEST_PLAN.md) — atalhos globais executados no Chromium:
 * Espaço = drum play/stop · R = REC do looper · Esc fecha painéis do topo.
 * Doc de usuário: aba Help do Settings.
 *
 * Refactor POM (V-7): seletores encapsulados em e2e/pages/_pages.ts —
 * comportamento e asserts 1:1 com a versão original.
 */
import { expect, test } from "@playwright/test";
import { ShellPage } from "./pages/_pages";

let shell: ShellPage;

test.beforeEach(async ({ page }) => {
  shell = new ShellPage(page);
  await shell.beforeStandard();
});

test("R-ATALHOS Espaço: liga/para o drum pelo toggle ⏵/⏹ da navbar", async () => {
  const toggle = shell.drum.toggle;
  await expect(toggle).toContainText("⏵");

  await shell.page.keyboard.press("Space");
  await expect(toggle).toContainText("⏹");
  await expect(toggle).toHaveAttribute("aria-pressed", "true");

  await shell.page.keyboard.press("Space");
  await expect(toggle).toContainText("⏵");
  await expect(toggle).toHaveAttribute("aria-pressed", "false");
});

test("R-ATALHOS R: REC→PLAY no looper; digitar 'r' na busca não dispara", async ({ page }) => {
  // digitando na busca, o R é TEXTO — looper segue vazio
  const search = shell.library.search();
  await search.click();
  await search.pressSequentially("rock");
  await expect(shell.looper.recButton()).toBeVisible();
  await expect(shell.looper.tapeText("● VAZIA")).toBeVisible();

  // fora de campo de texto: R = REC (mesmo comportamento do botão ●)
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  await page.keyboard.press("r");
  await expect(page.getByRole("button", { name: "Parar gravação e tocar" })).toBeVisible();
  await expect(shell.looper.tapeText("● REC")).toBeVisible();

  // R de novo: para a gravação e toca (REC→PLAY)
  await page.keyboard.press("r");
  await expect(shell.looper.playButton()).toHaveAttribute("aria-pressed", "true");
});

test("R-ATALHOS Esc: fecha Settings → Drum → pushes (precedência); modal bloqueia transporte", async ({ page }) => {
  const drum = shell.drum;
  const settings = shell.settings;

  // modal aberto: Espaço/R inertes (Espaço no ⚙ focado reativa o próprio
  // botão — ele só ABRE —, não o drum)
  await shell.openSettings();
  await page.keyboard.press("Space");
  await expect(drum.toggle).toContainText("⏵");
  await expect(shell.looper.tapeText("● VAZIA")).toBeVisible();

  // Esc 1: fecha o Settings (topo)
  await settings.pressEscape();
  await settings.expectHidden();

  // Esc 2: fecha o drum (os dois modais se cobrem — o estado de ambos
  // ABERTOS ao mesmo tempo é do teste unitário de precedência; por mouse
  // um modal de cada vez)
  await drum.open();
  await expect(drum.chip).toHaveAttribute("aria-expanded", "true");
  await page.keyboard.press("Escape");
  await expect(drum.chip).toHaveAttribute("aria-expanded", "false");
  await expect(drum.panel).toBeHidden();

  // Esc 3: fecha o drawer de pushes
  const details = await shell.openPushes();
  await expect(details).toHaveJSProperty("open", true);
  await page.keyboard.press("Escape");
  await expect(details).toHaveJSProperty("open", false);

  // doc de usuário: a aba Help do Settings documenta os 3 atalhos
  await shell.openSettings();
  await settings.openTab("Help");
  const tabela = settings.dialog.getByRole("table", { name: "Atalhos de teclado" });
  await expect(tabela).toContainText("Bateria: tocar/parar");
  await expect(tabela).toContainText("Looper: REC");
  await expect(tabela).toContainText("Fecha o painel aberto do topo");
});
