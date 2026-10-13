/**
 * DeviceBadge — o indicador de conexão da navbar em teste de UI.
 *
 * O componente é puramente apresentacional: recebe o `DeviceInfo` e decide o
 * dot de estado, a linha de status e o badge de mock. O teste cobre os
 * QUEBRA-LOUIS que o dono veria se alguém mexer nele:
 *
 *   - real → dot `.live-dot`, "on", "+N presets · escrita liberada", SEM
 *     badge de mock;
 *   - boot NÃO validado (conectado=false) → "off" mesmo com backend
 *     respondendo — o LED nunca mente sobre um device que o gate recusou
 *     (#161 / e2e mid-boot);
 *   - real sem write-verified (build de leitura) → "escrita travada";
 *   - mock → dot ligado + o badge "Mock Device" (estado simulado fica explícito);
 *   - none → dot `.idle-dot`, "off", "sem aparelho", sem pulso;
 *   - null (sem device_info) → dot apagado, "off" sem linha de ausência;
 *   - SEM SVG (dono 12/10: "está ruim com svg, remova") — nenhuma miniatura;
 *   - o "on"/"off" mora em span PRÓPRIO (o e2e ancora
 *     `getByText("on", { exact: true })` no cluster de conexão);
 *   - honestidade de dado (#150): o badge NÃO afirma serial/nome de fio —
 *     o `currentName` do real é vazio, então o texto só usa o que o
 *     `device_info` expõe (backend, presetCount, writeVerified).
 */
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, describe, expect, it } from "vitest";
import { DeviceBadge } from "../src/components/DeviceBadge";
import { MSG } from "../src/i18n/messages";
import type { DeviceInfo } from "../src/ipc/types";

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

/** Fixture base: só o que o `device_info` REAL expõe (contrato #150). */
function info(patch: Partial<DeviceInfo> = {}): DeviceInfo {
  return {
    backend: "real",
    detail: "",
    presetCount: 198,
    currentPp: 0,
    currentName: "", // no aparelho real o nome do pp vem VAZIO (meta6 indecifrada)
    currentPpType: 0,
    irSlotsWithCrc: 0,
    irSlots: [],
    writeVerified: true,
    ...patch,
  };
}

/**
 * Fórmula do App: `pronto && info != null && backend !== "none"` — o helper
 * assume boot pronto; passe `conectado` para exercitar o gate (#161).
 */
function mountBadge(
  el: DeviceInfo | null,
  conectado: boolean = el != null && el.backend !== "none",
): { root: Root; host: HTMLElement } {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  act(() => {
    root.render(<DeviceBadge info={el} conectado={conectado} />);
  });
  return { root, host };
}

function teardown(root: Root, host: HTMLElement) {
  act(() => root.unmount());
  host.remove();
}

const dot = (host: HTMLElement, classe: "live-dot" | "idle-dot") =>
  host.querySelector(`.${classe}`);

/** span PRÓPRIO com o texto exato (contrato do `getByText(exact)` do e2e). */
const spanExato = (host: HTMLElement, texto: string) =>
  Array.from(host.querySelectorAll("span")).find((s) => s.textContent === texto);

