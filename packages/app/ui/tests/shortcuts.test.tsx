/**
 * Atalhos globais (useGlobalShortcuts) — helpers puros + integração com o App:
 * Espaço = drum play/stop, R = REC do looper, Esc fecha o painel do topo.
 * Documentação de usuário: aba Help do Settings (SettingsModal).
 */
import { describe, expect, it, beforeAll, beforeEach } from "vitest";
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import App from "../src/App";
import { isSpaceNativeTarget, isTextEntryTarget, shouldHandleShortcut } from "../src/hooks/useGlobalShortcuts";

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

beforeEach(() => {
  localStorage.clear();
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

/** dispara um keydown real no alvo (default: body — chega ao listener global) */
function pressKey(key: string, init: KeyboardEventInit = {}, target: Element = document.body): void {
  target.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...init }));
}

/** evento solto com target forçado (para testar os helpers sem DOM real) */
function makeEvent(key: string, targetEl: Element, init: KeyboardEventInit = {}): KeyboardEvent {
  const ev = new KeyboardEvent("keydown", { key, bubbles: true, ...init });
  Object.defineProperty(ev, "target", { value: targetEl });
  return ev;
}

describe("useGlobalShortcuts — helpers puros", () => {
  it("isTextEntryTarget reconhece campos de digitação (e ignora o resto)", () => {
    const input = document.createElement("input");
    const textarea = document.createElement("textarea");
    const select = document.createElement("select");
    const ce = document.createElement("div");
    ce.setAttribute("contenteditable", "true");
    const button = document.createElement("button");
    const span = document.createElement("span");
    document.body.append(input, textarea, select, ce, button, span);

    expect(isTextEntryTarget(input)).toBe(true);
    expect(isTextEntryTarget(textarea)).toBe(true);
    expect(isTextEntryTarget(select)).toBe(true);
    expect(isTextEntryTarget(ce)).toBe(true);
    expect(isTextEntryTarget(button)).toBe(false);
    expect(isTextEntryTarget(span)).toBe(false);
    expect(isTextEntryTarget(null)).toBe(false);

    [input, textarea, select, ce, button, span].forEach((el) => el.remove());
  });

  it("isSpaceNativeTarget protege a ativação nativa (botões/switches/options)", () => {
    const btn = document.createElement("button");
    const sw = document.createElement("span");
    sw.setAttribute("role", "switch");
    const option = document.createElement("div");
    option.setAttribute("role", "option");
    document.body.append(btn, sw, option);

    expect(isSpaceNativeTarget(btn)).toBe(true);
    expect(isSpaceNativeTarget(sw)).toBe(true);
    expect(isSpaceNativeTarget(option)).toBe(true);
    expect(isSpaceNativeTarget(document.body)).toBe(false);

    [btn, sw, option].forEach((el) => el.remove());
  });

  it("shouldHandleShortcut aplica modificadores, auto-repeat e guardas por alvo", () => {
    const btn = document.createElement("button");
    const input = document.createElement("input");
    document.body.append(btn, input);

    // Espaço: body dispara; botão NÃO (ativação nativa); repeat e Ctrl ignorados
    expect(shouldHandleShortcut(makeEvent(" ", document.body), "space")).toBe(true);
    expect(shouldHandleShortcut(makeEvent(" ", btn), "space")).toBe(false);
    expect(shouldHandleShortcut(makeEvent(" ", document.body, { repeat: true }), "space")).toBe(false);
    expect(shouldHandleShortcut(makeEvent(" ", document.body, { ctrlKey: true }), "space")).toBe(false);

    // R: input não dispara; body sim
    expect(shouldHandleShortcut(makeEvent("r", input), "letter")).toBe(false);
    expect(shouldHandleShortcut(makeEvent("r", document.body), "letter")).toBe(true);

    // Esc: sempre (mesmo com foco em campo de texto)
    expect(shouldHandleShortcut(makeEvent("Escape", input), "escape")).toBe(true);
    expect(shouldHandleShortcut(makeEvent("Escape", document.body, { altKey: true }), "escape")).toBe(false);

    btn.remove();
    input.remove();
  });
});

describe("App — atalhos globais em ação", () => {
  it("Espaço liga/para o drum (chip ⏵/⏹)", async () => {
    const { root, host } = mount(<App />);
    await settle();

    const chip = host.querySelector<HTMLButtonElement>('button[aria-label^="Bateria (drum)"]');
    expect(chip, "chip do drum no topbar").not.toBeNull();
    expect(chip!.textContent).toContain("⏵");

    act(() => pressKey(" "));
    await settle();
    expect(chip!.textContent).toContain("⏹");

    act(() => pressKey(" "));
    await settle();
    expect(chip!.textContent).toContain("⏵");

    act(() => root.unmount());
    host.remove();
  });

  it("R dispara REC→PLAY no looper; digitar em campo de texto não dispara", async () => {
    const { root, host } = mount(<App />);
    await settle();

    act(() => pressKey("r"));
    await settle();
    expect(host.querySelector('[aria-label="Parar gravação e tocar"]'), "R = REC").not.toBeNull();

    act(() => pressKey("r"));
    await settle();
    expect(host.querySelector('[aria-label="Parar gravação e tocar"]')).toBeNull();
    expect(host.querySelector('[aria-label="Tocar loop (PLAY)"]')?.getAttribute("aria-pressed")).toBe("true");

    // digitando num input, R e Espaço NÃO mexem no transporte (segue em PLAY)
    const input = document.createElement("input");
    document.body.appendChild(input);
    input.focus();
    act(() => pressKey("r", {}, input));
    act(() => pressKey(" ", {}, input));
    await settle();
    expect(host.querySelector('[aria-label="Tocar loop (PLAY)"]')?.getAttribute("aria-pressed")).toBe("true");
    expect(host.querySelector('[aria-label="Parar gravação e tocar"]')).toBeNull();
    input.remove();

    act(() => root.unmount());
    host.remove();
  });

  it("Esc fecha Settings → Drum → pushes com precedência; modal bloqueia transporte", async () => {
    const { root, host } = mount(<App />);
    await settle();

    const chip = host.querySelector<HTMLButtonElement>('button[aria-label^="Bateria (drum)"]')!;
    act(() => chip.click()); // popover do drum aberto
    await settle();
    expect(host.querySelector('[aria-label="Gestão de ritmos da bateria (drum)"]')).not.toBeNull();

    act(() => host.querySelector<HTMLButtonElement>('button[aria-label="Abrir configurações"]')!.click());
    await settle();
    expect(host.querySelector('[role="dialog"]')).not.toBeNull();

    // com o modal aberto, o transporte fica inerte
    act(() => pressKey(" "));
    act(() => pressKey("r"));
    await settle();
    expect(chip.textContent).toContain("⏵");
    expect(host.querySelector('[aria-label="Parar gravação e tocar"]')).toBeNull();

    // Esc 1: fecha o Settings (topo); drum continua aberto
    act(() => pressKey("Escape"));
    await settle();
    expect(host.querySelector('[role="dialog"]')).toBeNull();
    expect(chip.getAttribute("aria-expanded")).toBe("true");

    // Esc 2: fecha o drum
    act(() => pressKey("Escape"));
    await settle();
    expect(chip.getAttribute("aria-expanded")).toBe("false");

    // Esc 3: fecha o drawer de pushes
    const details = host.querySelector("details")!;
    act(() => {
      details.open = true;
    });
    expect(details.open).toBe(true);
    act(() => pressKey("Escape"));
    await settle();
    expect(details.open).toBe(false);

    act(() => root.unmount());
    host.remove();
  });
});
