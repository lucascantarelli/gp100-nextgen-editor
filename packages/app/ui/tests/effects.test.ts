/**
 * `effects` — a lista de efeitos que um módulo da cadeia oferece (issue #19).
 *
 * O contrato testado aqui é o da "Effects List" do modal de edição:
 *   - a lista é do MÓDULO (PRE oferece PREs, DST oferece DSTs) e nunca mostra
 *     as entradas de nome vazio do dicionário (slots internos do firmware);
 *   - o `effectCode` sai do `effectCode` do `.prst` (nibble << 24 | index), a
 *     mesma conta que o `device.ts` faz ao ler o board;
 *   - trocar o efeito preserva POSIÇÃO, FAMÍLIA e ligado/desligado, e devolve
 *     os controles no DEFAULT do algoritmo novo — herdar o valor do efeito
 *     anterior seria a UI mentindo sobre o som que está saindo.
 */
import { describe, expect, it } from "vitest";
import { algorithmOf, algorithmsOf, codeOf, filterAlgorithms, knobsOf, withAlgorithm } from "../src/effects";
import { FX_MODULES } from "../src/artifacts/fxData";
import { ARCHETYPE_OF } from "../src/ipc/types";
import type { BoardSlot } from "../src/ipc/types";

const comp = FX_MODULES.PRE.find((a) => a.name === "COMP")!;
const cWah = FX_MODULES.PRE.find((a) => a.name === "C-Wah")!;

const slotWith = (algName: string, family: BoardSlot["family"] = "PRE"): BoardSlot => {
  const alg = FX_MODULES[family].find((a) => a.name === algName)!;
  return {
    slot: 0,
    family,
    archetype: ARCHETYPE_OF[family],
    name: alg.name,
    variant: alg.variant,
    state: true,
    code: codeOf(alg),
    knobs: knobsOf(alg).map((k) => ({ ...k, value: "77" })),
  };
};

describe("effects — a lista do módulo", () => {
  it("cada módulo oferece só os algoritmos DELE (PRE não oferece boosts de AMP)", () => {
    expect(algorithmsOf("PRE").map((a) => a.name)).toContain("COMP");
    expect(algorithmsOf("PRE").map((a) => a.name)).not.toContain("Bog RedM"); // é AMP
    expect(algorithmsOf("AMP").map((a) => a.name)).toContain("Bog RedM");
  });

  it("nunca lista as entradas de nome vazio (slots internos do firmware)", () => {
    for (const family of ["PRE", "DST", "AMP", "NR", "CAB", "EQ", "MOD", "DLY", "RVB"] as const) {
      const list = algorithmsOf(family);
      expect(list.length, family).toBeGreaterThan(0);
      expect(list.filter((a) => a.name.trim() === ""), `${family} sem linha em branco`).toEqual([]);
    }
  });

  it("busca por nome ignora caixa (nome e slug do variant)", () => {
    const pre = algorithmsOf("PRE");
    // "wah" pega os 4 wahs do módulo, independente da caixa
    expect(filterAlgorithms(pre, "WAH").map((a) => a.name)).toEqual([
      "C-Wah",
      "V-Wah",
      "T-Wah",
      "A-WAH",
    ]);
    // o slug do variant também casa: "step-filter" acha "Step Filter"
    expect(filterAlgorithms(pre, "STEP-FILTER").map((a) => a.name)).toEqual(["Step Filter"]);
    expect(filterAlgorithms(pre, "")).toHaveLength(pre.length);
    expect(filterAlgorithms(pre, "zzz-nada")).toEqual([]);
  });

  it("effectCode = nibble do módulo no byte alto + índice (o do .prst)", () => {
    expect(codeOf(comp)).toBe((comp.nibble << 24) | comp.index);
    expect(codeOf(cWah)).toBe(83886088); // 0x05000008, o PRE/C-Wah do P01
  });

  it("encontra o algoritmo que o palco está desenhando", () => {
    expect(algorithmOf(slotWith("C-Wah"))?.name).toBe("C-Wah");
    expect(algorithmOf(slotWith("Bog RedM", "AMP"))?.name).toBe("Bog RedM");
  });
});

describe("effects — trocar o efeito do slot", () => {
  it("mantém posição, família e ligado/desligado; troca nome, code e variant", () => {
    const before = slotWith("C-Wah");
    const after = withAlgorithm(before, comp);
    expect(after.slot).toBe(before.slot);
    expect(after.family).toBe("PRE");
    expect(after.state).toBe(before.state);
    expect(after.archetype).toBe(before.archetype);
    expect(after.name).toBe("COMP");
    expect(after.code).toBe(codeOf(comp));
    expect(after.variant).toBe(comp.variant);
  });

  it("os controles voltam ao DEFAULT do algoritmo novo (nada é inventado)", () => {
    const before = slotWith("C-Wah"); // knobs com value "77" (editados)
    const after = withAlgorithm(before, comp); // COMP = Sustain/Output
    expect(after.knobs.map((k) => k.name)).toEqual(["Sustain", "Output"]);
    expect(after.knobs.map((k) => k.value)).toEqual(["20.0", "50.0"]);
    expect(after.knobs.every((k) => k.kind === "knob")).toBe(true);
  });

  it("o botão (pedal) decompõe em switch e combox com as opções do dicionário", () => {
    const after = withAlgorithm(slotWith("C-Wah"), FX_MODULES.PRE.find((a) => a.name === "Boost")!);
    const bright = after.knobs.find((k) => k.name === "Bright");
    expect(bright?.kind).toBe("switch");
    expect(bright?.options).toEqual(["On", "Off"]);
    // combox: kind próprio, mesmas opções
    const tWah = withAlgorithm(slotWith("C-Wah"), FX_MODULES.PRE.find((a) => a.name === "T-Wah")!);
    expect(tWah.knobs.find((k) => k.name === "Mode")?.kind).toBe("combox");
    expect(tWah.knobs.find((k) => k.name === "Mode")?.options).toEqual(["Guitar", "Bass"]);
  });

  it("trocar para o MESMO efeito não reconstrói nada (identity, sem churn)", () => {
    const before = slotWith("C-Wah");
    expect(withAlgorithm(before, cWah)).toBe(before);
  });
});