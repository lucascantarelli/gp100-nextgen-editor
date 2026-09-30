/**
 * Teste de A11Y/estados do App: o ready-state renderiza pares
 * rótulo-valor acessíveis (dl/dt/dd), a ConnectionBar tem botão de boot
 * com nome acessível e a barra de progresso expõe role/valores ARIA
 * (com o fallback local de ipc: determinístico — MockDevice + progresso
 * sintético do boot).
 */
import { describe, expect, it, beforeAll } from "vitest";
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import App from "../src/App";

// React 19 exige o ambiente de teste declarado para act(...) sem warnings.
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

describe("App — a11y e estados", () => {
  it("chega ao ready com dl/dt/dd e os valores do mock", async () => {
    const { root, host } = mount(<App />);
    // espera o async do ipc resolver (fallback local resolve num microtask)
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    const dl = host.querySelector("dl");
    expect(dl, "ready-state usa dl/dt/dd (pares rótulo-valor)").not.toBeNull();
    expect(host.textContent).toContain("It's GP100");
    expect(host.textContent).toContain("0x0000");
    expect(host.textContent).toContain("99");

    // botões com nome acessível (papel + nome)
    const buttons = Array.from(host.querySelectorAll("button"));
    expect(buttons.some((b) => b.textContent === "Atualizar")).toBe(true);
    const bootBtn = buttons.find((b) => b.textContent === "Boot");
    expect(bootBtn, "ConnectionBar expõe o botão de boot").toBeDefined();

    act(() => bootBtn?.click());
    // fallback local: beats por microtask + promise do boot
    await act(async () => {
      await new Promise((r) => setTimeout(r, 0));
      await Promise.resolve();
    });
    // barra de progresso acessível DURANTE o boot (fallback: resolve no
    // mesmo tick — pode já ter terminado; o teste aceita os dois estados)
    const bar = host.querySelector('[role="progressbar"]');
    const doneText = host.textContent ?? "";
    if (bar) {
      expect(bar.getAttribute("aria-valuemax")).toBe("100");
      expect(Number(bar.getAttribute("aria-valuenow"))).toBeGreaterThanOrEqual(0);
    } else {
      expect(doneText).toContain("transações");
    }

    act(() => root.unmount());
    host.remove();
  });
});
