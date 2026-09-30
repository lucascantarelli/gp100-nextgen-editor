/**
 * Testes de DESIGN (gate de regressão de tokens): a escala de Fibonacci é a
 * que docs/UI_DESIGN.md declara; TODOS os pares de texto atingem AA 4.5:1;
 * pisos WCAG de foco/alvo presentes. Quebrou aqui = quebrou o design system.
 */
import { describe, expect, it } from "vitest";
import {
  CONTRAST_RATIOS,
  SPACE,
  TEXT_PAIRS,
  WCAG,
} from "../src/design/tokens";
import { readFileSync } from "node:fs";
import { join } from "node:path";

/** Caminho p/ arquivos do front (vitest roda com cwd = ui/). */
const uiFile = (...p: string[]) => join(process.cwd(), ...p);

describe("escala de espaçamento (Fibonacci base 4)", () => {
  it("tem exatamente os 7 degraus F4–F11 arredondados a múltiplos de 4", () => {
    expect(Object.keys(SPACE).map(Number).sort((a, b) => a - b)).toEqual([
      4, 8, 12, 20, 32, 52, 84,
    ]);
    for (const px of Object.values(SPACE)) {
      expect(px % 4).toBe(0);
    }
  });

  it("cresce ~φ entre degraus vizinhos (razão entre 1.2 e 2.2)", () => {
    const steps = Object.values(SPACE);
    for (let i = 1; i < steps.length; i++) {
      const ratio = steps[i] / steps[i - 1];
      expect(ratio).toBeGreaterThan(1.2);
      expect(ratio).toBeLessThan(2.2);
    }
  });

  it("design.css usa os mesmos valores (tokens não divergem do CSS)", () => {
    const css = readFileSync(uiFile("src", "design", "design.css"), "utf8");
    for (const [k, px] of Object.entries(SPACE)) {
      expect(css).toContain(`--space-${k}: ${px}px`);
    }
  });
});

describe("contraste (WCAG 2.2 AA) — docs/UI_DESIGN.md §2", () => {
  it("todo par de texto atinge 4.5:1 nos dois temas", () => {
    for (const theme of ["dark", "light"] as const) {
      for (const [pair, ratio] of Object.entries(TEXT_PAIRS[theme])) {
        expect(
          ratio,
          `par ${pair} no tema ${theme} abaixo de AA`,
        ).toBeGreaterThanOrEqual(WCAG.textAA);
      }
    }
  });

  it("os rácios medidos batem com a tabela de CONTRAST_RATIOS", () => {
    expect(TEXT_PAIRS.dark["text/bg"]).toBe(CONTRAST_RATIOS.textDark);
    expect(TEXT_PAIRS.light["text/bg"]).toBe(CONTRAST_RATIOS.textLight);
    expect(TEXT_PAIRS.dark["error/bg"]).toBe(CONTRAST_RATIOS.errorDark);
  });

  it("pisos de foco/alvo presentes (2.4.13 + alvo 32px)", () => {
    expect(WCAG.focusWidthPx).toBeGreaterThanOrEqual(2);
    expect(WCAG.focusContrast).toBeGreaterThanOrEqual(3);
    expect(WCAG.minTargetPx).toBeGreaterThanOrEqual(32);
  });
});

describe("design.css — regras globais de a11y", () => {
  const css = readFileSync(uiFile("src", "design", "design.css"), "utf8");

  it("declara foco visível global (outline 2px + :focus-visible)", () => {
    expect(css).toMatch(/:focus-visible\s*\{/);
    expect(css).toMatch(/outline:\s*2px solid var\(--focus-ring\)/);
  });

  it("respeita prefers-reduced-motion", () => {
    expect(css).toContain("prefers-reduced-motion");
  });

  it("declara color-scheme (scrollbars/form controls nativos corretos)", () => {
    expect(css).toContain("color-scheme: dark light");
  });
});
