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

/* ── Kill switch (topbar): mute global REVERSÍVEL, alcançável por teclado ── */
test("kill switch: mute global (master→0 + drum off) por teclado e volta", async ({ page }) => {
  const kill = shell.killSwitch();
  const master = shell.masterVolume();
  await expect(kill).toBeVisible();
  await expect(kill).toHaveAttribute("aria-pressed", "false");
  await expect(master).toHaveValue("99");

  // Enter (teclado) dispara o mute global: valor TROCADO e visível
  await kill.focus();
  await page.keyboard.press("Enter");
  await expect(kill).toHaveAttribute("aria-pressed", "true");
  await expect(master).toHaveValue("0");
  await expect(shell.drum.chip).not.toContainText("⏹"); // bateria parada
  await expect(shell.banner).toBeVisible();
  await expect(shell.library.listbox()).toBeVisible();

  // de novo restaura o master anterior (não é botão morto)
  await kill.click();
  await expect(kill).toHaveAttribute("aria-pressed", "false");
  await expect(master).toHaveValue("99");
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

/* ── afinador do palco (V-7): display sempre visível, monitor + demo ── */
test("Afinador: painel fixo, monitor visual, demo move agulha e nota, ref pitch", async ({ page }) => {
  const tuner = shell.tuner;
  await expect(tuner.group).toBeVisible();

  // repouso honesto: nota "—" (LED próprio presente)
  await expect(tuner.led()).toBeVisible();
  await expect(tuner.note()).toHaveText("—");

  // #8: o painel NÃO colapsa — modo e REF PITCH seguem na grade com o
  // monitor desligado (antes eles sumiam e abriam um buraco no painel)
  await expect(tuner.modeButton()).toBeVisible();
  await expect(tuner.group.getByLabel(/Pitch de referência/)).toBeVisible();
  await expect(tuner.powerLed(), "LED do botão vermelho com o monitor off").toHaveAttribute(
    "data-tuner-power-led",
    "off",
  );

  // monitor liga (persistência testada no fim)
  await tuner.powerButton().click();
  await expect(tuner.powerButton()).toHaveAttribute("aria-pressed", "true");
  await expect(tuner.powerLed(), "LED do botão verde com o monitor on").toHaveAttribute(
    "data-tuner-power-led",
    "on",
  );

  // demo alimenta o MOTOR REAL com senoide varrendo ±30 cents em A2:
  // nota aparece e a agulha desloca (esquerda flat → direita sharp)
  await tuner.demoButton().click();
  await expect(tuner.note()).toHaveText(/A\d/);
  const left1 = await tuner.group.locator("[data-tuner-needle]").evaluate(
    (el) => (el as HTMLElement).style.left,
  );
  await page.waitForTimeout(700);
  const left2 = await tuner.group.locator("[data-tuner-needle]").evaluate(
    (el) => (el as HTMLElement).style.left,
  );
  expect(left1).not.toBe(left2); // agulha se move de verdade

  // demo desliga sozinha: volta ao repouso (monitor segue ligado)
  await expect(tuner.note()).toHaveText("—", { timeout: 10_000 });

  // #8: o monitor MANDA na leitura — desligado não ouve nem deixa demo rodando
  await tuner.powerButton().click();
  await expect(tuner.powerButton()).toHaveAttribute("aria-pressed", "false");
  await expect(tuner.note()).toHaveText("—");

  // ▶ demo com o monitor desligado liga o monitor junto (demonstrar exige ouvir)
  await tuner.demoButton().click();
  await expect(tuner.powerButton()).toHaveAttribute("aria-pressed", "true");
  await expect(tuner.demoButton()).toHaveAttribute("aria-pressed", "true");
  await expect(tuner.demoButton()).toContainText("■");
  await expect(tuner.note()).toHaveText(/A\d/);

  // ref pitch padrão visível; persistência do monitor após reload
  await expect(tuner.refPitch()).resolves.toBe("440Hz");
  await shell.reload();
  const tuner2 = new ShellPage(page).tuner;
  await expect(tuner2.powerButton()).toHaveAttribute("aria-pressed", "true");
});
