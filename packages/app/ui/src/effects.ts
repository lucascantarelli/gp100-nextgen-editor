/**
 * effects — a LISTA DE EFEITOS de um módulo da cadeia (issue #19, U-3).
 *
 * O manual do firmware (EDIT mode, pág. 7-8) descreve exatamente esta tela:
 * escolher o módulo, ver os parâmetros e trocar o algoritmo que está nele. O
 * dicionário vem de `fxData.ts` (GERADO de `analysis/parameters.json` — 185
 * algoritmos / 639 controles validados 3 vias), o mesmo que o `device.ts` usa
 * para montar os knobs do palco.
 *
 * Trocar o efeito é, no `.prst`, mudar `effectName` + `effectCode` do `<Effect>`
 * daquele slot. Como não há comando no fio para isso (o `change-effect`/`0x47`
 * da família `0x4X` ainda não foi caracterizado — PROTOCOL §4, BLOCKERS item
 * 10b), a troca aqui é **PRÉVIA LOCAL**: muda o estado do board e o pedal do
 * palco já mostra o novo efeito com os knobs novos. A escrita fica para a
 * captura (roteiro em docs/CAPTURE_PLAN.md, CAPTURA 5).
 */
import { FX_MODULES } from "./artifacts/fxData";
import type { FxAlgorithm } from "./artifacts/fxData";
import type { BoardKnob, BoardSlot } from "./ipc/types";

/** `effectCode` do `.prst`: nibble do módulo no byte alto + índice do algoritmo. */
export const codeOf = (alg: FxAlgorithm): number => (alg.nibble << 24) | alg.index;

/**
 * Algoritmos que o módulo oferece na UI.
 *
 * O dicionário traz entradas de nome VAZIO (variante "fx", índice 1.048.576+):
 * são slots internos do firmware, não efeitos escolhíveis pelo usuário — o
 * Suite também não os lista. Fica de fora para a lista não mostrar linhas em
 * branco.
 */
export function algorithmsOf(family: BoardSlot["family"]): FxAlgorithm[] {
  return (FX_MODULES[family] ?? []).filter((a) => a.name.trim() !== "");
}

/** Filtro da busca da lista (nome do efeito, sem diferenciar caixa). */
export function filterAlgorithms(list: FxAlgorithm[], q: string): FxAlgorithm[] {
  const needle = q.trim().toLowerCase();
  if (!needle) return list;
  return list.filter((a) => a.name.toLowerCase().includes(needle) || a.variant.toLowerCase().includes(needle));
}

/** Algoritmo correspondente a um `effectCode` (o que o palco está desenhando). */
export function algorithmOf(slot: BoardSlot): FxAlgorithm | undefined {
  return algorithmsOf(slot.family).find((a) => codeOf(a) === slot.code);
}

/** Controles do algoritmo na ordem do dicionário (knob, switch, combox). */
export function knobsOf(alg: FxAlgorithm): BoardKnob[] {
  const knob = (
    k: { name: string; pos: number; default: string | null; min: number | null; max: number | null },
  ): BoardKnob => ({
    name: k.name,
    pos: k.pos,
    kind: "knob",
    range: k.min != null && k.max != null ? [k.min, k.max] : undefined,
    options: [],
    value: k.default ?? undefined,
    default: k.default ?? undefined,
  });
  const choice = (k: { name: string; pos: number; options: string[]; default: string | null }): BoardKnob => ({
    name: k.name,
    pos: k.pos,
    kind: "switch",
    range: undefined,
    options: [...k.options],
    value: k.default ?? undefined,
    default: k.default ?? undefined,
  });
  return [
    ...alg.knobs.map(knob),
    ...alg.switches.map(choice),
    ...alg.comboxes.map((c) => ({ ...choice(c), kind: "combox" as const })),
  ];
}

/**
 * Slot com o MESMO efeito, outro algoritmo.
 *
 * Preserva posição na cadeia (`slot`), família, `state` (ligado/desligado) e o
 * `archetype` — o que muda é o efeito: nome, `code`, `variant` (o modelo do
 * pedal) e os controles, que voltam aos DEFAULTS do novo algoritmo: os valores
 * do efeito anterior não significam nada no novo, e chutar um valor seria a UI
 * mentindo sobre o que o som está usando.
 */
export function withAlgorithm(slot: BoardSlot, alg: FxAlgorithm): BoardSlot {
  if (codeOf(alg) === slot.code) return slot;
  return {
    ...slot,
    name: alg.name,
    code: codeOf(alg),
    variant: alg.variant,
    knobs: knobsOf(alg),
  };
}