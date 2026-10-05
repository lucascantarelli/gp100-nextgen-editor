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
import { ARCHETYPE_OF } from "./types";
import type { BoardSlot, ChainFamily } from "./types";
import { FX_MODULES } from "../artifacts/fxData";
import { FACTORY_PRESETS } from "../artifacts/presetData";
import { PRESET_CHAINS } from "../artifacts/presetChains";

/** Total de transações do script de boot real (inventário default 0..198). */
const BOOT_TOTAL = 2297;

/**
 * Gancho de teste/e2e — `localStorage[gp100.debug.failDevice]`:
 *   - `"<op>"` (info|boot|board|select|set_param|save|dump|log_start|preview)
 *     ou `"all"` — falha SEMPRE;
 *   - `"<op>:<n>"` — falha as PRÓXIMAS n chamadas (falha TRANSITÓRIA: o
 *     retry/backoff a esconde do usuário — cenário do USB instável);
 *   - `"boot-mid"` — o boot emite progresso até ~40% e REJEITA (device
 *     desconectado NO MEIO do boot).
 * Fora do fallback (webview real) não tem efeito; sem a chave, custo zero.
 *
 * **Exportado para `ipc/diag`** — a porta de diagnóstico de campo usa O MESMO
 * gancho (é o mesmo aparelho), e duplicar o leitor de localStorage aqui e lá
 * faria os dois divergirem na primeira edição.
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
export function debugFail(op: string): void {
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

/**
 * POLÍTICA DE EXECUÇÃO — declarada por chamada, nunca herdada (#82).
 *
 * Antes, `withRetry` tinha o nº de tentativas fixo no corpo: toda chamada nova
 * nascia com retry sem ninguém decidir isso, e a ÚNICA exceção (o boot) era a
 * ausência de uma função — um detalhe que se perde na leitura. Um chamador
 * não conseguia saber se a chamada repetia sem abrir a implementação.
 *
 * Agora a política é um PARÂMETRO OBRIGATÓRIO: o compilador cobra a decisão em
 * toda chamada nova. E `once` exige `reason`, porque "não repetir" sem motivo
 * escrito é exatamente o comentário que o próximo author apaga sem pensar.
 *
 * **Exportada para a porta de tons (#25),** que precisa do MESMO tipo com um
 * timeout diferente: o envio de SnapTone leva ~36 s (143 blocos × 250 ms do
 * §4), e o timeout global de `runCommand` (8 s) o declararia falhado logo no
 * começo. A porta do envio declara a política e o prazo, e o motivo fica escrito
 * no mesmo lugar da decisão.
 */
export type CommandPolicy =
  /** Repete até `attempts` vezes com backoff. Só para operation IDEMPOTENTE. */
  | { readonly kind: "retry"; readonly attempts: number }
  /** Uma tentativa só. `reason` é obrigatório e aparece no log de debug. */
  | { readonly kind: "once"; readonly reason: string };

/**
 * Política padrão do projeto: repetir é seguro para tudo que é idempotente
 * (leitura, selecionar o mesmo preset, gravar o mesmo parâmetro no mesmo
 * slot). O que NÃO é idempotente declara `once` com o motivo.
 */
export const IDEMPOTENTE: CommandPolicy = { kind: "retry", attempts: COMMAND_ATTEMPTS };

/**
 * Constrói a política de "uma tentativa só". Existe como função — e não como
 * objeto literal solto — para que o `reason` seja obrigatório no ponto da
 * chamada, onde a decisão acontece.
 */
export function semRetry(reason: string): CommandPolicy {
  if (!reason.trim()) throw new Error("semRetry exige um motivo: a exceção precisa ser justificada");
  return { kind: "once", reason };
}

/** Descrição da política, para log de debug e para o relatório de gates. */
export function descrevePolitica(p: CommandPolicy): string {
  return p.kind === "retry" ? `retry x${p.attempts}` : `once (${p.reason})`;
}

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

