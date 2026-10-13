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
  BOOT_TIMEOUT_MS,
  CommandTimeoutError,
  COMMAND_ATTEMPTS,
  COMMAND_BASE_MS,
  COMMAND_TIMEOUT_MS,
  deviceBoot,
  deviceInfo,
  deviceSelectPreset,
  deviceSetParam,
  IDEMPOTENTE_ESPERA_BOOT,
  retryDelayMs,
  runCommand,
  semRetry,
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

/* ── Prazo declarado na política (o boot de campo leva ~25 s) ──────────────
 * Regressão do campo de 12/10: o `device_boot` ia pelo default de 8 s e
 * rejeitava no MEIO do scan — a UI mostrava "Falha no boot do device" com o
 * aparelho sendo lido certinho (wire log: scan terminou aos 30,3 s e o
 * relatório saiu 2297/198/198, dentro do gate #161).
 */
describe("ipc/device — prazo declarado na política", () => {
  it("quem declara prazo passa do default de 8 s e só cai no SEU", async () => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    vi.useFakeTimers();

    let erro: unknown = null;
    const pendente = runCommand(
      "lento",
      semRetry("prazo maior que o default: o command e lento de proposito", 60_000),
      () => new Promise(() => {}),
    ).catch((e: unknown) => {
      erro = e;
    });

    // Além do default de 8 s o command segue de pé — é exatamente o que o
    // boot precisa; sem o prazo declarado ele teria morrido aqui.
    await vi.advanceTimersByTimeAsync(COMMAND_TIMEOUT_MS + 500);
    expect(erro, "o default de 8 s nao pode derrubar um prazo declarado").toBeNull();

    await vi.advanceTimersByTimeAsync(60_000);
    await pendente;
    expect(erro, "no prazo declarado ele rejeita (nunca spinner eterno)").toBeInstanceOf(
      CommandTimeoutError,
    );
  });

  it("o BOOT sobrevive aos ~25 s medidos em campo em vez de rejeitar aos 8 s", async () => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    vi.useFakeTimers();
    expect(BOOT_TIMEOUT_MS, "o prazo tem de cobrir os 25 s medidos").toBeGreaterThan(25_000);

    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockClear();
    const relatorio = { transactions: 2297, presets: 198, names: 198 };
    vi.mocked(invoke).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          setTimeout(() => resolve(relatorio), 25_000);
        }) as Promise<unknown>,
    );

    const pendente = deviceBoot();
    await vi.advanceTimersByTimeAsync(0);
    expect(vi.mocked(invoke), "o boot foi disparado UMA vez (sem retry)").toHaveBeenCalledTimes(1);

    // 25 s de aparelho lendo + margem: com o default de 8 s esta promise já
    // teria rejeitado com CommandTimeoutError e a UI mostraria o banner.
    await vi.advanceTimersByTimeAsync(25_000 + 100);
    await expect(pendente).resolves.toEqual(relatorio);
  });

  it("quem disputa a FILA com o boot sobrevive aos 8 s E aos 25 s do scan", async () => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    vi.useFakeTimers();

    let erro: unknown = null;
    const pendente = runCommand(
      "library",
      IDEMPOTENTE_ESPERA_BOOT,
      () => new Promise(() => {}),
    ).catch((e: unknown) => {
      erro = e;
    });

    // Além do default de 8 s (foi o que derrubou a biblioteca em campo).
    await vi.advanceTimersByTimeAsync(COMMAND_TIMEOUT_MS + 500);
    expect(erro, "o default de 8 s nao pode derrubar a espera do boot").toBeNull();

    // E além dos ~25 s do scan: a fila só libera quando o boot termina.
    await vi.advanceTimersByTimeAsync(25_000);
    expect(erro, "o boot de 25 s ainda esta dentro do prazo declarado").toBeNull();

    // No SEU prazo ele cai mesmo — o "nunca spinner eterno" continua valendo.
    await vi.advanceTimersByTimeAsync(BOOT_TIMEOUT_MS * COMMAND_ATTEMPTS + 2_000);
    await pendente;
    expect(erro).toBeInstanceOf(CommandTimeoutError);
  });
});
