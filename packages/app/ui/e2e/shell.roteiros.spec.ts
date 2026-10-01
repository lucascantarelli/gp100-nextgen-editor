/**
 * Roteiros de teste manual (docs/UI_TEST_PLAN.md) executados no Chromium.
 * Cada teste referencia o ID do roteiro (R1–R6 + drum/looper). O resultado
 * desta suíte é registrado na tabela de execução do próprio UI_TEST_PLAN.
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

/* ── R1. Topbar e identidade ── */
test("R1 topbar: logo, conexão, patch na navbar e foco de teclado", async ({ page }) => {
  const brand = shell.brand;
  await expect(brand.logo()).toBeVisible();
  await expect(brand.tagline()).toBeVisible();
  // status de conexão no cluster da navbar (texto; o cluster abriga boot/badge)
  // patch corrente no formato Pnn Nome (1-based, como o app oficial)
  await expect(shell.patchLabel()).toBeVisible();

  // tab alcança os controles globais com foco visível (outline definido no CSS)
  await page.keyboard.press("Tab");
  const focused = await page.evaluate(() => {
    const el = document.activeElement as HTMLElement | null;
    if (!el) return null;
    const s = getComputedStyle(el);
    return { tag: el.tagName, outlineStyle: s.outlineStyle, outlineWidth: s.outlineWidth };
  });
  expect(focused).not.toBeNull();

  // ◀ ▶ navegam presets em ciclo (coluna do patch do app oficial) —
  // navbar, display LED do board e biblioteca acompanhando juntos
  const listbox = shell.library.listbox();
  // O NOME precisa acompanhar o número nos 3 lugares (o fallback do board
  // já ignorou o pp e congelou o nome — cenário permanente no e2e)
  await shell.prevPatch();
  await expect(shell.patchLabel()).toHaveText("P99 Dreamy Aco");
  await expect(listbox.getByRole("option").last()).toHaveAttribute("aria-selected", "true");
  await expect(listbox.getByRole("option").last()).toHaveAccessibleName(/Dreamy Aco/);
  await expect(shell.board.display()).toContainText("Dreamy Aco");
  await shell.nextPatch();
  await expect(shell.patchLabel()).toHaveText("P01 It's GP100");
  await expect(listbox.getByRole("option").first()).toHaveAttribute("aria-selected", "true");
  await expect(shell.board.display()).toContainText("It's GP100");
});

/* ── R2. Biblioteca de fábrica (99) ── */
test("R2 biblioteca: 99 presets reais, busca, seleção e empty state", async ({ page }) => {
  const lib = shell.library;
  await expect(lib.listbox()).toBeVisible();
  await expect(lib.options()).toHaveCount(99);
  await expect(lib.optionAt(0)).toContainText("It's GP100");
  await expect(lib.options().last()).toContainText("Dreamy Aco");

  await lib.searchFor("mist");
  await expect(lib.options()).toHaveCount(1);
  await expect(lib.optionAt(0)).toHaveAccessibleName(/P25 Mist Rock/);

  // busca por estilo e vazio com dica
  await lib.searchFor("acoustic");
  const n = await lib.options().count();
  expect(n).toBeGreaterThan(10);
  await lib.searchFor("zzzz");
  await expect(lib.options()).toHaveCount(0);
  await expect(page.getByText(/Nenhum preset para/)).toBeVisible();

  // abrir preset (select real no shell; fallback dev troca o corrente)
  await lib.searchFor("mist");
  const mistOption = lib.optionAt(0);
  await mistOption.click();
  // seleção: accessible name é "P25 Mist Rock" (os spans não têm espaços no textContent)
  await expect(mistOption).toHaveAccessibleName(/P25 Mist Rock/);
  await expect(mistOption).toHaveAttribute("aria-selected", "true");
  // nome COMPLETO na navbar e no LED (não só o nº — o fallback já congelou)
  await expect(shell.patchLabel()).toHaveText("P25 Mist");
  await expect(shell.board.display()).toContainText("Mist");

  // User Patch bloqueada com explicação
  await expect(lib.userTab()).toBeDisabled();
});

