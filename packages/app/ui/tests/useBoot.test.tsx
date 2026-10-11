/**
 * useBoot — a máquina de estados do boot sob teste DIRETO (componente
 * probe + act, mesmo padrão dos testes de casca; sem dependências novas).
 * O ipc/device é mockado para controlar resolução/erro — o fallback real
 * já está coberto em ipc.device.test.ts. Contratos aqui:
 *   - boot AUTOMÁTICO no mount, 1× mesmo sob StrictMode (duplo efeito);
 *   - origem manual registrada (startBoot explícito);
 *   - progresso por listener com throttle rAF (último beat ganha);
 *   - erro → estado error com retry() que resolve na 2ª tentativa;
 *   - relatório INVÁLIDO (#161) → error nomeando a leitura, nunca ready;
 *   - mensagem da UI é amigável (o detalhe técnico só vai pro console);
 *   - reset() volta a idle.
 */
import { act, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import {
  INVENTARIO_ESPERADO,
  mensagemBootInvalido,
  useBoot,
  validaRelatorioBoot,
} from "../src/hooks/useBoot";
import { MSG } from "../src/i18n/messages";
import type { BootProgress, BootReport } from "../src/ipc/types";

/** Relatório VÁLIDO do gate (#161): script completo, 198/198 do catálogo. */
function relatorioValido(): BootReport {
  return { transactions: 2297, presets: INVENTARIO_ESPERADO, names: INVENTARIO_ESPERADO };
}

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
  mocks.deviceBoot.mockReset().mockResolvedValue(relatorioValido());
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

  it("erro → estado error com mensagem AMIGÁVEL; detalhe técnico só no console; retry recupera", async () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    mocks.deviceBoot
      .mockRejectedValueOnce(new Error("boom no boot"))
      .mockResolvedValueOnce(relatorioValido());

    mountProbe();
    await flush();
    // #161: a UI nunca vê o erro cru ("debug:" / stack) — só o texto do catálogo.
    expect(captured!.state).toMatchObject({ kind: "error", message: MSG.connBootError });
    expect(spy).toHaveBeenCalledWith("boot falhou", expect.any(Error));
    expect(captured!.progress).toBeNull();

    const st = captured!.state;
    if (st.kind !== "error") throw new Error("esperado estado error");
    act(() => st.retry());
    await flush();
    expect(mocks.deviceBoot).toHaveBeenCalledTimes(2);
    expect(captured!.state).toMatchObject({ kind: "ready" });
    spy.mockRestore();
  });

  it("erro não-Error (string do invoke): a UI recebe o texto amigável, o cru vai pro console", async () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    mocks.deviceBoot.mockRejectedValueOnce("falha crua do backend");
    mountProbe();
    await flush();
    expect(captured!.state).toMatchObject({ kind: "error", message: MSG.connBootError });
    expect(spy).toHaveBeenCalledWith("boot falhou", "falha crua do backend");
    spy.mockRestore();
  });

  it("relatório INVÁLIDO (#161): nunca vira ready — error nomeando a leitura; retry recupera", async () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    mocks.deviceBoot
      .mockResolvedValueOnce({ transactions: 2297, presets: 150, names: 150 })
      .mockResolvedValueOnce(relatorioValido());

    mountProbe();
    await flush();
    expect(captured!.state).toMatchObject({ kind: "error" });
    const st = captured!.state;
    if (st.kind !== "error") throw new Error("esperado estado error");
    // O erro NOMEIA a leitura (inventário) com os números medidos.
    expect(st.message).toBe(MSG.bootInvalidoInventario(150, INVENTARIO_ESPERADO));
    expect(spy).toHaveBeenCalledWith(
      "boot: relatório fora do contrato",
      { leitura: "inventario", lido: 150, esperado: INVENTARIO_ESPERADO },
      { transactions: 2297, presets: 150, names: 150 },
    );

    act(() => st.retry());
    await flush();
    expect(mocks.deviceBoot).toHaveBeenCalledTimes(2);
    expect(captured!.state).toMatchObject({ kind: "ready" });
    spy.mockRestore();
  });

  it("relatório VARIANTE do scan (#161): 198/197 → error NOMEANDO nomes; retry recupera", async () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    // Os números que o fio EMITE com um device saudável que tem um slot
    // sem nome (prova rust: boot_mock::relatorio_mede_197_de_198 e o
    // actor::boot_via_actor_entrega_relatorio_honesto — o DTO que o
    // commands.rs devolve carrega exatamente isto). O gate não pode
    // montar a casca sobre um aparelho pela metade, e o erro tem de
    // NOMEAR a leitura errada (nomes, não inventário).
    const variante = { transactions: 2297, presets: INVENTARIO_ESPERADO, names: 197 };
    mocks.deviceBoot.mockResolvedValueOnce(variante).mockResolvedValueOnce(relatorioValido());

    mountProbe();
    await flush();
    expect(captured!.state).toMatchObject({ kind: "error" });
    const st = captured!.state;
    if (st.kind !== "error") throw new Error("esperado estado error");
    // O erro NOMEIA a leitura (nomes) com os números medidos no fio.
    expect(st.message).toBe(MSG.bootInvalidoNomes(197, INVENTARIO_ESPERADO));
    expect(spy).toHaveBeenCalledWith(
      "boot: relatório fora do contrato",
      { leitura: "nomes", lido: 197, esperado: INVENTARIO_ESPERADO },
      variante,
    );

    act(() => st.retry());
    await flush();
    expect(mocks.deviceBoot).toHaveBeenCalledTimes(2);
    expect(captured!.state).toMatchObject({ kind: "ready" });
    spy.mockRestore();
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
      resolveBoot(relatorioValido());
      await Promise.resolve();
      await Promise.resolve();
    });

    // O hook não estourou e não há novo render (o probe foi desmontado).
    expect(mocks.deviceBoot).toHaveBeenCalledTimes(1);
  });
});

