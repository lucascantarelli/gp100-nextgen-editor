/**
 * useTones — o estado do gestor de SnapTone (issue #25).
 *
 * O hook é onde mora a política que a tela não mostra: quem recarrega o quadro
 * depois de cada escrita, qual falha vira banner com retry, e o que acontece
 * quando o banco recusa uma atribuição. Os testes do painel provam o que o
 * dono vê; estes provam o que a tela NÃO vê e ainda assim depende.
 */
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it } from "vitest";
import { useTones } from "../src/hooks/useTones";
import type { Tones } from "../src/hooks/useTones";
import { resetaFallback } from "../src/ipc/tones";

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

beforeEach(() => {
  resetaFallback();
  // jsdom TEM `URL.createObjectURL`, mas ele quebra com um `Blob` vindo do
  // realm do Node (`Cannot read properties of undefined (reading '_bytes')`).
  // No WebView real os dois são do mesmo realm; aqui o stub é o que permite
  // exercitar o caminho do player sem um browser de verdade.
  URL.createObjectURL = () => "blob:fake";
});

const CLO_A = Array.from({ length: 40 }, (_, i) => (i * 7 + 3) & 0xff);
const SEM = Symbol("sem valor");
const CLO_B = Array.from({ length: 37 }, (_, i) => (i * 13 + 200) & 0xff);
const WAV = Array.from({ length: 64 }, (_, i) => i);

/** Monta o hook e devolve o ÚLTIMO estado renderizado. */
function mount() {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root: Root = createRoot(host);
  let api: Tones | null = null;
  function Casca() {
    api = useTones();
    return null;
  }
  act(() => root.render(<Casca />));
  const pega = (): Tones => {
    if (api == null) throw new Error("hook nao renderizou");
    return api;
  };
  /**
   * Executa uma operação do hook dentro de `act` e DEVOLVE o que ela devolveu.
   *
   * O `act` engole o retorno (ele devolve `void`), e sem devolver o valor o
   * teste teria que reler o quadro para descobrir o id do tom que acabou de
   * importar — o que trocaria “o import devolve a linha” por “o quadro tem a
   * linha”, que são asserções diferentes.
   */
  const roda = async <R,>(op: (t: Tones) => Promise<R> | R): Promise<R> => {
    // sentinel: uma operação pode legitimamente devolver `undefined`
    // (`abrir`, `atribui`), e distinguir isso de “não rodou” precisa de algo
    // que `undefined` não seja
    let resultado: R | typeof SEM = SEM;
    await act(async () => {
      resultado = await op(pega());
    });
    // um segundo turno para o `setState` que a operação encadeou (recarregar
    // o quadro é sempre um passo DEPOIS do que a tela pediu)
    await act(async () => {
      await Promise.resolve();
    });
    return resultado as R;
  };
  return {
    pega,
    roda,
    /** Abre o painel (o quadro é relido na abertura). */
    abrir: () => roda((t) => t.abrir()),
    done: () => {
      act(() => root.unmount());
      host.remove();
    },
  };
}

