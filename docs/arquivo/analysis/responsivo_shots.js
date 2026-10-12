// responsivo_shots.js — screenshots do shell no Google Chrome do sistema
// (channel: "chrome") em 1440/1280/1024 + medidas de alinhamento.
// Uso: node analysis/responsivo_shots.js  (precisa do pnpm dev na :5173)
const { createRequire } = require("module");
// resolve @playwright/test do pacote da UI (o script vive em analysis/)
const req = createRequire(require("path").resolve(process.cwd(), "packages/app/ui/package.json"));
const { chromium } = req("@playwright/test");

const SIZES = [
  [1440, 900],
  [1280, 800],
  [1024, 768],
];

(async () => {
  const browser = await chromium.launch({ channel: "chrome", headless: true });
  for (const [w, h] of SIZES) {
    const page = await browser.newPage({ viewport: { width: w, height: h } });
    await page.goto("http://localhost:5173/", { waitUntil: "networkidle" });
    await page.waitForSelector('[aria-label="Looper (máquina de fita)"]');

    const m = await page.evaluate(() => {
      const q = (sel) => document.querySelector(sel);
      const rect = (el) => {
        const r = el.getBoundingClientRect();
        return { x: +r.x.toFixed(1), y: +r.y.toFixed(1), w: +r.width.toFixed(1), h: +r.height.toFixed(1) };
      };
      const slots = [...document.querySelectorAll('[aria-label^="Slot "]')].map((el) => rect(el).w);
      const board = rect(q('[aria-label="Pedalboard (9 lugares da cadeia)"]'));
      const looper = rect(q('[aria-label="Looper (máquina de fita)"]'));
      const lib = rect(q('[aria-label="Biblioteca de presets"]'));
      const looperCols = getComputedStyle(q('[aria-label="Looper (máquina de fita)"]')).gridTemplateColumns.split(" ").length;
      return {
        overflowX: document.documentElement.scrollWidth - window.innerWidth,
        slotWidths: slots,
        boardCols: getComputedStyle(q(".board-slots")).gridTemplateColumns.split(" ").length,
        looperCols,
        sameSlotWidth: Math.max(...slots) - Math.min(...slots) < 1,
        board: { x: board.x, w: board.w },
        lib: { x: lib.x, w: lib.w, sameRowAsBoard: Math.abs(lib.y - board.y) < 2 },
        looper: { w: looper.w },
      };
    });

    const file = `files/images/responsivo-${w}.png`;
    await page.screenshot({ path: file, fullPage: true });
    console.log(`--- ${w}x${h} → ${file}`);
    console.log(JSON.stringify(m));
    await page.close();
  }
  await browser.close();
})();