/**
 * Executa o command conforme a política DECLARADA na chamada.
 *
 * `policy` é parâmetro obrigatório e não tem default: é o que impede a
 * herança silenciosa. `IDEMPOTENTE` é o valor explícito para o caso comum.
 *
 * **Exportada para o `ipc/library.ts` (#26).** Quando a política virou tipo, a
 * pergunta seguinte foi "e a porta que aparecer depois?". A resposta é esta: a
 * execução é uma coisa só, e a decisão é de quem chama. A biblioteca escolhe
 * `semRetry("...")` justamente porque I/O local não melhora com repetição — e
 * sem este `export`, ela ficaria sem timeout nem backoff, que é pior.
 */
export async function runCommand<T>(
  op: string,
  policy: CommandPolicy,
  run: () => Promise<T>,
): Promise<T> {
  const attempts = policy.kind === "retry" ? Math.max(1, policy.attempts) : 1;
  let last: unknown;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      return await withTimeout(run(), op);
    } catch (e) {
      last = e;
      if (attempt < attempts) await sleep(retryDelayMs(attempt));
    }
  }
  throw last;
}

/**
 * Detecta o ambiente Tauri (webview) vs browser/teste.
 *
 * **Exportado para `ipc/diag`:** toda porta precisa decidir entre `invoke` e o
 * fallback local, e essa decisão é uma só — duas cópias significam dois lugares
 * para o fallback e o aparelho discordarem.
 */
export function inTauri(): boolean {
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
    // A tabela de IRs no browser e a MESMA forma que o device real devolve
    // (§13.12): 20 slots, nomes vazios ate a primeira importacao.
    irSlots: Array.from({ length: 20 }, (_unused, slot) => ({ slot, name: "" })),
    // No mock a escrita e liberada por construcao (ADR-5: a trava e do
    // transporte real). Aqui o valor e `true` pelo mesmo motivo.
    writeVerified: true,
  };
}

