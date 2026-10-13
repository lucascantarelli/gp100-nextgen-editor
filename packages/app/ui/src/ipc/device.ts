/**
 * ipc/device — porta dos commands/eventos de device (componentes NUNCA
 * chamam `invoke`/`listen` espalhado nem importam gp100-core — o lint
 * bloqueia import direto de @tauri-apps).
 *
 * Fora do webview do Tauri (vitest/jsdom) `window.__TAURI_INTERNALS__` não
 * existe — o fallback de DEV/TESTE usa o MESMO shape do backend real
 * (MockDevice) e SIMULA o boot com progresso determinístico (2297 transações
 * no inventário default), p/ faixa de progresso e log de pushes fora do shell.
 *
 * Todo command passa por RETRY COM BACKOFF + TIMEOUT por tentativa (#20):
 * falha transitória não chega ao usuário; falha vira erro visível com
 * recuperação — nunca spinner eterno.
 */
import type {
  BoardView,
  BootProgress,
  BootReport,
  DeviceGlobals,
  DeviceInfo,
  PresetLibrary,
} from "./types";
import {
  FIXTURE_NOME_TESTE,
  fallbackGlobals,
  fallbackSetGlobal,
  localMockBoard,
  localMockLibrary,
} from "./fallbackData";
import { PRESET_COUNT } from "../i18n/facts";

/** Total de transações do script de boot real (inventário default 0..198). */
const BOOT_TOTAL = 2297;
/** Inventário do catálogo atual: 99 de fábrica + 99 de usuário (o gate da
 *  #161 valida o relatório do boot contra ESTA conta — mesma fonte do
 *  `PRESET_COUNT` que os dicionários usam: os números só mudam num lugar). */
const BOOT_INVENTARIO = PRESET_COUNT * 2;

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

/**
 * Gancho de teste/e2e — `localStorage[gp100.debug.writeVerified] = "false"`
 * faz o fallback LOCAL relatar um build de LEITURA (face (A) da #126), para a
 * política de botões ser exercitável sem compilar `write-verified`. Sem a
 * chave o mock segue a ADR-5: SEMPRE permite (`writeVerified: true`).
 *
 * Fora do fallback (webview real) não tem efeito: quem decide ali é a
 * constante de compilação do transporte (`real.rs`).
 */
const DEBUG_WRITE_KEY = "gp100.debug.writeVerified";

/** O gancho acima está armado? (seguro sem localStorage — ver `readFailRaw`). */
function leituraSimulada(): boolean {
  try {
    return localStorage.getItem(DEBUG_WRITE_KEY) === "false";
  } catch {
    return false;
  }
}

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
/**
 * Timeout POR TENTATIVA, DEFAULT de toda chamada — o command mais longo
 * comum (board) fica bem abaixo dele. **Quem é mais lento declara o próprio
 * prazo na política** ([`BOOT_TIMEOUT_MS`]): embrulhar o `device_boot` neste
 * default de 8 s o declarava falhado no MEIO do scan (a UI mostrava
 * "Falha no boot do device" enquanto o backend seguia lendo o aparelho).
 */
export const COMMAND_TIMEOUT_MS = 8_000;

/**
 * Prazo por tentativa do `device_boot`. **Medido em campo, não estimado:**
 * o script inteiro (T1 40 + scan dos 198 pps + sonda 11 + estado5 5 +
 * nomes 61 = 2297) levou 24,5 s — wire log `wire-20261012155337.jsonl`,
 * t=5851 → 30313 ms. 60 s cobre um aparelho lento sem virar spinner eterno:
 * um device que MORRE no meio do boot cai antes, no timeout de 3 s por
 * transação (D6 do ADR-6), e o invoke rejeita rápido.
 */
export const BOOT_TIMEOUT_MS = 60_000;

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
 *
 * **`timeoutMs` é o prazo POR TENTATIVA da política.** Omitido =
 * `COMMAND_TIMEOUT_MS` (8 s), que é decisão do `runCommand` e não da
 * política: quem não precisa de prazo próprio não carrega um. Declarar é
 * para o caso em que o default MENTIRIA — o `device_boot` leva ~25 s em
 * campo e o default de 8 s o declarava falhado no MEIO do scan, derrubando
 * a UI de campo com o aparelho sendo lido certinho.
 */
export type CommandPolicy =
  /** Repete até `attempts` vezes com backoff. Só para operation IDEMPOTENTE. */
  | { readonly kind: "retry"; readonly attempts: number; readonly timeoutMs?: number }
  /** Uma tentativa só. `reason` é obrigatório e aparece no log de debug. */
  | { readonly kind: "once"; readonly reason: string; readonly timeoutMs?: number };

