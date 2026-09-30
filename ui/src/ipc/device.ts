/**
 * ipc/device — ÚNICA porta do front para os commands/eventos de device
 * (R1 no front: componentes NUNCA chamam `invoke`/`listen` espalhado nem
 * importam gp100-core).
 *
 * Fora do webview do Tauri (vitest/jsdom), `window.__TAURI_INTERNALS__`
 * não existe — o fallback de DEV/TESTE usa o MESMO shape do backend real
 * (MockDevice) e SIMULA o boot com progresso determinístico (mesmos totais
 * do script real: 2297 transações no inventário default), para a
 * ConnectionBar/barra de progresso e o log de pushes serem exercitados
 * fora do shell.
 */
import type { BootProgress, BootReport, DeviceInfo } from "./types";

/** Totais do script real (boot_inner do core; default inventory 0..198). */
const BOOT_TOTAL = 2297;

/** Detecta o ambiente Tauri (webview) vs browser/teste. */
function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Fallback determinístico p/ dev-fora-do-tauri e testes (= MockDevice). */
function localMockInfo(): DeviceInfo {
  return {
    backend: "mock",
    presetCount: 99,
    currentPp: 0x0000,
    currentName: "It's GP100",
    currentPpType: 4,
    irSlotsWithCrc: 20,
  };
}

/** `device_info` — estado do device (mock: sem tráfego; real: B1 do H1). */
export async function deviceInfo(): Promise<DeviceInfo> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<DeviceInfo>("device_info");
  }
  return localMockInfo();
}

/**
 * `device_boot` — boot+scan completos (§13.10) com barra de progresso.
 * Os beats chegam pelo listener registrado em `onBootProgress`; a promise
 * resolve com o relatório final (2297 no mock).
 */
export async function deviceBoot(): Promise<BootReport> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<BootReport>("device_boot");
  }
  // Fallback: progresso sintético determinístico (mesmos stages do script).
  const stages: Array<[BootProgress["stage"], number]> = [
    ["tables", 40],
    ["scan", 2179],
    ["probe", 11],
    ["setlist", 5],
    ["names", 60],
    ["keepalive", 2],
  ];
  let done = 0;
  for (const [stage, n] of stages) {
    for (let i = 0; i < n; i += 1) {
      done += 1;
      listeners.forEach((l) =>
        l({ stage, done, total: BOOT_TOTAL, currentPp: Math.min(done % 198, 197) }),
      );
    }
  }
  return { transactions: BOOT_TOTAL };
}

/** Subscrições locais de evento (fallback dev/teste; Tauri usa o próprio). */
type ProgressListener = (p: BootProgress) => void;
const listeners = new Set<ProgressListener>();
const pushListeners = new Set<(hex: string) => void>();

/**
 * Evento `device://progress` — 1 beat por transação do boot. Devolve o
 * unlisten (chamar no cleanup do hook).
 */
export async function onBootProgress(
  cb: ProgressListener,
): Promise<() => void> {
  if (inTauri()) {
    const { listen } = await import("@tauri-apps/api/event");
    const un = await listen<BootProgress>("device://progress", (e) => cb(e.payload));
    return un;
  }
  listeners.add(cb);
  return () => {
    listeners.delete(cb);
  };
}

/**
 * Evento `device://push` — pushes do device (backlog D7 drenado pelo
 * actor), hex cru para o log da UI. (M1.2 liga o producer no actor; o
 * listener já existe para a UI.)
 */
export async function onDevicePush(cb: (hex: string) => void): Promise<() => void> {
  if (inTauri()) {
    const { listen } = await import("@tauri-apps/api/event");
    const un = await listen<string>("device://push", (e) => cb(e.payload));
    return un;
  }
  pushListeners.add(cb);
  return () => {
    pushListeners.delete(cb);
  };
}
