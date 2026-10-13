/**
 * ipc/push — parsing/dedupe dos pushes (issue #20), testado PURO: sem hook,
 * sem DOM. Contratos:
 *   - só SysEx bem formado (F0…F7, par, hex) passa; o resto é `null`;
 *   - repetição CONSECUTIVA do mesmo hex vira contador na mesma linha;
 *   - hex diferente depois da repetição abre linha nova;
 *   - cap de entradas mantém as MAIS NOVAS.
 */
import { describe, expect, it } from "vitest";
import { PUSH_LOG_MAX, parsePushHex, pushLogReducer } from "../src/ipc/push";
import type { PushLogEntry } from "../src/ipc/types";

const SYSEX = "F021257F47502D6412001000F7";

describe("ipc/push — parsePushHex", () => {
  it("aceita SysEx F0…F7 e normaliza para MAIÚSCULAS (trim incluso)", () => {
    expect(parsePushHex(SYSEX)).toBe(SYSEX);
    expect(parsePushHex(`  ${SYSEX.toLowerCase()}  `)).toBe(SYSEX);
  });

  it("rejeita payload que não é SysEx bem formado", () => {
    expect(parsePushHex("12|12001008")).toBeNull(); // pipe (não-hex)
    expect(parsePushHex("F021257F47502D64 00F7")).toBeNull(); // espaço interno
    expect(parsePushHex("F02125")).toBeNull(); // curto/fora do envelope
    expect(parsePushHex("F0212")).toBeNull(); // tamanho ímpar
    expect(parsePushHex("1201002F7")).toBeNull(); // sem F0
    expect(parsePushHex("F021257F47502D64")).toBeNull(); // sem F7
    expect(parsePushHex("")).toBeNull();
  });

  it("rejeita payload que nem é string (o evento não é confiável)", () => {
    expect(parsePushHex(undefined)).toBeNull();
    expect(parsePushHex(1234)).toBeNull();
    expect(parsePushHex(null)).toBeNull();
  });
});

describe("ipc/push — pushLogReducer", () => {
  it("primeira chegada entra com count 1", () => {
    const log = pushLogReducer([], SYSEX, 1000);
    expect(log).toEqual([{ hex: SYSEX, at: 1000, count: 1 }]);
  });

  it("push INVÁLIDO não muda o log (mesma referência — zero render)", () => {
    const prev: PushLogEntry[] = [{ hex: SYSEX, at: 1, count: 1 }];
    expect(pushLogReducer(prev, "lixo", 2)).toBe(prev);
  });

  it("repetição CONSECUTIVA fica na MESMA linha com contador", () => {
    let log = pushLogReducer([], SYSEX, 1000);
    log = pushLogReducer(log, SYSEX, 1100);
    log = pushLogReducer(log, SYSEX.toLowerCase(), 1200);
    expect(log).toEqual([{ hex: SYSEX, at: 1200, count: 3 }]);
  });

  it("hex DIFERENTE depois da repetição abre linha nova (dedupe é consecutivo)", () => {
    const outro = "F021257F47502D6412001001F7";
    let log = pushLogReducer([], SYSEX, 1);
    log = pushLogReducer(log, SYSEX, 2);
    log = pushLogReducer(log, outro, 3);
    log = pushLogReducer(log, SYSEX, 4);
    expect(log.map((e) => [e.hex, e.count])).toEqual([
      [SYSEX, 2],
      [outro, 1],
      [SYSEX, 1],
    ]);
  });

  it("cap: acima do limite as MAIS NOVAS ficam (ordem de chegada preservada)", () => {
    let log: PushLogEntry[] = [];
    for (let i = 0; i < PUSH_LOG_MAX + 5; i += 1) {
      log = pushLogReducer(log, `F021257F47502D6412001000${i.toString(16).padStart(2, "0")}F7`, i);
    }
    expect(log.length).toBe(PUSH_LOG_MAX);
    expect(log[0].at).toBe(5); // as 5 primeiras saíram
    expect(log.at(-1)!.at).toBe(PUSH_LOG_MAX + 4);
  });
});