/**
 * Política padrão do projeto: repetir é seguro para tudo que é idempotente
 * (leitura, selecionar o mesmo preset, gravar o mesmo parâmetro no mesmo
 * slot). O que NÃO é idempotente declara `once` com o motivo.
 */
export const IDEMPOTENTE: CommandPolicy = { kind: "retry", attempts: COMMAND_ATTEMPTS };

/**
 * `IDEMPOTENTE` que sabe que o `device_boot` está na sua frente.
 *
 * Na montagem do App o palco pede `device_preset_library` e os números da
 * biblioteca ENQUANTO o boot de ~25 s roda, e a fila do actor é serializada
 * (D8): essas chamadas só respondem quando o boot liberar. Com o default de
 * 8 s × 3 tentativas = 24 s, TODAS as tentativas morriam ~0,5 s ANTES do boot
 * terminar — o backend respondia (o wire log de 12/10 mostra os 11 bursts de
 * snapshot em t=31597..33828, logo após o boot fechar em 31577) e a UI já
 * tinha desistido, ficando com "Aparelho não conectado" e "Não foi possível
 * ler os números da biblioteca".
 *
 * O prazo é o do [`BOOT_TIMEOUT_MS`]: o que estas chamadas esperam É o boot.
 */
export const IDEMPOTENTE_ESPERA_BOOT: CommandPolicy = {
  kind: "retry",
  attempts: COMMAND_ATTEMPTS,
  timeoutMs: BOOT_TIMEOUT_MS,
};

/**
 * Constrói a política de "uma tentativa só". Existe como função — e não como
 * objeto literal solto — para que o `reason` seja obrigatório no ponto da
 * chamada, onde a decisão acontece.
 */
export function semRetry(reason: string, timeoutMs?: number): CommandPolicy {
  if (!reason.trim()) throw new Error("semRetry exige um motivo: a exceção precisa ser justificada");
  // O prazo só entra no objeto quando é DECLARADO: sem ele a política fica
  // exatamente como sempre esteve e herda o default do `runCommand`.
  return timeoutMs === undefined ? { kind: "once", reason } : { kind: "once", reason, timeoutMs };
}

/** Descrição da política, para log de debug e para o relatório de gates. */
export function descrevePolitica(p: CommandPolicy): string {
  const base = p.kind === "retry" ? `retry x${p.attempts}` : `once (${p.reason})`;
  // O prazo DECLARADO aparece na descrição: é a prova, no log de debug,
  // de que aquele command esperou mais que o default por decisão escrita.
  return p.timeoutMs === undefined ? base : `${base} · prazo ${p.timeoutMs} ms`;
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

/** Corrida do command contra o timeout da tentativa (o prazo é da política). */
function withTimeout<T>(run: Promise<T>, op: string, ms: number): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new CommandTimeoutError(`command ${op} excedeu ${ms} ms`)),
      ms,
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
  // Prazo da política, senão o global: o default de 8 s é para o command
  // comum; o que leva minutos (boot) declara o seu na própria política.
  const prazo = policy.timeoutMs ?? COMMAND_TIMEOUT_MS;
  let last: unknown;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      return await withTimeout(run(), op, prazo);
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

