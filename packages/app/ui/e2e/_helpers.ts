/**
 * Helpers compartilhados dos testes e2e (medidas de layout/alinhamento).
 * Tudo roda no browser (page.evaluate) — os asserts ficam nos specs.
 * Origem: analysis/responsivo_checks.js.
 */
import type { Page } from "@playwright/test";

export interface ShellMeasure {
  overflowX: number;
  slotCols: number;
  slotRows: number;
  cssCols: number;
  slotOverlap: number;
  board: { x: number; w: number; right: number; bottom: number; y: number };
  lib: { x: number; w: number; y: number; bottom: number; sameRowAsBoard: boolean };
  looper: { x: number; w: number; tracks: number; trackWs: number[] };
  kidOverlap: number;
  rackW: number | null;
  vuInside: boolean;
  clippedCount: number;
  clippedSample: string[];
}

/** mede o shell inteiro: board × biblioteca, slots e looper */
export async function measureShell(page: Page): Promise<ShellMeasure> {
  return page.evaluate(() => {
    const q = (sel: string) => document.querySelector(sel);
    const rectOf = (el: Element) => {
      const r = el.getBoundingClientRect();
      return { x: r.x, y: r.y, right: r.right, bottom: r.bottom, w: r.width, h: r.height };
    };
    const r1 = (n: number) => Math.round(n * 10) / 10;

    const boardEl = q('[aria-label="Pedalboard (9 lugares da cadeia)"]')!;
    const libEl = q('[aria-label="Biblioteca de presets"]')!;
    const looperEl = q('[aria-label="Looper (máquina de fita)"]')!;
    const board = rectOf(boardEl);
    const lib = rectOf(libEl);
    const looper = rectOf(looperEl);

    // slots: colunas/linhas contadas POR LINHA (a cadeia é uma fileira só;
    // a contagem por linha segue como rede de segurança) + sobreposição
    const slotRects = [...document.querySelectorAll('[aria-label^="Slot "]')].map(rectOf);
    const overlapArea = (a: ReturnType<typeof rectOf>, b: ReturnType<typeof rectOf>) =>
      Math.max(0, Math.min(a.right, b.right) - Math.max(a.x, b.x)) *
      Math.max(0, Math.min(a.bottom, b.bottom) - Math.max(a.y, b.y));
    const byRow = new Map<number, number[]>();
    for (const r of slotRects) {
      const key = Math.round(r.y);
      byRow.set(key, [...(byRow.get(key) ?? []), r.x]);
    }
    const rowXs = [...byRow.values()].map((xs) => {
      const s = [...xs].sort((a, b) => a - b);
      const out = [s[0]];
      for (const v of s.slice(1)) if (v - out[out.length - 1] > 2) out.push(v);
      return out.length;
    });
    let slotOverlap = 0;
    for (let i = 0; i < slotRects.length; i++)
      for (let j = i + 1; j < slotRects.length; j++)
        slotOverlap += overlapArea(slotRects[i], slotRects[j]);

    // looper: filhos diretos (deck | VU/transporte | rack) sem sobreposição
    const kids = [...looperEl.children].map(rectOf);
    let kidOverlap = 0;
    for (let i = 0; i < kids.length; i++)
      for (let j = i + 1; j < kids.length; j++) kidOverlap += overlapArea(kids[i], kids[j]);
    const rack = kids.length ? kids[kids.length - 1] : null;

    const vus = [...looperEl.querySelectorAll('svg[aria-label^="VU meter"]')].map(rectOf);
    const vuInside = vus.every((v) => v.x >= looper.x - 0.5 && v.right <= looper.right + 0.5);

    // texto clipado (aviso): folhas com texto onde scrollWidth > clientWidth
    const clipped: string[] = [];
    for (const root of [boardEl, libEl, looperEl]) {
      for (const el of root.querySelectorAll("*")) {
        if (el.children.length > 0) continue;
        const txt = (el.textContent || "").trim();
        if (!txt) continue;
        if (el.scrollWidth > el.clientWidth + 1) clipped.push(txt.slice(0, 24));
      }
    }

    const looperCols = getComputedStyle(looperEl)
      .gridTemplateColumns.split(" ")
      .map((v) => parseFloat(v));
    return {
      overflowX: document.documentElement.scrollWidth - window.innerWidth,
      slotCols: rowXs.length ? Math.max(...rowXs) : 0,
      slotRows: byRow.size,
      cssCols: getComputedStyle(q(".board-slots")!).gridTemplateColumns.split(" ").length,
      slotOverlap: r1(slotOverlap),
      board: { x: r1(board.x), w: r1(board.w), right: r1(board.right), bottom: r1(board.bottom), y: r1(board.y) },
      lib: { x: r1(lib.x), w: r1(lib.w), y: r1(lib.y), bottom: r1(lib.bottom), sameRowAsBoard: Math.abs(lib.y - board.y) < 2 },
      looper: { x: r1(looper.x), w: r1(looper.w), tracks: looperCols.length, trackWs: looperCols.map(r1) },
      kidOverlap: r1(kidOverlap),
      rackW: rack ? r1(rack.w) : null,
      vuInside,
      clippedCount: clipped.length,
      clippedSample: clipped.slice(0, 5),
    };
  });
}

