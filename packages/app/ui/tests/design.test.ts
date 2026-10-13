/**
 * Testes de DESIGN (gate de regressão de tokens): a escala de Fibonacci é a
 * declarada no documento de design; TODOS os pares de texto atingem AA 4.5:1;
 * pisos WCAG de foco/alvo presentes. Quebrou aqui = quebrou o design system.
 */
import { describe, expect, it } from "vitest";
import {
  CONTRAST_RATIOS,
  ELEVATION,
  FONT_STACKS,
  LEADING,
  LIGHT,
  LIGHT_VAR,
  MOTION,
  RADIUS,
  SPACE,
  TEXT_PAIRS,
  TRACKING,
  TYPE,
  WCAG,
  WEIGHT,
} from "../src/design/tokens";
import { DRUM_GENRES, DRUM_TOTAL_STYLES } from "../src/artifacts/drumData";
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

/*
 * Luz/profundidade (#9): tokens.ts declara OS VALORES (alfa) e design.css
 * os declara como custom properties — o comentário dos dois arquivos diz que
 * ESTE teste garante o espelho. Escuro = base; o tema claro sobrescreve os
 * mesmos nomes com alfas próprios (por isso a busca é pelo par rgb+alfa).
 */
describe("luz e profundidade (tokens de luz × design.css)", () => {
  const css = readFileSync(uiFile("src", "design", "design.css"), "utf8");

  it("cada alfa de LIGHT existe no CSS com o MESMO valor no tema escuro", () => {
    for (const [key, alpha] of Object.entries(LIGHT)) {
      const varName = LIGHT_VAR[key as keyof typeof LIGHT];
      const rgb = key.startsWith("shade") ? "0, 0, 0" : "255, 255, 255";
      expect(css, `${varName} espelha LIGHT.${key} = ${alpha}`).toContain(
        `${varName}: rgba(${rgb}, ${alpha})`,
      );
    }
  });

  it("raios e passos de texto do CSS batem com RADIUS/TYPE", () => {
    for (const [k, px] of Object.entries(RADIUS)) {
      expect(css).toContain(`--radius-${k}: ${px}px`);
    }
    for (const [k, px] of Object.entries(TYPE)) {
      expect(css).toContain(`--text-${k}: ${px}px`);
    }
  });

  it("stacks de fonte do CSS batem com FONT_STACKS", () => {
    for (const [k, stack] of Object.entries(FONT_STACKS)) {
      expect(css).toContain(`--font-${k}: ${stack}`);
    }
  });
});

/**
 * Tipografia e motion no MESMO esquema do LIGHT (#78): ate aqui o TS declarava
 * ser a fonte unica de peso/entrelinha/tracking/motion, mas o CSS escrevia
 * `font-weight: 700` cru e o teste nao olhava. Fonte unica que so vale pela
 * metade e pior que nenhuma — ela promete uma garantia que ninguem cumpre.
 */
/** Texto do CSS sem o bloco `:root` — la os tokens sao declarados, nao usados. */
function semRoot(css: string): string[] {
  return css.replace(/:root\s*\{[^}]*\}/, "").split("\n");
}

