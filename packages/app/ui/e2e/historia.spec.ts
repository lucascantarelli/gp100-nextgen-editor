/**
 * M3-1 — Biblioteca versionada: o fluxo completo (issue #113).
 *
 * Este é o teste que a DoD pede: **editar, salvar 3x, diff, restaurar**. Não
 * são três testes de tela soltos; é um ROTEIRO, porque o que a issue promete é
 * uma história que cresce e não pode ser reescrita — e isso só aparece quando
 * os passos acontecem na ordem em que o dono faria.
 *
 * **O roteiro roda no navegador**, fora do webview do Tauri, e é por isso que
 * ele exercita o fallback em memória da porta. É o mesmo caminho que o dono
 * vê no `pnpm dev`; contra o SQLite a prova é a suíte Rust (`versions.rs`).
 *
 * **O passo que parece burocracia é o que prova a feature.** Salvar três
 * vezes o MESMO patch só vira três versões porque salvar um patch já aberto
 * atualiza o id dele. Antes dessa regra, cada `save` criava um patch novo
 * (`snapshotOf` gera `u<instante><n>`) e o histórico de cada um tinha uma
 * versão só — o que teria feito o e2e passar sem nunca exercitar o histórico.
 */
import { expect, test } from "@playwright/test";
import { ShellPage } from "./pages/_pages";

let shell: ShellPage;

test.beforeEach(async ({ page }) => {
  shell = new ShellPage(page);
  await shell.beforeStandard();
});

/** Abre o modal do PRE e mexe o primeiro knob `passos` vezes (teclado). */
async function giraKnob(page: import("@playwright/test").Page, passos: number): Promise<void> {
  const board = shell.board;
  const pedalSlot = board.slot(1, "PRE");
  const pedal = pedalSlot.locator('svg[role="group"]');
  // o PRE do P01 no all.prst é C-Wah: o roteiro assume os 3 knobs reais
  await expect(pedal).toHaveAttribute("aria-label", /C-Wah/);

  // A trava de mover precisa estar NO REPOUSO — e ela JÁ nasce no repouso.
  // Chamar `toggleMover()` aqui ligaria o arrasto, e aí o clique no pedal
  // pertence ao drag e o modal nunca abre (é por isso que o R7 alterna duas
  // vezes: liga para provar o drag e desliga para provar a edição).
  await board.expectMoverPressed(false);
  await pedal.locator('svg[role="img"]').first().click();
  const dlg = page.getByRole("dialog", { name: /Edição do pedal/ });
  await expect(dlg).toBeVisible();
  await expect(dlg.locator('svg[role="slider"]')).toHaveCount(3);
  for (let i = 0; i < passos; i++) await dlg.locator('svg[role="slider"]').first().press("ArrowUp");
  await page.keyboard.press("Escape");
  await expect(dlg).toHaveCount(0);
}

test("M3-1: editar, salvar 3x, ver o diff e restaurar cria uma versão nova", async ({ page }) => {
  const lib = shell.library;
  const hist = lib.history();

  /* 1. salvar a cadeia corrente como patch de usuário (versão 1) */
  await lib.saveAs("Meu clean");
  await expect(lib.userRows()).toHaveCount(1);

  /* 2. ABRIR o patch: só agora "salvar" passa a versionar em vez de criar
        outro patch — e sem abrir, o roteiro não provaria nada */
  await lib.userRowAt(0).getByRole("button").first().click();
  await expect(shell.board.display()).toContainText("U01");

  /* 3. mexer e salvar (versão 2) */
  await giraKnob(page, 1);
  await lib.saveAs("Meu clean");
  await expect(lib.userRows()).toHaveCount(1); // continua UM patch

  /* 4. mexer mais e salvar (versão 3) */
  await giraKnob(page, 2);
  await lib.saveAs("Meu clean");
  await expect(lib.userRows()).toHaveCount(1);

  /* 5. o histórico mostra as três versões, e o par padrão é a última contra a
        penúltima */
  await lib.historyButton("Meu clean").click();
  await expect(hist.root).toBeVisible();
  // "escolha uma versão" + v3, v2, v1 (da mais nova para a mais antiga)
  await expect(hist.versionOptions("after")).toHaveCount(4);
  await expect(hist.versionOptions("after").nth(1)).toContainText("v3");

  /* 6. o diff diz o que mudou, no nível do knob */
  const knob = page.getByRole("button", { name: /Restaurar Range de/ });
  await expect(knob).toHaveCount(1);
  await expect(hist.root).toContainText("51 → 53");

  /* 7. restaurar o knob cria a versão 4 — e as três anteriores continuam */
  await knob.click();
  await expect(hist.status()).toContainText("4");
  await expect(hist.versionOptions("after")).toHaveCount(5);
  await expect(hist.versionOptions("after").nth(4)).toContainText("v1");

  /* 8. o palco recebeu a cadeia restaurada (o board reabriu o patch) */
  await hist.close().click();
  await expect(hist.root).toHaveCount(0);
});

test("M3-1: restaurar a versão inteira também vira uma versão nova", async ({ page }) => {
  const lib = shell.library;
  const hist = lib.history();

  await lib.saveAs("Take 1");
  await lib.userRowAt(0).getByRole("button").first().click();
  await giraKnob(page, 1);
  await lib.saveAs("Take 1");

  await lib.historyButton("Take 1").click();
  await expect(hist.root).toBeVisible();
  await expect(hist.versionOptions("after")).toHaveCount(3); // placeholder + v2 + v1

  await hist.restoreAll(1).click();
  await expect(hist.status()).toContainText("3");
  await expect(hist.versionOptions("after")).toHaveCount(4);
  // e a versão 1 continua listada: restaurar NÃO reescreve a história
  await expect(hist.versionOptions("after").nth(3)).toContainText("v1");
});