/** asserts do alinhamento do shell para uma largura (usado pelos 3 viewports) */
export async function expectShellAligned(page: Page, width: number): Promise<void> {
  const { expect } = await import("@playwright/test");
  const m = await measureShell(page);

  expect(m.overflowX, `overflow horizontal @${width}`).toBe(0);

  // NAVBAR EM 1 LINHA (≥1024px): quebrar em duas fileiras foi defeito
  // reportado — logo curta, status on/off, drum empilhado e Boot compacto
  // mantêm o banner com ~50px (2 linhas passariam de 80px).
  if (width >= 1024) {
    const bannerH = await page
      .getByRole("banner")
      .evaluate((el) => el.getBoundingClientRect().height);
    expect(bannerH, `navbar em 1 linha @${width} (altura ${bannerH})`).toBeLessThan(70);
  }

  const stacked = width <= 1100; // breakpoint .shell-main (design.css)
  if (stacked) {
    expect(m.lib.sameRowAsBoard, `biblioteca empilhada @${width}`).toBe(false);
    expect(m.lib.y, `biblioteca abaixo do board @${width}`).toBeGreaterThanOrEqual(m.board.bottom - 1);
    expect(m.lib.x).toBe(m.board.x);
    expect(m.lib.w).toBe(m.board.w);
  } else {
    // hierarquia nova: biblioteca (300px) à ESQUERDA, pedalboard esticando à
    // direita — MESMA altura (align stretch), sem lacuna entre eles
    expect(m.lib.sameRowAsBoard, `lado a lado @${width}`).toBe(true);
    expect(m.lib.w, `biblioteca com 300px @${width}`).toBeLessThanOrEqual(300.5);
    expect(m.lib.x + m.lib.w, `biblioteca à esquerda do board @${width}`).toBeLessThanOrEqual(m.board.x + 0.5);
    expect(Math.abs(m.lib.y - m.board.y), `topos alinhados @${width}`).toBeLessThanOrEqual(1);
  }

  // .board-slots (design.css): cadeia INTEIRA numa única fileira — 9 colunas
  // fluidas em qualquer largura (o pedal do palco desenha na largura da
  // coluna; abaixo do piso de 88px por coluna a faixa rola na horizontal)
  expect(m.slotCols, `colunas dos slots @${width}`).toBe(9);
  expect(m.cssCols, `colunas do CSS @${width}`).toBe(9);
  expect(m.slotRows, `fileira única @${width}`).toBe(1);
  expect(m.slotOverlap, `slots sem sobreposição @${width}`).toBe(0);

  expect(m.kidOverlap, `sobreposição dos blocos do looper @${width}`).toBe(0);
  expect(m.rackW ?? 0, `rack do looper ≥ 240 @${width}`).toBeGreaterThanOrEqual(239.5);
  expect(m.vuInside, `VU meters contidos @${width}`).toBe(true);
  expect(m.clippedCount, `texto clipado @${width}: ${m.clippedSample.join(" | ")}`).toBe(0);
}
