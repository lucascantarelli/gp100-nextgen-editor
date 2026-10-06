/**
 * Gain staging — a regra do assistente e o fallback (issue #115).
 *
 * Este arquivo prova a MESMA coisa que a suíte Rust
 * (`packages/core/tests/gain_staging.rs`) prova contra o core, só que do lado
 * de cá da porta: a cópia em TS que faz a tela existir no navegador. Duas
 * cópias da mesma regra só valem se as duas concordarem — este é o contrato.
 *
 * O que ele tranca, e por quê:
 *
 *  1. **Cada número tem ORIGEM.** A DoD diz "a origem de cada número — nada de
 *     número mágico". O teste não confere um valor fixo: ele RECALCULA a
 *     posição a partir do valor e da faixa que vêm do próprio board e compara
 *     com o que o relatório publicou.
 *  2. **O que fica de fora está declarado.** Módulo desligado não conta, mix
 *     não soma nível, banda de EQ não é nível, algoritmo fora do dicionário é
 *     LISTADO em vez de virar silêncio.
 *  3. **Um preset armado para clipar é acusado, com a ordem de ajuste.** O
 *     risco é regra declarada (threshold de posição), então o caso do extremo é
 *     construído à mão a partir de um preset real.
 *  4. **O método viaja no relatório.** A limitação não pode ficar só no doc: ela
 *     é campo do `metodo` precisamente para a tela poder mostrá-la.
 *
 * O ppIDs usados são reais do `all.prst` embutido e a lista NÃO é contígua
 * (0x0a não existe) — por isso os números são explícitos, não `0..n`.
 */
import { describe, expect, it } from "vitest";
import { deviceBoard } from "../src/ipc/device";
import { analisa, presetGainReport } from "../src/ipc/gain";
import type { Leitura, Metodo, Modulo, Papel, Relatorio, Risco, Sugestao } from "../src/ipc/gain";
import type { BoardView } from "../src/ipc/types";

/** ppIDs reais do all.prst (mesmos da suíte Rust). */
const PPS = [0x00, 0x18, 0x19, 0x30];

/** Cópia mutável do board de um preset (o "preset armado" dos testes). */
async function boardDe(pp: number): Promise<BoardView> {
  return structuredClone(await deviceBoard(pp));
}

/** Muda o valor de um controle do board (o "preset armado"). */
function mexe(b: BoardView, slot: number, knob: string, valor: string): void {
  const s = b.slots.find((x) => x.slot === slot)!;
  const k = s.knobs.find((x) => x.name === knob)!;
  k.value = valor;
}

describe("gain staging — a leitura da cadeia", () => {
  it("lê a cadeia inteira na ORDEM DO SINAL (o mesmo desenho do palco)", async () => {
    const r: Relatorio = analisa(await deviceBoard(0));
    expect(r.modulos).toHaveLength(9);
    expect(r.modulos.map((m: Modulo) => m.slot)).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8]);
    expect(r.modulos.map((m) => m.familia)).toEqual([
      "PRE",
      "DST",
      "AMP",
      "NR",
      "CAB",
      "EQ",
      "MOD",
      "DLY",
      "RVB",
    ]);
    expect(r.pp).toBe(0);
  });

  it("cada número vem do valor e da faixa do PRÓPRIO board (nada de mágico)", async () => {
    for (const pp of PPS) {
      const b = await deviceBoard(pp);
      const r = analisa(b);
      let conferidos = 0;
      for (const m of r.modulos) {
        const s = b.slots.find((x) => x.slot === m.slot)!;
        for (const l of m.leituras) {
          const k = s.knobs.find((x) => x.pos === l.pos)!;
          expect(k.name).toBe(l.knob);
          expect(k.value).toBe(l.valor);
          const [lo, hi] = l.faixa;
          expect(l.faixa).toEqual(k.range);
          const esperada = Math.min(1, Math.max(0, (Number(l.valor) - lo) / (hi - lo)));
          expect(Math.abs(l.posicao - esperada), `${l.knob} de ${m.nome}`).toBeLessThan(1e-9);
          expect(Math.abs(l.folga - (1 - l.posicao))).toBeLessThan(1e-9);
          conferidos += 1;
        }
      }
      expect(conferidos, `pp ${pp} tem controles de nível`).toBeGreaterThanOrEqual(4);
    }
  });

  it("o método e a LIMITAÇÃO viajam no relatório (para a tela mostrar)", async () => {
    const r = analisa(await deviceBoard(0));
    const metodo: Metodo = r.metodo;
    expect(metodo.nomesDeGanho).toEqual(["Gain"]);
    expect(metodo.nomesDeSaida).toContain("Volume");
    expect(metodo.nomesDeSaida).toContain("Master");
    expect(metodo.nomesDeMix).toContain("Mix");
    expect(metodo.limiarTeto).toBeCloseTo(0.95);
    expect(metodo.ordem.length).toBeGreaterThan(20);
    // o modo de falha da feature é o número parecer medição — a limitação diz
    // que não é, e ela é CAMPO do relatório, não prosa do README.
    expect(metodo.limitacao).toMatch(/POSIÇÃO/);
    expect(metodo.limitacao).toMatch(/não é nível de sinal medido/);
  });
});