describe("tipografia e motion (tokens x design.css)", () => {
  const css = readFileSync(uiFile("src", "design", "design.css"), "utf8");
  const looper = readFileSync(uiFile("src", "design", "looper.css"), "utf8");

  it("todo peso de WEIGHT existe no CSS com o MESMO valor", () => {
    for (const [k, peso] of Object.entries(WEIGHT)) {
      expect(css, `--weight-${k} espelha WEIGHT.${k} = ${peso}`).toContain(`--weight-${k}: ${peso};`);
    }
  });

  it("toda entrelinha de LEADING existe no CSS com o MESMO valor", () => {
    for (const [k, valor] of Object.entries(LEADING)) {
      expect(css, `--leading-${k} espelha LEADING.${k} = ${valor}`).toContain(`--leading-${k}: ${valor};`);
    }
  });

  it("todo tracking de TRACKING existe no CSS, com a unidade que o token omite", () => {
    // O token e fracao de 1em; sem a unidade no CSS a mesma escala teria duas
    // grafias e a comparacao nao diria nada.
    for (const [k, valor] of Object.entries(TRACKING)) {
      expect(css, `--tracking-${k} espelha TRACKING.${k} = ${valor}em`).toContain(
        `--tracking-${k}: ${valor}em;`,
      );
    }
  });

  it("todo tempo de MOTION existe no CSS com o MESMO valor", () => {
    for (const [k, ms] of Object.entries(MOTION)) {
      expect(css, `--motion-${k} espelha MOTION.${k} = ${ms}ms`).toContain(`--motion-${k}: ${ms}ms;`);
    }
  });

  it("nenhum CSS escreve peso cru — so var(--weight-*)", () => {
    // `font-weight: 700` cru era o estado anterior, em 9 lugares. O peso e
    // escala do design system: sair da custom property e o unico jeito de
    // ele divergir de WEIGHT sem ninguem perceber.
    for (const [nome, texto] of [
      ["design.css", css],
      ["looper.css", looper],
    ] as const) {
      const cru = semRoot(texto).filter((l) => /font-weight:\s*[0-9]/.test(l));
      expect(cru, `${nome} ainda escreve font-weight cru`).toEqual([]);
    }
  });

  it("todo letter-spacing tem unidade — sem ela a declaracao e descartada", () => {
    // `letter-spacing: 1.2` era o estado anterior em `.pg-jack` e `.pg-mark`:
    // nao era so feio, era DECLARACAO INVALIDA. O navegador jogava a linha
    // fora e o efeito nunca existiu. Nudge local em px/em continua livre.
    for (const [nome, texto] of [
      ["design.css", css],
      ["looper.css", looper],
    ] as const) {
      const semUnidade = semRoot(texto).filter((l) => /letter-spacing:\s*-?[0-9.]+\s*;/.test(l));
      expect(semUnidade, `${nome} tem letter-spacing sem unidade`).toEqual([]);
    }
  });
});

describe("elevacao (tokens x design.css)", () => {
  const css = readFileSync(uiFile("src", "design", "design.css"), "utf8");
  /** Niveis que sao o `.gp-surface` base, sem modificador proprio. */
  const BASE = ["raised", "panel"];
  const modificadores = () => [...css.matchAll(/\.gp-surface--([a-z]+)/g)].map((m) => m[1]);

  it("todo modificador .gp-surface--X do CSS e um nivel declarado em ELEVATION", () => {
    // O sentido que pega o erro caro: um classe nova no CSS sugerindo um nivel
    // que o TS nao conhece — e o sistema de elevacao passa a ter um degrau
    // invisivel para quem le os tokens.
    expect(modificadores().length).toBeGreaterThan(0);
    for (const nivel of modificadores()) {
      expect(
        Object.keys(ELEVATION),
        `classe .gp-surface--${nivel} sem nivel correspondente em ELEVATION`,
      ).toContain(nivel);
    }
  });

  it("todo nivel de ELEVATION e o base da superficie ou tem modificador", () => {
    // O sentido contrario: nivel declarado que nada desenha.
    const mods = new Set(modificadores());
    for (const nivel of Object.keys(ELEVATION)) {
      expect(
        mods.has(nivel) || BASE.includes(nivel),
        `nivel ${nivel} nao tem modificador .gp-surface--${nivel} nem e nivel base`,
      ).toBe(true);
    }
  });
});

describe("dados do drum derivam, nao declaram", () => {
  it("DRUM_TOTAL_STYLES e a soma dos generos (derivado, nunca literal)", () => {
    const soma = DRUM_GENRES.reduce((t, g) => t + g.styles.length, 0);
    expect(DRUM_TOTAL_STYLES).toBe(soma);
  });

  it("o total bate com o firmware: 87 estilos em 5 generos", () => {
    // Se o dado mudar, este e o teste que avisa — o painel mostra o total.
    expect(DRUM_TOTAL_STYLES).toBe(87);
    expect(DRUM_GENRES.length).toBe(5);
  });

  it("nenhum genero tem lista vazia", () => {
    for (const g of DRUM_GENRES) {
      expect(g.styles.length, `genero ${g.genre} sem estilos`).toBeGreaterThan(0);
    }
  });
});

describe("contraste (WCAG 2.2 AA)", () => {
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
