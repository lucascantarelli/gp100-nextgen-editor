/**
 * ipc/device — ÚNICA porta do front para os commands de device (R1 no front:
 * componentes NUNCA chamam `invoke` espalhado nem importam gp100-core).
 *
 * Fora do webview do Tauri (vitest/jsdom), `window.__TAURI__` não existe —
 * o fallback de DEV/TESTE usa o MESMO shape do backend real (MockDevice:
 * 99 presets, "It's GP100" @ 0x0000 — valores do mock do core, não inventados).
 * O fallback é explicitamente marcado `kind: "mock-local"` para a UI nunca
 * confundir com o backend real.
 */
import type { DeviceInfo } from "./types";

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
