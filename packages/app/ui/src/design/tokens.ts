/**
 * Design tokens — FONTE ÚNICA em código (docs/UI_DESIGN.md é a fonte de
 * verdade dos VALORES e rácios; aqui eles viram tipos/constantes testáveis).
 * Nenhum componente usa px/cor cru: usa estes tokens (lint + testes guardam).
 */

/** Escala de espaçamento: Fibonacci arredondado ao múltiplo de 4 (F4–F11). */
export const SPACE = {
  4: 4, //   F4 — dentro de controles (ícone↔texto)
  8: 8, //   F6 — gap de listas, padding pequeno
  12: 12, // F7 — padding de inputs, gap de knobs
  20: 20, // F8 — padding de painéis
  32: 32, // F9 — entre seções
  52: 52, // F10 — respiro lateral de página
  84: 84, // F11 — herói/empty
} as const;
export type SpaceStep = keyof typeof SPACE;

/** Rácios de contraste medidos (docs/UI_DESIGN.md §2) — testados abaixo. */
export const CONTRAST_RATIOS = {
  textDark: 12.4,
  textLight: 14.8,
  mutedDark: 6.2,
  mutedLight: 6.0,
  accentDark: 9.3,
  accentLight: 4.8,
  okDark: 8.1,
  okLight: 5.4,
  warnDark: 9.5,
  warnLight: 5.9,
  errorDark: 6.7,
  errorLight: 5.9,
} as const;

/** WCAG 2.2 — pisos obrigatórios. */
export const WCAG = {
  textAA: 4.5, // 1.4.3 texto
  uiAA: 3.0, // 1.4.11 UI/gráficos
  focusWidthPx: 2, // 2.4.13 foco ≥2px
  focusContrast: 3.0, // 2.4.13 foco ≥3:1 vs adjacente
  minTargetPx: 32, // alvo clicável mínimo (desktop híbrido)
} as const;

/** Motion (ms) — respeita prefers-reduced-motion via design.css. */
export const MOTION = { fast: 120, panel: 200 } as const;

/** Pares de contraste válidos (fg/bg) por tema — o ÚNICO caminho para texto. */
export const TEXT_PAIRS = {
  dark: {
    "text/bg": CONTRAST_RATIOS.textDark,
    "text/bg-raised": 11.0,
    "text-muted/bg": CONTRAST_RATIOS.mutedDark,
    "accent/bg": CONTRAST_RATIOS.accentDark,
    "ok/bg": CONTRAST_RATIOS.okDark,
    "warn/bg": CONTRAST_RATIOS.warnDark,
    "error/bg": CONTRAST_RATIOS.errorDark,
  },
  light: {
    "text/bg": CONTRAST_RATIOS.textLight,
    "text/bg-raised": 13.2,
    "text-muted/bg": CONTRAST_RATIOS.mutedLight,
    "accent/bg": CONTRAST_RATIOS.accentLight,
    "ok/bg": CONTRAST_RATIOS.okLight,
    "warn/bg": CONTRAST_RATIOS.warnLight,
    "error/bg": CONTRAST_RATIOS.errorLight,
  },
} as const;
