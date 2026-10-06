/**
 * abLevel — o NÍVEL e a TROCA do A/B (issue #116).
 *
 * Este arquivo tranca as três promessas que o módulo faz e que a tela repete:
 *
 *  1. **O número é POSIÇÃO, e vem do board.** O teste recalcula a posição a
 *     partir do valor e da faixa do PRÓPRIO knob — se `nivel()` virar um
 *     número fixo ou passar a somar ganho, o cálculo diverge e o teste cai.
 *     Módulo desligado não conta (bypass não é nível), e "sem controle de
 *     saída" é `null` (a tela diz "não há o que calibrar", não inventa 0).
 *  2. **A calibração só olha o que os DOIS lados têm.** Um controle que só
 *     existe num lado não tem com o que ser igualado — ele sobra como
 *     residual, e é justamente por isso que `calibracao` o exclui.
 *  3. **A troca só manda o que é GRAVÁVEL.** knob (`kind === "knob"`) de slot
 *     cujo algoritmo é o mesmo nos dois lados; o `0x47` não tem formato
 *     capturado (BLOCKERS 10b), então slot com algoritmo trocado fica de fora
 *     — mandar `set_param` nele seria gravar num controle que não é mais
 *     aquele.
 *
 * Tudo puro: `BoardView` entra, valor sai. Nenhum IPC, nenhum React.
 */
import { describe, expect, it } from "vitest";
import { algoritmosTrocados, calibracao, deltaNivel, nivel, troca } from "../src/abLevel";
import type { Calibracao } from "../src/abLevel";
import type { BoardKnob, BoardSlot, BoardView } from "../src/ipc/types";
import { ARCHETYPE_OF } from "../src/ipc/types";

function knob(pos: number, nome: string, valor: string | undefined, extra: Partial<BoardKnob> = {}): BoardKnob {
  return { name: nome, pos, kind: "knob", range: [0, 100], options: [], value: valor, ...extra };
}

function slot(n: number, family: BoardSlot["family"], code: number, knobs: BoardKnob[], state = true): BoardSlot {
  return {
    slot: n,
    family,
    archetype: ARCHETYPE_OF[family],
    name: family,
    variant: family.toLowerCase(),
    state,
    code,
    knobs,
  };
}

function board(slots: BoardSlot[]): BoardView {
  return { pp: -1, name: "Teste", ppType: 0, ppTypeName: "", slots, bank: "user", ppLabel: "U01" };
}

/** AMP ligado com Volume=40 (saída) e Gain=20 (ganho, não soma nível). */
function ampCom(volume: string, state = true): BoardView {
  return board([slot(2, "AMP", 10, [knob(0, "Volume", volume), knob(1, "Gain", "20")], state)]);
}

describe("abLevel — o nível", () => {
  it("é a média das POSIÇÕES dos controles de saída, recalculada do próprio board", () => {
    // Volume 40/0..100 → 0.40 ; Master 80/0..100 → 0.80 → média 0.60 → 60.0
    const b = board([
      slot(2, "AMP", 10, [knob(0, "Volume", "40"), knob(1, "Gain", "100")]),
      slot(8, "RVB", 20, [knob(0, "Master", "80"), knob(1, "Mix", "0")]),
    ]);
    const saida = [40, 80];
    const esperado = Math.round((saida.reduce((a, x) => a + x / 100, 0) / saida.length) * 1000) / 10;
    expect(nivel(b)).toBe(esperado);
    expect(nivel(b)).toBe(60);
  });

  it("a posição vem do valor e da FAIXA do knob (faixa estreita não vira 0)", () => {
    // (60 − 20) / (100 − 20) = 0.5 → 50.0 — origem no dado, não em 40/100
    const b = board([slot(2, "AMP", 10, [knob(0, "Volume", "60", { range: [20, 100] })])]);
    expect(nivel(b)).toBe(50);
  });

  it("módulo DESLIGADO não conta (bypass não soma nível)", () => {
    const ligado = ampCom("40");
    const desligado = ampCom("40", false);
    expect(nivel(ligado)).toBe(40);
    expect(nivel(desligado)).toBeNull(); // o único controle de saída saiu de cena
  });

  it("sem controle de nível, o nível é null — e a diferença de um lado só também", () => {
    const semNivel = board([slot(2, "AMP", 10, [knob(0, "Gain", "50")])]); // só ganho
    expect(nivel(semNivel)).toBeNull();
    expect(deltaNivel(semNivel, ampCom("40"))).toBeNull();
    expect(deltaNivel(ampCom("40"), semNivel)).toBeNull();
  });

  it("a diferença é A − B em pontos, com 1 casa", () => {
    expect(deltaNivel(ampCom("70"), ampCom("40"))).toBe(30);
    expect(deltaNivel(ampCom("40"), ampCom("70"))).toBe(-30);
    expect(deltaNivel(ampCom("40"), ampCom("40"))).toBe(0);
    // 1 casa mesmo quando a média tem mais: 40.5 → 40.5, diferença 0
    const meio = board([slot(2, "AMP", 10, [knob(0, "Volume", "40"), knob(1, "Level", "41")])]);
    expect(nivel(meio)).toBe(40.5);
  });

  it("num preset real o nível existe e fica na faixa 0..100", async () => {
    const { deviceBoard } = await import("../src/ipc/device");
    const b = await deviceBoard(0);
    const n = nivel(b);
    expect(n).not.toBeNull();
    expect(n!).toBeGreaterThanOrEqual(0);
    expect(n!).toBeLessThanOrEqual(100);
    expect(deltaNivel(b, b)).toBe(0);
  });
});

