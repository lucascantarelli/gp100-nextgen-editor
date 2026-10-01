/**
 * Gancho de teste `gp100.debug.failDevice` (src/ipc/device.ts): com a
 * chave setada, o fallback de device_info REJEITA antes de responder — é o
 * mesmo ganho que os cenários e2e de error state usam (e2e/estados.spec.ts).
 * Sem a chave, custo zero (fallback mock normal).
 */
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { deviceInfo } from "../src/ipc/device";

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
});
