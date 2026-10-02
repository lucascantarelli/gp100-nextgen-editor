/**
 * Fundação do PALCO sob teste direto (unit, jsdom — a base declarada da
 * Fase 2 de pedais, antes coberta só "de raspão" pelo App):
 *   - fxModels: variante conhecida, fallback por archetype e capacidade
 *     para TODOS os slots possíveis do mock (contrato de nunca-undefined);
 *   - Pedalboard: 1 fileira de 9 pedais, display LED, ordem custom com slots
 *     ausentes preservados e pulsos de sinal por par ON;
 *   - Pedal: LED on/off, truncamento de nome e ValueBox (Enter commit,
 *     Esc restaura);
 *   - Knob: teclado (setas/Shift/Home/End), duplo-clique = reset e
 *     ciclo de switch/combox.
 */
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { Pedalboard } from "../src/components/Pedalboard";
import { Pedal } from "../src/components/Pedal";
import { Knob } from "../src/components/Knob";
import { PEDAL_FAMILIES_READY, Stage } from "../src/components/Stage";
import { MODAL_SCALE, PedalModal } from "../src/components/PedalModal";
import { loadTuner } from "../src/components/TunerPanel";
import { modelFor } from "../src/artifacts/fxModels";
import { CHAIN_FAMILIES } from "../src/ipc/types";
import type { BoardKnob, BoardSlot, BoardView } from "../src/ipc/types";

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

const ARCHETYPE_OF = {
  PRE: "BUFFER",
  DST: "DISTORTION",
  AMP: "AMPLIFIER",
  NR: "NOISEGATE",
  CAB: "CABINET",
  EQ: "EQ",
  MOD: "MODULATION",
  DLY: "DELAY",
  RVB: "REVERB",
} as const;

function mkSlot(i: number, family: BoardSlot["family"], variant = "comp"): BoardSlot {
  return {
    slot: i,
    family,
    archetype: ARCHETYPE_OF[family],
    name: `${family}-${i}`,
    variant,
    code: i + 1,
    state: true,
    knobs: [
      { name: "Gain", pos: 0, kind: "knob", options: [], value: "50", default: "50", range: [0, 100] },
      { name: "Mode", pos: 1, kind: "switch", options: ["off", "on1", "on2"], value: "off", default: "off" },
      { name: "Type", pos: 2, kind: "combox", options: ["A", "B"], value: "A", default: "A" },
    ],
  };
}

const BOARD: BoardView = {
  pp: 0,
  name: "Preset Teste",
  ppType: 0,
  ppTypeName: "Factory",
  slots: CHAIN_FAMILIES.map((f, i) => mkSlot(i, f)),
};

function mount(ui: React.ReactElement): { root: Root; host: HTMLElement } {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  act(() => root.render(ui));
  return { root, host };
}

const noop = () => {};
const noopSlot = () => {};

function groupLabels(host: HTMLElement): string[] {
  return Array.from(host.querySelectorAll('svg[role="group"]')).map((g) =>
    g.getAttribute("aria-label") ?? "",
  );
}

describe("fxModels — catálogo de modelos por variante", () => {
  it("variante conhecida resolve a entrada do catálogo (a-chorus → box CE-2)", () => {
    const m = modelFor({ ...mkSlot(0, "MOD"), variant: "a-chorus" });
    expect(m.shape).toBe("box");
    expect(m.ref).toContain("CE-2");
  });

  it("variante desconhecida cai no fallback do archetype (nunca undefined)", () => {
    const m = modelFor({ ...mkSlot(0, "RVB"), variant: "nao-existe-xyz" });
    expect(m.shape).toBe("widebox");
    expect(m.ref).toContain("big box");
  });

  it("capacidade garantida: TODOS os 9 slots do mock resolvem modelo", () => {
    for (const s of BOARD.slots) {
      const m = modelFor(s);
      expect(m.w).toBeGreaterThan(0);
      expect(m.cols).toBeGreaterThan(0);
      expect(m.ref.length).toBeGreaterThan(0);
    }
  });
});