describe("DeviceBadge — indicador de conexão da navbar", () => {
  it("real: dot pulsante, presets + escrita liberada, sem badge de mock e SEM svg", () => {
    const { root, host } = mountBadge(info());

    const status = host.querySelector<HTMLElement>("[data-luz]");
    expect(status, "o badge expõe o estado em data-luz").toBeTruthy();
    expect(status!.getAttribute("data-luz")).toBe("on");
    expect(status!.getAttribute("data-conectado")).toBe("true");

    expect(dot(host, "live-dot"), "dot de device ligado").toBeTruthy();
    expect(dot(host, "idle-dot")).toBeNull();

    expect(host.textContent).toContain(String(info().presetCount));
    expect(host.textContent).toContain(MSG.connBadgePresets);
    expect(host.textContent).toContain(MSG.connBadgeWriteOk);
    expect(host.textContent, "sem badge de mock com aparelho real").not.toContain(MSG.mockBadge);

    // o "on" num span próprio — o e2e busca `getByText("on", { exact: true })`
    expect(spanExato(host, MSG.connShortOn), "'on' em span próprio").toBeTruthy();

    // (dono 12/10) a miniatura SVG foi removida: nada de desenho de chassi
    expect(host.querySelector("svg"), "sem miniatura SVG").toBeNull();

    teardown(root, host);
  });

  it("real sem write-verified (build de leitura): a linha diz escrita TRAVADA", () => {
    const { root, host } = mountBadge(info({ writeVerified: false }));

    expect(host.textContent).toContain(MSG.connBadgeWriteLocked);
    expect(host.textContent).not.toContain(MSG.connBadgeWriteOk);

    teardown(root, host);
  });

  it("mock: dot ligado + o badge 'Mock Device' fica explícito na tela", () => {
    const { root, host } = mountBadge(info({ backend: "mock", presetCount: 99 }));

    const status = host.querySelector<HTMLElement>("[data-luz]")!;
    expect(status.getAttribute("data-luz")).toBe("mock");
    expect(dot(host, "live-dot"), "simulação responde → dot ligado").toBeTruthy();
    expect(host.textContent, "estado simulado nunca se disfarça de aparelho").toContain(
      MSG.mockBadge,
    );
    expect(host.textContent).toContain(String(99));

    teardown(root, host);
  });

  it("none: dot apagado, 'sem aparelho' — sem pulso de ligado", () => {
    const { root, host } = mountBadge(info({ backend: "none", detail: "aparelho não conectado" }));

    const status = host.querySelector<HTMLElement>("[data-luz]")!;
    expect(status.getAttribute("data-luz")).toBe("off");
    expect(status.getAttribute("data-conectado")).toBe("false");
    expect(dot(host, "idle-dot")).toBeTruthy();
    expect(dot(host, "live-dot"), "sem dot pulsante sem aparelho").toBeNull();
    expect(spanExato(host, MSG.connShortOff), "'off' em span próprio").toBeTruthy();
    expect(host.textContent).toContain(MSG.connBadgeNone);

    teardown(root, host);
  });

  it("boot não validado (conectado=false): 'off' SEM linha de presets mesmo com backend mock respondendo", () => {
    // cenário do e2e mid-boot: o device_info respondeu, mas o boot FALHOU —
    // o cluster antigo mostrava off (pronto=false) e o badge precisa herdar
    // esse gating, senão a navbar mostra "on 99 presets" com a casca barrada.
    const { root, host } = mountBadge(info({ backend: "mock", presetCount: 99 }), false);

    const status = host.querySelector<HTMLElement>("[data-luz]")!;
    expect(status.getAttribute("data-luz")).toBe("off");
    expect(status.getAttribute("data-conectado")).toBe("false");
    expect(dot(host, "idle-dot"), "sem pulso de ligado com boot falho").toBeTruthy();
    expect(spanExato(host, MSG.connShortOff), "'off' em span próprio").toBeTruthy();
    expect(host.textContent, "sem linha de presets sem boot validado").not.toContain(MSG.connBadgePresets);
    expect(host.textContent, "sem badge de mock quando o gate recusou").not.toContain(MSG.mockBadge);

    teardown(root, host);
  });

  it("info=null (sem device_info): dot apagado e 'off' — sem afirmar ausência de aparelho", () => {
    // spec do e2e estados: "sem device_info não há conexão" → "off" curto;
    // mas null não PROVA aparelho ausente (pode estar lendo/falhou o invoke),
    // então a linha de "sem aparelho" fica de fora.
    const { root, host } = mountBadge(null);

    const status = host.querySelector<HTMLElement>("[data-luz]")!;
    expect(status.getAttribute("data-luz")).toBe("off");
    expect(status.getAttribute("data-conectado")).toBe("false");
    expect(spanExato(host, MSG.connShortOff), "'off' em span próprio").toBeTruthy();
    expect(host.textContent, "ainda não sabe se há aparelho").not.toContain(MSG.connBadgeNone);

    teardown(root, host);
  });

  it("honestidade de dado: o texto só usa o que o device_info expõe (sem serial/nome de fio)", () => {
    // `currentName` VAZIO (real) e `detail` de mock não podem virar "nome do
    // aparelho" no badge — o que muda por estado é o dot e a linha de status.
    const { root, host } = mountBadge(info({ currentName: "", detail: "algo cru" }));

    expect(host.textContent, "não inventa nome/serial do fio").not.toContain("algo cru");
    expect(host.textContent).toContain(String(info().presetCount));

    teardown(root, host);
  });
});
