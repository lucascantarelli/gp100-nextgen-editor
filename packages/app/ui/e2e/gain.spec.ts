/**
 * M3-3 — o assistente de gain staging: abrir, ler e fechar (issue #115).
 *
 * A DoD pede "e2e do fluxo abrir/ler/fechar". O roteiro passa pela PORTA do
 * conteúdo (o ∿ do rodapé da biblioteca) — o caminho do dono, não um atalho de
 * teste — e confere o que só a tela pode prometer: o relatório da cadeia, a
 * limitação impressa JUNTO do número, e a AUSÊNCIA de qualquer botão de escrita
 * (o assistente aponta; ajustar é do dono).
 *
 * Roda no navegador, fora do webview do Tauri, e por isso exercita o fallback
 * em memória da porta (`ipc/gain.ts`) — o mesmo caminho do `pnpm dev`. Contra o
 * core Rust, a prova da regra é a suíte `packages/core/tests/gain_staging.rs`.
 */
import { expect, test } from "@playwright/test";
import { ShellPage } from "./pages/_pages";

let shell: ShellPage;

test.beforeEach(async ({ page }) => {
  shell = new ShellPage(page);
  await shell.beforeStandard();
});

test("M3-3: o assistente abre pela porta, lê a cadeia e não oferece escrita", async () => {
  const gain = await shell.openGain();

  // o preset de fábrica do palco (P01) não é acusado
  await expect(gain.risk("BAIXO")).toBeVisible();

  // a cadeia inteira, 1-based como no palco: o 1º lugar e o último
  await expect(gain.modulo(/1º · PRE · C-Wah/)).toBeVisible();
  await expect(gain.modulo(/9º · RVB · Hall/)).toBeVisible();

  // a limitação está NA TELA (o modo de falha é o número parecer medição)
  await expect(gain.metodo()).toBeVisible();

  // e não há botão que escreve: só o ✕ de fechar
  await expect(gain.botoes()).toHaveCount(1);

  await gain.close().click();
  await expect(gain.root).toHaveCount(0);
});
