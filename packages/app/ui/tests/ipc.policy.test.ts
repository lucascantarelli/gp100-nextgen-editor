/**
 * Política de execução dos commands (#82) — a exceção é DECLARADA, não herdada.
 *
 * Antes, `withRetry` tinha o nº de tentativas no corpo: toda chamada nova
 * nascia repetindo sem ninguém decidir, e o boot (a única operação que NÃO
 * pode repetir) era a simples ausência de uma função. Isso é o tipo de dívida
 * que só morde quando alguém escreve a chamada #50 e não sabe que está
 * herdando retry num command de escrita.
 *
 * Aqui o contrato testado é: (1) o que é idempotente repete; (2) o boot não
 * repete; (3) "não repetir" exige motivo escrito.
 *
 * O gancho `gp100.debug.failDevice` em modo transitório (`op:n`) falha n vezes
 * e depois deixa passar — o que sobrou na chave PROVA quantas tentativas o
 * command consumiu.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  COMMAND_ATTEMPTS,
  IDEMPOTENTE,
  deviceBoot,
  deviceInfo,
  deviceSelectPreset,
  deviceSetParam,
  descrevePolitica,
  semRetry,
} from "../src/ipc/device";

const KEY = "gp100.debug.failDevice";

/** invoke que nunca responde — só o branch Tauri o usa. */
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

/** Nº de tentativas que sobrou no gancho: prova quantas o command consumiu. */
function tentativasRestantes(): number {
  const raw = localStorage.getItem(KEY) ?? "";
  const n = Number(raw.split(":")[1]);
  return Number.isFinite(n) ? n : 0;
}

describe("política: a exceção é declarada, não herdada", () => {
  it("IDEMPOTENTE repete COMMAND_ATTEMPTS vezes", () => {
    expect(IDEMPOTENTE).toEqual({ kind: "retry", attempts: COMMAND_ATTEMPTS });
  });

  it("semRetry sem motivo é ERRO no ponto da chamada", () => {
    // O motivo não é documentação: é a decisão. Aceitar string vazia traria de
    // volta o "não repetir" sem justificativa que a issue descreve.
    expect(() => semRetry("")).toThrow(/motivo/);
    expect(() => semRetry("   ")).toThrow(/motivo/);
  });

  it("semRetry guarda o motivo e descreve a política", () => {
    const p = semRetry("boot sao 2297 transacoes");
    expect(p).toEqual({ kind: "once", reason: "boot sao 2297 transacoes" });
    expect(descrevePolitica(p)).toContain("boot sao 2297 transacoes");
    expect(descrevePolitica(IDEMPOTENTE)).toBe(`retry x${COMMAND_ATTEMPTS}`);
  });

  it("o BOOT não repete: uma falha é o fim (o ⟳ do usuário recupera)", async () => {
    localStorage.setItem(KEY, "boot:3");
    await expect(deviceBoot()).rejects.toThrow(/boot/);
    // De 3 falhas permitidas sobraram 2: o comando consumiu UMA tentativa.
    // Com retry herdado seriam 3 tentativas — é exatamente a diferença que a
    // política declarada torna visível.
    expect(tentativasRestantes(), "boot consumiu uma unica tentativa").toBe(2);
  });

  it("um command idempotente repete e só então falha", async () => {
    localStorage.setItem(KEY, "select:5");
    await expect(deviceSelectPreset(7)).rejects.toThrow(/select/);
    // Duas tentativas consumidas das três: a política de retry está valendo.
    expect(tentativasRestantes(), "select consumiu as tentativas da política").toBeLessThan(5);
    expect(tentativasRestantes()).toBe(5 - COMMAND_ATTEMPTS);
  });

  it("info e set_param também repetem (idempotentes)", async () => {
    for (const [op, chamar] of [
      ["info", () => deviceInfo()],
      ["set_param", () => deviceSetParam(0, 0x0700006e, 2, 50)],
    ] as const) {
      localStorage.clear();
      localStorage.setItem(KEY, `${op}:5`);
      await expect(chamar()).rejects.toThrow();
      expect(tentativasRestantes(), `${op} consumiu COMMAND_ATTEMPTS`).toBe(5 - COMMAND_ATTEMPTS);
    }
  });
});