describe("validaRelatorioBoot — função pura do gate (#161)", () => {
  it("válida (script completo + 198/198 do catálogo): null", () => {
    expect(validaRelatorioBoot(relatorioValido())).toBeNull();
  });

  it("vazia (null / undefined / objeto sem campos): progresso", () => {
    expect(validaRelatorioBoot(null)).toEqual({ leitura: "progresso" });
    expect(validaRelatorioBoot(undefined)).toEqual({ leitura: "progresso" });
    expect(validaRelatorioBoot({} as unknown as BootReport)).toEqual({
      leitura: "progresso",
    });
  });

  it("transactions fora de faixa (0, negativo, float, NaN, absurdo): progresso", () => {
    for (const transactions of [0, -1, 1.5, Number.NaN, 99_999]) {
      expect(validaRelatorioBoot({ ...relatorioValido(), transactions })).toEqual({
        leitura: "progresso",
      });
    }
  });

  it("inventário fora do catálogo: nomeia a leitura com os números medidos", () => {
    expect(validaRelatorioBoot({ ...relatorioValido(), presets: 150 })).toEqual({
      leitura: "inventario",
      lido: 150,
      esperado: INVENTARIO_ESPERADO,
    });
    expect(validaRelatorioBoot({ ...relatorioValido(), presets: 0 })).toEqual({
      leitura: "inventario",
      lido: 0,
      esperado: INVENTARIO_ESPERADO,
    });
    expect(
      validaRelatorioBoot({ ...relatorioValido(), presets: "198" as unknown as number }),
    ).toEqual({ leitura: "inventario", lido: 0, esperado: INVENTARIO_ESPERADO });
  });

  it("nomes ≠ presets (leitura pela metade): nomeia nomes", () => {
    expect(validaRelatorioBoot({ ...relatorioValido(), names: 197 })).toEqual({
      leitura: "nomes",
      lido: 197,
      esperado: INVENTARIO_ESPERADO,
    });
    expect(validaRelatorioBoot({ ...relatorioValido(), names: 0 })).toEqual({
      leitura: "nomes",
      lido: 0,
      esperado: INVENTARIO_ESPERADO,
    });
  });

  it("mensagem nomeia a leitura e traz os números medidos (pt-BR)", () => {
    expect(mensagemBootInvalido({ leitura: "progresso" })).toBe(MSG.bootInvalidoProgresso);
    const inv = mensagemBootInvalido({
      leitura: "inventario",
      lido: 150,
      esperado: INVENTARIO_ESPERADO,
    });
    expect(inv).toContain("150");
    expect(inv).toContain(String(INVENTARIO_ESPERADO));
    const nom = mensagemBootInvalido({
      leitura: "nomes",
      lido: 97,
      esperado: INVENTARIO_ESPERADO,
    });
    expect(nom).toContain("97");
    expect(nom).toContain(String(INVENTARIO_ESPERADO));
  });
});