/* ── R3. Board vazio (9 lugares) + trava ⇄ mover ── */
test("R3 board: 9 slots na ordem do sinal e trava do drag", async () => {
  const board = shell.board;
  const fams = ["PRE", "DST", "AMP", "NR", "CAB", "EQ", "MOD", "DLY", "RVB"];
  for (let i = 0; i < fams.length; i += 1) {
    await board.expectSlotVisible(i + 1, fams[i]);
  }
  await expect(board.root.getByText("vazio").first()).toBeVisible();

  // display LED: nº (2 dígitos) + nome + tipo
  await expect(board.display()).toContainText("01");

  await board.expectMoverPressed(false);
  const slot = board.slot(1, "PRE");
  await expect(slot).not.toHaveAttribute("draggable", "true");

  await board.toggleMover();
  await board.expectMoverPressed(true);
  await expect(slot).toHaveAttribute("draggable", "true");
  await expect(slot).toContainText("arraste ⇄");

  // kill/drum/master não soltam erro
  await shell.killSwitch().click();
  await expect(shell.alerts).toHaveCount(0);
});

/* ── R4. Modal Settings (⚙) ── */
test("R4 settings: 6 abas, persistência local, Global EQ e fechamento", async () => {
  const settings = await shell.openSettings();
  await settings.expectVisible();
  for (const t of ["General", "Global EQ", "About", "Info Frame", "Help", "Release Note"]) {
    await expect(settings.tab(t)).toBeVisible();
  }
  await expect(settings.dialog.getByText(/prévia local/)).toBeVisible();

  // General opera e PERSISTE após reload
  const input = settings.label("Input level");
  await input.fill("42");
  await shell.reload();
  const reopened = await shell.openSettings();
  await expect(reopened.label("Input level")).toHaveValue("42");

  // Footswitch Mode presente e desabilitado (aguardando captura)
  await expect(reopened.dialog.getByLabel(/Footswitch mode/)).toBeDisabled();

  // Global EQ: 5 bandas + controles desabilitados
  await reopened.openTab("Global EQ");
  for (const n of [1, 2, 3, 4, 5]) {
    await expect(reopened.role("button", `B${n}/5`)).toBeVisible();
  }
  await expect(reopened.label("Banda 1 freq")).toBeDisabled();
  await expect(reopened.label("L-CUT freq")).toBeDisabled();

  // Esc fecha; clique no fundo fecha
  await reopened.pressEscape();
  await reopened.expectClosed();
  await shell.openSettings();
  await shell.page.mouse.click(8, 8);
  await reopened.expectClosed();
});

/* ── R5. Boot e pushes ── */
test("R5 boot: roda sem erro, status segue conectado e pushes abre", async () => {
  await shell.rescanButton.click();
  // o boot sintético resolve rápido; a UI não pode soltar alerta de erro
  await expect(shell.alerts).toHaveCount(0);
  await expect(shell.banner).toContainText("on");

  const details = await shell.openPushes();
  await expect(details).toHaveJSProperty("open", true);
});

/* ── R6. Acessibilidade (gate manual) ── */
test("R6 a11y: alvos ≥32px, estado não só por cor, reduced-motion", async ({ page }) => {
  // alvos clicáveis ≥32px (altura) nos controles da casca
  const small = await page.evaluate(() => {
    const bad: string[] = [];
    document.querySelectorAll<HTMLElement>("button, select, input[type=range], input[type=number]").forEach((el) => {
      const r = el.getBoundingClientRect();
      if (r.height > 0 && r.height < 32) bad.push(`${el.tagName}:${el.getAttribute("aria-label") ?? el.textContent?.slice(0, 20)}`);
    });
    return bad;
  });
  expect(small).toEqual([]);

  // reduced-motion desliga as animações (live-dot e rolos): duração ≈ 0
  await page.emulateMedia({ reducedMotion: "reduce" });
  const animMs = await page.evaluate(() => {
    const d = getComputedStyle(document.querySelector(".live-dot")!).animationDuration;
    return parseFloat(d) * 1000; // converte s→ms ("1e-05s" = 0.01ms)
  });
  expect(animMs).toBeLessThan(5);

  // 800×600 sobrevive (scroll horizontal permitido, sem erro de layout)
  await page.setViewportSize({ width: 800, height: 600 });
  await expect(shell.banner).toBeVisible();
  await expect(shell.board.root).toBeVisible();
});

