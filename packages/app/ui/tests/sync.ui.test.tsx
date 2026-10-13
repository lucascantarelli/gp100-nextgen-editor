/**
 * SyncChip + useSyncState — o indicador de sincronização (#164).
 *
 * O chip é a superfície onde a UI pode MENTIR: um "sincronizado" antes de
 * qualquer leitura, um contador inventado, um erro sem saída. Estes testes
 * fecham os três:
 *
 *   - rótulo = estado do BACKEND (idle/sync/pending/failed/offline), sem
 *     número nenhum na tela (o protocolo não tem notificação espontânea —
 *     é poll + read-back, e a interface assume isso);
 *   - erro tem AÇÃO (#20): `failed` → "Reler" (device_sync_now) e
 *     `offline` → "Reconectar" (device_conectar); os estados sem erro não
 *     têm botão;
 *   - o hook nasce `idle` (sem leitura = sem número falso), é semeado pelo
 *     MESMO `device_info` do resto da UI e só muda com `device://sync`.
 *
 * O producer é o `emitSync` do fallback — MESMO shape do evento real.
 */
import { act } from "react";
import type { ReactElement } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { SyncChip } from "../src/components/SyncChip";
import { useSyncState } from "../src/hooks/useSyncState";
import { deviceConectar } from "../src/ipc/device";
import { deviceSyncNow, emitSync } from "../src/ipc/sync";
import type { SyncEvent, SyncStatus } from "../src/ipc/types";
import { MSG } from "../src/i18n/messages";

// mock PARCIAL: as ações do chip são ESPIAS que mantêm o fallback real — o
// "Reler" clica → deviceSyncNow → o mock local EMITE o sync de volta, e o
// teste vê o ciclo inteiro (clique → fio simulado → chip re-renderiza).
// As ações moram em portas distintas (#164: sync em ipc/sync.ts).
vi.mock("../src/ipc/device", async (importOriginal) => {
  const mod = await importOriginal<typeof import("../src/ipc/device")>();
  return {
    ...mod,
    deviceConectar: vi.fn(mod.deviceConectar),
  };
});
vi.mock("../src/ipc/sync", async (importOriginal) => {
  const mod = await importOriginal<typeof import("../src/ipc/sync")>();
  return {
    ...mod,
    deviceSyncNow: vi.fn(mod.deviceSyncNow),
  };
});

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

beforeEach(() => {
  localStorage.clear();
});

/** Monta um elemento e devolve o host vivo (mesmo padrão dos testes de painel). */
function mount(el: ReactElement) {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root: Root = createRoot(host);
  act(() => root.render(el));
  return {
    host,
    unmount: () =>
      act(() => {
        root.unmount();
      }),
  };
}

describe("SyncChip — o chip de sincronização não mente (#164)", () => {
  beforeEach(() => {
    vi.mocked(deviceSyncNow).mockClear();
    vi.mocked(deviceConectar).mockClear();
  });

  /** Monta o chip e espera a semente do deviceInfo (fallback = "sync"). */
  async function montaChip() {
    const r = mount(<SyncChip />);
    await act(async () => {
      await new Promise((res) => setTimeout(res, 0));
    });
    return r;
  }

  it("cada estado mostra o rótulo do backend, sem contador inventado", async () => {
    const casos: Array<[SyncStatus, string]> = [
      ["idle", MSG.syncIdle],
      ["sync", MSG.syncSync],
      ["pending", MSG.syncPending],
      ["failed", MSG.syncFailed],
      ["offline", MSG.syncOffline],
    ];
    for (const [status, rotulo] of casos) {
      const { host, unmount } = await montaChip();
      act(() => emitSync({ status, changed: false, detail: "" }));
      expect(host.textContent).toContain(rotulo);
      // nenhum dígito no chip: "tempo real" aqui é poll + read-back, não métrica.
      expect(host.textContent ?? "").not.toMatch(/\d/);
      unmount();
    }
  });

  it("failed tem AÇÃO: \"Reler\" chama device_sync_now E o chip volta a sync", async () => {
    const { host, unmount } = await montaChip();
    act(() => emitSync({ status: "failed", changed: false, detail: "pagina fora de ordem" }));
    // o detalhe humano fica no chip (title) — o operador lê o motivo sem
    // abrir o painel de diagnóstico.
    expect(host.querySelector('[role="status"]')?.getAttribute("title")).toBe(
      "pagina fora de ordem",
    );
    const btn = [...host.querySelectorAll("button")].find((b) => b.textContent === MSG.syncReler);
    expect(btn).toBeDefined();
    await act(async () => {
      btn!.click();
      await new Promise((res) => setTimeout(res, 0));
    });
    expect(deviceSyncNow).toHaveBeenCalledTimes(1);
    // o mock local "relê" e emite o resultado: o chip volta a sincronizado
    // (o ciclo clique → fio simulado → estado fecha sem App nenhum no meio).
    expect(host.textContent).toContain(MSG.syncSync);
    // DESMONTA: um chip vivo assina o bus de eventos deste arquivo, e o
    // emitSync do próximo teste avisaria React de update fora de act
    // (#142 — o setup converte o aviso em falha, e com razão).
    unmount();
  });

  it("offline tem AÇÃO: \"Reconectar\" chama device_conectar", async () => {
    const { host, unmount } = await montaChip();
    act(() => emitSync({ status: "offline", changed: false, detail: "desconectado: cabo" }));
    const btn = [...host.querySelectorAll("button")].find(
      (b) => b.textContent === MSG.syncReconnect,
    );
    expect(btn).toBeDefined();
    await act(async () => {
      btn!.click();
      await new Promise((res) => setTimeout(res, 0));
    });
    expect(deviceConectar).toHaveBeenCalledTimes(1);
    unmount();
  });

  it("idle/sync/pending NÃO têm botão (sem erro não há recuperação)", async () => {
    for (const status of ["idle", "sync", "pending"] as const) {
      const { host, unmount } = await montaChip();
      act(() => emitSync({ status, changed: false, detail: "" }));
      expect(host.querySelectorAll("button").length).toBe(0);
      unmount();
    }
  });
});

describe("useSyncState — o chip nasce com a verdade do backend (#164)", () => {
  let st: SyncEvent | null = null;
  function Probe() {
    st = useSyncState();
    return null;
  }

  beforeEach(() => {
    st = null;
  });

  it("nasce idle, semeia do deviceInfo e só muda com device://sync", async () => {
    const { unmount } = mount(<Probe />);
    // nasce sem leitura: idle é a única resposta honesta (número falso é
    // pior que "—").
    expect(st!.status).toBe("idle");
    await act(async () => {
      await new Promise((r) => setTimeout(r, 0)); // o deviceInfo do fallback resolve
    });
    // semente: a MESMA verdade do backend (mock que leu tudo = "sync").
    expect(st!.status).toBe("sync");
    act(() => emitSync({ status: "pending", changed: false, detail: "" }));
    expect(st!.status).toBe("pending");
    act(() => emitSync({ status: "offline", changed: true, detail: "desconectado: cabo" }));
    expect(st).toMatchObject({
      status: "offline",
      changed: true,
      detail: "desconectado: cabo",
    });
    unmount();
  });

  it("depois do unmount os eventos não escrevem estado", async () => {
    const { unmount } = mount(<Probe />);
    await act(async () => {
      await new Promise((r) => setTimeout(r, 0));
    });
    unmount();
    const antes = st;
    emitSync({ status: "failed", changed: false, detail: "x" });
    expect(st).toBe(antes);
  });
});
