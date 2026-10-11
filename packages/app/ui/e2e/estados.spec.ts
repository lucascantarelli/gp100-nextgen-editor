/**
 * Error/empty states com cenário: device_info, boot e openPreset
 * falhando. O ganho de teste é `localStorage["gp100.debug.failDevice"]`
 * (= "info" | "boot" | "board" | "all"), lido pelo fallback do ipc/device
 * ANTES do app renderizar — por isso o addInitScript roda no load.
 *
 * Contrato de UI (rodada de error states):
 *  - device_info falha  → app segue de pé, sem conexão (sem crash);
 *  - device_boot falha  → o GATE de boot (#161): só navbar +
 *    `role="alert"` amigável (MSG.connBootError) — a casca NÃO monta;
 *    o detalhe técnico nunca vaza ao usuário e a ação "Refazer o boot"
 *    recupera até a casca montar (ciclo falha → OK = cenário e2e de
 *    boot do DoD);
 *  - device_board falha → banner role="alert" amigável (MSG.errOpenPreset).
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

/* ── device_info falha: casca viva, sem device ── */
test("device_info falha: casca segue de pé e mostra estado sem conexão", async () => {
  await shell.failDevice("info");
  await shell.reload();

  // sem device_info não há conexão: status curto "off" no cluster da navbar
  await expect(shell.banner).toContainText("off");
  // biblioteca (artefato local) segue navegável — o erro é do device, não da UI
  await expect(shell.library.listbox()).toBeVisible();
});

/* ── boot falha: role="alert" amigável no banner (MSG.connBootError) ── */
test("device_boot falha: banner mostra alerta amigável (MSG.connBootError)", async () => {
  await shell.failDevice("boot");
  await shell.reload();

  await shell.rescanButton.click();
  const alert = shell.alerts.filter({ hasText: "Falha no boot do device" });
  await expect(alert).toBeVisible();
  // mensagem AMIGÁVEL (sem stack/mensagem técnica do erro simulado)
  await expect(alert).not.toContainText("debug:");
});

/* ── openPreset falha: banner amigável (MSG.errOpenPreset) ── */
test("device_board falha: banner de erro amigável ao abrir preset", async () => {
  await shell.failDevice("board");
  await shell.reload();

  // abrir qualquer preset da biblioteca aciona deviceBoard → falha simulada
  await shell.library.optionAt(0).click();
  const alert = shell.alerts.filter({ hasText: "Não foi possível abrir o preset" });
  await expect(alert).toBeVisible();
  await expect(alert).not.toContainText("debug:");
  // a casca segue de pé: biblioteca ainda navegável
  await expect(shell.library.listbox()).toBeVisible();
});

/* ── "all": com o boot falho o GATE não monta a casca (#161) ── */
test("failDevice=all: gate de boot — sem casca, erro amigável; refazer o boot recupera", async () => {
  await shell.failDevice("all");
  await shell.reload();

  await expect(shell.banner).toContainText("off");
  // boot falhou → só navbar + alert (a casca NÃO existe no DOM)
  const alert = shell.alerts.filter({ hasText: "Falha no boot do device" });
  await expect(alert).toBeVisible();
  await expect(alert).not.toContainText("debug:");
  await expect(shell.library.listbox()).toHaveCount(0);

  // recuperação: limpa o gancho (boot volta a funcionar), armar só o
  // select e usar a AÇÃO do gate — validado, a casca monta; aí o select
  // falha com o banner amigável, sem crash.
  await shell.clearFailDevice();
  await shell.armFailDevice("select");
  await alert.getByRole("button", { name: "Refazer o boot" }).click();
  await expect(shell.library.listbox()).toBeVisible(); // casca montou
  await shell.library.optionAt(0).click();
  await expect(shell.alerts.filter({ hasText: "não aceitou a troca de preset" })).toBeVisible();
  await expect(shell.alertRetry()).toBeVisible(); // ação de recuperação
  await expect(shell.banner).toBeVisible();
});