function localMockInfo(): DeviceInfo {
  // pp 0 é o primeiro do inventário e o default do boot (§13.4).
  return {
    backend: "mock",
    detail: "",
    presetCount: 99,
    currentPp: 0,
    currentName: FIXTURE_NOME_TESTE,
    currentPpType: 0,
    irSlotsWithCrc: 20,
    // A tabela de IRs no browser e a MESMA forma que o device real devolve
    // (§13.12): 20 slots, nomes vazios ate a primeira importacao.
    irSlots: Array.from({ length: 20 }, (_unused, slot) => ({ slot, name: "" })),
    // No mock a escrita e liberada por construcao (ADR-5: a trava e do
    // transporte real). Aqui o valor e `true` pelo mesmo motivo — salvo o
    // gancho `gp100.debug.writeVerified`, que simula o build de leitura.
    writeVerified: !leituraSimulada(),
    // O mock local "funciona": o boot simulado leu tudo e nada divergiu —
    // mesma verdade que o MockDevice do backend diria (#164).
    sync: "sync",
    syncDetail: "",
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
 * A política de escrita da TELA (#126 face (A); ADR-5): este aparelho deixa a
 * UI oferecer botões que gravam (knob, IR, SnapTone, gravar preset)?
 *
 * `true` só quando o backend disse que sim (`writeVerified`); sem informação
 * (null, ainda carregando) a resposta é `false` — uma trava que abre sozinha
 * antes de saber quem é o aparelho não é trava. É o ÚNICO lugar que lê o
 * campo para decidir botão: o `FieldDiagPanel` e os painéis de escrita
 * passam a concordar por construção, não por três leituras iguais.
 */
export function escritaLiberada(info: DeviceInfo | null | undefined): boolean {
  return info?.writeVerified === true;
}

/**
 * `device_boot` — boot+scan completos com barra de progresso.
 * Os beats chegam pelo listener registrado em `onBootProgress`; a promise
 * resolve com o relatório final (2297 no mock).
 *
 * **O prazo é declarado aqui** ([`BOOT_TIMEOUT_MS`]): em campo o script
 * leva ~25 s, e sem isto o default de 8 s rejeitava no MEIO do scan — a UI
 * caía no `.catch` com "Falha no boot do device" enquanto o aparelho estava
 * sendo lido certinho (wire log de 12/10: scan completo só terminou aos
 * 30,3 s e o relatório saiu 2297/198/198, dentro do gate #161).
 */
export async function deviceBoot(): Promise<BootReport> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return runCommand(
      "boot",
      semRetry(
        "boot sao 2297 transacoes: repetir mascararia o device morto. " +
          "A recuperacao e o re-scan explicito do usuario (issue #20).",
        BOOT_TIMEOUT_MS,
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
    ["state5", 5],
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
      else resolve({ transactions: BOOT_TOTAL, presets: BOOT_INVENTARIO, names: BOOT_INVENTARIO });
    };
    tick();
  });
}

/** Subscrições locais de evento (fallback dev/teste; Tauri usa o próprio). */
type ProgressListener = (p: BootProgress) => void;
const listeners = new Set<ProgressListener>();
const pushListeners = new Set<(hex: string) => void>();

/**
 * `device_conectar` — reconectar o aparelho (issue #150): o botão do aviso
 * "não conectado" vem para cá; no aparelho real o backend re-enumera as
 * portas MIDI e promove o estado de `Desligado` para `Real`.
 */
export async function deviceConectar(): Promise<void> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    // UMA tentativa: reconectar é a própria tentativa — repetir automático
    // esconde o estado real do aparelho (ausente) atrás de backoff.
    await runCommand(
      "conectar",
      semRetry("reconectar e a propria tentativa: repetir automático mascararia o aparelho ausente"),
      () => invoke("device_conectar"),
    );
    return;
  }
  await runCommand("conectar", IDEMPOTENTE, async () => {
    debugFail("conectar");
  });
}

/**
 * `device_board` — board do preset (pp null/ausente = corrente).
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

/**
 * `device_preset_library` — biblioteca completa + corrente.
 *
 * **(#150) a lista vem do APARELHO:** no shell real o backend devolve o
 * inventário medido; sem aparelho o erro do invoke é o sinal para a UI
 * mostrar o aviso de conexão (nunca uma lista falsa de fábrica).
 *
 * A política é [`IDEMPOTENTE_ESPERA_BOOT`] porque este invoke nasce na
 * montagem — no MEIO do boot (ver o motivo lá).
 */
export async function devicePresetLibrary(): Promise<PresetLibrary> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return runCommand("library", IDEMPOTENTE_ESPERA_BOOT, () =>
      invoke<PresetLibrary>("device_preset_library"),
    );
  }
  // Fallback de TESTE: inventário com nome vazio (mesma forma do real).
  return runCommand("library", IDEMPOTENTE, async () => localMockLibrary());
}

/**
 * `device_globals` — o bloco medido de registradores globais (§13.13:
 * SYSTEM `000200xx` + DRUM `000102xx`), lido DO device no boot validado.
 * O seed da UI (master/DRUM) vem daqui; sem device ou sem bloco a UI
 * segue no localStorage (fallback honesto, sem banner).
 */
export async function deviceGlobals(): Promise<DeviceGlobals> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return runCommand("globals", IDEMPOTENTE, () => invoke<DeviceGlobals>("device_globals"));
  }
  return runCommand("globals", IDEMPOTENTE, async () => fallbackGlobals());
}

/**
 * `device_set_global` — escreve num registrador global (§13.13.1) pela
 * CHAVE do allowlist do backend (o front nunca manda endereço cru). Sem
 * write-verified (ADR-5) o transporte recusa antes do driver e o erro
 * sobe: a UI trata como prévia local (mesmo tratamento do knob).
 */
export async function deviceSetGlobal(chave: string, valor: number): Promise<void> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    // Retentável: reescrever o MESMO valor não acumula efeito (write idempotente).
    await runCommand("set_global", IDEMPOTENTE, () =>
      invoke("device_set_global", { chave, valor }),
    );
    return;
  }
  await runCommand("set_global", IDEMPOTENTE, async () => {
    fallbackSetGlobal(chave, valor);
  });
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

/* Sync (#164: onDeviceSync/emitSync/device_sync_now) saiu para `ipc/sync.ts`. */