/** `device_info` — estado do device (mock: sem tráfego; real: B1 do H1). */
export async function deviceInfo(): Promise<DeviceInfo> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return runCommand("info", IDEMPOTENTE, () => invoke<DeviceInfo>("device_info"));
  }
  return runCommand("info", IDEMPOTENTE, async () => {
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
    return runCommand(
      "boot",
      semRetry(
        "boot sao 2297 transacoes: repetir mascararia o device morto. " +
          "A recuperacao e o re-scan explicito do usuario (issue #20).",
      ),
      () => invoke<BootReport>("device_boot"),
    );
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
 * Cadeia do fallback de DEV/TESTE — MESMA leitura do core Rust
 * (`pedalboard::board_view_for`): efeito, `effectCode` e `params_0..14`
 * REAIS do preset alvo, vindos de `presetChains.ts` (GERADO do all.prst).
 *
 * Antes desta fonte a cadeia era FIXA (COMP/Green OD/Bog RedM…): trocar de
 * preset mudava o nome do LED e os 9 pedais seguiam iguais — o preset não
 * aparecia no pedalboard. Agora cada preset abre a SUA cadeia, inclusive a
 * ordem real dos pedais (20 dos 99 têm a cadeia trocada: `@x` manda).
 *
 * O valor do knob vem de `params[pos]` (mesma ordem do dicionário) só quando
 * é plausível: dentro do range do controle, e nunca o sentinel 0xFFFF
 * (65535 = “não configurado”) — fora disso cai no default do dicionário.
 * Regra do core para algoritmo fora do dicionário: pedal SEM knobs (nunca
 * adivinhar controle).
 */
function localMockBoard(pp?: number): BoardView {
  const preset = FACTORY_PRESETS[pp ?? 0] ?? FACTORY_PRESETS[0];
  const chain = PRESET_CHAINS.find((c) => c.pp === preset.pp) ?? PRESET_CHAINS[0];

  const algFor = (family: ChainFamily, code: number, name: string) =>
    FX_MODULES[family]?.find((a) => ((a.nibble << 24) | a.index) === code) ??
    FX_MODULES[family]?.find((a) => a.name === name);

  /** valor cru do preset → valor de knob (ou undefined = usa o default) */
  const realValue = (
    raw: string | null,
    range: [number, number] | undefined,
    options: string[],
  ): string | undefined => {
    if (raw == null) return undefined;
    const n = Number(raw);
    if (!Number.isFinite(n)) return undefined;
    if (options.length > 0) {
      // switch/combox: índice da lista (o que o dicionário usa como default)
      return Number.isInteger(n) && n >= 0 && n < options.length ? String(n) : undefined;
    }
    if (n === 65535) return undefined; // sentinel 0xFFFF = não configurado
    if (range && (n < Math.min(...range) || n > Math.max(...range))) return undefined;
    return String(n);
  };

  const slots: BoardSlot[] = chain.slots.map((s) => {
    const alg = algFor(s.family, s.code, s.name);
    const knob = (
      k: { name: string; pos: number; default?: string | null; min?: number | null; max?: number | null },
      kind: "knob" | "switch" | "combox",
      options: string[] = [],
    ) => {
      const range =
        kind === "knob" && k.min != null && k.max != null ? ([k.min, k.max] as [number, number]) : undefined;
      const dflt = k.default ?? undefined;
      return {
        name: k.name,
        pos: k.pos,
        kind,
        range,
        options,
        value: realValue(s.params[k.pos], range, options) ?? dflt,
        default: dflt,
      };
    };

    return {
      slot: s.slot,
      family: s.family,
      archetype: ARCHETYPE_OF[s.family],
      name: s.name,
      // algoritmo fora do dicionário: o pedal renderiza sem knobs (regra R1)
      variant: alg?.variant ?? "generic",
      state: s.state,
      code: s.code,
      knobs: alg
        ? [
            ...alg.knobs.map((k) => knob(k, "knob")),
            ...alg.switches.map((k) => knob(k, "switch", k.options)),
            ...alg.comboxes.map((k) => knob(k, "combox", k.options)),
          ]
        : [],
    };
  });

  return {
    pp: preset.pp,
    name: preset.name,
    ppType: preset.ppType,
    ppTypeName: preset.ppTypeName,
    slots,
    bank: "factory" as const,
    ppLabel: `P${String(preset.pp + 1).padStart(2, "0")}`,
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
    return runCommand("board", IDEMPOTENTE, () =>
      invoke<BoardView>("device_board", { pp: pp ?? null }),
    );
    // NOTE: quando o backend real responder, os knobs vêm do dicionário.
  }
  return runCommand("board", IDEMPOTENTE, async () => {
    debugFail("board");
    return localMockBoard(pp);
  });
}

/** `device_preset_library` — biblioteca completa + corrente. */
export async function devicePresetLibrary(): Promise<PresetLibrary> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return runCommand("library", IDEMPOTENTE, () =>
      invoke<PresetLibrary>("device_preset_library"),
    );
  }
  // A biblioteca vem de ARTEFATO local (nunca do fio) — sem gancho de falha;
  // timeout/retry valem por uniformidade da porta única.
  return runCommand("library", IDEMPOTENTE, async () => localMockLibrary());
}

/** `device_select_preset` — select real no device (fallback troca local). */
export async function deviceSelectPreset(pp: number): Promise<void> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    await runCommand("select", IDEMPOTENTE, () => invoke("device_select_preset", { pp }));
    return;
  }
  // fallback: sem estado global local além do corrente informado.
  // Retentável: reenviar o MESMO pp não acumula efeito (o select é destino).
  await runCommand("select", IDEMPOTENTE, async () => {
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
    await runCommand(
      "set_param",
      IDEMPOTENTE,
      () => invoke("device_set_param", { slot, code, ctrl, value }),
    );
    return;
  }
  // Retentável: set de knob é fire-and-forget (D4) e reaplicar o MESMO valor
  // é idempotente no device.
  await runCommand("set_param", IDEMPOTENTE, async () => {
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