describe("useTones — leitura e escrita", () => {
  it("o quadro chega no mount (a tela mostra os números com o painel fechado)", async () => {
    const { roda, pega, done } = mount();
    await roda(() => undefined);
    expect(pega().board).not.toBeNull();
    expect(pega().board?.slots).toBe(5);
    done();
  });

  it("importar relê o quadro: a lista da tela é a do banco, não um estado local", async () => {
    const { roda, pega, done } = mount();
    await roda((t) => t.importa("Bass", CLO_A));
    expect(pega().board?.tones).toHaveLength(1);
    expect(pega().board?.tones[0].name).toBe("Bass");
    expect(pega().board?.usados).toBe(0);
    done();
  });

  it("atribuir slot move o contador do rodapé", async () => {
    const { roda, pega, done } = mount();
    const linha = await roda(async (t) => t.importa("Bass", CLO_A));
    const id = (linha as { id: string }).id;
    await roda((t) => t.atribui(id, 2));
    expect(pega().board?.usados).toBe(1);
    expect(pega().board?.tones[0].slot).toBe(2);
    done();
  });

  it("slot ocupado vira banner com o NOME de quem ocupa (o que o dono precisa)", async () => {
    const { roda, pega, done } = mount();
    const a = (await roda((t) => t.importa("Vintage", CLO_A))) as { id: string };
    const b = (await roda((t) => t.importa("Moderno", CLO_B))) as { id: string };
    await roda((t) => t.atribui(a.id, 3));
    await roda((t) => t.atribui(b.id, 3));
    expect(pega().err?.message).toContain("Vintage");
    // E o quadro NÃO mudou: o banco recusou, a lista continua dizendo quem
    // está no 3 (uma UI que "aplicasse" na mesma hora mentiria aqui).
    expect(pega().board?.tones.find((t) => t.id === b.id)?.slot).toBeNull();
    done();
  });

  it("renomear e apagar refletem no quadro", async () => {
    const { roda, pega, done } = mount();
    const t1 = (await roda((t) => t.importa("Antes", CLO_A))) as { id: string };
    await roda((t) => t.renomeia(t1.id, "Depois"));
    expect(pega().board?.tones[0].name).toBe("Depois");
    await roda((t) => t.apaga(t1.id));
    expect(pega().board?.tones).toHaveLength(0);
    done();
  });

  it("abrir o painel liga o estado e relê o quadro", async () => {
    const { abrir, pega, done } = mount();
    expect(pega().aberto).toBe(false);
    await abrir();
    expect(pega().aberto).toBe(true);
    done();
  });
});

describe("useTones — envio ao aparelho", () => {
  it("sem slot o envio falha e a tela mostra o motivo", async () => {
    const { roda, pega, done } = mount();
    const t1 = (await roda((t) => t.importa("Sem slot", CLO_A))) as { id: string };
    await roda((t) => t.envia(t1.id));
    expect(pega().err).not.toBeNull();
    expect(pega().relatorio).toBeNull();
    done();
  });

  it("com slot o relatório traz o stream de §5 (blocos e ACKs)", async () => {
    const { roda, pega, done } = mount();
    const t1 = (await roda((t) => t.importa("Bass", CLO_A))) as { id: string };
    await roda((t) => t.atribui(t1.id, 1));
    await roda((t) => t.envia(t1.id));
    expect(pega().relatorio).toEqual({ slot: 1, blocks: 3, acks: 3, bytes: 40 });
    expect(pega().enviando).toBeNull();
    done();
  });
});

describe("useTones — o A/B toca o audio do Suite", () => {
  it("sem preview devolve null (o botao fica desabilitado com o motivo)", async () => {
    const { roda, done } = mount();
    const t1 = (await roda((t) => t.importa("Sem audio", CLO_A))) as { id: string };
    await roda((t) => t.atribui(t1.id, 1));
    let url: string | null = "x";
    await roda(async (t) => {
      url = await t.toca(1);
    });
    expect(url).toBeNull();
    done();
  });

  it("com preview devolve uma URL de audio", async () => {
    const { roda, done } = mount();
    const t1 = (await roda((t) => t.importa("Com audio", CLO_A, WAV))) as { id: string };
    await roda((t) => t.atribui(t1.id, 2));
    let url: string | null = null;
    await roda(async (t) => {
      url = await t.toca(2);
    });
    expect(url).toMatch(/^blob:/);
    done();
  });

  it("os lados do A/B sao numeros de SLOT (o que o aparelho tem)", async () => {
    const { roda, pega, done } = mount();
    await roda((t) => t.chooseLado("a", 3));
    await roda((t) => t.chooseLado("b", 4));
    expect(pega().ladoA).toBe(3);
    expect(pega().ladoB).toBe(4);
    done();
  });
});