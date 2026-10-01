/**
 * Interação ponta-a-ponta da casca: todo controle visível tem cenário
 * (muda → UI reflete → persiste). Regra da casa: o assert é do
 * valor TROCADO, não só do inicial — e persistência é provada RECARREGANDO
 * a página (master/drum/looper/settings vivem em localStorage, prévia local).
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

/* ── Master VOL (topbar) ── */
test("master: slider muda o display numérico e persiste após reload", async ({ page }) => {
  const slider = shell.masterVolume();
  await expect(slider).toHaveValue("99"); // inicial
  await slider.fill("42");
  await expect(slider).toHaveValue("42");
  // o número EXIBIDO acompanha (não só o value do input)
  const display = await page.evaluate(
    () => document.querySelector('[aria-label="Master volume"]')?.nextElementSibling?.textContent ?? null,
  );
  expect(display).toBe("42");

  await shell.reload();
  await expect(shell.masterVolume()).toHaveValue("42");
});

/* ── Drum: sliders do painel de ritmos ── */
test("drum: volume e speed refletem e persistem no localStorage", async ({ page }) => {
  await shell.drum.open();

  const vol = shell.drum.volume();
  const speed = shell.drum.speed();
  await vol.fill("55");
  await speed.fill("30");
  await expect(vol).toHaveValue("55");
  await expect(speed).toHaveValue("30");

  const drum = await page.evaluate(() => JSON.parse(localStorage.getItem("gp100.drum.v2") ?? "{}"));
  expect(drum.volume).toBe(55);
  expect(drum.speed).toBe(30);

  await shell.reload();
  await shell.drum.open();
  await expect(shell.drum.volume()).toHaveValue("55");
  await expect(shell.drum.speed()).toHaveValue("30");
});

/* ── Looper: rack Rec VOL / Play VOL / P-VOL + rota PRE/POST ── */
test("looper: volumes refletem no display, persistem e a rota PRE/POST volta do reload", async ({ page }) => {
  const looper = shell.looper;
  const rec = looper.rackSliderByLabel("Rec VOL");
  const pvol = looper.rackSliderByLabel("P-VOL");
  await rec.fill("12");
  await pvol.fill("7");
  await expect(rec).toHaveValue("12");
  await expect(pvol).toHaveValue("7");
  // número exibido ao lado de cada slider (span irmão do input)
  await expect(looper.sliderDisplay("loop-rec")).toHaveText("12");
  await expect(looper.sliderDisplay("loop-pvol")).toHaveText("7");

  // rota: POST ativo (aria-pressed troca nos dois botões)
  const pre = looper.routeButton("PRE");
  const post = looper.routeButton("POST");
  await expect(pre).toHaveAttribute("aria-pressed", "true");
  await post.click();
  await expect(post).toHaveAttribute("aria-pressed", "true");
  await expect(pre).toHaveAttribute("aria-pressed", "false");

  const saved = await page.evaluate(() => JSON.parse(localStorage.getItem("gp100.looper.v1") ?? "{}"));
  expect(saved).toMatchObject({ recVol: 12, pVol: 7, pre: false });

  await shell.reload();
  await expect(looper.rackSliderByLabel("Rec VOL")).toHaveValue("12");
  await expect(looper.rackSliderByLabel("P-VOL")).toHaveValue("7");
  await expect(looper.rackSliderByLabel("Play VOL")).toHaveValue("80"); // intocado
  await expect(looper.routeButton("POST")).toHaveAttribute("aria-pressed", "true");
});

/* ── Kill switch (topbar) — alcançável por teclado, sem quebrar a casca ── */
test("kill switch: foco por teclado e Enter não tiram a casca do ar", async ({ page }) => {
  const kill = shell.killSwitch();
  await expect(kill).toBeVisible();
  await kill.focus();
  await page.keyboard.press("Enter");
  // Fase 1: o kill ainda não escreve no device (fios G3–G6) — o cenário
  // trava a ALCANÇABILIDADE: o botão segue no lugar e o shell responde.
  await expect(kill).toBeVisible();
  await expect(shell.banner).toBeVisible();
  await expect(shell.library.listbox()).toBeVisible();
});

