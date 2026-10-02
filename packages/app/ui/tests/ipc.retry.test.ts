/**
 * ipc/device — retry/backoff e timeout de command (issue #20).
 *
 * A política é exercitada pelo GANCHO `gp100.debug.failDevice` em modo
 * transitório (`op:n`): n falhas reais do command, depois sucesso. Os
 * asserts usam a própria chave do gancho como contador de tentativas — o
 * que sobrou nela PROVA quantas tentativas o command consumiu.
 * O timeout é provado no branch Tauri (invoke que nunca responde).
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  CommandTimeoutError,
  COMMAND_ATTEMPTS,
  COMMAND_BASE_MS,
  COMMAND_TIMEOUT_MS,
  deviceInfo,
  deviceSelectPreset,
  deviceSetParam,
  retryDelayMs,
} from "../src/ipc/device";

const KEY = "gp100.debug.failDevice";

/** invoke que NUNCA responde (branch Tauri do ipc/device). */
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => new Promise(() => {})),
}));

beforeEach(() => {
  localStorage.clear();
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
});

afterEach(() => {
  localStorage.clear();
  vi.useRealTimers();
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
});

describe("ipc/device — backoff exponencial com jitter", () => {
  it("cresce a cada tentativa e o jitter fica em ±30% da base", () => {
    // rand determinístico: 0 = jitter mínimo, 0.5 = sem jitter, 1 = máximo
    expect(retryDelayMs(1, () => 0.5)).toBe(COMMAND_BASE_MS);
    expect(retryDelayMs(1, () => 0)).toBe(Math.round(COMMAND_BASE_MS * 0.7));
    expect(retryDelayMs(1, () => 1)).toBe(Math.round(COMMAND_BASE_MS * 1.3));
    expect(retryDelayMs(2, () => 0.5)).toBe(COMMAND_BASE_MS * 2);
    expect(retryDelayMs(3, () => 0.5)).toBe(COMMAND_BASE_MS * 4);
    // nunca negativo (jitter não pode zerar o atraso)
    expect(retryDelayMs(1, () => 0)).toBeGreaterThan(0);
  });
});

describe("ipc/device — retry de falha transitória", () => {
  it("falha TRANSITÓRIA (info:1) não chega ao usuário: resolve na 2ª tentativa", async () => {
    localStorage.setItem(KEY, "info:1");
    await expect(deviceInfo()).resolves.toMatchObject({ backend: "mock" });
    expect(localStorage.getItem(KEY)).toBeNull(); // 1 falha consumida
  });

  it("consome até o limite: info:2 resolve e esvazia o plano", async () => {
    localStorage.setItem(KEY, "info:2");
    await expect(deviceInfo()).resolves.toMatchObject({ presetCount: 99 });
    expect(localStorage.getItem(KEY)).toBeNull();
  });

  it(`no máximo ${COMMAND_ATTEMPTS} tentativas: ${COMMAND_ATTEMPTS - 1} falhas passam; uma a mais rejeita`, async () => {
    // o gancho falha as n PRÓXIMAS chamadas: com 2 falhas, a 3ª tentativa resolve
    localStorage.setItem(KEY, `info:${COMMAND_ATTEMPTS - 1}`);
    await expect(deviceInfo()).resolves.toMatchObject({ backend: "mock" });
    expect(localStorage.getItem(KEY)).toBeNull();

    // com falhas ALÉM do limite, TODAS as tentativas são consumidas e o erro sobe
    // (o que sobra na chave prova o nº de tentativas: 5 − 3 = 2)
    localStorage.setItem(KEY, `info:${COMMAND_ATTEMPTS + 2}`);
    await expect(deviceInfo()).rejects.toThrow("falha simulada");
    expect(localStorage.getItem(KEY)).toBe("info:2");
  });

  it("plano zerado (info:0) não falha nada — e o retry fica dormente", async () => {
    localStorage.setItem(KEY, "info:0");
    await expect(deviceInfo()).resolves.toMatchObject({ backend: "mock" });
  });

  it("select e set_param passam pelo MESMO retry (escrita idempotente)", async () => {
    localStorage.setItem(KEY, "select:1");
    await expect(deviceSelectPreset(24)).resolves.toBeUndefined();
    localStorage.setItem(KEY, "set_param:2");
    await expect(deviceSetParam(0, 0x0700006e, 2, 50)).resolves.toBeUndefined();
    expect(localStorage.getItem(KEY)).toBeNull();
  });

  it("falha PERMANENTE de write sobe como erro (o App mostra o banner com retry)", async () => {
    localStorage.setItem(KEY, "select");
    await expect(deviceSelectPreset(7)).rejects.toThrow("falha simulada de device em select");
  });
});

describe("ipc/device — timeout por tentativa (branch Tauri)", () => {
  it("command que nunca responde rejeita (nunca spinner eterno) e respeita o nº de tentativas", async () => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    vi.useFakeTimers();

    const { invoke } = await import("@tauri-apps/api/core");
    const pending = deviceInfo();
    const rejection = expect(pending).rejects.toBeInstanceOf(CommandTimeoutError);

    // tempo total = 3 timeouts + 2 backoffs (120/240 ms)
    await vi.advanceTimersByTimeAsync(COMMAND_TIMEOUT_MS * COMMAND_ATTEMPTS + 1_000);
    await rejection;
    expect(vi.mocked(invoke)).toHaveBeenCalledTimes(COMMAND_ATTEMPTS);
  });
});
