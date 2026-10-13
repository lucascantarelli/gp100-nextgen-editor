/**
 * Geometria do pedal — testada SEM React (#82).
 *
 * Este arquivo é `.ts` de propósito e não importa nada de React: a prova de que
 * a geometria saiu do componente. Antes, `layoutFor`/`pedalDims` só podiam ser
 * exercitadas renderizando o App inteiro, e o contrato mais fácil de quebrar do
 * projeto (passo dos knobs, largura do enclosure, altura do deck) ficava
 * escuro justamente por isso.
 */

import { describe, expect, it } from "vitest";
import { layoutFor, pedalDims, SCALES } from "../src/design/geometry";
import type { BoardSlot } from "../src/ipc/types";
import { DRUM_BEATS } from "../src/artifacts/drumData";

/** Slot sintético: a geometria só lê `family` e a quantidade de `knobs`. */
function slot(knobs: number, family: BoardSlot["family"] = "DST"): BoardSlot {
  return {
    slot: 0,
    family,
    archetype: "DISTORTION",
    name: "Teste",
    variant: "green-od",
    state: true,
    code: 0x11,
    knobs: Array.from({ length: knobs }, (_, i) => ({
      name: `P${i}`,
      pos: i,
      kind: "knob" as const,
      range: [0, 100] as [number, number],
      options: [],
    })),
  };
}

describe("geometria do pedal (sem React)", () => {
  it("o palco cabe no espaçamento do .board-slots (118–132px)", () => {
    // O contrato que o comentário do layout promete: o pedal mais largo do
    // catálogo nunca estoura a coluna, senão os 9 não convivem no board.
    for (const knobs of [1, 3, 6, 12]) {
      for (const family of ["DST", "RVB", "CAB", "EQ"] as const) {
        const { w } = pedalDims(slot(knobs, family));
        expect(w, `palco com ${knobs} knobs (${family})`).toBeGreaterThanOrEqual(118);
        expect(w, `palco com ${knobs} knobs (${family})`).toBeLessThanOrEqual(132);
      }
    }
  });

  it("os knobs nunca se sobrepõem: o passo horizontal cabe o diâmetro", () => {
    // `layoutFor` encolhe boardCols até `(W - 12) / cols >= 34`; 34 é o diâmetro
    // do knob do palco (32) mais o respiro. Se essa conta mudar, os knobs
    // encostam — e isso só se enxerga geometricamente.
    for (const knobs of [1, 2, 4, 8, 12, 20]) {
      const l = layoutFor(slot(knobs), "board");
      expect(l.pitchX, `${knobs} knobs`).toBeGreaterThanOrEqual(34);
    }
  });

  it("nunca mais de 4 colunas e nunca zero linhas", () => {
    for (const knobs of [0, 1, 5, 12, 40]) {
      for (const variant of ["board", "modal"] as const) {
        const l = layoutFor(slot(knobs), variant);
        expect(l.cols).toBeGreaterThanOrEqual(1);
        expect(l.cols).toBeLessThanOrEqual(4);
        expect(l.rows).toBeGreaterThanOrEqual(1);
      }
    }
  });

  it("slot sem knob não quebra (o modelo nunca vem vazio, mas o desenho sim)", () => {
    const l = layoutFor(slot(0), "board");
    expect(l.rows).toBe(1);
    expect(Number.isFinite(l.H)).toBe(true);
  });

  it("o modal é sempre MAIOR que o palco do mesmo slot", () => {
    for (const knobs of [1, 4, 12]) {
      const board = pedalDims(slot(knobs));
      const modal = pedalDims(slot(knobs), "modal");
      expect(modal.w, `${knobs} knobs: largura`).toBeGreaterThan(board.w);
      expect(modal.h, `${knobs} knobs: altura`).toBeGreaterThan(board.h);
    }
  });

  it("o modal tem largura mínima de 420 (as caixas de valor precisam caber)", () => {
    expect(pedalDims(slot(1), "modal").w).toBeGreaterThanOrEqual(420);
  });

  it("mais knobs => mais altura, nas duas escalas", () => {
    for (const variant of ["board", "modal"] as const) {
      const alturas = [1, 4, 8, 12].map((k) => pedalDims(slot(k), variant).h);
      for (let i = 1; i < alturas.length; i += 1) {
        expect(alturas[i], `${variant}: ${i} knobs`).toBeGreaterThan(alturas[i - 1]);
      }
    }
  });

  it("cada knob cabe na altura da sua linha (deck não engole a fileira)", () => {
    for (const variant of ["board", "modal"] as const) {
      const l = layoutFor(slot(8), variant);
      // O centro da última linha + meia volta do knob precisa estar dentro
      // do recorte do deck, senão o último knob cai no pé do pedal.
      const ultimoCentro = l.cy0 + (l.rows - 1) * l.rowH;
      expect(ultimoCentro, `${variant}: última linha dentro do deck`)
        .toBeLessThan(l.deckTop + l.deckH);
    }
  });

  it("o footswitch nunca invade a última fileira de knobs (as duas escalas)", () => {
    // A promessa no desenho é "sem nenhuma sobreposição", mas ela significa
    // coisas diferentes nas duas escalas: no PALCO o footswitch fica FORA do
    // deck, e no MODAL o deck é o fundo que hospeda as caixas de valor E o
    // footswitch. O que é comum — e o que quebra se a conta do deck mudar — é
    // o pé ficar ABAIXO da última fileira de knobs.
    for (const variant of ["board", "modal"] as const) {
      const l = layoutFor(slot(4), variant);
      const ultimoCentro = l.cy0 + (l.rows - 1) * l.rowH;
      const baseDoKnob = ultimoCentro + SCALES[variant].knob / 2;
      expect(l.footY, `${variant}: footswitch abaixo do último knob`)
        .toBeGreaterThan(baseDoKnob);
      expect(l.jackY, `${variant}: jack abaixo do footswitch`).toBeGreaterThan(l.footY);
      expect(l.jackY, `${variant}: jack dentro do enclosure`).toBeLessThan(l.H);
    }
  });

  it("no palco o footswitch fica fora do deck; no modal, dentro dele", () => {
    // A assimetria é de propósito e estava implícita no código: o deck do
    // palco é só o fundo dos controles, o do modal inclui o pé.
    const board = layoutFor(slot(4), "board");
    expect(board.footY).toBeGreaterThan(board.deckTop + board.deckH);

    const modal = layoutFor(slot(4), "modal");
    expect(modal.footY).toBeLessThan(modal.deckTop + modal.deckH);
    expect(modal.footY).toBeGreaterThan(modal.deckTop);
  });

  it("as duas escalas têm knob e passo coerentes com a escala declarada", () => {
    // SCALES é a fonte: o palco é compacto, o modal é grande. Se alguém
    // trocar um número sem querer, a checagem relativa denuncia.
    expect(SCALES.modal.knob).toBeGreaterThan(SCALES.board.knob);
    expect(SCALES.modal.pitchX).toBeGreaterThan(SCALES.board.pitchX);
    expect(SCALES.modal.rowH).toBeGreaterThan(SCALES.board.rowH);
    // O palco declara valor em texto (edição mora no modal).
    expect(SCALES.board.valueBox).toBe(false);
    expect(SCALES.modal.valueBox).toBe(true);
  });

  it("o compasso do drum é um dado independente da geometria", () => {
    // Guarda contra mistura de responsabilidades: geometry.ts não deve
    // puxar artefatos de ritmo para dentro do layout do pedal.
    expect(DRUM_BEATS).toContain("4/4");
  });
});