/* ── Settings: campos remanescentes, um a um ── */
test("settings: input/normal level, USB Audio, Hint Mode e Tap Tempo — reflexo + persistência", async () => {
  const settings = await shell.openSettings();
  await settings.expectVisible();

  // Input Level: 100 → 42 (display troca junto)
  const inputLevel = settings.label("Input level");
  await inputLevel.fill("42");
  await expect(inputLevel).toHaveValue("42");
  await expect(settings.displayAfter("Input level")).toHaveText("42");

  // USB Audio: toggle off → on
  const usb = settings.role("switch", "USB Audio");
  await expect(usb).toHaveAttribute("aria-checked", "false");
  await usb.click();
  await expect(usb).toHaveAttribute("aria-checked", "true");

  // Normal Level: 100 → 66
  const normal = settings.label("Normal level");
  await normal.fill("66");
  await expect(normal).toHaveValue("66");
  await expect(settings.displayAfter("Normal level")).toHaveText("66");

  // Hint Mode: select left → right
  const hint = settings.label("Hint mode");
  await hint.selectOption("right");
  await expect(hint).toHaveValue("right");

  // Tap Tempo: PRE ligado por default; liga MOD e DLY
  const mod = settings.role("checkbox", "MOD");
  const dly = settings.role("checkbox", "DLY");
  await expect(settings.role("checkbox", "PRE")).toBeChecked();
  await mod.check();
  await dly.check();
  await expect(mod).toBeChecked();
  await expect(dly).toBeChecked();

  // persistência integral + recarga restaura TUDO
  await settings.pressEscape();
  await settings.expectHidden();
  await settings.expectPersisted({
    inputLevel: 42,
    normalLevel: 66,
    usbAudio: true,
    hintMode: "right",
    tapTempo: { pre: true, mod: true, dly: true },
  });

  await shell.reload();
  const again = await shell.openSettings();
  await expect(again.label("Input level")).toHaveValue("42");
  await expect(again.label("Normal level")).toHaveValue("66");
  await expect(again.role("switch", "USB Audio")).toHaveAttribute("aria-checked", "true");
  await expect(again.label("Hint mode")).toHaveValue("right");
  await expect(again.role("checkbox", "MOD")).toBeChecked();
  await expect(again.role("checkbox", "DLY")).toBeChecked();
});

/* ── VU do palco (V-7): modos com animações DIFERENTES de verdade ── */
test("VU do palco: LED acende segmentos, EQ anima barras, drum ativa", async ({ page }) => {
  const vu = shell.vu;
  await expect(vu.group).toBeVisible();

  // modo LED é o default: 12 colunas × 6 segmentos apagados (sem som)
  const led = vu.modeButton("led");
  const eq = vu.modeButton("eq");
  await expect(led).toHaveAttribute("aria-pressed", "true");
  await expect(vu.ledSegments(false)).toHaveCount(72);
  await expect(vu.ledSegments(true)).toHaveCount(0);

  // liga o drum (Espaço) → há "som": segmentos ACENDEM (LED ladder)
  await page.keyboard.press("Space");
  await page.waitForTimeout(600);
  expect(await vu.ledSegments(true).count()).toBeGreaterThan(0);

  // troca para modo equalizador (valor TROCADO, não só o inicial): o DOM
  // muda de escada de segmentos p/ barras contínuas e a animação muda
  await eq.click();
  await expect(eq).toHaveAttribute("aria-pressed", "true");
  await expect(led).toHaveAttribute("aria-pressed", "false");
  await expect(vu.eqBars()).toHaveCount(12);
  const h1 = await vu.barHeights();
  await page.waitForTimeout(400);
  const h2 = await vu.barHeights();
  expect(h1.some((h, i) => Math.abs(h - h2[i]) > 1)).toBe(true); // EQ anima

  // persistência do modo
  await shell.reload();
  const vu2 = new ShellPage(page).vu;
  await expect(vu2.modeButton("eq")).toHaveAttribute("aria-pressed", "true");
});
