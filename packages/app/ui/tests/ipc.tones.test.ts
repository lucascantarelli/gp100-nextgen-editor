/**
 * ipc/tones — o contrato da porta do gestor de SnapTone (#25).
 *
 * O Tauri não existe no jsdom, então É O FALLBACK que roda — e o fallback é a
 * metade da porta que pode divergir do crate sem ninguém perceber. O teste mais
 * importante deste arquivo é o do CRC: o id do tom é derivado do conteúdo nos
 * DOIS lados (Rust e TS), e se as duas implementações divergissem o dono
 * importaria o mesmo arquivo e veria dois tons onde o banco tem um — sem erro
 * em nenhum lugar.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  resetaFallback,
  toneAssignSlot,
  toneBoard,
  toneDelete,
  toneDoSlot,
  toneGet,
  toneImport,
  toneRename,
  toneSend,
} from "../src/ipc/tones";

/** CRC-32 IEEE de referência, escrito de um jeito diferente do da porta. */
function crcDeRef(dados: number[]): number {
  let crc = 0xffffffff;
  for (const b of dados) {
    crc = (crc ^ (b & 0xff)) >>> 0;
    for (let _ = 0; _ < 8; _ += 1) {
      crc = (crc & 1) !== 0 ? ((crc >>> 1) ^ 0xedb88320) >>> 0 : crc >>> 1;
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

/** Um `.clo` sintético: 40 bytes = 2 blocos de 19 + cauda de 2 (§5). */
const CLO_A = Array.from({ length: 40 }, (_, i) => (i * 7 + 3) & 0xff);
const CLO_B = Array.from({ length: 37 }, (_, i) => (i * 13 + 200) & 0xff);

beforeEach(() => resetaFallback());
afterEach(() => resetaFallback());

describe("ipc/tones — o id é o CRC do conteúdo", () => {
  it("o id bate com o CRC-32 (mesmo valor que o crate)", async () => {
    const linha = await toneImport("A", CLO_A);
    expect(linha.id).toBe(`t${crcDeRef(CLO_A).toString(16).padStart(8, "0")}`);
    expect(linha.crc32).toBe(crcDeRef(CLO_A));
    expect(linha.bytes).toBe(40);
  });

  it("reimportar o mesmo arquivo ATUALIZA em vez de duplicar", async () => {
    await toneImport("A", CLO_A);
    const segundo = await toneImport("A renomeado", CLO_A);
    const quadro = await toneBoard();
    expect(quadro.tones).toHaveLength(1);
    expect(segundo.name).toBe("A renomeado");
  });

  it("arquivos diferentes são tons diferentes (o id não colide)", async () => {
    await toneImport("A", CLO_A);
    await toneImport("B", CLO_B);
    const quadro = await toneBoard();
    expect(quadro.tones).toHaveLength(2);
    expect(new Set(quadro.tones.map((t) => t.id)).size).toBe(2);
  });

  it("a lista NÃO carrega o modelo — só o tom lido por id", async () => {
    await toneImport("A", CLO_A);
    const quadro = await toneBoard();
    expect(quadro.tones[0]).not.toHaveProperty("model");
    const comModelo = await toneGet(quadro.tones[0].id);
    expect(comModelo?.model).toEqual(CLO_A);
  });
});

describe("ipc/tones — slots", () => {
  it("o quadro traz os 5 slots do firmware (§5)", async () => {
    await toneImport("A", CLO_A);
    const quadro = await toneBoard();
    expect(quadro.slots).toBe(5);
    expect(quadro.usados).toBe(0);
  });

  it("um slot, um tom: o segundo dono é recusado COM o nome do primeiro", async () => {
    const a = await toneImport("Vintage", CLO_A);
    await toneImport("Moderno", CLO_B);
    await toneAssignSlot(a.id, 3);
    await expect(toneAssignSlot(quadroIdDe(await toneBoard(), "Moderno"), 3)).rejects.toThrow(/Vintage/);
  });

  it("desligar o slot libera o número", async () => {
    const a = await toneImport("A", CLO_A);
    const b = await toneImport("B", CLO_B);
    await toneAssignSlot(a.id, 2);
    await expect(toneAssignSlot(b.id, 2)).rejects.toThrow();
    await toneAssignSlot(a.id, null);
    await expect(toneAssignSlot(b.id, 2)).resolves.toBeUndefined();
  });

  it("slot fora de 1..=5 é erro", async () => {
    const a = await toneImport("A", CLO_A);
    for (const slot of [0, 6, 99]) {
      await expect(toneAssignSlot(a.id, slot)).rejects.toThrow(/1\.\.=5/);
    }
  });

  it("reimportar o MESMO arquivo não perde o slot nem o áudio", async () => {
    const wav = Array.from({ length: 64 }, (_, i) => i);
    const a = await toneImport("A", CLO_A, wav);
    await toneAssignSlot(a.id, 4);
    await toneImport("A (reimportado)", CLO_A);
    const depois = await toneGet(a.id);
    // Quem atribuiu o slot 4 não perde a atribuição por reimportar, e o áudio
    // que o dono tinha exportado continua lá.
    expect(depois?.slot).toBe(4);
    expect(depois?.preview).toEqual(wav);
  });
});

describe("ipc/tones — envio e A/B", () => {
  it("sem slot não há envio: o relatório de §5 exigiria um destino", async () => {
    const a = await toneImport("A", CLO_A);
    await expect(toneSend(a.id)).rejects.toThrow(/slot/);
  });

  it("o relatório bate com o stream de §5 (19B por bloco, ACK por bloco)", async () => {
    const a = await toneImport("A", CLO_A);
    await toneAssignSlot(a.id, 1);
    const rel = await toneSend(a.id);
    // 40 bytes = 2 blocos de 19 + cauda de 2 (§5).
    expect(rel).toEqual({ slot: 1, blocks: 3, acks: 3, bytes: 40 });
  });

  it("o tom do slot devolve o modelo, e o slot vazio devolve null", async () => {
    const a = await toneImport("A", CLO_A);
    await toneAssignSlot(a.id, 5);
    expect((await toneDoSlot(5))?.model).toEqual(CLO_A);
    expect(await toneDoSlot(4)).toBeNull();
  });

  it("apagar leva o tom do quadro e o slot junto (apagar 2x = false)", async () => {
    const a = await toneImport("A", CLO_A);
    await toneAssignSlot(a.id, 2);
    expect(await toneDelete(a.id)).toBe(true);
    expect(await toneDelete(a.id)).toBe(false);
    expect((await toneBoard()).tones).toHaveLength(0);
    expect(await toneDoSlot(2)).toBeNull();
  });

  it("renomear um tom que não existe devolve false (não inventa registro)", async () => {
    expect(await toneRename("tdeadbeef", "Fantasma")).toBe(false);
  });

  it("no shell, o envio que não volta é recusado aos 60s (NÃO aos 8s do runCommand) e não repete", async () => {
    // O caminho do SHELL é o único que bloqueia de verdade: o `invoke` do
    // Tauri não resolve até o aparelho terminar os 143 blocos (§4/§5). Aqui ele
    // nunca resolve — que é o caso que o timeout do envio existe para tratar.
    // Um timeout herdado do `runCommand` (8 s) mostraria erro para um envio
    // que estava dando certo, e o retry repetiria o stream no aparelho.
    const internals = { invoke: vi.fn(() => new Promise<never>(() => {})) };
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = internals;
    // O `invoke` da porta é um import dinâmico: aquecer o módulo com o relógio
    // real antes de congelar é o que garante que ele chegue a chamar o
    // `__TAURI_INTERNALS__.invoke` (e que a contagem abaixo signifique algo).
    await import("@tauri-apps/api/core");
    vi.useFakeTimers();
    try {
      let erro: unknown = null;
      const envio = toneSend("tqualquer").catch((e) => {
        erro = e;
      });

      await vi.advanceTimersByTimeAsync(9_000);
      expect(erro, "aos 9s o envio ainda está a caminho (o global de 8s não vale aqui)").toBeNull();

      await vi.advanceTimersByTimeAsync(52_000);
      await envio;
      expect(String(erro), "aos 60s o envio é recusado com o motivo").toMatch(/excedeu 60000 ms/);
      expect(internals.invoke, "sem retry: repetir deixaria o aparelho no meio de 2 streams").toHaveBeenCalledTimes(1);
    } finally {
      vi.useRealTimers();
      delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    }
  });
});

/** O id do tom com este nome no quadro (o teste fala de tom, não de id). */
function quadroIdDe(quadro: { tones: { id: string; name: string }[] }, nome: string): string {
  const t = quadro.tones.find((x) => x.name === nome);
  if (!t) throw new Error(`tom ${nome} ausente do quadro`);
  return t.id;
}