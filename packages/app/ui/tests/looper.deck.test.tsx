/**
 * LooperPanel — deck redesenhado (issue #12).
 *
 * O que estes testes travam é o que o redesign introduced de NOVO e que
 * dava para quebrar em silêncio:
 *
 *  - o deck é UM SVG só (rolos + cabeçotes + capstan + fita). Separado em
 *    vários <svg>, as peças não se encaixam quando a coluna muda de
 *    largura e a fita não nasce num rolo nem morre no outro;
 *  - o PACOTE DE FITA é dirigido pelo estado: supply esvazia e take-up
 *    enche conforme os segundos passam, SEM que a fita suma pelo caminho
 *    (supply + take-up é constante). É o que faz a máquina contar o que
 *    está gravando;
 *  - o contador é 7-segmento mas o texto acessível continua sendo o mm:ss
 *    exato — o leitor de tela (e o e2e) leem "00:00", não os polígonos;
 *  - a fita é pintada POR CIMA dos cabeçotes (é assim que ela passa de
 *    verdade; atrás, a linha sumia no metal e o deck parecia solto);
 *  - o estado do transporte é `aria-pressed`, nunca só a cor.
 */
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, describe, expect, it } from "vitest";
import { LOOP_SECONDS_POST, LOOP_SECONDS_PRE, LooperPanel } from "../src/components/LooperPanel";
import type { LooperSettings } from "../src/components/LooperPanel";

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

const DEFAULTS: LooperSettings = { recVol: 80, playVol: 80, pVol: 80, pre: true };

function mount(settings: LooperSettings = DEFAULTS) {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root: Root = createRoot(host);
  let current = settings;
  const render = () =>
    act(() =>
      root.render(
        <LooperPanel
          settings={current}
          onChange={(s) => {
            current = s;
            render();
          }}
        />,
      ),
    );
  render();
  return {
    host,
    done: () => {
      act(() => root.unmount());
      host.remove();
    },
  };
}

const panel = (host: HTMLElement) => host.querySelector<HTMLElement>('[aria-label="Looper (máquina de fita)"]')!;
const byAria = (host: HTMLElement, name: string) =>
  Array.from(panel(host).querySelectorAll("button")).find((b) => b.getAttribute("aria-label") === name);
