/**
 * ipc/device — ÚNICA porta do front para os commands/eventos de device
 * (Porta única do front: componentes NUNCA chamam `invoke`/`listen` espalhado
 * nem importam gp100-core — o lint bloqueia import direto de @tauri-apps).
 *
 * Fora do webview do Tauri (vitest/jsdom), `window.__TAURI_INTERNALS__`
 * não existe — o fallback de DEV/TESTE usa o MESMO shape do backend real
 * (MockDevice) e SIMULA o boot com progresso determinístico (mesmos totais
 * do script real: 2297 transações no inventário default), para a faixa de
 * progresso do boot e o log de pushes serem exercitados fora do shell.
 */
import type {
  BoardView,
  BootProgress,
  BootReport,
  DeviceInfo,
  PresetLibrary,
} from "./types";
import { ARCHETYPE_OF, CHAIN_FAMILIES } from "./types";
import type { ChainFamily } from "./types";
import { FX_MODULES } from "../artifacts/fxData";
import { FACTORY_PRESETS } from "../artifacts/presetData";

/** Total de transações do script de boot real (inventário default 0..198). */
const BOOT_TOTAL = 2297;

/**
 * Gancho de teste/e2e: com `localStorage[gp100.debug.failDevice]`
 * = "info" | "boot" | "board" | "all", o fallback correspondente REJEITA —
 * permite exercitar os estados de erro da UI no browser. Fora do fallback
 * (webview real) não tem efeito; sem a chave, custo zero.
 */
const DEBUG_FAIL_KEY = "gp100.debug.failDevice";
function debugFail(op: "info" | "boot" | "board"): void {
  let raw: string | null;
  try {
    raw = localStorage.getItem(DEBUG_FAIL_KEY);
  } catch {
    return; // sem localStorage (teste sem storage) — ganho inativo
  }
  if (raw === op || raw === "all") throw new Error(`debug: falha simulada de device em ${op}`);
}

/** Detecta o ambiente Tauri (webview) vs browser/teste. */
function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Fallback determinístico p/ dev-fora-do-tauri e testes (= MockDevice). */
function localMockInfo(): DeviceInfo {
  // O preset corrente vem do MESMO artefato da biblioteca (all.prst):
  // nome/tipo reais, nunca transcritos à mão.
  const current = FACTORY_PRESETS[0];
  return {
    backend: "mock",
    presetCount: FACTORY_PRESETS.length,
    currentPp: current.pp,
    currentName: current.name,
    currentPpType: current.ppType,
    irSlotsWithCrc: 20,
  };
}

/** `device_info` — estado do device (mock: sem tráfego; real: B1 do H1). */
export async function deviceInfo(): Promise<DeviceInfo> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<DeviceInfo>("device_info");
  }
  debugFail("info");
  return localMockInfo();
}

/**
 * `device_boot` — boot+scan completos com barra de progresso.
 * Os beats chegam pelo listener registrado em `onBootProgress`; a promise
 * resolve com o relatório final (2297 no mock).
 */
export async function deviceBoot(): Promise<BootReport> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<BootReport>("device_boot");
  }
  debugFail("boot");
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
  // Lote CAP: 2297 beats síncronos travam a main thread (~300 ms congelados).
  // Emite em lotes de 64 via macrotask — o rAF do useBoot segue fluindo e a
  // UI nunca congela; mesmos totais/stages do script real.
  return new Promise<BootReport>((resolve) => {
    const emit = (stage: BootProgress["stage"]) =>
      listeners.forEach((l) =>
        l({ stage, done, total: BOOT_TOTAL, currentPp: Math.min(done % 198, 197) }),
      );
    let si = 0;
    let i = 0;
    const tick = () => {
      for (let c = 0; c < 64 && si < stages.length; c += 1) {
        const [stage, n] = stages[si];
        done += 1;
        i += 1;
        emit(stage);
        if (i >= n) {
          si += 1;
          i = 0;
        }
      }
      if (si < stages.length) setTimeout(tick, 0);
      else resolve({ transactions: BOOT_TOTAL });
    };
    tick();
  });
}

/** Subscrições locais de evento (fallback dev/teste; Tauri usa o próprio). */
type ProgressListener = (p: BootProgress) => void;
const listeners = new Set<ProgressListener>();
const pushListeners = new Set<(hex: string) => void>();

/* ─── Board/biblioteca (dados do pedalboard artístico) ─── */