describe("Pedalboard — o palco em 1 fileira de 9 pedais", () => {
  it("monta os 9 pedais com display LED do preset (pp + nome + contagem ON)", () => {
    const { root, host } = mount(
      <Pedalboard
        board={BOARD}
        states={{}}
        onToggle={noopSlot}
        onKnobChange={noopSlot}
        onKnobReset={noopSlot}
        onReorder={noop}
      />,
    );
    expect(groupLabels(host).length).toBe(9);
    const status = host.querySelector('[role="status"]');
    expect(status?.textContent).toContain("00");
    expect(status?.textContent).toContain("Preset Teste");
    expect(status?.textContent).toContain("9/9"); // states vazio = estado do slot (todos ON)
    // cadeia inteira em 1 fileira: 9 pedais lado a lado, nenhum vazio
    expect(host.querySelectorAll('svg[role="group"]').length).toBe(9);
    act(() => root.unmount());
    host.remove();
  });

  it("reorder: ordem custom aplicada e slots ausentes preservados no fim", () => {
    const { root, host } = mount(
      <Pedalboard
        board={BOARD}
        states={{}}
        order={[4, 0, 1, 2, 3, 5, 6, 7]} // CAB vai para a 1ª posição; RVB-8 fica de fora
        onToggle={noopSlot}
        onKnobChange={noopSlot}
        onKnobReset={noopSlot}
        onReorder={noop}
      />,
    );
    const labels = groupLabels(host);
    expect(labels.length).toBe(9); // nenhum slot perdido
    expect(labels[0]).toContain("CAB-4"); // slot 4 (CAB) vai para a 1ª posição
    expect(labels[8]).toContain("RVB-8"); // slot 8, ausente da ordem → preservado no fim
    // depois do reorder a cadeia ainda é 1 fileira de 9 pedais
    expect(host.querySelectorAll('svg[role="group"]').length).toBe(9);
    act(() => root.unmount());
    host.remove();
  });

  it("pulsos de sinal: só em cabos entre pares ON (com todos ON: 6 segmentos)", () => {
    const { root, host } = mount(
      <Pedalboard
        board={BOARD}
        states={{}}
        onToggle={noopSlot}
        onKnobChange={noopSlot}
        onKnobReset={noopSlot}
        onReorder={noop}
      />,
    );
    // o Pedalboard artístico monta 3 fileiras de 3 pedais; cada fileira tem
    // 2 cabos entre vizinhos → 3 × 2 = 6 pulsos com todos os pares ON
    expect(host.querySelectorAll('circle[fill="#ffd23f"]').length).toBe(6);
    act(() => root.unmount());
    host.remove();
  });

  it("pares com pedal OFF não geram pulso (states sobrepõe o estado do slot)", () => {
    const states = Object.fromEntries(CHAIN_FAMILIES.map((_, i) => [i, i % 2 === 0]));
    const { root, host } = mount(
      <Pedalboard
        board={BOARD}
        states={states}
        onToggle={noopSlot}
        onKnobChange={noopSlot}
        onKnobReset={noopSlot}
        onReorder={noop}
      />,
    );
    expect(host.querySelectorAll('circle[fill="#ffd23f"]').length).toBe(0);
    act(() => root.unmount());
    host.remove();
  });

  it("toggle do footswitch sobe o callback com o slot certo (PRE-0)", () => {
    const onToggle = vi.fn();
    const { root, host } = mount(
      <Pedalboard
        board={BOARD}
        states={{}}
        onToggle={onToggle}
        onKnobChange={noopSlot}
        onKnobReset={noopSlot}
        onReorder={noop}
      />,
    );
    const foot = Array.from(host.querySelectorAll('[role="button"]')).find((b) =>
      (b.getAttribute("aria-label") ?? "").startsWith("Desligar efeito"),
    );
    expect(foot, "footswitch do 1º pedal (ON → Desligar)").toBeTruthy();
    act(() => foot!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    expect(onToggle).toHaveBeenCalledTimes(1);
    expect(onToggle.mock.calls[0][0]).toMatchObject({ slot: 0, family: "PRE" });
    act(() => root.unmount());
    host.remove();
  });
});

describe("Pedal — o pedal individual", () => {
  it("LED verde quando ON e vermelho quando OFF (states sobrepõe)", () => {
    const { root, host } = mount(
      <Pedalboard
        board={BOARD}
        states={{ 0: false }}
        onToggle={noopSlot}
        onKnobChange={noopSlot}
        onKnobReset={noopSlot}
        onReorder={noop}
      />,
    );
    const leds = host.querySelectorAll("[data-led]");
    expect(leds.length).toBe(9);
    expect(host.querySelector('[data-led="off"]')).toBeTruthy(); // slot 0 forçado OFF
    expect(host.querySelectorAll('[data-led="on"]').length).toBe(8);
    act(() => root.unmount());
    host.remove();
  });

  it("nome longo é truncado com reticências (o corte do palco é pela LARGURA)", () => {
    const long = { ...mkSlot(0, "PRE"), name: "N".repeat(30) };
    const board = mount(
      <Pedal slot={long} onToggle={noopSlot} onKnobChange={noopSlot} onKnobReset={noopSlot} />,
    );
    const boardTexts = Array.from(board.host.querySelectorAll("text")).map(
      (t) => t.textContent ?? "",
    );
    expect(boardTexts.some((t) => t.endsWith("…")), "corte com reticências").toBe(true);
    expect(boardTexts.every((t) => t.length < 30), "o nome inteiro não vaza").toBe(true);
    expect(boardTexts.some((t) => t === `${"N".repeat(25)}…`), "no compacto cabe menos").toBe(false);
    act(() => board.root.unmount());
    board.host.remove();

    // o modal é a escala GRANDE: mais caracteres antes do corte
    const modal = mount(
      <Pedal slot={long} variant="modal" onToggle={noopSlot} onKnobChange={noopSlot} onKnobReset={noopSlot} />,
    );
    const modalTexts = Array.from(modal.host.querySelectorAll("text")).map(
      (t) => t.textContent ?? "",
    );
    expect(modalTexts.some((t) => t === `${"N".repeat(25)}…`)).toBe(true);
    act(() => modal.root.unmount());
    modal.host.remove();
  });

  it("palco (compacto): valor de cada knob em TEXTO e knobs travados, sem caixa de edição", () => {
    const { root, host } = mount(
      <Pedal slot={mkSlot(0, "PRE")} onToggle={noopSlot} onKnobChange={noopSlot} onKnobReset={noopSlot} />,
    );
    const values = Array.from(host.querySelectorAll("[data-value]")).map((t) =>
      t.getAttribute("data-value"),
    );
    expect(values, "um valor por knob (numérico, switch e combox)").toEqual(["50", "off", "A"]);
    expect(host.querySelectorAll('svg[role="img"]').length, "3 knobs travados").toBe(3);
    expect(host.querySelectorAll('svg[role="slider"]').length).toBe(0);
    expect(
      host.querySelector('input[aria-label="Valor (Enter para editar)"]'),
      "sem textbox no palco — edição só no modal",
    ).toBeNull();
    act(() => root.unmount());
    host.remove();
  });

  it("modal (escala grande): ValueBox Enter aplica; Esc restaura; valor não-numérico é ignorado", () => {
    const onKnobChange = vi.fn();
    const { root, host } = mount(
      <Pedal
        slot={mkSlot(0, "PRE")}
        variant="modal"
        onToggle={noopSlot}
        onKnobChange={onKnobChange}
        onKnobReset={noopSlot}
      />,
    );
    const box = host.querySelector<HTMLInputElement>('input[aria-label="Valor (Enter para editar)"]');
    expect(box, "caixa de valor em modo leitura").toBeTruthy();
    expect(box!.value).toBe("50");

    // click → edição (input de draft aparece com aria-label de edição)
    act(() => box!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    const editing = host.querySelector<HTMLInputElement>('input[aria-label^="Valor do knob"]');
    expect(editing).toBeTruthy();
    const target = editing as HTMLInputElement;

    const commit = (raw: string, key: string) => {
      act(() => {
        const setter = Object.getOwnPropertyDescriptor(
          window.HTMLInputElement.prototype,
          "value",
        )!.set!;
        setter.call(target, raw);
        target.dispatchEvent(new Event("input", { bubbles: true }));
        target.dispatchEvent(
          new KeyboardEvent("keydown", { key, bubbles: true }),
        );
      });
    };

    commit("12.7", "Enter");
    expect(onKnobChange).toHaveBeenCalledTimes(1);
    expect(onKnobChange.mock.calls[0][0]).toMatchObject({ family: "PRE" });
    expect(onKnobChange.mock.calls[0][2]).toBe("13"); // range inteiro → Math.round

    // commit inválido (NaN) não chama o callback
    act(() => box!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    commit("abc", "Enter");
    expect(onKnobChange).toHaveBeenCalledTimes(1);

    // Esc restaura: volta ao modo leitura mostrando o valor da PROP (o
    // parent é estático aqui — o commit de verdade já foi provado pelo mock)
    commit("99", "Escape");
    expect(onKnobChange).toHaveBeenCalledTimes(1);
    expect(
      host.querySelector<HTMLInputElement>('input[aria-label="Valor (Enter para editar)"]')!.value,
    ).toBe("50");
    act(() => root.unmount());
    host.remove();
  });
});

describe("Knob — controle rotativo paramétrico", () => {
  const KNOB: BoardKnob = {
    name: "Tone",
    pos: 2,
    kind: "knob",
    options: [],
    value: "50",
    default: "50",
    range: [0, 100],
  };

  it("é um slider acessível com aria now/min/max", () => {
    const onChange = vi.fn();
    const onReset = vi.fn();
    const { root, host } = mount(<Knob knob={KNOB} onChange={onChange} onReset={onReset} />);
    const svg = host.querySelector('svg[role="slider"][aria-label="Tone"]');
    expect(svg).toBeTruthy();
    expect(svg!.getAttribute("aria-valuenow")).toBe("50");
    expect(svg!.getAttribute("aria-valuemin")).toBe("0");
    expect(svg!.getAttribute("aria-valuemax")).toBe("100");
    act(() => root.unmount());
    host.remove();
  });

  it("teclado: ↑ 1%, Shift+↓ 5%, Home/End/PageUp/PageDown (cada passo do valor de partida)", () => {
    const onChange = vi.fn();
    const onReset = vi.fn();
    // O Knob é STATELESS sobre value: cada passo parte do valor da prop e
    // o parent re-renderiza (Pedalboard). Cada passo aqui = mount com o
    // valor de partida, provando o delta exato da tecla.
    const pressFrom = (start: string, key: string, expected: string, shift = false) => {
      const k: BoardKnob = { ...KNOB, value: start };
      const h = mount(<Knob knob={k} onChange={onChange} onReset={onReset} />);
      const svg = h.host.querySelector('svg[role="slider"]')!;
      act(() => svg.dispatchEvent(new KeyboardEvent("keydown", { key, shiftKey: shift, bubbles: true })));
      expect(onChange).toHaveBeenLastCalledWith(2, expected);
      h.root.unmount();
      h.host.remove();
    };
    pressFrom("50", "ArrowUp", "51");
    pressFrom("50", "ArrowDown", "49");
    pressFrom("50", "ArrowDown", "45", true); // Shift = 5%
    pressFrom("50", "Home", "0");
    pressFrom("50", "End", "100");
    pressFrom("100", "PageDown", "95");
    pressFrom("0", "PageUp", "5");
  });

  it("duplo clique reseta ao default e range fracionário formata com 1 decimal", () => {
    const onChange = vi.fn();
    const onReset = vi.fn();
    const { root, host } = mount(<Knob knob={KNOB} onChange={onChange} onReset={onReset} />);
    const svg = host.querySelector('svg[role="slider"]')!;
    act(() => svg.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    expect(onReset).toHaveBeenCalledWith(2);
    act(() => root.unmount());
    host.remove();

    const frac: BoardKnob = { ...KNOB, range: [0, 0.5], value: "0.25", default: "0.25" };
    const host2 = mount(<Knob knob={frac} onChange={onChange} onReset={onReset} />);
    const svg2 = host2.host.querySelector('svg[role="slider"]')!;
    act(() => svg2.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true })));
    const v = onChange.mock.lastCall?.[1] as string;
    expect(v).toMatch(/^\d+\.\d$/); // 1 decimal em range fracionário
    host2.root.unmount();
    host2.host.remove();
  });

  it("switch e combox ciclam opções por clique (parent re-renderiza); knob não cicla", () => {
    const onChange = vi.fn();
    const onReset = vi.fn();
    // O Knob é STATELESS sobre value: o ciclo vem do callback + parent
    // re-renderizando com o novo valor. Cada passo = remount com o valor
    // atualizado (mesma sequência do Pedalboard real).
    const cycleStep = (value: string, expected: string) => {
      const sw: BoardKnob = { name: "Mode", pos: 1, kind: "switch", options: ["off", "on1", "on2"], value, default: "off" };
      const h = mount(<Knob knob={sw} onChange={onChange} onReset={onReset} />);
      const svg = h.host.querySelector('svg[role="button"][aria-label="Mode"]')!;
      act(() => svg.dispatchEvent(new MouseEvent("click", { bubbles: true })));
      expect(onChange).toHaveBeenLastCalledWith(1, expected);
      h.root.unmount();
      h.host.remove();
    };
    cycleStep("off", "on1");
    cycleStep("on1", "on2");
    cycleStep("on2", "off"); // ciclo fecha

    // combox: mesmo caminho de ciclo (A → B → A)
    const cycleCombox = (value: string, expected: string) => {
      const cb: BoardKnob = { name: "Type", pos: 2, kind: "combox", options: ["A", "B"], value, default: "A" };
      const h2 = mount(<Knob knob={cb} onChange={onChange} onReset={onReset} />);
      const svg2 = h2.host.querySelector('svg[role="button"][aria-label="Type"]')!;
      act(() => svg2.dispatchEvent(new MouseEvent("click", { bubbles: true })));
      expect(onChange).toHaveBeenLastCalledWith(2, expected);
      h2.root.unmount();
      h2.host.remove();
    };
    cycleCombox("A", "B");
    cycleCombox("B", "A");

    // knob NÃO cicla por clique (arrasto/teclado é o caminho)
    const before = onChange.mock.calls.length;
    const h3 = mount(<Knob knob={KNOB} onChange={onChange} onReset={onReset} />);
    const svg3 = h3.host.querySelector('svg[role="slider"]')!;
    act(() => svg3.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    expect(onChange.mock.calls.length).toBe(before);
    h3.root.unmount();
    h3.host.remove();
  });
});

describe("Stage — palco real da Fase 2 (cadeia inteira)", () => {
  const stageProps = {
    celebrate: false,
    arrangeMode: false,
    onToggleArrange: noop,
    tuner: loadTuner(),
    onTunerChange: noop,
    onToggle: noopSlot,
    onKnobChange: noopSlot,
    onKnobReset: noopSlot,
    onEdit: noopSlot,
  };

  it("a cadeia INTEIRA vira pedal REAL: as 9 famílias (visão completa)", () => {
    const { root, host } = mount(<Stage board={BOARD} {...stageProps} />);
    for (const fam of CHAIN_FAMILIES) {
      expect(PEDAL_FAMILIES_READY.has(fam), `${fam} desenha pedal no palco`).toBe(true);
    }
    expect(groupLabels(host).length, "9 pedais no palco").toBe(9);
    expect(groupLabels(host)[0]).toContain("PRE-0");
    expect(groupLabels(host)[8]).toContain("RVB-8");
    expect(
      host.querySelector('[aria-label="Slot 5: CAB"] svg[role="group"]'),
      "o pedal ocupa o slot da própria família",
    ).toBeTruthy();
    // 9 lugares, nenhum vazio: o hint só existe ANTES da 1ª leitura do board
    expect(host.querySelectorAll('[aria-label^="Slot "]').length).toBe(9);
    const hints = Array.from(host.querySelectorAll("span")).filter(
      (s) => s.textContent === "vazio",
    );
    expect(hints.length, "0 lugares vazios").toBe(0);
    act(() => root.unmount());
    host.remove();
  });

  it("board ausente (1ª leitura): 9 placeholders, nenhum pedal", () => {
    const { root, host } = mount(<Stage board={null} {...stageProps} />);
    expect(groupLabels(host).length).toBe(0);
    expect(host.querySelectorAll('[aria-label^="Slot "]').length).toBe(9);
    act(() => root.unmount());
    host.remove();
  });

  it("com a trava ATIVA o pedal reordena (prévia local) sem perder nenhum lugar", () => {
    const { root, host } = mount(<Stage board={BOARD} {...stageProps} arrangeMode />);
    const slot = (n: number, fam: string) =>
      host.querySelector(`[aria-label="Slot ${n}: ${fam}"]`)!;
    // arrasta o pedal do slot 1 (PRE) para a posição 5
    act(() => {
      slot(1, "PRE").dispatchEvent(new Event("dragstart", { bubbles: true }));
      slot(5, "CAB").dispatchEvent(new Event("dragover", { bubbles: true }));
      slot(5, "CAB").dispatchEvent(new Event("drop", { bubbles: true }));
    });
    expect(slot(5, "PRE").querySelector('svg[role="group"]')).toBeTruthy();
    expect(groupLabels(host).length).toBe(9);
    expect(host.querySelectorAll('[aria-label^="Slot "]').length).toBe(9);
    act(() => root.unmount());
    host.remove();
  });

  it("knobs do palco são SÓ LEITURA; clique em qualquer ponto do pedal abre a edição", () => {
    const onEdit = vi.fn();
    const { root, host } = mount(<Stage board={BOARD} {...stageProps} onEdit={onEdit} />);
    // knob travado: role img (sem foco/aria-valuenow) e valor em texto — quem
    // ajusta é o modal, então o clique sobre o knob TAMBÉM abre a edição
    const knob = host.querySelector('[aria-label="Slot 1: PRE"] svg[role="img"]')!;
    expect(knob, "knob do palco é display (não slider)").toBeTruthy();
    expect(knob.getAttribute("tabindex")).toBeNull();
    expect(knob.getAttribute("aria-label")).toContain("Gain: 50");
    expect(host.querySelector('[aria-label="Slot 1: PRE"] svg[role="slider"]')).toBeNull();
    act(() => knob.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    expect(onEdit).toHaveBeenCalledTimes(1);
    expect(onEdit.mock.calls[0][0]).toMatchObject({ slot: 0, family: "PRE" });

    // footswitch continua com o clique próprio (liga/desliga o efeito)
    const foot = host.querySelector('[aria-label="Slot 1: PRE"] [role="button"]')!;
    act(() => foot.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    expect(onEdit).toHaveBeenCalledTimes(1);

    // corpo do pedal (svg do grupo) abre com o slot certo
    const pedal = host.querySelector('[aria-label="Slot 1: PRE"] svg[role="group"]')!;
    act(() => pedal.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    expect(onEdit).toHaveBeenCalledTimes(2);
    expect(onEdit.mock.calls[1][0]).toMatchObject({ slot: 0, family: "PRE" });

    // teclado: Enter no grupo do pedal também abre (R7 — acessível sem mouse)
    act(() =>
      host
        .querySelector('[aria-label="Slot 1: PRE"]')!
        .dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })),
    );
    expect(onEdit).toHaveBeenCalledTimes(3);
    act(() => root.unmount());
    host.remove();
  });

  it("com a trava ATIVA o clique não abre a edição (o clique pertence ao drag)", () => {
    const onEdit = vi.fn();
    const { root, host } = mount(<Stage board={BOARD} {...stageProps} arrangeMode onEdit={onEdit} />);
    act(() =>
      host
        .querySelector('[aria-label="Slot 1: PRE"] svg[role="group"]')!
        .dispatchEvent(new MouseEvent("click", { bubbles: true })),
    );
    expect(onEdit).not.toHaveBeenCalled();
    act(() => root.unmount());
    host.remove();
  });

  it("modo engenheiro: tooltip com addr/code/ctrl; desligado fica o texto simples", () => {
    // desligado (default): tooltip de uso (palco = só leitura), SEM endereço
    // e SEM vocabulário interno
    const off = mount(<Stage board={BOARD} {...stageProps} />);
    const titleOff = off.host.querySelector('svg[role="img"] title')!.textContent ?? "";
    expect(titleOff).toContain("clique no pedal para editar");
    expect(titleOff).not.toContain("addr");
    act(() => off.root.unmount());
    off.host.remove();

    // ligado: addr do SET (10 01 00 02), code do efeito e ctrl do knob
    const on = mount(<Stage board={BOARD} {...stageProps} engineer />);
    const titleOn = on.host.querySelector('svg[role="img"] title')!.textContent ?? "";
    expect(titleOn).toContain("SET");
    expect(titleOn).toContain("addr 10 01 00 02");
    expect(titleOn).toContain("code 0x00000001");
    expect(titleOn).toContain("ctrl 0");
    expect(titleOn, "referência à doc interna não vaza (Q-8)").not.toContain("§13.11");
    act(() => on.root.unmount());
    on.host.remove();
  });

  it("sem a trava o arrasto do pedal não reordena (protege o knob)", () => {
    const { root, host } = mount(<Stage board={BOARD} {...stageProps} />);
    const slot = (n: number, fam: string) =>
      host.querySelector(`[aria-label="Slot ${n}: ${fam}"]`)!;
    act(() => {
      slot(1, "PRE").dispatchEvent(new Event("dragstart", { bubbles: true }));
      slot(5, "CAB").dispatchEvent(new Event("dragover", { bubbles: true }));
      slot(5, "CAB").dispatchEvent(new Event("drop", { bubbles: true }));
    });
    expect(slot(1, "PRE").querySelector('svg[role="group"]')).toBeTruthy();
    act(() => root.unmount());
    host.remove();
  });
});

describe("PedalModal — edição ampliada do pedal (U-3)", () => {
  it("amplia o pedal (1.25×) com os mesmos controles; ✕, Esc e backdrop fecham", () => {
    const onClose = vi.fn();
    const slot = mkSlot(0, "PRE");
    const { root, host } = mount(
      <PedalModal
        slot={slot}
        onToggle={noopSlot}
        onKnobChange={noopSlot}
        onKnobReset={noopSlot}
        onClose={onClose}
      />,
    );
    const dlg = host.querySelector('[role="dialog"]')!;
    expect(dlg).toBeTruthy();
    expect(dlg.getAttribute("aria-label")).toBe("Edição do pedal PRE-0");
    // pedal AMPLIADO: transform scale(1.25) (knob 64 → 80px) e controles vivos
    const scaled = Array.from(dlg.querySelectorAll<HTMLDivElement>("div")).find(
      (d) => d.style.transform !== "",
    );
    expect(scaled?.style.transform).toBe(`scale(${MODAL_SCALE})`);
    expect(dlg.querySelectorAll('svg[role="slider"], svg[role="button"]').length).toBe(3);

    // três saídas: ✕, Esc e clique no backdrop
    act(() =>
      dlg.querySelector<HTMLButtonElement>("button")!.dispatchEvent(new MouseEvent("click", { bubbles: true })),
    );
    expect(onClose).toHaveBeenCalledTimes(1);
    act(() => dlg.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(onClose).toHaveBeenCalledTimes(2);
    act(() =>
      dlg.parentElement!.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })),
    );
    expect(onClose).toHaveBeenCalledTimes(3);
    act(() => root.unmount());
    host.remove();
  });

  it("slot null não renderiza nada (fechado)", () => {
    const { root, host } = mount(
      <PedalModal
        slot={null}
        onToggle={noopSlot}
        onKnobChange={noopSlot}
        onKnobReset={noopSlot}
        onClose={noop}
      />,
    );
    expect(host.querySelector('[role="dialog"]')).toBeNull();
    act(() => root.unmount());
    host.remove();
  });
});
