/**
 * A TELA do preset em arquivo (issue #114).
 *
 * O que este arquivo prova é o que o dono faz e o que ele vê quando dá errado:
 *
 * - os três caminhos existem e o botão da folha se ANUNCIA quando não há motor
 *   (em vez de existir para sempre falhar);
 * - exportar grava o arquivo com o nome do preset que está no palco;
 * - importar devolve a cadeia para o PALCO (o callback) e RELATA o que entrou;
 * - arquivo alheio vira relato com o motivo — não um banner de "tente de novo"
 *   ao lado de um arquivo que nunca vai mudar;
 * - falha de leitura (device) vira banner COM ação, e a ação recupera.
 *
 * O `pp` do palco é o preset de fábrica exportado; o teste usa o artefato real
 * (`FACTORY_PRESETS`) para não inventar nome.
 */
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterAll, beforeAll, beforeEach, describe, expect, it } from "vitest";
import { FACTORY_PRESETS } from "../src/artifacts/presetData";
import { PresetFilePanel } from "../src/components/PresetFilePanel";
import { MSG } from "../src/i18n/messages";
import { presetExportJson } from "../src/ipc/preset";
import type { BoardView } from "../src/ipc/types";

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

/** Os downloads que a tela disparou (o `<a download>` clicado). */
const baixados: string[] = [];
let clickOriginal: (() => void) | null = null;

beforeEach(() => {
  localStorage.clear();
  baixados.length = 0;
  clickOriginal = HTMLAnchorElement.prototype.click;
  HTMLAnchorElement.prototype.click = function clickFalso(this: HTMLAnchorElement) {
    baixados.push(this.download);
  };
  // jsdom TEM `URL.createObjectURL`, mas ele quebra com um `Blob` vindo do
  // realm do Node (`Cannot read properties of undefined (reading '_bytes')`).
  URL.createObjectURL = () => "blob:fake";
});

afterAll(() => {
  if (clickOriginal) HTMLAnchorElement.prototype.click = clickOriginal;
});

interface Cena {
  host: HTMLElement;
  board: () => BoardView | null;
  fechou: () => number;
  fecha: () => void;
}

function monta(pp = 0): Cena {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root: Root = createRoot(host);
  let recebido: BoardView | null = null;
  let fechou = 0;
  act(() =>
    root.render(
      <PresetFilePanel
        pp={pp}
        onImport={(b) => {
          recebido = b;
        }}
        onClose={() => {
          fechou += 1;
        }}
      />,
    ),
  );
  return {
    host,
    board: () => recebido,
    fechou: () => fechou,
    fecha: () => {
      act(() => root.unmount());
      host.remove();
    },
  };
}

async function flush(): Promise<void> {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
}