/**
 * Cadeia de dev p/ o fallback do board (mesmos nomes reais do all.prst;
 * knobs com ranges reais e code DERIVADO do dicionário). Pre/DST/AMP/NR/
 * CAB/EQ/MOD/DLY/RVB.
 */
function localMockBoard(pp?: number): BoardView {
  // Nome/tipo REAIS do preset alvo (artefato da biblioteca); a cadeia é a
  // de fábrica com os dados do catálogo gerado (fxData.ts — parameters.json):
  // nomes, ranges, defaults e a variante que escolhe o MODELO do pedal.
  const preset = FACTORY_PRESETS[pp ?? 0] ?? FACTORY_PRESETS[0];
  const mk = (module: ChainFamily, name: string, state: boolean): BoardView["slots"][number] => {
    const alg = FX_MODULES[module]?.find((a) => a.name === name);
    if (!alg) throw new Error(`fxData sem ${module}/${name}`);
    // code derivado do ARTEFATO (nibble do módulo no byte alto + index do
    // algoritmo) — nunca transcrita à mão, que já divergiu do device.
    const code = (alg.nibble << 24) | alg.index;
    return {
      slot: CHAIN_FAMILIES.indexOf(module),
      family: module,
      archetype: ARCHETYPE_OF[module],
      name,
      variant: alg.variant,
      code,
      state,
      knobs: [
        ...alg.knobs.map((k) => ({
          name: k.name,
          pos: k.pos,
          kind: "knob" as const,
          range: k.min != null && k.max != null ? ([k.min, k.max] as [number, number]) : undefined,
          options: [],
          value: k.default ?? undefined,
          default: k.default ?? undefined,
        })),
        ...alg.switches.map((k) => ({
          name: k.name,
          pos: k.pos,
          kind: "switch" as const,
          options: k.options,
          value: k.default ?? undefined,
          default: k.default ?? undefined,
        })),
        ...alg.comboxes.map((k) => ({
          name: k.name,
          pos: k.pos,
          kind: "combox" as const,
          options: k.options,
          value: k.default ?? undefined,
          default: k.default ?? undefined,
        })),
      ],
    };
  };
  return {
    pp: preset.pp,
    name: preset.name,
    ppType: preset.ppType,
    ppTypeName: preset.ppTypeName,
    slots: [
      mk("PRE", "COMP", true),
      mk("DST", "Green OD", true),
      mk("AMP", "Bog RedM", true),
      mk("NR", "Gate 1", false),
      mk("CAB", "UK-GN 4x12", true),
      mk("EQ", "EQ 1", true),
      mk("MOD", "A-Chorus", true),
      mk("DLY", "Vin-Rack", true),
      mk("RVB", "N-Star", true),
    ],
  };
}

/** Biblioteca do fallback (as 99 REAIS do artefato gerado do all.prst). */
function localMockLibrary(): PresetLibrary {
  return { entries: [...FACTORY_PRESETS], currentPp: 0 };
}

/**
 * `device_board` — board do preset (pp null/ausente = corrente).
 * O fallback usa o MESMO artefato da biblioteca: nome/tipo do preset vêm
 * de presetData (histórico: o fallback ignorava o pp e a navbar/LED
 * congelavam em "It's GP100" — comportamento travado por unit e e2e).
 */
export async function deviceBoard(pp?: number): Promise<BoardView> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<BoardView>("device_board", { pp: pp ?? null });
    // NOTE: quando o backend real responder, os knobs vêm do dicionário.
  }
  debugFail("board");
  return localMockBoard(pp);
}

/** `device_preset_library` — biblioteca completa + corrente. */
export async function devicePresetLibrary(): Promise<PresetLibrary> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<PresetLibrary>("device_preset_library");
  }
  return localMockLibrary();
}

/** `device_select_preset` — select real no device (fallback troca local). */
export async function deviceSelectPreset(pp: number): Promise<void> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("device_select_preset", { pp });
    return;
  }
  // fallback: sem estado global local além do corrente informado
}

/** `device_set_param` — ajuste de knob real no device (fallback aceita e segue). */
export async function deviceSetParam(
  slot: number,
  code: number,
  ctrl: number,
  value: number,
): Promise<void> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("device_set_param", { slot, code, ctrl, value });
  }
}

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
 * Evento `device://push` — pushes não solicitados do device (drenados pelo
 * actor), hex cru para o log da UI. (O producer no actor entra no
 * próximo passo; o listener já existe para a UI.)
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
