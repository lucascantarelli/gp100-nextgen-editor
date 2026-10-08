/**
 * ipc/fallbackData — os DADOS do fallback de TESTE (vitest/jsdom, fora da
 * webview do Tauri): o board da cadeia, o inventário de 198 pps e o nome
 * fixture. São fixtures rotulados — NÃO representam aparelho nenhum e
 * existem para os asserts terem valor estável (issue #150).
 *
 * **(#150) o app real nunca lê isto:** na webview a resposta vem do
 * backend; sem aparelho é `backend: "none"` + motivo. O simulador fiel do
 * protocolo é o `MockDevice` do core (spec §13 e gates H1/H2) — este
 * arquivo é só a mesa de teste do front.
 *
 * Saiu de `ipc/device.ts` na #150 para a PORTA caber no teto de tamanho
 * (docs/ARCHITECTURE.md §4): a porta cresce por operação de device, e
 * fixture de teste não é operação de device.
 */
import { FX_MODULES } from "../artifacts/fxData";
import { PRESET_CHAINS } from "../artifacts/presetChains";
import { ARCHETYPE_OF, rotuloPp } from "./types";
import type { BoardSlot, BoardView, ChainFamily, PresetLibrary } from "./types";

/**
 * Fallback determinístico p/ TESTES (vitest/jsdom), espelhando o MockDevice
 * do core. NÃO é o caminho do app real: na webview a resposta vem sempre do
 * backend — sem aparelho é `backend: "none"` + motivo (issue #150). O nome
 * aqui é um LITERAL DE FIXTURE rotulado como tal: não representa aparelho
 * nenhum e existe para os asserts dos testes terem um valor estável.
 */
export const FIXTURE_NOME_TESTE = "Fixture de teste (não é aparelho)";

/**
 * Cadeia do fallback de DEV/TESTE — MESMA leitura do core Rust
 * (`pedalboard::board_view_for`): efeito, `effectCode` e `params_0..14`
 * REAIS do preset alvo, vindos de `presetChains.ts` (GERADO do all.prst).
 *
 * Antes desta fonte a cadeia era FIXA (COMP/Green OD/Bog RedM…): trocar de
 * preset mudava o nome do LED e os 9 pedais seguiam iguais — o preset não
 * aparecia no pedalboard. Agora cada preset abre a SUA cadeia, inclusive a
 * ordem real dos pedais (20 dos 99 têm a cadeia trocada: `@x` manda).
 *
 * O valor do knob vem de `params[pos]` (mesma ordem do dicionário) só quando
 * é plausível: dentro do range do controle, e nunca o sentinel 0xFFFF
 * (65535 = "não configurado") — fora disso cai no default do dicionário.
 * Regra do core para algoritmo fora do dicionário: pedal SEM knobs (nunca
 * adivinhar controle).
 */
export function localMockBoard(pp?: number): BoardView {
  // **A partir da #150 o fallback de teste não serve NOME do artefato:** o
  // nome é a fixture rotulada (como no info), e o tipo é neutro — o que os
  // testes travam aqui é a ESTRUTURA do board (slots/knobs da cadeia), não
  // dados de fábrica que não representam aparelho nenhum.
  const target = pp ?? 0;
  const chain = PRESET_CHAINS.find((c) => c.pp === target) ?? PRESET_CHAINS[0];
  const preset = { pp: target, name: FIXTURE_NOME_TESTE, ppType: 0, ppTypeName: "" };

  const algFor = (family: ChainFamily, code: number, name: string) =>
    FX_MODULES[family]?.find((a) => ((a.nibble << 24) | a.index) === code) ??
    FX_MODULES[family]?.find((a) => a.name === name);

  /** valor cru do preset → valor de knob (ou undefined = usa o default) */
  const realValue = (
    raw: string | null,
    range: [number, number] | undefined,
    options: string[],
  ): string | undefined => {
    if (raw == null) return undefined;
    const n = Number(raw);
    if (!Number.isFinite(n)) return undefined;
    if (options.length > 0) {
      // switch/combox: índice da lista (o que o dicionário usa como default)
      return Number.isInteger(n) && n >= 0 && n < options.length ? String(n) : undefined;
    }
    if (n === 65535) return undefined; // sentinel 0xFFFF = não configurado
    if (range && (n < Math.min(...range) || n > Math.max(...range))) return undefined;
    return String(n);
  };

  const slots: BoardSlot[] = chain.slots.map((s) => {
    const alg = algFor(s.family, s.code, s.name);
    const knob = (
      k: { name: string; pos: number; default?: string | null; min?: number | null; max?: number | null },
      kind: "knob" | "switch" | "combox",
      options: string[] = [],
    ) => {
      const range =
        kind === "knob" && k.min != null && k.max != null ? ([k.min, k.max] as [number, number]) : undefined;
      const dflt = k.default ?? undefined;
      return {
        name: k.name,
        pos: k.pos,
        kind,
        range,
        options,
        value: realValue(s.params[k.pos], range, options) ?? dflt,
        default: dflt,
      };
    };

    return {
      slot: s.slot,
      family: s.family,
      archetype: ARCHETYPE_OF[s.family],
      name: s.name,
      // algoritmo fora do dicionário: o pedal renderiza sem knobs (regra R1)
      variant: alg?.variant ?? "generic",
      state: s.state,
      code: s.code,
      knobs: alg
        ? [
            ...alg.knobs.map((k) => knob(k, "knob")),
            ...alg.switches.map((k) => knob(k, "switch", k.options)),
            ...alg.comboxes.map((k) => knob(k, "combox", k.options)),
          ]
        : [],
    };
  });

  return {
    pp: preset.pp,
    name: preset.name,
    ppType: preset.ppType,
    ppTypeName: preset.ppTypeName,
    slots,
    bank: "factory" as const,
    // pp de APARELHO tem endereço 0x0100+: rótulo é o endereço (ADR-12 —
    // o mapeamento LED P/F é para MEDIR em campo), não um "P42" chutado.
    ppLabel: preset.pp >= 0x0100 ? rotuloPp(preset.pp) : `P${String(preset.pp + 1).padStart(2, "0")}`,
  };
}

/**
 * Biblioteca do fallback de TESTE: o INVENTÁRIO com nome vazio — a MESMA
 * resposta do aparelho real (issue #150: nome do slot é trabalho do decode
 * #152, não do artefato de UI). Os 198 pps espelham o espaço medido
 * (`0x0000..0x0062` + `0x0100..0x0162`, ADR-12).
 */
export function localMockLibrary(): PresetLibrary {
  const pps: number[] = [];
  for (let pp = 0x0100; pp <= 0x0162; pp += 1) pps.push(pp);
  for (let pp = 0x0000; pp <= 0x0062; pp += 1) pps.push(pp);
  return {
    entries: pps.map((pp) => ({ pp, name: "", ppTypeName: "" })),
    currentPp: 0,
  };
}
