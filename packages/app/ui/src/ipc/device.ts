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
 *
 * Todo command passa por RETRY COM BACKOFF + TIMEOUT por tentativa (issue
 * #20): falha transitória (USB instável) não chega ao usuário; falha
 * permanente vira erro visível com recuperação — nunca spinner eterno.
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

/** Operações do fallback com gancho de falha simulada (issue #20). */
type FailOp = "info" | "boot" | "board" | "select" | "set_param";

/**
 * Gancho de teste/e2e — `localStorage[gp100.debug.failDevice]`:
 *   - `"<op>"` (info|boot|board|select|set_param) ou `"all"` — falha SEMPRE;
 *   - `"<op>:<n>"` — falha as PRÓXIMAS n chamadas (falha TRANSITÓRIA: o
 *     retry/backoff a esconde do usuário — cenário do USB instável);
 *   - `"boot-mid"` — o boot emite progresso até ~40% e REJEITA (device
 *     desconectado NO MEIO do boot).
 * Fora do fallback (webview real) não tem efeito; sem a chave, custo zero.
 */
const DEBUG_FAIL_KEY = "gp100.debug.failDevice";

function readFailRaw(): string | null {
  try {
    return localStorage.getItem(DEBUG_FAIL_KEY);
  } catch {
    return null; // sem localStorage (teste sem storage) — ganho inativo
  }
}

/** Consome 1 uso do plano `op:n` (decrementa; em 0 remove a chave). */
function consumeTransient(op: string, left: number): void {
  try {
    if (left <= 1) localStorage.removeItem(DEBUG_FAIL_KEY);
    else localStorage.setItem(DEBUG_FAIL_KEY, `${op}:${left - 1}`);
  } catch {
    /* noop */
  }
}

/** Falha simulada da operação `op` (no-op sem a chave/para outra operação). */
function debugFail(op: FailOp): void {
  const raw = readFailRaw();
  if (raw === null) return;
  if (raw === "all" || raw === op) {
    throw new Error(`debug: falha simulada de device em ${op}`);
  }
  const [nome, n] = raw.split(":");
  if (nome === op) {
    const left = Number(n);
    if (Number.isInteger(left) && left > 0) {
      consumeTransient(op, left);
      throw new Error(`debug: falha simulada de device em ${op} (${left} restantes)`);
    }
  }
}

/* ─── Retry/backoff e timeout de command (issue #20) ──────────────────────
 * A política vive AQUI (porta única do front) porque a garantia pedida é de
 * UI: "recuperação visível, nunca spinner eterno". O backend já tem timeout
 * POR TRANSAÇÃO (D6 do ADR-6); retry aqui = nova transação na fila
 * serializada do actor (D8) — mesmo caminho, observável em teste.
 *
 * Leitura é sempre retentável (idempotente). `select`/`set_param` reenviam o
 * MESMO destino/valor (set de knob é fire-and-forget §13.11 — reaplicar o
 * mesmo valor não acumula efeito). O `device_boot` fica FORA da política:
 * são 2297 transações e um retry automático mascararia device morto — a
 * recuperação ali é o ⟳ explícito do usuário.
 */
export const COMMAND_ATTEMPTS = 3;
/** Base do backoff exponencial (120 → 240 ms entre tentativas). */
export const COMMAND_BASE_MS = 120;
/** Timeout POR TENTATIVA — o command mais longo (board) fica bem abaixo. */
export const COMMAND_TIMEOUT_MS = 8_000;

/** Timeout do command (classe própria: o teste distingue de erro do device). */
export class CommandTimeoutError extends Error {}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

/**
 * Backoff exponencial com jitter de ±30% — retries simultâneos não batem no
 * device no mesmo instante.
 *
 * @param attempt Nº da tentativa que FALHOU (1 = primeira falha).
 * @param rand Fonte de aleatoriedade (injetável no teste).
 * @returns Atraso em ms antes da próxima tentativa.
 */
export function retryDelayMs(attempt: number, rand: () => number = Math.random): number {
  const base = COMMAND_BASE_MS * 2 ** (attempt - 1);
  const jitter = base * 0.3 * (rand() * 2 - 1);
  return Math.max(0, Math.round(base + jitter));
}

/** Corrida do command contra o timeout da tentativa. */
function withTimeout<T>(run: Promise<T>, op: string): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new CommandTimeoutError(`command ${op} excedeu ${COMMAND_TIMEOUT_MS} ms`)),
      COMMAND_TIMEOUT_MS,
    );
    run.then(
      (v) => {
        clearTimeout(timer);
        resolve(v);
      },
      (e: unknown) => {
        clearTimeout(timer);
        reject(e);
      },
    );
  });
}

