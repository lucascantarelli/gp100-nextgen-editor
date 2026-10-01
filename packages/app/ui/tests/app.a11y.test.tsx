/**
 * Teste de A11Y/estados do App — casca: biblioteca de fábrica (99 presets
 * reais do all.prst), board vazio com os 9 lugares marcados, trava ⇄ mover
 * no header e modal Settings. Pedais/knobs ganham testes próprios por
 * pedal quando entrarem no board.
 */
import { describe, expect, it, beforeAll } from "vitest";
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import App from "../src/App";
import { FACTORY_PRESETS } from "../src/artifacts/presetData";

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

function mount(ui: React.ReactElement): { root: Root; host: HTMLElement } {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  act(() => {
    root.render(ui);
  });
  return { root, host };
}

async function settle() {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
}

describe("App — casca (a11y e estados)", () => {
  it("renderiza topbar, biblioteca de fábrica e board vazio com 9 lugares", async () => {
    const { root, host } = mount(<App />);
    await settle();

    // patch corrente no display LED e na navbar
    expect(host.textContent).toContain("It's GP100");

    // 9 lugares da cadeia com os nomes das famílias
    const slots = Array.from(host.querySelectorAll('[aria-label^="Slot "]'));
    expect(slots.length).toBe(9);
    const fams = slots.map((s) => s.getAttribute("aria-label"));
    expect(fams).toContain("Slot 1: PRE");
    expect(fams).toContain("Slot 5: CAB");
    expect(fams).toContain("Slot 9: RVB");

    // biblioteca: os 99 de fábrica aparecem (lista com role listbox)
    const listbox = host.querySelector('[role="listbox"]');
    expect(listbox, "lista de presets acessível").not.toBeNull();
    const options = Array.from(listbox!.querySelectorAll('[role="option"]'));
    expect(options.length).toBe(99);
    // nomes reais do all.prst na lista
    expect(host.textContent).toContain("Dirty Funk");
    expect(host.textContent).toContain("Dreamy Aco");

    // ações globais com nome acessível
    const buttons = Array.from(host.querySelectorAll("button"));
    expect(buttons.some((b) => (b.getAttribute("aria-label") ?? "").startsWith("Trava de mover"))).toBe(true);
    expect(buttons.some((b) => b.getAttribute("aria-label") === "Abrir configurações")).toBe(true);
    expect(buttons.some((b) => (b.getAttribute("aria-label") ?? "").startsWith("Kill switch"))).toBe(true);

    act(() => root.unmount());
    host.remove();
  });

  it("drum: painel de ritmos com os 87 do firmware, gênero→estilo e compassos reais", async () => {
    const { root, host } = mount(<App />);
    await settle();

    // chip do topbar anuncia estilo/BPM/compasso e abre o painel
    const chip = Array.from(host.querySelectorAll("button")).find((b) =>
      (b.getAttribute("aria-label") ?? "").includes("abrir gestão de ritmos"),
    );
    expect(chip, "chip do drum presente").toBeTruthy();
    act(() => {
      chip!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await settle();

    const group = host.querySelector('[aria-label^="Gestão de ritmos"]');
    expect(group, "painel de ritmos aberto").not.toBeNull();

    // gênero → estilo dependente (Rock = 33 estilos do firmware)
    const genreSel = host.querySelector<HTMLSelectElement>("#drum-genre");
    expect(genreSel).not.toBeNull();
    const genreOpts = Array.from(genreSel!.options).map((o) => o.value);
    expect(genreOpts).toEqual(["Electronic", "Rock", "Pop", "World", "Jazz"]);

    const styleSel = host.querySelector<HTMLSelectElement>("#drum-style");
    const rockCount = Array.from(styleSel!.options).length;
    expect(rockCount).toBe(33);

    act(() => {
      genreSel!.value = "World";
      genreSel!.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await settle();
    const worldCount = Array.from(styleSel!.options).length;
    expect(worldCount).toBe(20);
    expect(host.textContent).toContain("87 ritmos");

    // compassos reais do firmware (inclui 6/8 e 9/8)
    const beatSel = host.querySelector<HTMLSelectElement>("#drum-beat");
    const beats = Array.from(beatSel!.options).map((o) => o.value);
    expect(beats).toEqual(["2/4", "3/4", "4/4", "6/4", "7/4", "6/8", "7/8", "9/8"]);

    act(() => root.unmount());
    host.remove();
  });

  it("settings: Footswitch Mode presente (desabilitado) e Global EQ com 5 bandas FREQ/Q/GAIN", async () => {
    const { root, host } = mount(<App />);
    await settle();

    const gear = Array.from(host.querySelectorAll("button")).find((b) => b.getAttribute("aria-label") === "Abrir configurações");
    act(() => {
      gear!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await settle();

    // Footswitch Mode visível e desabilitado (recurso do hardware ainda não ativo)
    const fsm = host.querySelector<HTMLSelectElement>('select[aria-label^="Footswitch mode"]');
    expect(fsm).not.toBeNull();
    expect(fsm!.disabled).toBe(true);

    // campos que NÃO existem no device não podem voltar a aparecer (Noise
    // Gate 1/2 e Noise Mode já vazaram para a UI por engano)
    expect(host.textContent).not.toContain("Noise Gate");
    expect(host.textContent).not.toContain("Noise Mode");

    // aba Global EQ: bandas com FREQ/Q/GAIN reais
    const eqTab = Array.from(host.querySelectorAll('[role="tab"]')).find((t) => t.textContent === "Global EQ");
    act(() => {
      eqTab!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await settle();
    expect(host.textContent).toContain("Band 1 FREQ");
    expect(host.textContent).toContain("Band 1 GAIN");

    act(() => root.unmount());
    host.remove();
  });

  it("trava de mover: drag-and-drop só com o modo ativo (aria-pressed)", async () => {
    const { root, host } = mount(<App />);
    await settle();

    const toggle = Array.from(host.querySelectorAll("button")).find(
      (b) => (b.getAttribute("aria-label") ?? "").startsWith("Trava de mover"),
    );
    expect(toggle, "botão de trava de mover presente").toBeTruthy();
    expect(toggle!.getAttribute("aria-pressed")).toBe("false");

    act(() => {
      toggle!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await settle();
    expect(toggle!.getAttribute("aria-pressed")).toBe("true");

    act(() => root.unmount());
    host.remove();
  });

  it("seleciona preset na biblioteca (fallback local determinístico)", async () => {
    const { root, host } = mount(<App />);
    await settle();

    const mist = Array.from(host.querySelectorAll('[role="option"]')).find((o) => o.textContent?.includes("Mist"));
    expect(mist, "preset P25 Mist visível").toBeTruthy();
    act(() => {
      mist!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await settle();

    // NOME do preset CLICADO aparece no LED e na navbar (o fallback do board
    // já ignorou o pp e congelou o nome em "It's GP100" — agora nome/tipo vêm
    // do mesmo artefato da biblioteca)
    const navbar = Array.from(host.querySelectorAll("strong")).find((s) => /^P\d{2}/.test(s.textContent ?? ""));
    expect(navbar?.textContent).toBe("P25 Mist");
    const board = host.querySelector('[aria-label^="Pedalboard"]');
    expect(board?.querySelector('[role="status"]')?.textContent).toContain("Mist");

    act(() => root.unmount());
    host.remove();
  });

  it("modal Settings abre, mostra abas e fecha com Esc", async () => {
    const { root, host } = mount(<App />);
    await settle();

    const gear = Array.from(host.querySelectorAll("button")).find((b) => b.getAttribute("aria-label") === "Abrir configurações");
    expect(gear).toBeTruthy();
    act(() => {
      gear!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await settle();

    const dialog = host.querySelector('[role="dialog"]');
    expect(dialog, "modal aberto").not.toBeNull();
    expect(host.textContent).toContain("Global EQ");
    expect(host.textContent).toContain("Release Note");

    act(() => {
      dialog!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    await settle();
    expect(host.querySelector('[role="dialog"]')).toBeNull();

    act(() => root.unmount());
    host.remove();
  });

  it("looper: máquina de fita com transporte acessível e tempos reais PRE/POST", async () => {
    const { root, host } = mount(<App />);
    await settle();

    const panel = host.querySelector('[aria-label="Looper (máquina de fita)"]');
    expect(panel, "painel do looper presente").not.toBeNull();

    // transporte com nomes acessíveis
    const byLabel = (s: string) => Array.from(panel!.querySelectorAll("button")).find((b) => (b.getAttribute("aria-label") ?? "") === s);
    const rec = byLabel("Gravar loop (REC)");
    const play = byLabel("Tocar loop (PLAY)");
    expect(rec).toBeTruthy();
    expect(play!.hasAttribute("disabled"), "PLAY desabilitado com fita vazia").toBe(true);

    // REC grava → PLAY habilita e status vira PLAY; timer aparece
    act(() => {
      rec!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await settle();
    expect(host.querySelector('[role="timer"]')?.textContent).toMatch(/\d{2}:\d{2}/);

    // PRE/POST mostram os tempos reais do device (90s/45s)
    expect(panel!.textContent).toContain("PRE · 90s");
    expect(panel!.textContent).toContain("POST · 45s");

    // parâmetros do firmware no rack
    for (const id of ["loop-rec", "loop-play", "loop-pvol"]) {
      expect(host.querySelector(`#${id}`), id).not.toBeNull();
    }

    act(() => root.unmount());
    host.remove();
  });

  it("catálogo gerado tem os 99 presets de fábrica com pp 0..98", () => {
    expect(FACTORY_PRESETS.length).toBe(99);
    expect(FACTORY_PRESETS[0]).toMatchObject({ pp: 0, name: "It's GP100" });
    expect(FACTORY_PRESETS[98]).toMatchObject({ pp: 98, name: "Dreamy Aco" });
    for (let i = 0; i < 99; i += 1) {
      expect(FACTORY_PRESETS[i].pp).toBe(i);
      expect(FACTORY_PRESETS[i].name.length).toBeGreaterThan(0);
    }
  });
});
