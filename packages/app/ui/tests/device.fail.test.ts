/**
 * Gancho de teste `gp100.debug.failDevice` (src/ipc/device.ts): com a
 * chave setada, o fallback de device REJEITA antes de responder — é o
 * mesmo ganho que os cenários e2e de error state usam (e2e/estados.spec.ts
 * e e2e/ipc.edge.spec.ts). Sem a chave, custo zero (fallback mock normal).
 *
 * Valores do gancho (issue #20):
 *   - "<op>" (info|boot|board|select|set_param) ou "all" — falha SEMPRE;
 *   - "<op>:<n>" — falha transitória (retry/backoff cobre; ver ipc.retry);
 *   - "boot-mid" — progresso até ~40% e então REJEITA (disconnect mid-boot).
 */
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { PRESET_COUNT } from "../src/i18n/facts";
import {
  deviceBoard,
  deviceBoot,
  deviceInfo,
  deviceSelectPreset,
  deviceSetParam,
  onBootProgress,
} from "../src/ipc/device";
import type { BootProgress } from "../src/ipc/types";

const KEY = "gp100.debug.failDevice";

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  localStorage.clear();
});

describe("ipc/device — gancho de falha simulada", () => {
  it("sem a chave: device_info resolve com o fallback mock normal", async () => {
    await expect(deviceInfo()).resolves.toMatchObject({ backend: "mock", presetCount: 99 });
  });

  it("com failDevice=info: device_info rejeita com erro simulado", async () => {
    localStorage.setItem(KEY, "info");
    await expect(deviceInfo()).rejects.toThrow("debug: falha simulada de device em info");
  });

  it("com failDevice=all: qualquer operação rejeita", async () => {
    localStorage.setItem(KEY, "all");
    await expect(deviceInfo()).rejects.toThrow("debug: falha simulada");
  });

  it("valor de outra operação não interfere (ganho é por operação)", async () => {
    localStorage.setItem(KEY, "boot");
    await expect(deviceInfo()).resolves.toMatchObject({ backend: "mock" });
  });

  it("select e set_param têm gancho próprio (eram void-infallible)", async () => {
    localStorage.setItem(KEY, "select");
    await expect(deviceSelectPreset(3)).rejects.toThrow("falha simulada de device em select");
    // set_param NÃO é afetado pelo gancho do select
    await expect(deviceSetParam(0, 0x0700006e, 2, 50)).resolves.toBeUndefined();

    localStorage.setItem(KEY, "set_param");
    await expect(deviceSetParam(0, 0x0700006e, 2, 50)).rejects.toThrow(
      "falha simulada de device em set_param",
    );
    // board NÃO é afetado pelo gancho do set_param
    await expect(deviceBoard()).resolves.toMatchObject({ pp: 0 });
  });

  it("falha transitória (op:1) esvazia a chave depois de consumida", async () => {
    localStorage.setItem(KEY, "board:1");
    await expect(deviceBoard()).resolves.toMatchObject({ pp: 0 }); // 2ª tentativa
    expect(localStorage.getItem(KEY)).toBeNull();
    await expect(deviceBoard()).resolves.toBeTruthy(); // retry dormente
  });
});

describe("ipc/device — disconnect mid-boot (boot-mid)", () => {
  it("emite progresso real e só então rejeita (a UI tem o que mostrar e some o spinner)", async () => {
    localStorage.setItem(KEY, "boot-mid");
    const beats: BootProgress[] = [];
    const un = await onBootProgress((p) => {
      beats.push(p);
    });

    await expect(deviceBoot()).rejects.toThrow("device desconectado durante o boot");
    un();

    // progresso PARCIAL (device caiu no meio: não é um boot que nunca começou)
    expect(beats.length).toBeGreaterThan(0);
    expect(beats.length).toBeLessThan(2297);
    expect(beats.at(0)!.stage).toBe("tables");
  });

  it("boot normal segue completo quando a chave é outro valor", async () => {
    localStorage.setItem(KEY, "board");
    await expect(deviceBoot()).resolves.toEqual({
      transactions: 2297,
      presets: PRESET_COUNT * 2,
      names: PRESET_COUNT * 2,
    });
  });
});
