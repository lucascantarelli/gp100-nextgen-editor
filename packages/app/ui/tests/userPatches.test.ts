/**
 * `userPatches` — patches de USUÁRIO (patches salvos pelo dono).
 *
 * Depois da #26 este arquivo é a FRONTEIRA entre o retrato (o que o palco
 * desenha) e o registro do banco (o que o SQLite guarda). O contrato testado
 * aqui é o que sustenta a aba User Patch:
 *   - o patch é um RETRATO: mexer no pedalboard depois de salvar NÃO altera o
 *     que foi guardado (clone profundo dos slots e dos knobs);
 *   - abrir um patch devolve um BoardView no MESMO formato do device (o Stage
 *     não distingue banco de fábrica de banco de usuário);
 *   - registro ↔ retrato é um round-trip: o que o dono salva é o que volta.
 */
import { describe, expect, it } from "vitest";
import { boardOfUserPatch, patchDeRegistro, registroDePatch, snapshotOf } from "../src/userPatches";
import { ARCHETYPE_OF } from "../src/ipc/types";
import type { BoardSlot, BoardView } from "../src/ipc/types";
import type { LibraryRecord } from "../src/ipc/library";

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
  ppType: 4,
  ppTypeName: "Rock",
  slots: [slot(0), slot(1)],
  bank: "factory",
  ppLabel: "P25",
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

  it("o patch guarda o estilo de onde saiu e a DATA em que foi salvo", () => {
    const saved = snapshotOf(board(), "Meu som", 0);
    expect(saved.ppType).toBe(4);
    expect(saved.ppTypeName).toBe("Rock");
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
    // pp = −1: patch de usuário não tem cursor de fábrica (◀ ▶ seguem a coluna real)
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

describe("userPatches — a fronteira com o banco (#26)", () => {
  it("gravar serializa a cadeia e marcar que ela veio junto", () => {
    const rec = registroDePatch(snapshotOf(board(), "Meu som", 0));
    expect(rec.bank).toBe("user");
    // patch de usuário não tem pp de fábrica: o cursor do palco é o U##
    expect(rec.pp).toBeNull();
    expect(rec.hasPayload).toBe(true);
    expect(typeof rec.payload).toBe("string");
    expect(JSON.parse(rec.payload!)).toHaveLength(2);
  });

  it("round-trip: o que o dono salva volta idêntico ao abrir", () => {
    const salvo = snapshotOf(board(), "Meu som", 0);
    const volta = patchDeRegistro(registroDePatch(salvo));
    expect(volta).toEqual(salvo);
  });

  it("payload ilegível devolve null — a UI decide, não um throw no palco", () => {
    // É o caso de registro escrito por outra versão do app ou editado à mão.
    const base: LibraryRecord = { ...registroDePatch(snapshotOf(board(), "x", 0)), payload: "{isto nao eh json" };
    expect(patchDeRegistro(base)).toBeNull();
    expect(patchDeRegistro({ ...base, payload: null })).toBeNull();
    expect(patchDeRegistro({ ...base, payload: '{"a":1}' })).toBeNull();
  });
});