/* ── Drum: gestão de ritmos ── */
test("drum: painel com 87 ritmos do firmware, gênero→estilo e compassos reais", async () => {
  const drum = shell.drum;
  await expect(drum.chip).toContainText("Rock 1");
  await drum.open();

  await expect(drum.panel).toBeVisible();
  await expect(drum.panel.getByText(/87 ritmos/)).toBeVisible();

  await expect(drum.genreOptions()).toHaveCount(5);
  await expect(drum.styleOptions()).toHaveCount(33); // Rock
  await drum.selectGenre("World");
  await expect(drum.styleOptions()).toHaveCount(20);
  await drum.selectStyle("Samba");
  await drum.setBpm("140");

  // BPM (input number) ALINHADO com os selects: mesma caixa 240×32 border-box.
  // (O UA só aplica border-box a <select>; sem .gp-num{box-sizing} o BPM
  // media 258×46 — regressão travada por teste.)
  const boxes = await drum.measureBoxes();
  expect(boxes.bpm).not.toBeNull();
  expect(boxes.bpm!.boxSizing).toBe("border-box");
  expect(Math.abs(boxes.bpm!.w - boxes.genre!.w)).toBeLessThanOrEqual(1);
  expect(Math.abs(boxes.bpm!.h - boxes.genre!.h)).toBeLessThanOrEqual(1);

  await drum.close();

  // chip reflete o ritmo escolhido (nome acessível tem o resumo completo)
  await expect(drum.chipByName()).toHaveAccessibleName(/Samba, 140 BPM, compasso 4\/4/);
  await expect(drum.chipByName()).toContainText("Samba");
});

/* ── Looper: máquina de fita ── */
test("looper: transporte acessível, fita vazia, REC→PLAY e PRE/POST", async () => {
  const looper = shell.looper;
  await expect(looper.root).toBeVisible();
  await expect(looper.plate()).toBeVisible();

  const play = looper.playButton();
  await expect(play).toBeDisabled();
  await expect(looper.timer()).toHaveText("00:00");
  await expect(looper.tapeText(/vazia/)).toBeVisible();

  // REC grava; segundo toque vira PLAY (o label do botão muda com o modo)
  await looper.recButton().click();
  await expect(looper.status()).toContainText("REC");
  await looper.recButton().click();
  await expect(looper.status()).toContainText("PLAY");
  await expect(play).toBeEnabled();

  // timer anda enquanto roda
  await expect(looper.timer()).not.toHaveText("00:00", { timeout: 4000 });

  // REW volta ao início; STOP para
  await looper.rewButton().click();
  await expect(looper.timer()).toHaveText("00:00");
  await looper.stopButton().click();
  await expect(looper.status()).toContainText("STOP");

  // PRE/POST com os tempos reais do device (estado único no deck da fita)
  await looper.routeButton("POST").click();
  await expect(looper.tapeText(/fita: 45s \(POST\)/)).toBeVisible();
  await looper.routeButton("PRE").click();
  await expect(looper.tapeText(/fita: 90s \(PRE\)/)).toBeVisible();

  // CLEAR pede confirmação e zera
  await looper.clearButton().click();
  await looper.confirmClearButton().click();
  await expect(looper.tapeText(/vazia/)).toBeVisible();
  await expect(play).toBeDisabled();

  // parâmetros do firmware no rack
  for (const id of ["loop-rec", "loop-play", "loop-pvol"] as const) {
    await expect(looper.rackSlider(id)).toBeVisible();
  }
});