describe("abLevel — a calibração", () => {
  it("só iguala o que os DOIS lados têm, e que está em valores DIFERENTES", () => {
    const alto = board([
      slot(2, "AMP", 10, [knob(0, "Volume", "70"), knob(1, "Gain", "50")]),
      slot(6, "MOD", 30, [knob(0, "Level", "90")]), // só este lado tem
    ]);
    const baixo = board([
      slot(2, "AMP", 10, [knob(0, "Volume", "40"), knob(1, "Gain", "50")]), // Gain igual
      slot(7, "DLY", 40, [knob(0, "Level", "10")]), // só o outro lado tem
    ]);
    const esperado: Calibracao[] = [{ slot: 2, pos: 0, knob: "Volume", a: "70", b: "40" }];
    expect(calibracao(alto, baixo)).toEqual(esperado);
  });

  it("a lista vem ordenada na ordem da cadeia (slot, depois posição)", () => {
    const a = board([
      slot(8, "RVB", 20, [knob(3, "Master", "90")]),
      slot(2, "AMP", 10, [knob(5, "Level", "80")]),
    ]);
    const b = board([
      slot(8, "RVB", 20, [knob(3, "Master", "30")]),
      slot(2, "AMP", 10, [knob(5, "Level", "20")]),
    ]);
    expect(calibracao(a, b).map((c) => [c.slot, c.pos])).toEqual([
      [2, 5],
      [8, 3],
    ]);
  });

  it("nada igualado = lista vazia (os dois lados já estavam no mesmo nível)", () => {
    expect(calibracao(ampCom("40"), ampCom("40"))).toEqual([]);
  });
});

describe("abLevel — algoritmo trocado", () => {
  it("acusa os slots cujo code MUDOU entre os lados", () => {
    const a = board([slot(2, "AMP", 10, []), slot(4, "CAB", 11, [])]);
    const b = board([slot(2, "AMP", 99, []), slot(4, "CAB", 11, [])]);
    expect(algoritmosTrocados(a, b)).toEqual([2]);
    expect(algoritmosTrocados(a, a)).toEqual([]);
  });
});

describe("abLevel — a troca (o que vai para o aparelho)", () => {
  const de = board([
    slot(2, "AMP", 10, [knob(0, "Volume", "70"), knob(1, "Mode", "A", { kind: "switch", range: undefined })]),
    slot(4, "CAB", 11, [knob(0, "Level", "50")]),
  ]);

  it("só knobs de MESMO algoritmo com valor diferente — switch é prévia local", () => {
    const para = board([
      slot(2, "AMP", 10, [knob(0, "Volume", "40"), knob(1, "Mode", "B", { kind: "switch", range: undefined })]),
      slot(4, "CAB", 11, [knob(0, "Level", "50")]), // igual → fora
    ]);
    expect(troca(de, para)).toEqual([{ slot: 2, code: 10, pos: 0, valor: "40" }]);
  });

  it("slot com algoritmo TROCADO não vai para o fio (0x47 sem formato capturado)", () => {
    const para = board([
      slot(2, "AMP", 99, [knob(0, "Volume", "40")]), // algoritmo mudou
      slot(4, "CAB", 11, [knob(0, "Level", "40")]), // mesmo algoritmo, valor mudou
    ]);
    expect(troca(de, para)).toEqual([{ slot: 4, code: 11, pos: 0, valor: "40" }]);
  });

  it("knob sem valor não manda (não dá pra gravar o vazio)", () => {
    const para = board([slot(2, "AMP", 10, [knob(0, "Volume", undefined)])]);
    expect(troca(de, para)).toEqual([]);
  });

  it("idempotente: do lado para ele mesmo, nada sai", () => {
    expect(troca(de, de)).toEqual([]);
  });
});