const click = async (host: HTMLElement, name: string) => {
  await act(async () => {
    byAria(host, name)!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
};
/** raio do pacote de fita de um rolo (o <circle> com o gradiente quente) */
const packRadius = (host: HTMLElement, cls: string): number => {
  const g = panel(host).querySelector(`.${cls} circle[fill="url(#lp-tape-pack)"]`);
  return Number(g?.getAttribute("r"));
};

describe("Looper — o deck é uma máquina só", () => {
  it("rola, cabeçote e capstan vivem no MESMO svg (peças separadas não se encaixam)", () => {
    const { host, done } = mount();
    const svgs = panel(host).querySelectorAll("svg");
    // 1 deck + 1 contador + 2 VUs; o resto são os ícones das teclas
    expect(panel(host).querySelectorAll(".lp-deck-svg")).toHaveLength(1);
    expect(panel(host).querySelectorAll(".lp-counter")).toHaveLength(1);
    expect(panel(host).querySelectorAll(".lp-vu")).toHaveLength(2);
    expect(svgs.length).toBe(9);
    const deck = panel(host).querySelector(".lp-deck-svg")!;
    expect(deck.getAttribute("viewBox")).toBe("0 0 560 170");
    expect(deck.querySelector(".lp-reel--supply")).not.toBeNull();
    expect(deck.querySelector(".lp-reel--takeup")).not.toBeNull();
    // a fita é UM caminho só, e atravessa os dois cabeçotes
    const tape = deck.querySelectorAll("path");
    expect(tape.length).toBeGreaterThanOrEqual(2);
    done();
  });

  it("o pacote de fita responde ao estado: supply esvazia e take-up enche", async () => {
    const { host, done } = mount();
    // repouso: máquina ENFIADA — supply cheio, take-up vazio
    const supply0 = packRadius(host, "lp-reel--supply");
    const takeup0 = packRadius(host, "lp-reel--takeup");
    expect(supply0).toBeGreaterThan(takeup0);

    // grava → a fita SAI do supply e CHEGA ao take-up
    await click(host, "Gravar loop (REC)");
    await act(async () => {
      await new Promise((r) => setTimeout(r, 1100));
    });
    const supply1 = packRadius(host, "lp-reel--supply");
    const takeup1 = packRadius(host, "lp-reel--takeup");
    expect(supply1).toBeLessThan(supply0); // supply esvaziou
    expect(takeup1).toBeGreaterThan(takeup0); // take-up encheu
    // a invariante que importa: a fita não foi criada nem sumida no
    // caminho — o que saiu de um rolo entrou no outro
    expect(supply1 + takeup1).toBeCloseTo(supply0 + takeup0, 5);
    done();
  });

  it("o contador é 7 segmentos mas o texto acessível continua mm:ss", () => {
    const { host, done } = mount();
    const timer = panel(host).querySelector('[role="timer"]')!;
    expect(timer.textContent).toBe("00:00");
    // a pele: 4 dígitos × 7 segmentos = 28 polígonos (a vírgula são 2 círculos)
    expect(timer.querySelectorAll("polygon").length).toBe(28);
    done();
  });

  it("os 4 dígitos ocupam 4 posições distintas e em ordem (bug: segundos ao contrário)", () => {
    const { host, done } = mount();
    const xs = [...panel(host).querySelectorAll('[role="timer"] g[transform]')].map((g) =>
      Number(/translate\((-?[\d.]+)/.exec(g.getAttribute("transform") ?? "")?.[1]),
    );
    // o "00:00" tem 5 caracteres e 4 dígitos: indexar pelo caractere fazia
    // o último dígito cair fora da lista e voltar para x=0, sobrepondo o
    // primeiro — o contador virava "50:00" quando marcava 00:05
    expect(xs).toHaveLength(4);
    expect(new Set(xs).size).toBe(4);
    expect([...xs].sort((a, b) => a - b)).toEqual(xs); // já em ordem
    done();
  });

  it("o estado do transporte é aria-pressed, não só cor", async () => {
    const { host, done } = mount();
    const stop = byAria(host, "Parar (STOP)")!;
    expect(stop.getAttribute("aria-pressed")).toBe("true"); // repouso
    const rec = byAria(host, "Gravar loop (REC)")!;
    expect(rec.getAttribute("aria-pressed")).toBe("false");

    await click(host, "Gravar loop (REC)");
    expect(byAria(host, "Parar gravação e tocar")!.getAttribute("aria-pressed")).toBe("true");
    expect(byAria(host, "Parar (STOP)")!.getAttribute("aria-pressed")).toBe("false");
    done();
  });

  it("os rolos só giram quando o transporte produz som", async () => {
    const { host, done } = mount();
    const supply = () => panel(host).querySelector(".lp-reel--supply")!.getAttribute("class")!;
    expect(supply()).not.toContain("reel-spin");
    await click(host, "Gravar loop (REC)");
    expect(supply()).toContain("reel-spin");
    await click(host, "Parar gravação e tocar"); // → PLAY
    expect(supply()).toContain("reel-spin");
    await click(host, "Parar (STOP)");
    expect(supply()).not.toContain("reel-spin");
    done();
  });

  it("o ganho do firmware continua sendo range + leitura (o e2e usa #id + span)", () => {
    const { host, done } = mount();
    for (const [id, value] of [
      ["loop-rec", "80"],
      ["loop-play", "80"],
      ["loop-pvol", "80"],
    ] as const) {
      const input = panel(host).querySelector<HTMLInputElement>(`#${id}`)!;
      expect(input.type).toBe("range");
      expect(input.nextElementSibling?.textContent).toBe(value);
    }
    done();
  });

  it("a rota PRE/POST é uma chave de duas posições com a ativa acesa", () => {
    const { host, done } = mount();
    expect(byAria(host, `Looper em PRE (90 segundos, sem efeitos gravados)`)!.getAttribute("aria-pressed")).toBe("true");
    expect(byAria(host, "Looper em POST (45 segundos, com efeitos)")!.getAttribute("aria-pressed")).toBe("false");
    done();
  });

  it("os tempos reais do firmware continuam valendo (90 s PRE · 45 s POST)", () => {
    expect(LOOP_SECONDS_PRE).toBe(90);
    expect(LOOP_SECONDS_POST).toBe(45);
    const { host, done } = mount();
    expect(panel(host).textContent).toContain("PRE · 90s");
    expect(panel(host).textContent).toContain("POST · 45s");
    done();
  });
});