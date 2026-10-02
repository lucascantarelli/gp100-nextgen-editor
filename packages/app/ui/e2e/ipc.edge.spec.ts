/**
 * Edge cases de IPC nível 2 (issue #20) — cenários que o MockDevice não
 * produz sozinho, armados pelo gancho `gp100.debug.failDevice`:
 *
 *   1. falha TRANSITÓRIA de select  → retry/backoff esconde do usuário;
 *   2. falha PERMANENTE de select   → banner com AÇÃO + UI no preset REAL
 *      (select falho nunca vira estado da UI) → retry aplica a intenção;
 *   3. disconnect MID-BOOT          → alerta amigável, LED off, barra morta
 *      (nunca spinner eterno) e recuperação pelo ⟳.
 *
 * `armFailDevice` (POM) arma depois do load: o mount roda limpo e o cenário
 * começa exatamente na ação do usuário.
 */
import { expect, test } from "@playwright/test";
import { ShellPage } from "./pages/_pages";

let shell: ShellPage;

test.beforeEach(async ({ page }) => {
  shell = new ShellPage(page);
  await shell.beforeStandard();
});

test("select transitório (1 falha): retry cobre e o preset abre sem alerta", async () => {
  await expect(shell.patchLabel()).toContainText("P01");
  await shell.armFailDevice("select:1");

  await shell.nextPatch();

  // o usuário nunca vê a falha: o command tentou de novo e abriu P02
  await expect(shell.patchLabel()).toContainText("P02");
  await expect(shell.alerts).toHaveCount(0);
});

test("select permanente: banner com ação, UI no preset REAL e retry recupera", async () => {
  await shell.armFailDevice("select");

  await shell.nextPatch();

  const alert = shell.alerts.filter({ hasText: "não aceitou a troca de preset" });
  await expect(alert).toBeVisible();
  // a UI NÃO mente: o device segue em P01 (o select não passou)
  await expect(shell.patchLabel()).toContainText("P01");

  // recuperação pela AÇÃO do banner (nunca spinner eterno)
  await shell.clearFailDevice();
  await shell.alertRetry().click();
  await expect(shell.patchLabel()).toContainText("P02");
  await expect(shell.alerts).toHaveCount(0);
});

test("disconnect mid-boot: alerta amigável, LED off, barra morta e ⟳ recupera", async () => {
  await shell.failDevice("boot-mid");
  await shell.reload();

  await expect(shell.alerts.filter({ hasText: "Falha no boot do device" })).toBeVisible();
  // device que caiu no meio do boot não pode continuar com o LED de conectado
  await expect(shell.statusText("off")).toBeVisible();
  // a barra morre COM o boot (sem spinner eterno) e a casca segue de pé
  await expect(shell.bootProgress).toHaveCount(0);
  await expect(shell.library.listbox()).toBeVisible();

  // recuperação: ⟳ re-escaneia e o device volta ao estado REAL
  await shell.clearFailDevice();
  await shell.rescanButton.click();
  await expect(shell.statusText("on")).toBeVisible();
  await expect(shell.bootProgress).toHaveCount(0);
});
