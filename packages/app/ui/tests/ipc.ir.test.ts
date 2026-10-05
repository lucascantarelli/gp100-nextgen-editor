/**
 * ipc/ir — o contrato da porta do laboratório de IRs (#24).
 *
 * O Tauri não existe no jsdom, então É O FALLBACK que roda — e o fallback é a
 * metade da porta que pode divergir do crate sem ninguém perceber. O teste mais
 * importante deste arquivo é o do CRC: o id do IR é derivado do conteúdo nos
 * DOIS lados (Rust e TS), e se as duas implementações divergissem o dono
 * importaria o mesmo arquivo e veria dois IRs onde o banco tem um — sem erro em
 * nenhum lugar.
 *
 * Os números do protocolo (`IR_SLOTS`, `IR_CHUNK_BYTES`) são testados pelo
 * VALOR, e não pela constante: o que importa é que a tela e o `upload_ir`
 * concordem sobre 20 slots e 15 bytes por chunk, e um teste que repetisse o
 * número passaria junto com uma divergência.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  IR_CHUNK_BYTES,
  IR_SLOTS,
  deviceIrTable,
  irAssignSlot,
  irBoard,
  irDelete,
  irEnviavel,
  irImport,
  irRename,
  irSend,
  resetaFallback,
} from "../src/ipc/ir";

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

/** Um `.ir` sintético: múltiplo de 15B, como o fio exige. */
function irDe(n_chunks: number, semente: number): number[] {
  return Array.from({ length: n_chunks * IR_CHUNK_BYTES }, (_, i) => (i * 7 + semente) & 0xff);
}

beforeEach(() => resetaFallback());
afterEach(() => resetaFallback());

describe("ipc/ir — os números do protocolo", () => {
  it("20 slots e 15 bytes por chunk (§13.7 e §13.12)", () => {
    // `ppIRInfo0..19` no .prst são 20 tags, e quem define o slot é a POSIÇÃO
    // delas — o que torna o 0 o primeiro slot, não um "slot vazio".
    expect(IR_SLOTS).toBe(20);
    // 30 nibbles na SysEx = 15 bytes reais por chunk, e o pedaço final é
    // RECUSADO (strict) em vez de completado.
    expect(IR_CHUNK_BYTES).toBe(15);
  });

  it("o botão de enviar só acende para arquivo que o fio aceita", () => {
    expect(irEnviavel({ bytes: 15 })).toBe(true);
    expect(irEnviavel({ bytes: 300 * 1024 })).toBe(true);
    // 0 bytes: o `upload_ir` daria panic (o bug que o #24 fecha).
    expect(irEnviavel({ bytes: 0 })).toBe(false);
    // 22 bytes: a cauda de 7 faria o contador de chunks divergir.
    expect(irEnviavel({ bytes: 22 })).toBe(false);
  });
});

describe("ipc/ir — o id é o CRC do conteúdo", () => {
  it("o id bate com o CRC-32 (mesmo valor que o crate)", async () => {
    const bytes = irDe(2, 3);
    const linha = await irImport("Vintage 4x12", bytes);
    expect(linha.id).toBe(`i${crcDeRef(bytes).toString(16).padStart(8, "0")}`);
    expect(linha.crc32).toBe(crcDeRef(bytes));
    expect(linha.bytes).toBe(30);
  });

  it("reimportar o mesmo arquivo ATUALIZA em vez de duplicar", async () => {
    const bytes = irDe(2, 3);
    await irImport("Vintage", bytes);
    const segundo = await irImport("Vintage (bônus)", bytes);
    const quadro = await irBoard();
    expect(quadro.irs).toHaveLength(1);
    expect(segundo.name).toBe("Vintage (bônus)");
  });

  it("arquivos diferentes são IRs diferentes (o id não colide)", async () => {
    await irImport("A", irDe(1, 1));
    await irImport("B", irDe(1, 2));
    const quadro = await irBoard();
    expect(quadro.irs).toHaveLength(2);
    expect(new Set(quadro.irs.map((i) => i.id)).size).toBe(2);
  });

  it("a lista NÃO carrega o conteúdo — só o que a tela mostra", async () => {
    await irImport("A", irDe(3, 5));
    const quadro = await irBoard();
    expect(quadro.irs[0]).not.toHaveProperty("blob");
  });
});

