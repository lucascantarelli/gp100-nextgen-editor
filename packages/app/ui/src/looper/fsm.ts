/**
 * FSM do looper — TRANSIÇÕES PURAS, sem React (#82).
 *
 * Por que este arquivo existe: o estado de transporte vivia dentro do
 * `LooperPanel.tsx` (636 linhas), em callbacks que chamavam três `setState`.
 * Duas consequências, uma delas séria:
 *
 *  1. o ciclo rec→play→dub→rec e a saturação da fita (que vira PLAY no
 *     último segundo) só eram testáveis renderizando o painel INTEIRO;
 *  2. o `tick` fazia `setHasTape(true); setMode("play")` DENTRO do updater de
 *     `setSecs`. Updater de estado precisa ser puro: o React pode chamá-lo
 *     mais de uma vez (StrictMode, re-render concorrente), e aí a transição dispara
 *     duas vezes. O `tick` puro abaixo resolve isso por construção — a
 *     transição é um valor de retorno, não um efeito colateral.
 *
 * Nenhuma transição aqui muta o estado recebido: todas devolvem um estado
 * novo. É o que torna o módulo testável sem DOM, sem timer e sem React.
 */

/** Estado do transporte. `idle` = máquina parada sem fita; `stop` = parada COM fita. */
export type LooperMode = "idle" | "rec" | "play" | "dub" | "stop";

/**
 * Chave do rótulo no painel. `idle` NÃO aparece aqui: parado sem fita o
 * rótulo é `empty`, parado com fita é `ready`. O tipo diz isso para o `tsc`
 * cobrar no acesso a `MSG.looperMode` — que não tem (e não deve ter) `idle`.
 */
type ChaveModo = Exclude<LooperMode, "idle"> | "ready" | "empty";



/** Estado completo do transporte — tudo que a máquina sabe sobre a fita. */
export interface LooperState {
  readonly mode: LooperMode;
  /** Há fita na máquina (independe de o transporte estar rodando). */
  readonly hasTape: boolean;
  /** Posição em segundos no rolo. */
  readonly secs: number;
  /** O primeiro clique em "limpar" só ARMA; o segundo confirma. */
  readonly confirmClear: boolean;
}

/** Máquina nova: sem fita, parado no início. */
export function inicial(): LooperState {
  return { mode: "idle", hasTape: false, secs: 0, confirmClear: false };
}

/** O transporte está produzindo som? (VU da navbar só animate quando está.) */
export function rodando(state: LooperState): boolean {
  return state.mode === "rec" || state.mode === "play" || state.mode === "dub";
}

/**
 * ● REC — o botão e o atalho R compartilham esta transição:
 *   idle/stop → rec · rec → play (a fita recem-gravada) · play/dub → dub.
 */
export function rec(state: LooperState): LooperState {
  const base = { ...state, confirmClear: false };
  if (state.mode === "rec") return { ...base, mode: "play", hasTape: true };
  if (state.mode === "play" || state.mode === "dub") return { ...base, mode: "dub" };
  return { ...base, mode: "rec" };
}

/** ▶ PLAY — não faz nada sem fita; alterna play/stop quando há. */
export function play(state: LooperState): LooperState {
  if (!state.hasTape) return state;
  return { ...state, confirmClear: false, mode: state.mode === "play" ? "stop" : "play" };
}

/** ■ STOP — para mantendo a fita (é o que separa `stop` de `idle`). */
export function stop(state: LooperState): LooperState {
  return { ...state, mode: "stop", confirmClear: false };
}

/** ⏮ REW — volta ao início; não mexe no transporte nem na fita. */
export function rew(state: LooperState): LooperState {
  return { ...state, secs: 0 };
}

/**
 * Apagar a fita em DOIS passos: o primeiro clique só arma (e devolve o mesmo
 * estado se já estava armado, para o botão poder re-render), o segundo apaga.
 */
export function limpar(state: LooperState): LooperState {
  if (!state.confirmClear) return { ...state, confirmClear: true };
  return inicial();
}

/**
 * Um segundo de transporte.
 *
 * @param maxSecs Limite da fita (90 s em PRE, 45 s em POST).
 * @returns O estado no segundo seguinte. É aqui que a gravação SATURA: ao
 *   encher a fita ela vira PLAY e volta ao início — é o que uma máquina de
 *   fita faz ao chegar no fim do rolo durante a gravação.
 */
export function tick(state: LooperState, maxSecs: number): LooperState {
  if (!rodando(state)) return state;
  const limite = Math.max(1, maxSecs);

  if (state.mode === "rec" && !state.hasTape) {
    // Saturando a primeira gravação: a fita agora EXISTE e a máquina toca.
    if (state.secs + 1 >= limite) return { ...state, mode: "play", hasTape: true, secs: 0 };
    return { ...state, secs: state.secs + 1 };
  }

  // Tocando (ou dubando): o rolo dá a volta, infiniteamente.
  return { ...state, secs: (state.secs + 1) % limite };
}

/**
 * Rótulo do transporte, na ordem de precedência do painel.
 * `pronto`/`vazia` só valem com o transporte parado: rodando, o modo manda.
 */
export function chaveDoModo(state: LooperState): ChaveModo {
  // Só `idle` cai para o rótulo de fita. `stop` tem rótulo PRÓPRIO ("STOP"):
  // é o que diz ao usuário que a máquina parou com a fita dentro — STOP e
  // PRONTO não são a mesma informação.
  if (state.mode !== "idle") return state.mode;
  return state.hasTape ? "ready" : "empty";
}

/** Cor do modo: `rec`/`dub` acendem o tom de gravação, `play` o de toque. */
export function tomDoModo(state: LooperState): "rec" | "play" | "" {
  if (state.mode === "rec" || state.mode === "dub") return "rec";
  return state.mode === "play" ? "play" : "";
}

/**
 * Classes dos rolos: giram quando há transporte, e giram RÁPIDO só quando
 * grava ou sobrepõe (dub). Play gira na velocidade normal.
 *
 * Devolve as duas classes juntas porque elas entram no mesmo `className` — e
 * devolver só a "fast" obrigaria o componente a repetir a condição de `rodando`,
 * que é exatamente o que esta extração existe para não acontecer.
 */
export function classeDosRolos(state: LooperState): string {
  if (!rodando(state)) return "";
  const rapido = state.mode === "rec" || state.mode === "dub" ? " reel-fast" : "";
  return `reel-spin${rapido}`;
}

/** Fração da fita já gravada (0..1) — dirige o pacote dos dois rolos. */
export function fracaoGravada(state: LooperState, maxSecs: number): number {
  return Math.min(1, state.secs / Math.max(1, maxSecs));
}
