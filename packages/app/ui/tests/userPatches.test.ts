/**
 * `userPatches` — patches de USUÁRIO (patches salvos pelo dono).
 *
 * O contrato testado aqui é o que sustenta a aba User Patch da biblioteca:
 *   - a lista sobrevive à sessão (localStorage) e não quebra com lixo no storage;
 *   - o patch é um RETRATO: mexer no pedalboard depois de salvar NÃO altera o
 *     que foi guardado (clone profundo dos slots e dos knobs);
 *   - abrir um patch devolve um BoardView no MESMO formato do device (o Stage
 *     não distingue banco de fábrica de banco de usuário).
 */
import { beforeEach, describe, expect, it } from "vitest";
import { boardOfUserPatch, loadUserPatches, snapshotOf, storeUserPatches } from "../src/userPatches";
import type { UserPatch } from "../src/userPatches";
import { ARCHETYPE_OF } from "../src/ipc/types";
import type { BoardSlot, BoardView } from "../src/ipc/types";

const slot = (n: number): BoardSlot => ({
  slot: n,
  family: "PRE",
  archetype: ARCHETYPE_OF.PRE,
  name: `Pedal ${n}`,
  variant: "generic",
  code: n,
  state: true,
  knobs: [
    { name: "Gain", pos: 0, kind: "knob", options: [], value: "50", default: "50", range: [0, 99] },
    { name: "Mode", pos: 1, kind: "switch", options: ["off", "on"], value: "0", default: "0" },
  ],
});

/** board NOVO a cada chamada: os testes mexem nele de propósito */
const board = (): BoardView => ({
  pp: 24,
  name: "Mist",
  ppType: 0,
  ppTypeName: "Rock",
  slots: [slot(0), slot(1)],
  bank: "factory",
  ppLabel: "P25",
});

const BOARD_KEY = "gp100.userpatch.v1";

beforeEach(() => {
  localStorage.clear();
});

describe("userPatches — armazenamento local", () => {
  it("começa vazio e sobrevive à sessão (round-trip)", () => {
    expect(loadUserPatches()).toEqual([]);
    const saved: UserPatch[] = [snapshotOf(board(), "Meu som", 0)];
    storeUserPatches(saved);
    expect(loadUserPatches()).toEqual(saved);
  });

  it("storage indisponível/lixo nunca derruba a biblioteca", () => {
    localStorage.setItem(BOARD_KEY, "{ não é json");
    expect(loadUserPatches()).toEqual([]);
    // objeto solto (não é lista) e entradas sem slots são descartadas
    localStorage.setItem(BOARD_KEY, JSON.stringify({ nao: "lista" }));
    expect(loadUserPatches()).toEqual([]);
    localStorage.setItem(BOARD_KEY, JSON.stringify([{ id: "u1", name: "sem slots" }]));
    expect(loadUserPatches()).toEqual([]);
  });
});

describe("userPatches — snapshot é um retrato", () => {
  it("ajustar o pedalboard depois de salvar não altera o patch guardado", () => {
    const live = board();
    const saved = snapshotOf(live, "Meu som", 0);
    // mexemos na cadeia viva E no range de um knob: o retrato não pode sentir
    live.slots[0].knobs[0].value = "99";
    live.slots[0].knobs[0].range![0] = -12;
    live.slots[0].knobs[1].options.push("extra");
    expect(saved.slots[0].knobs[0].value).toBe("50");
    expect(saved.slots[0].knobs[0].range).toEqual([0, 99]);
    expect(saved.slots[0].knobs[1].options).toEqual(["off", "on"]);
  });

  it("o patch guarda de onde veio (o rótulo do patch de fábrica)", () => {
    const saved = snapshotOf(board(), "Meu som", 0);
    expect(saved.fromLabel).toBe("P25");
    expect(saved.savedAt).toMatch(/^\d{4}-\d{2}-\d{2}T/);
  });

  it("ids distintos por save: a lista não deduplica nomes iguais", () => {
    const a = snapshotOf(board(), "Mesmo nome", 0);
    const b = snapshotOf(board(), "Mesmo nome", 1);
    expect(a.id).not.toBe(b.id);
  });
});

describe("userPatches — abrir no palco", () => {
  it("devolve um BoardView do MESMO formato do device, marcado como usuário", () => {
    const saved = snapshotOf(board(), "Meu som", 0);
    const b = boardOfUserPatch(saved, 0);
    expect(b.bank).toBe("user");
    expect(b.ppLabel).toBe("U01");
    // pp = −1: patch local não tem cursor de fábrica (◀ ▶ seguem a coluna real)
    expect(b.pp).toBe(-1);
    expect(b.name).toBe("Meu som");
    expect(b.slots).toEqual(saved.slots);
  });

  it("o rótulo U## acompanha a posição na lista do dono", () => {
    const saved = snapshotOf(board(), "Meu som", 0);
    expect(boardOfUserPatch(saved, 2).ppLabel).toBe("U03");
  });

  it("abrir o patch não dá acesso ao snapshot guardado (cópia de novo)", () => {
    const saved = snapshotOf(board(), "Meu som", 0);
    const b = boardOfUserPatch(saved, 0);
    b.slots[0].knobs[0].value = "0";
    expect(saved.slots[0].knobs[0].value).toBe("50");
  });
});