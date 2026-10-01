/**
 * useBoot — a máquina de estados do boot sob teste DIRETO (componente
 * probe + act, mesmo padrão dos testes de casca; sem dependências novas).
 * O ipc/device é mockado para controlar resolução/erro — o fallback real
 * já está coberto em ipc.device.test.ts. Contratos aqui:
 *   - boot AUTOMÁTICO no mount, 1× mesmo sob StrictMode (duplo efeito);
 *   - origem manual registrada (startBoot explícito);
 *   - progresso por listener com throttle rAF (último beat ganha);
 *   - erro → estado error com retry() que resolve na 2ª tentativa;
 *   - reset() volta a idle.
 */
import { act, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { useBoot } from "../src/hooks/useBoot";
import type { BootProgress, BootReport } from "../src/ipc/types";

const mocks = vi.hoisted(() => ({
  deviceBoot: vi.fn(),
  onBootProgress: vi.fn(),
}));
vi.mock("../src/ipc/device", () => mocks);

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
  // jsdom pode não ter rAF (pretendToBeVisual off): shim mínimo p/ o
  // throttle do hook exercitar — mesmo contrato (cb no próximo frame).
  if (typeof globalThis.requestAnimationFrame !== "function") {
    Object.defineProperty(globalThis, "requestAnimationFrame", {
      value: (cb: FrameRequestCallback) =>
        window.setTimeout(() => cb(performance.now()), 16) as unknown as number,
      writable: true,
      configurable: true,
    });
    Object.defineProperty(globalThis, "cancelAnimationFrame", {
      value: (id: number) => window.clearTimeout(id),
      writable: true,
      configurable: true,
    });
  }
});

beforeEach(() => {
  mocks.deviceBoot.mockReset().mockResolvedValue({ transactions: 2297 } satisfies BootReport);
  mocks.onBootProgress.mockReset().mockResolvedValue(() => {});
});

type Boot = ReturnType<typeof useBoot>;
let captured: Boot | null = null;

function Probe() {
  captured = useBoot();
  return null;
}

function mountProbe(strict = false) {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  act(() => {
    root.render(strict ? <StrictMode><Probe /></StrictMode> : <Probe />);
  });
  return { root, host };
}

async function flush(times = 6) {
  for (let i = 0; i < times; i += 1) {
    await act(async () => {
      await Promise.resolve();
    });
  }
}

describe("useBoot — máquina de estados do boot", () => {
  it("boot AUTOMÁTICO no mount: resolve no relatório do device com origem auto", async () => {
    mountProbe();
    expect(captured!.state.kind).toBe("loading");
    await flush();

    expect(mocks.deviceBoot).toHaveBeenCalledTimes(1);
    expect(captured!.state).toMatchObject({ kind: "ready" });
    expect(captured!.origin).toBe("auto");
    expect(captured!.progress).toBe(100);
  });

  it("StrictMode (duplo efeito): o auto-boot roda 1× só — guarda da ref", async () => {
    mountProbe(true);
    await flush();
    expect(mocks.deviceBoot).toHaveBeenCalledTimes(1);
    expect(captured!.state).toMatchObject({ kind: "ready" });
  });

  it("startBoot manual: origem registrada e device chamado de novo", async () => {
    mountProbe();
    await flush();
    expect(captured!.origin).toBe("auto");

    act(() => captured!.startBoot());
    expect(captured!.origin).toBe("manual");
    expect(captured!.state.kind).toBe("loading");
    await flush();
    expect(mocks.deviceBoot).toHaveBeenCalledTimes(2);
    expect(captured!.state).toMatchObject({ kind: "ready" });
  });

  it("progresso: último beat do listener vence (throttle rAF), % e estágio corretos", async () => {
    mountProbe();
    await flush();

    const cbs = mocks.onBootProgress.mock.calls.map((c) => c[0]) as Array<(p: BootProgress) => void>;
    expect(cbs.length).toBeGreaterThan(0);

    // 3 beats síncronos: o throttle guarda o ÚLTIMO (574/2297 ≈ 25%)
    act(() => {
      for (const done of [100, 400, 574]) {
        cbs.forEach((cb) => cb({ stage: "scan", done, total: 2297, currentPp: done % 198 }));
      }
    });
    await act(async () => {
      await new Promise((r) => setTimeout(r, 30)); // 1 frame do shim
    });

    expect(captured!.progress).toBe(25);
    expect(captured!.stage).toBe("scan");
  });

  it("erro → estado error com a mensagem; retry() resolve na 2ª tentativa", async () => {
    mocks.deviceBoot
      .mockRejectedValueOnce(new Error("boom no boot"))
      .mockResolvedValueOnce({ transactions: 2297 });

    mountProbe();
    await flush();
    expect(captured!.state).toMatchObject({ kind: "error", message: "boom no boot" });
    expect(captured!.progress).toBeNull();

    const st = captured!.state;
    if (st.kind !== "error") throw new Error("esperado estado error");
    act(() => st.retry());
    await flush();
    expect(mocks.deviceBoot).toHaveBeenCalledTimes(2);
    expect(captured!.state).toMatchObject({ kind: "ready" });
  });

  it("erro não-Error (string do invoke): vira mensagem via String(e)", async () => {
    mocks.deviceBoot.mockRejectedValueOnce("falha crua do backend");
    mountProbe();
    await flush();
    expect(captured!.state).toMatchObject({ kind: "error", message: "falha crua do backend" });
  });

  it("reset(): volta a idle e zera progresso/estágio", async () => {
    mountProbe();
    await flush();
    expect(captured!.state.kind).toBe("ready");

    act(() => captured!.reset());
    expect(captured!.state.kind).toBe("idle");
    expect(captured!.progress).toBeNull();
    expect(captured!.stage).toBeNull();
  });

  it("boot em voo + unmount: promessa tardia NÃO atualiza estado (guarda alive)", async () => {
    /* Regressão da run 36937323423 (ui-rust macOS): o deviceBoot resolveu
     * depois do ambiente de teste morrer → setState → o React resolve a
     * prioridade do update e acessa `window` já destruído →
     * "ReferenceError: window is not defined" (unhandled error que derruba
     * a suíte). Aqui o caminho é exercitado: desmontamos com o boot em voo
     * e resolvemos a promessa depois — sem a guarda, o setState agendaria
     * update em componente morto. */
    let resolveBoot!: (r: BootReport) => void;
    mocks.deviceBoot.mockImplementationOnce(
      () =>
        new Promise<BootReport>((res) => {
          resolveBoot = res;
        }),
    );

    const { root } = mountProbe();
    expect(captured!.state.kind).toBe("loading");

    act(() => root.unmount());
    captured = null;
    await act(async () => {
      resolveBoot({ transactions: 2297 } satisfies BootReport);
      await Promise.resolve();
      await Promise.resolve();
    });

    // O hook não estourou e não há novo render (o probe foi desmontado).
    expect(mocks.deviceBoot).toHaveBeenCalledTimes(1);
  });
});
