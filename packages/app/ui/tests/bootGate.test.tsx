/**
 * bootGate — o gate de boot do App (#161) em teste de UI. Os três estados
 * provados sobre o DOM real:
 *
 *   - `loading` → a página é SÓ navbar + gate: NENHUM painel do
 *     `.shell-content`, nem rodapé, no DOM (a casca não existe em voo);
 *   - `ready` → o relatório validado chega e a casca inteira monta
 *     (biblioteca, rodapé) com a faixa de progresso já fora;
 *   - `error` → só o `role="alert"` + navbar ficam; a mensagem é
 *     AMIGÁVEL (o "debug:..." cru nunca aparece), o alert ganha FOCO e a
 *     ação "Refazer o boot" recupera até a casca montar;
 *   - relatório INVÁLIDO (150 presets) → erro nomeando a leitura de
 *     inventário — a casca nunca monta com dado incoerente.
 *
 * O `deviceBoot` é mockado com promise controlada (sem timers): os
 * estados são determinísticos, não corrida com o fallback em macrotasks.
 */
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../src/App";
import { INVENTARIO_ESPERADO } from "../src/hooks/useBoot";
import { MSG } from "../src/i18n/messages";
import type { BootReport } from "../src/ipc/types";

const mocks = vi.hoisted(() => ({ deviceBoot: vi.fn() }));
// Mock PARCIAL: só o deviceBoot é controlado; o resto do ipc/device fica
// real (fallback de webview — mesmo padrão do app.interactions).
vi.mock("../src/ipc/device", async (importOriginal) => {
  const mod = await importOriginal<typeof import("../src/ipc/device")>();
  return { ...mod, deviceBoot: mocks.deviceBoot };
});

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

beforeEach(() => {
  mocks.deviceBoot.mockReset();
  localStorage.clear();
});

/** Relatório VÁLIDO do gate (#161): script completo + 198/198 do catálogo. */
function relatorioValido(): BootReport {
  return { transactions: 2297, presets: INVENTARIO_ESPERADO, names: INVENTARIO_ESPERADO };
}

function mountApp(): { root: Root; host: HTMLElement } {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  act(() => {
    root.render(<App />);
  });
  return { root, host };
}

function teardown(root: Root, host: HTMLElement) {
  act(() => root.unmount());
  host.remove();
}

/** Drena promises/efeitos (sem timers: o boot é controlado pelo teste). */
async function pump(times = 4) {
  for (let i = 0; i < times; i += 1) {
    await act(async () => {
      await Promise.resolve();
    });
  }
}

const navbar = (host: HTMLElement) => host.querySelector('[role="banner"]');
const casca = (host: HTMLElement) => host.querySelector(".shell-content");
const alerta = (host: HTMLElement) => host.querySelector('[role="alert"]');

describe("App — gate de boot (#161): loading · ready · error", () => {
  it("loading: só navbar + gate — NENHUM painel do shell-content no DOM", async () => {
    let resolveBoot!: (r: BootReport) => void;
    mocks.deviceBoot.mockImplementationOnce(
      () =>
        new Promise<BootReport>((res) => {
          resolveBoot = res;
        }),
    );

    const { root, host } = mountApp();
    await pump();

    expect(navbar(host), "navbar (TopBar) sempre presente").toBeTruthy();
    expect(casca(host), "casca ABSENTA durante o boot").toBeNull();
    expect(host.querySelector("footer.page-footer"), "rodapé ausente").toBeNull();
    expect(host.textContent).toContain(MSG.bootPanelMsg);
    expect(
      host.querySelector('[aria-label="Progresso do boot"]'),
      "faixa de progresso do gate",
    ).toBeTruthy();

    // relatório validado chega → a casca inteira monta
    resolveBoot(relatorioValido());
    await pump();

    expect(casca(host), "casca monta com o boot validado").toBeTruthy();
    expect(host.querySelector('[role="listbox"]'), "biblioteca montada").toBeTruthy();
    expect(host.querySelector("footer.page-footer"), "rodapé montado").toBeTruthy();
    expect(
      host.querySelector('[aria-label="Progresso do boot"]'),
      "a faixa some com o fim do boot",
    ).toBeNull();
    expect(alerta(host), "sem erro para anunciar").toBeNull();

    teardown(root, host);
  });

  it("error: só alert + navbar, mensagem amigável com foco; refazer o boot recupera", async () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    mocks.deviceBoot.mockRejectedValueOnce(new Error("debug: device morreu no fio"));

    const { root, host } = mountApp();
    await pump();

    const alert = alerta(host);
    expect(alert, "role=alert do gate").toBeTruthy();
    expect(casca(host), "casca ABSENTA com o boot falho").toBeNull();
    expect(host.querySelector("footer.page-footer"), "rodapé ausente").toBeNull();
    expect(navbar(host), "navbar permanece").toBeTruthy();
    // mensagem amigável — o detalhe técnico só no console (#161)
    expect(alert!.textContent).toContain(MSG.connBootError);
    expect(alert!.textContent).not.toContain("debug:");
    expect(spy).toHaveBeenCalledWith("boot falhou", expect.any(Error));
    // a11y (#161): o alert é o foco quando o boot falha
    expect(document.activeElement, "foco no alert").toBe(alert);

    // a ação refaz o boot e a casca monta
    const refazer = Array.from(alert!.querySelectorAll("button")).find(
      (b) => b.textContent === MSG.bootRetry,
    );
    expect(refazer, "ação Refazer o boot").toBeTruthy();
    mocks.deviceBoot.mockResolvedValueOnce(relatorioValido());
    act(() => {
      refazer!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await pump();

    expect(casca(host), "casca monta após o retry").toBeTruthy();
    expect(alerta(host), "erro some").toBeNull();

    spy.mockRestore();
    teardown(root, host);
  });

  it("relatório INVÁLIDO (150 presets): erro nomeando a leitura — casca nunca monta", async () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    mocks.deviceBoot.mockResolvedValueOnce({
      transactions: 2297,
      presets: 150,
      names: 150,
    });

    const { root, host } = mountApp();
    await pump();

    const alert = alerta(host);
    expect(alert, "relatório inválido vira alert, não ready").toBeTruthy();
    expect(alert!.textContent).toContain(
      MSG.bootInvalidoInventario(150, INVENTARIO_ESPERADO),
    );
    expect(casca(host)).toBeNull();
    expect(spy).toHaveBeenCalledWith(
      "boot: relatório fora do contrato",
      { leitura: "inventario", lido: 150, esperado: INVENTARIO_ESPERADO },
      { transactions: 2297, presets: 150, names: 150 },
    );

    spy.mockRestore();
    teardown(root, host);
  });
});