describe("ipc/ir — slots", () => {
  it("o quadro traz os 20 slots do aparelho", async () => {
    const quadro = await irBoard();
    expect(quadro.slots).toBe(20);
    expect(quadro.usados).toBe(0);
  });

  it("o slot 0 é válido (≠ SnapTone, que começa em 1)", async () => {
    const ir = await irImport("A", irDe(1, 1));
    await expect(irAssignSlot(ir.id, 0)).resolves.toBeUndefined();
    expect((await irBoard()).usados).toBe(1);
  });

  it("um slot, um IR: o segundo dono é recusado COM o nome do primeiro", async () => {
    const a = await irImport("Vintage", irDe(1, 1));
    await irImport("Moderno", irDe(1, 2));
    await irAssignSlot(a.id, 0);
    const b = (await irBoard()).irs.find((i) => i.name === "Moderno");
    if (b == null) throw new Error("Moderno ausente do quadro");
    await expect(irAssignSlot(b.id, 0)).rejects.toThrow(/Vintage/);
  });

  it("desligar o slot libera o número", async () => {
    const a = await irImport("A", irDe(1, 1));
    const b = await irImport("B", irDe(1, 2));
    await irAssignSlot(a.id, 19);
    await expect(irAssignSlot(b.id, 19)).rejects.toThrow();
    await irAssignSlot(a.id, null);
    await expect(irAssignSlot(b.id, 19)).resolves.toBeUndefined();
  });

  it("slot fora de 0..=19 é erro", async () => {
    const a = await irImport("A", irDe(1, 1));
    for (const slot of [20, 99, 255]) {
      await expect(irAssignSlot(a.id, slot)).rejects.toThrow(/0\.\.=19/);
    }
  });

  it("reimportar o MESMO arquivo não perde o slot", async () => {
    const bytes = irDe(2, 7);
    const a = await irImport("A", bytes);
    await irAssignSlot(a.id, 4);
    await irImport("A (reimportado)", bytes);
    expect((await irBoard()).irs[0].slot).toBe(4);
  });
});

describe("ipc/ir — envio", () => {
  it("sem slot não há envio: o relatório exigiria um destino", async () => {
    const a = await irImport("A", irDe(2, 1));
    await expect(irSend(a.id)).rejects.toThrow(/slot/);
  });

  it("o relatório bate com o stream de §13.7 (15B por chunk + marcador)", async () => {
    const a = await irImport("A", irDe(4, 3));
    await irAssignSlot(a.id, 0);
    const rel = await irSend(a.id);
    // 4 chunks de dados + o último duplicado (o marcador de fim É a duplicação).
    expect(rel).toEqual({ slot: 0, chunks: 4, acks: 5, bytes: 60 });
  });

  it("apagar leva o IR do quadro e o slot junto (apagar 2x = false)", async () => {
    const a = await irImport("A", irDe(1, 1));
    await irAssignSlot(a.id, 2);
    expect(await irDelete(a.id)).toBe(true);
    expect(await irDelete(a.id)).toBe(false);
    expect((await irBoard()).irs).toHaveLength(0);
  });

  it("renomear um IR que não existe devolve false (não inventa registro)", async () => {
    expect(await irRename("ideadbeef", "Fantasma")).toBe(false);
  });

  it("no shell, o envio que não volta é recusado pelo timeout do IR e NÃO repete", async () => {
    // Um IR de 300 KB são ~20.000 chunks: na captura, 296 chunks levaram ~5 s,
    // o que dá ~17 ms por chunk e quase 6 minutos de envio. O timeout padrão do
    // `runCommand` (8 s) desistiria no primeiro minuto de um envio que estava
    // dando certo — e sem `no retry` haveria dois streams no aparelho.
    const internals = { invoke: vi.fn(() => new Promise<never>(() => {})) };
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = internals;
    // o `invoke` da porta é um import dinâmico: aquecer o módulo com o relógio
    // real antes de congelar é o que garante que ele chegue a chamar o
    // `__TAURI_INTERNALS__.invoke`.
    await import("@tauri-apps/api/core");
    vi.useFakeTimers();
    try {
      let erro: unknown = null;
      const envio = irSend("ialgum").catch((e) => {
        erro = e;
      });

      await vi.advanceTimersByTimeAsync(9_000);
      expect(erro, "aos 9s o envio ainda está a caminho (o global de 8s não vale aqui)").toBeNull();      // 9s de espera + 13 min: passa dos 12 min do timeout do IR, que e
      // ~2× o envio de um IR de 300 KB (296 chunks em ~5 s na captura).
      await vi.advanceTimersByTimeAsync(13 * 60_000);
      await envio;
      expect(String(erro), "o envio é recusado com o timeout próprio do IR").toMatch(/excedeu \d+ ms/);
      expect(internals.invoke, "sem retry: repetir deixaria o aparelho no meio de 2 streams").toHaveBeenCalledTimes(1);
    } finally {
      vi.useRealTimers();
      delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    }
  });
});

describe("ipc/ir — a tabela do APARELHO", () => {
  it("é uma leitura do DEVICE, e não da biblioteca", async () => {
    // A biblioteca tem IRs; a tabela do aparelho não pode vir deles: o dono
    // importou pelo painel de hardware e o arquivo local não sabe.
    await irImport("Meu", irDe(2, 1));
    const tabela = await deviceIrTable();
    expect(tabela).toHaveLength(20);
    expect(tabela.every((t) => t.name === ""), "sem device, todos os slots vêm vazios").toBe(true);
    expect(tabela.map((t) => t.slot)).toEqual(Array.from({ length: 20 }, (_, i) => i));
  });
});
