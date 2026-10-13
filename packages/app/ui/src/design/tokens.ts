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

/**
 * Raio de borda — MESMA escala do espaçamento (§1): o doc diz "raio usa a
 * escala (4/8/12)"; o 16 fica como passo de destaque (modais/flutuantes).
 */
export const RADIUS = { s: SPACE[4], m: SPACE[8], l: SPACE[12], xl: 16 } as const;

/**
 * Tipografia refinada (§3): passos, pesos, entrelinha e tracking. Os pesos
 * "variáveis" (500/600) só aparecem de verdade com fontes de eixo variável
 * (Segoe UI Variable no Windows 11) — nos fallbacks o SO aproxima.
 */
export const TYPE = {
  "2xs": 10, // micro-rótulos de hardware (mm:ss, contadores)
  xs: 11, // badges, atalhos
  sm: 13, // UI padrão (knobs, listas)
  md: 15, // corpo corrido
  lg: 21, // título de seção
  xl: 34, // herói/empty state
} as const;

/**
 * Pesos usados pela UI (nada de 100/900 fora do display).
 * Espelham `--weight-*` no design.css — o teste de token garante o valor,
 * e nenhum CSS escreve peso cru.
 */
export const WEIGHT = {
  regular: 400,
  medium: 500,
  semibold: 600,
  bold: 700,
  black: 800,
} as const;

/** Entrelinha por papel (a escala modular usa 1.2–1.55). Espelha `--leading-*`. */
export const LEADING = { tight: 1.2, snug: 1.3, normal: 1.45, body: 1.55 } as const;

/**
 * Tracking: negativo só em títulos grandes; positivo em texto caixa-alta.
 * O valor é FRAÇÃO de 1em (0.02 = 2%) e o CSS escreve a unidade:
 * `--tracking-caps: 0.08em`. Sem isso a mesma escala teria duas grafias.
 */
export const TRACKING = { tight: -0.01, normal: 0, wide: 0.02, caps: 0.08 } as const;

/** Stacks: nativas do SO primeiro (sem download de fonte — desktop offline). */
export const FONT_STACKS = {
  /** UI geral — variante "Text" da Segoe UI Variable, com fallbacks do SO */
  ui: '"Segoe UI Variable Text", "Segoe UI", Inter, system-ui, -apple-system, sans-serif',
  /** títulos/display — variante "Display" (pesos largos mais elegantes) */
  display: '"Segoe UI Variable Display", "Segoe UI", Inter, system-ui, -apple-system, sans-serif',
  /** números de fio/hex — monoespaçada com tabular para não tremer */
  mono: '"Cascadia Mono", "Cascadia Code", Consolas, ui-monospace, monospace',
} as const;

/**
 * CAMADA DE LUZ (issue #9) — direção ÚNICA: luz em cima/esquerda, sombra
 * embaixo/direita. Os alphas moram aqui (valores) e viram custom properties
 * no `design.css` (o teste de token garante que os dois batem).
 */
export const LIGHT = {
  /** brilho na aresta superior (fio de luz) */
  top: 0.16,
  /** brilho suave de superfície (topo do gradiente) */
  sheen: 0.06,
  /** brilho especular (ponto de luz refletida em metal/vidro) */
  specular: 0.6,
  /** luz de recorte nas laterais/arestas opostas */
  rim: 0.1,
  /** sombra difusa (logo abaixo do objeto) */
  shadeSoft: 0.35,
  /** sombra de contato (profundidade/oclusão) */
  shadeDeep: 0.6,
} as const;
type LightKey = keyof typeof LIGHT;

/** Custom property de cada token de luz (design.css) — teste de token usa. */
export const LIGHT_VAR: Record<LightKey, string> = {
  top: "--light-top",
  sheen: "--light-sheen",
  specular: "--light-specular",
  rim: "--light-rim",
  shadeSoft: "--shade-soft",
  shadeDeep: "--shade-deep",
} as const;

/**
 * Níveis de elevação (quanto o módulo "sai" da mesa). O CSS implementa
 * cada nível como modificador `.gp-surface--<nível>`; `raised`/`panel` são o
 * `.gp-surface` base e por isso não têm classe própria (o teste de token
 * confere os dois sentidos da ligação).
 */
export const ELEVATION = { flat: 0, raised: 1, panel: 2, floating: 3 } as const;

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

/**
 * Motion (ms) — respeita prefers-reduced-motion via design.css.
 * Espelha `--motion-*`; o CSS carrega a unidade (ms).
 */
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
