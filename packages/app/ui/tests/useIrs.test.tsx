/**
 * useIrs — o estado do laboratório (issue #24).
 *
 * O hook é o DONO da tela, e o que ele guarda tem consequência no aparelho:
 *
 *  - **Duas leituras na abertura.** A biblioteca (arquivo) e a tabela do
 *    aparelho (fio) são lidas em separado, porque divergem: o dono pode ter
 *    gravado um IR pelo painel de hardware e o arquivo local não fica sabendo.
 *  - **O envio trava em DUPLO clique.** O botão desabilitado não basta: dois
 *    cliques no mesmo evento chegam antes do re-render, e o segundo stream
 *    sobrescreveria o primeiro no meio da transferência — com o último chunk
 *    duplicado (§13.7) fechando a segunda por cima da primeira.
 *  - **Depois de enviar, a tabela do aparelho é relida.** Sem isso a tela
 *    continuaria mostrando o nome antigo no slot que acabou de mudar.
 *  - **Apagar avisa o que não acontece** — o registro sai do arquivo, e o que
 *    está no aparelho fica (o `erase` de slot não existe no protocolo).
 */
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { useIrs } from "../src/hooks/useIrs";
import type { Irs } from "../src/hooks/useIrs";
import { IR_CHUNK_BYTES, resetaFallback } from "../src/ipc/ir";
import { MSG } from "../src/i18n/messages";

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

let ultimo: Irs | null = null;

beforeEach(() => {
  resetaFallback();
  ultimo = null;
});

afterEach(() => resetaFallback());

/** Monta o hook e devolve as utilidades para olhar o estado. */
function mount() {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root: Root = createRoot(host);
  function Casca() {
    ultimo = useIrs();
    return null;
  }
  act(() => root.render(<Casca />));
  return {
    /** O estado atual (o hook é o dono). */
    get: () => {
      if (ultimo == null) throw new Error("hook não montado");
      return ultimo;
    },
    flush: async () => {
      await act(async () => {
        await Promise.resolve();
      });
    },
    /** Chama uma ação do hook dentro de `act`. */
    run: async (f: (i: Irs) => Promise<unknown> | unknown) => {
      await act(async () => {
        await f(ultimo as unknown as Irs);
      });
    },
    done: () => {
      act(() => root.unmount());
      host.remove();
    },
  };
}

const IR = Array.from({ length: 4 * IR_CHUNK_BYTES }, (_, i) => (i * 7 + 3) & 0xff);

describe("useIrs — a abertura", () => {
  it("lê a biblioteca no mount e a tabela do aparelho ao abrir", async () => {
    const m = mount();
    await m.flush();
    // A biblioteca é lida já no mount (o rodapé mostra os números com o painel
    // fechado); a tabela do APARELHO é uma transação de fio e espera a abertura.
    expect(m.get().board).not.toBeNull();
    expect(m.get().deviceSlots).toBeNull();
    await m.run((i) => i.abrir());
    await m.flush();
    expect(m.get().deviceSlots).toHaveLength(20);
    m.done();
  });

  it("a tabela do aparelho não vem da biblioteca", async () => {
    const m = mount();
    await m.flush();
    await m.run((i) => i.importa("Meu", IR));
    await m.flush();
    await m.run((i) => i.abrir());
    await m.flush();
    // Um IR na biblioteca não ocupa slot nenhum no aparelho — são listas
    // diferentes, e confundir as duas faria a tela dizer que o slot 0 tem
    // "Meu" quando o aparelho não recebeu nada.
    expect(m.get().board!.irs).toHaveLength(1);
    expect(m.get().deviceSlots!.every((d) => d.name === "")).toBe(true);
    m.done();
  });
});

describe("useIrs — o envio", () => {
  it("o relatório volta e a trava some no fim", async () => {
    const m = mount();
    await m.flush();
    await m.run((i) => i.importa("Vintage", IR));
    await m.flush();
    const id = m.get().board!.irs[0].id;
    await m.run((i) => i.atribui(id, 0));
    await m.flush();
    await m.run((i) => i.envia(id));
    await m.flush();
    expect(m.get().relatorio).toEqual({ slot: 0, chunks: 4, acks: 5, bytes: 60 });
    expect(m.get().enviando).toBeNull();
    m.done();
  });

  it("sem slot o envio falha com o motivo, e o relatório NÃO aparece", async () => {
    const m = mount();
    await m.flush();
    await m.run((i) => i.importa("Sem destino", IR));
    await m.flush();
    const id = m.get().board!.irs[0].id;
    await m.run((i) => i.envia(id));
    await m.flush();
    expect(m.get().relatorio).toBeNull();
    expect(m.get().err!.message).toMatch(/slot/);
    m.done();
  });

  it("um segundo envio durante o primeiro é IGNORADO (um stream só)", async () => {
    // Um `invoke` que nunca resolve é o envio em andamento: sem ele o
    // fallback resolve em milissegundos e a trava nem chega a ser exercida.
    // O nome do comando é tipado (e usado na contagem abaixo); o corpo ignora
    // os argumentos de propósito — é um `invoke` que nunca resolve.
    const nunca = () => new Promise<never>(() => {});
    const interna: { invoke: ReturnType<typeof vi.fn> } = { invoke: vi.fn() };
    interna.invoke.mockImplementation(nunca as never);
    const internals = { invoke: interna.invoke };
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = internals;
    await import("@tauri-apps/api/core");
    try {
      const m = mount();
      await m.flush();
      // Fora do shell não dá para importar pendente: importa no fallback
      // (sem `__TAURI_INTERNALS__`) e só depois liga o shell para o envio.
      delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
      await m.run((i) => i.importa("Vintage", IR));
      await m.flush();
      const id = m.get().board!.irs[0].id;
      await m.run((i) => i.atribui(id, 0));
      await m.flush();
      (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = internals;

      // Primeiro envio: fica pendente (não awaited) e trava o botão.
      await act(async () => {
        void m.get().envia(id);
      });
      expect(m.get().enviando, "o botão fica travado durante o envio").toBe(id);

      // Segundo clique com a trava posta: a função devolve sem tocar o fio.
      await act(async () => {
        void m.get().envia(id);
      });
      // Só as chamadas de `ir_send` contam: o `ir_board` da recarga e o
      // `list_user_irs` pós-envio também passam pelo `invoke`, e contá-los
      // aqui mediria a leitura de tela, não o stream.
      const sends = internals.invoke.mock.calls.filter((c) => c[0] === "ir_send");
      expect(sends.length, "um stream só — dois deixariam o aparelho no meio de 2 transferências").toBe(1);
      m.done();
    } finally {
      delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    }
  });
});

describe("useIrs — apagar", () => {
  it("apagar tira o IR da biblioteca e o slot fica livre", async () => {
    const m = mount();
    await m.flush();
    await m.run((i) => i.importa("Temp", IR));
    await m.flush();
    const id = m.get().board!.irs[0].id;
    await m.run((i) => i.atribui(id, 3));
    await m.flush();
    await m.run((i) => i.apaga(id));
    await m.flush();
    expect(m.get().board!.irs).toHaveLength(0);
    expect(m.get().board!.usados).toBe(0);
    m.done();
  });

  it("slot fora de 0..=19 vira erro com o motivo do crate", async () => {
    const m = mount();
    await m.flush();
    await m.run((i) => i.importa("Temp", IR));
    await m.flush();
    const id = m.get().board!.irs[0].id;
    await m.run((i) => i.atribui(id, 20));
    await m.flush();
    expect(m.get().err!.message).toContain("0..=19");
    expect(MSG.irSlotLabel).toBeTypeOf("function");
    m.done();
  });
});