/** Executa o command com timeout por tentativa e retry com backoff. */
async function withRetry<T>(op: string, run: () => Promise<T>): Promise<T> {
  let last: unknown;
  for (let attempt = 1; attempt <= COMMAND_ATTEMPTS; attempt += 1) {
    try {
      return await withTimeout(run(), op);
    } catch (e) {
      last = e;
      if (attempt < COMMAND_ATTEMPTS) await sleep(retryDelayMs(attempt));
    }
  }
  throw last;
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
    return withRetry("info", () => invoke<DeviceInfo>("device_info"));
  }
  return withRetry("info", async () => {
    debugFail("info");
    return localMockInfo();
  });
}

/**
 * `device_boot` — boot+scan completos com barra de progresso.
 * Os beats chegam pelo listener registrado em `onBootProgress`; a promise
 * resolve com o relatório final (2297 no mock).
 */
export async function deviceBoot(): Promise<BootReport> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    // SEM retry (política da issue #20): boot é longo e não-idempotente em
    // custo; falha → erro visível com ⟳.
    return invoke<BootReport>("device_boot");
  }
  debugFail("boot");
  // "boot-mid": o device cai no MEIO do boot — progresso real aparece e só
  // então o command rejeita (o que um cabo puxado faz com a UI aberta).
  const midFail = readFailRaw() === "boot-mid";
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
  return new Promise<BootReport>((resolve, reject) => {
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
      if (midFail && done >= Math.floor(BOOT_TOTAL * 0.4)) {
        reject(
          new Error(`debug: device desconectado durante o boot (${done}/${BOOT_TOTAL} transações)`),
        );
        return;
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
    return withRetry("board", () => invoke<BoardView>("device_board", { pp: pp ?? null }));
    // NOTE: quando o backend real responder, os knobs vêm do dicionário.
  }
  return withRetry("board", async () => {
    debugFail("board");
    return localMockBoard(pp);
  });
}

/** `device_preset_library` — biblioteca completa + corrente. */
export async function devicePresetLibrary(): Promise<PresetLibrary> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return withRetry("library", () => invoke<PresetLibrary>("device_preset_library"));
  }
  // A biblioteca vem de ARTEFATO local (nunca do fio) — sem gancho de falha;
  // timeout/retry valem por uniformidade da porta única.
  return withRetry("library", async () => localMockLibrary());
}

/** `device_select_preset` — select real no device (fallback troca local). */
export async function deviceSelectPreset(pp: number): Promise<void> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    await withRetry("select", () => invoke("device_select_preset", { pp }));
    return;
  }
  // fallback: sem estado global local além do corrente informado.
  // Retentável: reenviar o MESMO pp não acumula efeito (o select é destino).
  await withRetry("select", async () => {
    debugFail("select");
  });
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
    await withRetry("set_param", () => invoke("device_set_param", { slot, code, ctrl, value }));
    return;
  }
  // Retentável: set de knob é fire-and-forget (D4) e reaplicar o MESMO valor
  // é idempotente no device.
  await withRetry("set_param", async () => {
    debugFail("set_param");
  });
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
