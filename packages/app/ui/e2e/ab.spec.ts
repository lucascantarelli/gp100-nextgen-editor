/**
 * M3-4 — A/B com blind test entre versões (issue #116).
 *
 * A DoD pede "blind honesto com teste de UI". O roteiro passa pela PORTA do
 * conteúdo (o ∿ do rodape da biblioteca) — o caminho do dono — e confere o
 * que só a tela pode prometer:
 *
 *  1. o par A/B nasce do histórico versionado (#113): salvar duas vezes o
 *     MESMO patch é o que cria os dois lados (mesma gramática do M3-1);
 *  2. com o blind ARMADO, nada vaza antes da resposta — nem o marcador de
 *     lado, nem as versões, nem o nível, nem o relatório da troca feita no
 *     escuro (o relatório é o canto esquecido: ele não nomeia o lado, mas
 *     conta que a troca aconteceu, e a tela promete silêncio);
 *  3. o palpite é o que revela — a resposta, os lados e o relatório volte
 *     juntos.
 *
 * Roda no navegador, fora do webview do Tauri, e por isso exercita o fallback
 * em memória da porta — o mesmo caminho do `pnpm dev`.
 */
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";
import { ShellPage } from "./pages/_pages";

let shell: ShellPage;

test.beforeEach(async ({ page }) => {
  shell = new ShellPage(page);
  await shell.beforeStandard();
});

/** Abre o modal do PRE e mexe o primeiro knob `passos` vezes (teclado).
 *  Mesmo roteiro do M3-1: a edição é o que faz a v2 da v1 DIFERIR. */
async function giraKnob(page: Page, passos: number): Promise<void> {
  const board = shell.board;
  const pedalSlot = board.slot(1, "PRE");
  const pedal = pedalSlot.locator('svg[role="group"]');
  await expect(pedal).toHaveAttribute("aria-label", /C-Wah/);
  await board.expectMoverPressed(false);
  await pedal.locator('svg[role="img"]').first().click();
  const dlg = page.getByRole("dialog", { name: /Edição do pedal/ });
  await expect(dlg).toBeVisible();
  await expect(dlg.locator('svg[role="slider"]')).toHaveCount(3);
  for (let i = 0; i < passos; i++) await dlg.locator('svg[role="slider"]').first().press("ArrowUp");
  await page.keyboard.press("Escape");
  await expect(dlg).toHaveCount(0);
}

/** Salva duas vezes o MESMO patch (v1 e v2) e o deixa ABERTO no palco. */
async function patchComDuasVersoes(page: Page, nome: string): Promise<void> {
  const lib = shell.library;
  await lib.saveAs(nome);
  await expect(lib.userRows()).toHaveCount(1);
  await lib.userRowAt(0).getByRole("button").first().click();
  await expect(shell.board.display()).toContainText("U01");
  await giraKnob(page, 1);
  await lib.saveAs(nome);
  await expect(lib.userRows()).toHaveCount(1); // continua UM patch, agora com 2 versões
}

test("M3-4: o A/B abre pela porta com os dois lados, nível e método na tela", async ({ page }) => {
  await patchComDuasVersoes(page, "Meu A/B");

  const ab = await shell.openAb();

  // o par padrão: A = corrente (v2), B = anterior (v1)
  // `exact`: "Lado A" como substring casaria com o botão "Ouvir o lado A"
  await expect(ab.root.getByText("Lado A", { exact: true })).toBeVisible();
  await expect(ab.root.getByText("Lado B", { exact: true })).toBeVisible();
  await expect(ab.root.getByText(/v2 ·/)).toBeVisible();
  await expect(ab.root.getByText(/v1 ·/)).toBeVisible();
  await expect(ab.soando()).toBeVisible();

  // o nível é POSIÇÃO e a limitação vem junto do número (mesma regra da #115)
  await expect(ab.metodo()).toBeVisible();

  // sem blind, os dois lados são ouvidos por botões que NOMEIAM o destino
  await expect(ab.ouvir("A")).toBeVisible();
  await expect(ab.ouvir("B")).toBeVisible();

  await ab.close().click();
  await expect(ab.root).toHaveCount(0);
});

test("M3-4: com o blind armado, NADA vaza antes do palpite — e a resposta revela", async ({ page }) => {
  await patchComDuasVersoes(page, "Meu A/B");
  const ab = await shell.openAb();

  /* o relatório ainda não existe (nada foi trocado), mas o resto está à mostra */
  await expect(ab.soando()).toBeVisible();
  await expect(ab.metodo()).toBeVisible();

  /* arma o blind: a tela vira pergunta + palpites */
  await ab.blindToggle().click();
  await expect(ab.pergunta()).toBeVisible();

  // SOME: marcador de lado, versões e método/nível
  await expect(ab.soando()).toHaveCount(0);
  await expect(ab.root.getByText(/v\d ·/)).toHaveCount(0);
  await expect(ab.metodo()).toHaveCount(0);
  // SOME: os botões que dizem para onde cada clique vai
  await expect(ab.ouvir("A")).toHaveCount(0);
  await expect(ab.ouvir("B")).toHaveCount(0);
  // FICA: só o botão que não nomeia destino
  await expect(ab.trocar()).toBeVisible();

  /* troca NO ESCURRO: a troca acontece (é o ouvido quem julga)… */
  await ab.trocar().click();
  await expect(ab.pergunta()).toBeVisible();
  // …mas nem o relatório nem o lado ativo vazam
  await expect(ab.relato()).toHaveCount(0);
  await expect(ab.soando()).toHaveCount(0);
  await expect(ab.root.getByText(/v\d ·/)).toHaveCount(0);

  /* o palpite é o que revela: resposta, lado, versões e relatório juntos */
  await ab.palpite("A").click();
  await expect(ab.resposta()).toBeVisible();
  await expect(ab.soando()).toBeVisible();
  await expect(ab.root.getByText(/v\d ·/)).toHaveCount(2); // os DOIS lados de volta
  await expect(ab.metodo()).toBeVisible();
  await expect(ab.relato()).toBeVisible();
  await expect(ab.ouvir("A")).toBeVisible();

  /* "ouvir de novo" rearma a rodada (a próxima escuta começa limpa) */
  await ab.root.getByRole("button", { name: "Ouvir de novo" }).click();
  await expect(ab.resposta()).toHaveCount(0);
  await expect(ab.soando()).toHaveCount(0);
});