describe("gain staging — o que fica de fora, declarado", () => {
  it("módulo DESLIGADO aparece mas não conta (bypass não soma nível)", async () => {
    const b = await deviceBoard(0);
    expect(b.slots.find((s) => s.slot === 0)!.state).toBe(false);
    const r = analisa(b);
    const pre = r.modulos.find((m) => m.slot === 0)!;
    expect(pre.ligado).toBe(false);
    expect(pre.leituras.some((l: Leitura) => l.knob === "VOL")).toBe(true);
    expect(pre.folga).toBeNull();
    expect(pre.noTeto).toBe(false);
    expect(r.ajuste.some((s) => s.slot === 0)).toBe(false);
  });

  it("mix é MISTURA: no teto não põe o módulo no teto nem vira sugestão", async () => {
    const b = await boardDe(0);
    mexe(b, 7, "Mix", "99");
    const r = analisa(b);
    const dly = r.modulos.find((m) => m.slot === 7)!;
    const mix = dly.leituras.find((l) => l.knob === "Mix")!;
    const papel: Papel = mix.papel;
    expect(papel).toBe("mix");
    expect(dly.noTeto).toBe(false);
    expect(r.ajuste).toEqual([]);
    expect(r.risco).toBe("baixo");
  });

  it("ganho de BANDA do EQ não é nível de estágio", async () => {
    const b = await boardDe(0);
    mexe(b, 5, "6.6kHz", "50");
    const r = analisa(b);
    const eq = r.modulos.find((m) => m.slot === 5)!;
    expect(eq.leituras).toEqual([]);
    expect(eq.folga).toBeNull();
    expect(r.risco).toBe("baixo");
  });

  it("algoritmo fora do dicionário é LISTADO (não sei é resposta)", async () => {
    const b = await boardDe(0);
    b.slots.find((s) => s.slot === 4)!.knobs = [];
    const r = analisa(b);
    expect(r.foraDoDicionario).toEqual([4]);
    const m = r.modulos.find((x) => x.slot === 4)!;
    expect(m.leituras).toEqual([]);
    expect(m.folga).toBeNull();
  });

  it("valor ilegível ou faixa degenerada ficam DECLARADOS em `ignorados`", async () => {
    const b = await boardDe(0);
    mexe(b, 2, "Gain", "abc");
    const amp = analisa(b).modulos.find((m) => m.slot === 2)!;
    expect(amp.ignorados).toContain("Gain");
    expect(amp.leituras.some((l) => l.knob === "Gain")).toBe(false);

    const b2 = await boardDe(0);
    b2.slots.find((s) => s.slot === 2)!.knobs.find((k) => k.name === "Gain")!.range = [50, 50];
    const amp2 = analisa(b2).modulos.find((m) => m.slot === 2)!;
    expect(amp2.ignorados).toContain("Gain");
  });
});

describe("gain staging — o risco, que é regra declarada", () => {
  it("preset ARMADO para clipar acusa o teto e a ORDEM de ajuste", async () => {
    const b = await boardDe(0);
    mexe(b, 2, "Gain", "99"); // AMP: ganho de entrada no fim da faixa
    mexe(b, 4, "Volume", "99"); // CAB: nível de saída no fim da faixa
    const r = analisa(b);
    const alto: Risco = "alto";
    expect(r.risco).toBe(alto);
    expect(r.modulos.find((m) => m.slot === 2)!.noTeto).toBe(true);
    expect(r.modulos.find((m) => m.slot === 4)!.noTeto).toBe(true);
    expect(r.modulos.find((m) => m.slot === 2)!.folga).toBe(0);
    expect(r.folgaMinima?.[0]).toBe(2);
    const ajuste: Sugestao[] = r.ajuste;
    // saída primeiro (fim da cadeia), depois o ganho (começo) — a razão está
    // escrita no método.
    expect(ajuste.map((s) => [s.slot, s.knob])).toEqual([
      [4, "Volume"],
      [2, "Gain"],
    ]);
    expect(ajuste[0].valor).toBe("99");
    expect(ajuste[0].papel).toBe("saida");
  });

  it("AMP no teto com CAB ligado atrás já é risco ALTO (gain alto + IR)", async () => {
    const b = await boardDe(0);
    mexe(b, 2, "Gain", "99");
    const r = analisa(b);
    expect(r.risco).toBe("alto");
    expect(r.ajuste).toHaveLength(1);
    expect(r.ajuste[0].slot).toBe(2);
  });

  it("um controle no teto, sem o par AMP+CAB, é risco MÉDIO", async () => {
    const b = await boardDe(0);
    mexe(b, 4, "Volume", "99");
    const r = analisa(b);
    expect(r.risco).toBe("medio");
    expect(r.folgaMinima?.[0]).toBe(4);
    expect(r.ajuste).toHaveLength(1);
  });

  it("o preset de fábrica do palco NÃO é acusado", async () => {
    const r = analisa(await deviceBoard(0));
    expect(r.risco).toBe("baixo");
    expect(r.ajuste).toEqual([]);
    expect(r.foraDoDicionario).toEqual([]);
    const fm = r.folgaMinima!;
    expect(fm[0]).toBe(4); // o CAB Volume é o mais perto do teto
    expect(fm[1]).toBeGreaterThan(0.2);
  });
});

describe("gain staging — a porta", () => {
  it("presetGainReport devolve o mesmo relatório que a regra do fallback", async () => {
    expect(await presetGainReport(0)).toEqual(analisa(await deviceBoard(0)));
  });
});