async function clique(el: Element): Promise<void> {
  await act(async () => {
    el.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
}

/** Espera uma condição (o `runCommand` repete com backoff — ver #20). */
async function espera(pred: () => boolean, ms = 6000): Promise<void> {
  const fim = Date.now() + ms;
  while (!pred() && Date.now() < fim) {
    // O timer real tambem DENTRO do act: a promessa do shell resolve nesta
    // janela, e o update de estado fora do act e o aviso "not wrapped in act"
    // (o `waitFor` de `app.interactions.test.tsx` ja fazia assim — issue #142).
    await act(async () => {
      await flush();
      await new Promise((r) => setTimeout(r, 20));
    });
  }
  expect(pred(), "a tela nao chegou ao estado esperado").toBe(true);
}

function botao(host: HTMLElement, aria: string): HTMLButtonElement {
  return host.querySelector<HTMLButtonElement>(`button[aria-label="${aria}"]`)!;
}

/** Entrega um arquivo ao input invisível (o clique é do dono). */
async function escolheArquivo(host: HTMLElement, conteudo: string): Promise<void> {
  const input = host.querySelector<HTMLInputElement>('input[type="file"]')!;
  const file = new File([conteudo], "gp100.preset.json", { type: "application/json" });
  Object.defineProperty(input, "files", { value: [file], configurable: true });
  await act(async () => {
    input.dispatchEvent(new Event("change", { bubbles: true }));
  });
}

const nomeDe = (pp: number): string => FACTORY_PRESETS.find((p) => p.pp === pp)?.name ?? "";

describe("PresetFilePanel — o que a tela oferece", () => {
  it("diz QUAL preset está exportando e oferece os três caminhos", () => {
    const c = monta(0);
    const dlg = c.host.querySelector<HTMLElement>('[role="dialog"]')!;
    expect(dlg.getAttribute("aria-label")).toBe(MSG.presetFileTitle);
    expect(dlg.textContent).toContain(MSG.presetFileTarget(MSG.libPp(0), nomeDe(0)));
    expect(botao(c.host, MSG.presetFileExportAria).textContent).toBe(MSG.presetFileExport);
    expect(botao(c.host, MSG.presetFileImportAria).textContent).toBe(MSG.presetFileImport);
    // e diz que importar NÃO grava no aparelho
    expect(dlg.textContent).toContain(MSG.presetFileNote);
    c.fecha();
  });

  it("a folha de timbre se ANUNCIA quando não há motor (não é botão que sempre falha)", () => {
    const c = monta(0);
    const folha = botao(c.host, MSG.presetFileSheetAria);
    expect(folha.disabled).toBe(true);
    expect(folha.title).toBe(MSG.presetFileSheetHint);
    expect(folha.textContent).toContain(MSG.presetFileSheet);
    expect(folha.textContent).toContain(MSG.presetFileSheetOff);
    c.fecha();
  });

  it("o ✕ fecha a tela pelo dono", async () => {
    const c = monta(0);
    await clique(botao(c.host, MSG.presetFileClose));
    expect(c.fechou()).toBe(1);
    c.fecha();
  });
});

describe("PresetFilePanel — exportar", () => {
  it("grava o JSON com o nome do preset do palco", async () => {
    const c = monta(0);
    await clique(botao(c.host, MSG.presetFileExportAria));
    await espera(() => baixados.length > 0);
    expect(baixados).toEqual([`gp100.preset.${MSG.libPp(0)}.json`]);
    c.fecha();
  });

  it("falha de leitura vira banner com AÇÃO, e a ação recupera", async () => {
    localStorage.setItem("gp100.debug.failDevice", "board");
    const c = monta(0);
    await clique(botao(c.host, MSG.presetFileExportAria));
    await espera(() => c.host.querySelector('[role="alert"]') != null);
    expect(c.host.querySelector('[role="alert"]')!.textContent).toContain(MSG.presetFileFailed);

    // o device volta e a MESMA ação (tentar de novo) completa o download
    localStorage.removeItem("gp100.debug.failDevice");
    await clique(c.host.querySelector('[role="alert"]')!.querySelector("button")!);
    await espera(() => baixados.length > 0);
    expect(baixados).toEqual([`gp100.preset.${MSG.libPp(0)}.json`]);
    // `espera` volta ASSIM QUE o download aparece; o painel ainda limpa o
    // banner depois disso. O flush final (macrotask + microtasks, dentro do
    // `act`) da tempo a essa cadeia curta de resolver antes do `fecha()`.
    await act(async () => {
      await new Promise((r) => setTimeout(r, 20));
    });
    await flush();
    c.fecha();
  });
});

describe("PresetFilePanel — importar", () => {
  it("devolve a cadeia do arquivo para o palco e relata o que entrou", async () => {
    const c = monta(0);
    await escolheArquivo(c.host, await presetExportJson(1));
    await flush();
    const board = c.board();
    expect(board?.pp).toBe(1);
    expect(board?.slots).toHaveLength(9);
    const status = c.host.querySelector('[role="status"]')!;
    // **(#150)** o relato carrega o nome que VEIO NO ARQUIVO — e o export do
    // fallback de teste sai com a fixture rotulada (não é nome de fábrica).
    expect(status.textContent).toContain(
      MSG.presetFileImported(MSG.libPp(1), "Fixture de teste (não é aparelho)", 9),
    );
    // e NADA foi para o aparelho: nenhum banner de erro sobrou
    expect(c.host.querySelector('[role="alert"]')).toBeNull();
    c.fecha();
  });

  it("arquivo alheio vira relato com o MOTIVO (e não banner de tentar de novo)", async () => {
    const c = monta(0);
    await escolheArquivo(c.host, JSON.stringify({ format: "gp100.library", version: 1 }));
    await flush();
    expect(c.board()).toBeNull();
    const status = c.host.querySelector('[role="status"]')!.textContent ?? "";
    expect(status).toContain("Arquivo recusado");
    expect(status).toContain("gp100.library");
    expect(c.host.querySelector('[role="alert"]')).toBeNull();
    c.fecha();
  });

  it("o relato pode ser fechado (o dono já leu)", async () => {
    const c = monta(0);
    await escolheArquivo(c.host, JSON.stringify({ format: "gp100.library", version: 1 }));
    await flush();
    const status = c.host.querySelector('[role="status"]')!;
    await clique(status.querySelector("button")!);
    expect(c.host.querySelector('[role="status"]')).toBeNull();
    c.fecha();
  });
});
