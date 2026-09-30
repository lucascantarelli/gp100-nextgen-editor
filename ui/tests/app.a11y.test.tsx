/**
 * Teste de A11Y/estados do App (M1.0): o ready-state renderiza pares
 * rótulo-valor acessíveis (dl/dt/dd) e o botão "Atualizar" tem nome
 * acessível. O ipc usa o fallback local (jsdom não tem __TAURI_INTERNALS__)
 * — determinístico por desenho (= MockDevice, valores do core).
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

describe("App (M1.0) — a11y e estados", () => {
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

    // botão com nome acessível (papel + nome)
    const buttons = Array.from(host.querySelectorAll("button"));
    expect(buttons.some((b) => b.textContent === "Atualizar")).toBe(true);

    act(() => root.unmount());
    host.remove();
  });
});
