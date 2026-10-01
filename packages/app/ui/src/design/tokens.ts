/**
 * Design tokens — FONTE ÚNICA em código (o documento de design guarda os
 * VALORES e rácios; aqui eles viram tipos/constantes testáveis).
 * Nenhum componente usa px/cor cru: usa estes tokens (lint + testes guardam).
 */

/**
 * Escala de espaçamento: modelo Fibonacci arredondado ao múltiplo de 4
 * (F6=8px exato; F7=13→12, F8=21→20, F9=34→32, F10=55, F11=89→84).
 */
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

/** Rácios de contraste medidos (WCAG 2.x, paleta Valeton Violet) — testados abaixo. */
export const CONTRAST_RATIOS = {
  textDark: 18.9,
  textLight: 16.1,
  mutedDark: 6.9,
  mutedLight: 6.9,
  accentDark: 7.5, // accent-text (violeta claro) sobre bg escuro
  accentLight: 6.5, // violeta profundo sobre bg claro
  okDark: 11.3,
  okLight: 5.0,
  warnDark: 11.8,
  warnLight: 6.4,
  errorDark: 7.1,
  errorLight: 5.9,
  onAccentDark: 5.5, // texto claro sobre o violeta anodizado (botão ativo)
  onAccentLight: 7.6,
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
    "text/bg-raised": 17.4,
    "text-muted/bg": CONTRAST_RATIOS.mutedDark,
    "accent-text/bg": CONTRAST_RATIOS.accentDark,
    "ok/bg": CONTRAST_RATIOS.okDark,
    "warn/bg": CONTRAST_RATIOS.warnDark,
    "error/bg": CONTRAST_RATIOS.errorDark,
    "on-accent/accent": CONTRAST_RATIOS.onAccentDark,
  },
  light: {
    "text/bg": CONTRAST_RATIOS.textLight,
    "text/bg-raised": 17.7,
    "text-muted/bg": CONTRAST_RATIOS.mutedLight,
    "accent-text/bg": CONTRAST_RATIOS.accentLight,
    "ok/bg": CONTRAST_RATIOS.okLight,
    "warn/bg": CONTRAST_RATIOS.warnLight,
    "error/bg": CONTRAST_RATIOS.errorLight,
    "on-accent/accent": CONTRAST_RATIOS.onAccentLight,
  },
} as const;
