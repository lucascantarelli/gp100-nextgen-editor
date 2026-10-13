/**
 * Preset em arquivo — a porta e o fallback (issue #114).
 *
 * O que merece teste aqui NÃO é "o botão baixa um arquivo": é o contrato que a
 * issue promete nos dois sentidos —
 *
 * - o JSON é VERSIONADO (o `format`/`version` que um build futuro confere);
 * - a ida e a volta preservam a CADEIA inteira (é o DoD: "reimportar, com a
 *   cadeia preservada"), e o importado carrega o preset de onde ele saiu;
 * - um arquivo que NÃO é nosso é recusado com o motivo, incluindo o caso que
 *   mais importa: o `gp100.preset` do core, que o fallback não lê (ele não tem
 *   o `.prst`) e não pode aceitar em silêncio;
 * - a folha de timbre existe só onde há motor de PDF — fora do app a porta diz
 *   isso em vez de devolver bytes inventados.
 *
 * Os testes rodam no fallback em memória (fora do webview do Tauri não há core
 * Rust), que é o MESMO caminho que o e2e do Playwright exercita. A verdade do
 * round-trip byte-idêntico do `.prst` é dos testes Rust, onde o arquivo existe.
 */
import { describe, expect, it } from "vitest";
import { deviceBoard } from "../src/ipc/device";
import {
  folhaDeTimbreDisponivel,
  presetExportJson,
  presetExportToneSheet,
  presetImportJson,
} from "../src/ipc/preset";
import type { BoardView } from "../src/ipc/types";

/** O envelope do fallback, lido como dado (o teste não importa as constantes). */
interface Envelope {
  format: string;
  version: number;
  preset: Omit<BoardView, "bank" | "ppLabel">;
}

const FORMATO = "gp100.preset.chain";

/** O motivo da recusa — o import que ACEITAR seria o bug. */
async function recusa(json: string): Promise<string> {
  try {
    await presetImportJson(json);
  } catch (e) {
    return e instanceof Error ? e.message : String(e);
  }
  throw new Error("o import deveria ter recusado este arquivo");
}

describe("preset em arquivo — o cabeçalho do envelope", () => {
  it("o JSON é versionado e o cabeçalho é o que o import confere", async () => {
    const env = JSON.parse(await presetExportJson(0)) as Envelope;
    expect(env.format).toBe(FORMATO);
    expect(env.version).toBe(1);
    expect(env.preset.slots).toHaveLength(9);
  });
});

describe("preset em arquivo — a ida e a volta", () => {
  it("a cadeia que volta é a MESMA que o palco desenha", async () => {
    const antes = await deviceBoard(1);
    const depois = await presetImportJson(await presetExportJson(1));
    expect(depois).toEqual(antes);
  });

  it("o importado é do preset de ONDE ele saiu (pp, rótulo, nome e estilo)", async () => {
    const board = await presetImportJson(await presetExportJson(18));
    const origem = await deviceBoard(18);
    expect(board.pp).toBe(18);
    expect(board.ppLabel).toBe("P19");
    expect(board.name).toBe(origem.name);
    expect(board.ppTypeName).toBe(origem.ppTypeName);
    // o `.prst` vem do arquivo de fábrica embutido: o arquivo é sempre factory
    expect(board.bank).toBe("factory");
  });

  it("presets diferentes exportam cadeias DIFERENTES (não é um retrato fixo)", async () => {
    const p0 = await presetImportJson(await presetExportJson(0));
    const p1 = await presetImportJson(await presetExportJson(1));
    // P01 abre num C-Wah; P02, num COMP (o artefato gerado do all.prst)
    expect(p0.slots[0].name).toBe("C-Wah");
    expect(p1.slots[0].name).toBe("COMP");
  });
});

describe("preset em arquivo — o arquivo que NÃO é nosso", () => {
  it("texto que não é JSON nomeia o que veio", async () => {
    expect(await recusa("isto nao e um json")).toMatch(/nao e um JSON/);
  });

  it("JSON que não é objeto (lista) é recusado como envelope", async () => {
    expect(await recusa("[]")).toMatch(/sem objeto/);
    expect(await recusa("null")).toMatch(/sem objeto/);
  });

  it("o `gp100.preset` do core é recusado com o formato que veio", async () => {
    // O caso que justifica o formato próprio do fallback: dentro do app este
    // arquivo é um preset válido, e fora dele NÃO É (o front não tem o `.prst`).
    // Recusar dizendo qual formato veio é o que evita o dono achar que o
    // arquivo corrompeu.
    const doCore = JSON.stringify({
      format: "gp100.preset",
      version: 1,
      declaracao: [],
      depois: [],
      raiz: [],
    });
    expect(await recusa(doCore)).toMatch(/formato alheio: "gp100\.preset"/);
  });

  it("versão futura diz qual veio e até qual este build lê", async () => {
    const futuro = JSON.stringify({ format: FORMATO, version: 2, preset: { slots: [] } });
    const motivo = await recusa(futuro);
    expect(motivo).toMatch(/versao do arquivo: 2/);
    expect(motivo).toMatch(/le a 1/);
  });

  it("envelope sem a cadeia é recusado", async () => {
    expect(await recusa(JSON.stringify({ format: FORMATO, version: 1 }))).toMatch(
      /envelope incompleto/,
    );
    expect(
      await recusa(JSON.stringify({ format: FORMATO, version: 1, preset: { pp: 3 } })),
    ).toMatch(/envelope incompleto/);
  });
});

describe("folha de timbre — só onde existe motor de PDF", () => {
  it("fora do app não há motor, e a porta diz isso", async () => {
    expect(folhaDeTimbreDisponivel()).toBe(false);
    await expect(presetExportToneSheet(0)).rejects.toThrow(/motor de PDF/);
  });
});
