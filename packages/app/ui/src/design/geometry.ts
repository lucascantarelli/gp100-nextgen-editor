/**
 * Geometria do pedal — ARTE DO PALCO, extraída do componente (#82).
 *
 * Por que este módulo existe: `layoutFor`/`pedalDims` decidiam a geometria de
 * dentro do `.tsx`, o que os tornava testáveis SÓ renderizando React — e o
 * contrato mais fácil de quebrar do projeto (o passo dos knobs, o corte das
 * colunas, a altura do deck). Aqui não há React nenhum: o teste importa a
 * função e verifica a geometria direto.
 *
 * Morar em `design/` (e não em `components/`) é o que diz "isto é geometria,
 * não componente" — a mesma separação de `tokens.ts`, que o `design.test.ts`
 * já exercita.
 */

import type { BoardSlot } from "../ipc/types";
import { modelFor } from "../artifacts/fxModels";

/** Escala do pedal: palco (compacto) ou modal de edição (grande). */
export type PedalVariant = "board" | "modal";

/** Métricas de layout por escala (arte do palco — não é dado do device). */
export const SCALES = {
  board: {
    knob: 32,
    pitchX: 46, // passo horizontal (knob + respiro)
    rowH: 54, // passo vertical (knob + rótulo + valor)
    cy0: 78, // centro da 1ª linha de knobs
    deckTop: 58,
    footR: 12,
    valueBox: false, // palco: valor em texto (a edição mora no modal)
    plate: false,
  },
  modal: {
    knob: 64,
    pitchX: 116,
    rowH: 116,
    cy0: 200,
    deckTop: 154,
    footR: 19,
    valueBox: true,
    plate: true,
  },
} as const;

interface Layout {
  W: number;
  H: number;
  cols: number;
  rows: number;
  /** centro da 1ª linha de knobs */
  cy0: number;
  /** recorte do deck (fundo dos controles) */
  deckTop: number;
  deckH: number;
  footY: number;
  jackY: number;
  rowH: number;
  /** passo horizontal dos knobs (pode encolher p/ caber no enclosure) */
  pitchX: number;
  nameMax: number;
  labelMax: number;
}

/** Geometria determinística do pedal para um slot e uma escala. */
export function layoutFor(slot: BoardSlot, variant: PedalVariant): Layout {
  const m = modelFor(slot);
  const s = SCALES[variant];
  const cols = Math.min(modelCols(m.cols), 4);
  const rows = Math.max(1, Math.ceil(Math.max(slot.knobs.length, 1) / cols));
  if (variant === "modal") {
    // escala de EDIÇÃO: a geometria grande de sempre (deck com as caixas de
    // valor + footswitch e placa no pé, sem nenhuma sobreposição)
    const W = Math.max(m.w, 420);
    const H = 150 + rows * s.rowH + 110;
    return {
      W,
      H,
      cols,
      rows,
      cy0: s.cy0,
      deckTop: s.cy0 - 46,
      deckH: rows * s.rowH + 52,
      footY: H - 66,
      jackY: H - 22,
      rowH: s.rowH,
      pitchX: s.pitchX,
      nameMax: 26,
      labelMax: 16,
    };
  }
  // escala do PALCO: a largura fica DENTRO do espaçamento do .board-slots
  // (118–132px) — o pedal mais largo do catálogo nunca estoura a coluna, então
  // os 9 convivem no board. O passo dos knobs encolhe p/ caber no enclosure.
  const W = Math.max(118, Math.min(m.w, 132));
  let boardCols = Math.max(1, Math.min(modelCols(m.cols), 4));
  while (boardCols > 1 && (W - 12) / boardCols < 34) boardCols -= 1; // nunca sobrepõe
  const pitchX = Math.min(s.pitchX, (W - 12) / boardCols);
  const boardRows = Math.max(1, Math.ceil(Math.max(slot.knobs.length, 1) / boardCols));
  const cy0 = s.cy0;
  const lastVal = cy0 + (boardRows - 1) * s.rowH + (s.knob / 2 + 9 + 10);
  const deckTop = s.deckTop;
  const deckH = lastVal + 4 - deckTop;
  const footY = deckTop + deckH + 26;
  const H = footY + s.footR + 19;
  return {
    W,
    H,
    cols: boardCols,
    rows: boardRows,
    cy0,
    deckTop,
    deckH,
    footY,
    jackY: H - 11,
    rowH: s.rowH,
    pitchX,
    nameMax: Math.max(6, Math.floor((W - 22) / 6.4)),
    labelMax: Math.max(3, Math.floor((pitchX - 8) / 5.4)),
  };
}

/** `cols` do catálogo (sempre ≥1 — modelo nunca vem vazio). */
function modelCols(cols: number): number {
  return cols > 0 ? cols : 1;
}

/** Dimensões determinísticas do pedal (board/modal usam p/ layout). */
export function pedalDims(slot: BoardSlot, variant: PedalVariant = "board"): { w: number; h: number } {
  const l = layoutFor(slot, variant);
  return { w: l.W, h: l.H };
}
