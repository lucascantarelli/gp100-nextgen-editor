/**
 * ipc/device — o fallback de TESTE (mesmo shape do MockDevice real).
 * O Tauri não existe no jsdom (sem __TAURI_INTERNALS__), então É O FALLBACK
 * que roda em unit/e2e. **(#150) o fallback não serve dado de fábrica:** a
 * biblioteca é o INVENTÁRIO (198 slots, ADR-12) com nome vazio — a mesma
 * forma do aparelho real — e o nome do info/board é a fixture rotulada.
 * Aqui o device_boot sai do ponto cego: lotes de 64 beats (main thread
 * respira), estágios na ordem do script real, ciclo do currentPp e unlisten
 * dos eventos. Falha simulada por operação fecha o triângulo info/boot/board.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  deviceBoard,
  deviceBoot,
  deviceConectar,
  deviceInfo,
  devicePresetLibrary,
  deviceSelectPreset,
  deviceSetParam,
  onBootProgress,
  onDevicePush,
} from "../src/ipc/device";
import { FX_MODULES } from "../src/artifacts/fxData";
import { PRESET_COUNT } from "../src/i18n/facts";

const KEY = "gp100.debug.failDevice";

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  localStorage.clear();
});

describe("ipc/device — info e biblioteca (fallback de teste)", () => {
  it("deviceInfo é o shape do backend com fixture rotulada (nunca nome de fábrica)", async () => {
    const info = await deviceInfo();
    expect(info).toMatchObject({
      backend: "mock",
      detail: "",
      presetCount: 99,
      currentPp: 0,
      irSlotsWithCrc: 20,
    });
    expect(info.currentName).toContain("não é aparelho");
  });

  it("devicePresetLibrary é o INVENTÁRIO do aparelho: 198 slots, nome vazio (#150)", async () => {
    const lib = await devicePresetLibrary();
    expect(lib.entries.length).toBe(198);
    expect(lib.currentPp).toBe(0);
    // A ordem é a da captura S1: banco 0x0100 primeiro (ADR-12).
    expect(lib.entries[0]).toMatchObject({ pp: 0x0100, name: "" });
    expect(lib.entries[99]).toMatchObject({ pp: 0x0000, name: "" });
    expect(lib.entries.at(-1)).toMatchObject({ pp: 0x0062, name: "" });
  });

  it("gancho de falha é POR OPERAÇÃO: info/boot/board rejeitam, library não tem gancho", async () => {
    localStorage.setItem(KEY, "info");
    await expect(deviceInfo()).rejects.toThrow("falha simulada de device em info");

    localStorage.setItem(KEY, "boot");
    await expect(deviceBoot()).rejects.toThrow("falha simulada de device em boot");

    localStorage.setItem(KEY, "board");
    await expect(deviceBoard()).rejects.toThrow("falha simulada de device em board");

    // library não passa pelo debugFail — resolve mesmo com "all"
    localStorage.setItem(KEY, "all");
    await expect(devicePresetLibrary()).resolves.toBeTruthy();
  });
});

describe("ipc/device — boot em lotes (a main thread nunca congela)", () => {
  it("emite os 2297 beats, estágios na ordem do script e ciclo do currentPp", async () => {
    const beats: Array<{ stage: string; done: number; total: number; currentPp: number }> = [];
    const un = await onBootProgress((p) => beats.push(p));

    const setTimeoutSpy = vi.spyOn(window, "setTimeout");
    const report = await deviceBoot();

    expect(report).toEqual({
      transactions: 2297,
      presets: PRESET_COUNT * 2,
      names: PRESET_COUNT * 2,
    });
    expect(beats.length).toBe(2297);
    // done é incrementado ANTES do emit: o beat 1 já anuncia currentPp 1
    expect(beats[0]).toMatchObject({ stage: "tables", done: 1, total: 2297, currentPp: 1 });
    expect(beats.at(-1)).toMatchObject({ stage: "keepalive", done: 2297 });
    // ciclo do pp: done 197 (índice 196) → min(197 % 198, 197) = 197;
    // done 198 (índice 197) já recomeça o ciclo em 0
    expect(beats[196].currentPp).toBe(197);
    expect(beats[197].currentPp).toBe(0);
    // CAP DE LOTE: 2297/64 = 36 ticks → ≥35 re-agendamentos por setTimeout.
    // Sem o cap (1 tick síncrono) a suíte congelava ~300 ms — regressão
    // travada aqui na unidade, não só no "sentir" do app.
    expect(setTimeoutSpy.mock.calls.length).toBeGreaterThanOrEqual(35);
    setTimeoutSpy.mockRestore();
    un();
  });

  it("unlisten remove o listener: boot seguinte não emite nada para ele", async () => {
    const hits: number[] = [];
    const un = await onBootProgress((p) => hits.push(p.done));
    un();
    await deviceBoot();
    expect(hits.length).toBe(0);
  });
});

describe("ipc/device — board (estrutura do fallback de teste → BoardView)", () => {
  it("deviceBoard() sem pp usa o corrente (pp 0): 9 slots na ordem da cadeia", async () => {
    const b = await deviceBoard();
    expect(b.pp).toBe(0);
    expect(b.name).toContain("não é aparelho");
    expect(b.ppLabel).toBe("P01");
    expect(b.bank).toBe("factory");
    expect(b.slots.map((s) => s.family)).toEqual([
      "PRE", "DST", "AMP", "NR", "CAB", "EQ", "MOD", "DLY", "RVB",
    ]);
  });

  /* 20 dos 99 presets têm a cadeia TROCADA (`@x` manda, não a família) —
     P06 (ppID 5) é o caso mais visível: o DST vem antes do PRE. */
  it("presets com cadeia trocada trocam os pedais de posição (não só o nome)", async () => {
    const b = await deviceBoard(5);
    expect(b.slots.map((s) => s.family).slice(0, 2)).toEqual(["DST", "PRE"]);
    // e o exibido pelo palco é o slot REAL, não a ordem canônica das famílias
    expect(b.slots[0].name).not.toBe(b.slots[1].name);
  });

  it("slots carregam knobs/switches/comboxes DO CATÁLIGO com code derivado", async () => {
    const b = await deviceBoard();
    for (const slot of b.slots) {
      expect(slot.knobs.length, slot.family).toBeGreaterThan(0);
      for (const k of slot.knobs) {
        expect(["knob", "switch", "combox"], `${slot.family}/${k.name}`).toContain(k.kind);
        expect(k.name.length).toBeGreaterThan(0);
      }
    }
    // DERIVAÇÃO do code a partir do artefato: nibble do módulo no byte alto
    // + index do algoritmo — provado contra o catálogo para o PRE do P01
    // (que no all.prst é C-Wah, não COMP: o board segue a cadeia REAL)
    const pre = b.slots[0];
    const alg = FX_MODULES.PRE.find((a) => ((a.nibble << 24) | a.index) === pre.code);
    expect(alg?.name, "o code do PRE bate com um algoritmo do dicionário").toBeTruthy();
    expect(pre.name).toBe(alg!.name);
    // PRE/C-Wah: o 1º controle é knob com range real do dicionário
    expect(pre.knobs[0].kind).toBe("knob");
  });

  it("deviceBoard(pp) reflete o pp pedido — nada de pp congelado", async () => {
    const b = await deviceBoard(24);
    expect(b.pp).toBe(24);
  });

  it("pp do banco do aparelho (0x0100+) tem o ENDEREÇO como rótulo (#150)", async () => {
    const b = await deviceBoard(0x0100);
    expect(b.ppLabel).toBe("0x0100");
  });

  it("deviceConectar existe e resolve no fallback (o retry do aviso é testável)", async () => {
    await expect(deviceConectar()).resolves.toBeUndefined();
  });
});

describe("ipc/device — comandos e evento de push", () => {
  it("deviceSelectPreset e deviceSetParam resolvem no fallback (prévia local)", async () => {
    await expect(deviceSelectPreset(24)).resolves.toBeUndefined();
    await expect(deviceSetParam(0, 0x0700006e, 2, 50)).resolves.toBeUndefined();
  });

  it("onDevicePush assina e o unlisten devolvido desinscreve", async () => {
    const hexes: string[] = [];
    const un = await onDevicePush((hex) => hexes.push(hex));
    expect(typeof un).toBe("function");
    un();
    // sem producer no fallback: assinatura/desinscrição é o contrato testável
    expect(hexes.length).toBe(0);
  });
});
