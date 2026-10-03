/**
 * userPatches — patches de USUÁRIO (patches salvos pelo dono).
 *
 * Política de hardware (a mesma do resto do app): salvar no device precisa
 * do canal de escrita via USB, que ainda NÃO existe — então o patch de
 * usuário aqui é PRÉVIA LOCAL (localStorage). A cadeia salva é o SNAPSHOT
 * do board no momento do save (os 9 slots com nome/estado/knobs), e é ela
 * que volta ao pedalboard quando o patch é aberto.
 *
 * A escrita real (save_preset / upload da biblioteca) entra quando o canal
 * existir; o formato já é o mesmo do `.prst` (PP ppID + Effect), então a
 * migração é escrever o arquivo, não refazer a UI.
 */
import type { BoardSlot, BoardView } from "./ipc/types";
import { MSG } from "./i18n/messages";

const KEY = "gp100.userpatch.v1";

export interface UserPatch {
  /** id estável da lista (não é índice: apagar não renumera o resto). */
  id: string;
  name: string;
  /** rótulo do patch de fábrica de onde veio — só contexto na UI. */
  fromLabel: string;
  /** ISO do instante do save (a lista mostra a data). */
  savedAt: string;
  /** snapshot da cadeia: slots com knobs, tal como o palco leu. */
  slots: BoardSlot[];
}

/** Clona a cadeia: o patch é um RETRATO — mexer no pedal depois não altera. */
function cloneSlots(slots: BoardSlot[]): BoardSlot[] {
  return slots.map((s) => ({
    ...s,
    knobs: s.knobs.map((k) => ({ ...k, range: k.range ? ([...k.range] as [number, number]) : undefined, options: [...k.options] })),
  }));
}

export function loadUserPatches(): UserPatch[] {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as UserPatch[];
    return Array.isArray(parsed) ? parsed.filter((p) => p && p.id && Array.isArray(p.slots)) : [];
  } catch {
    /* localStorage indisponível (teste/privado) — começa vazio */
    return [];
  }
}

export function storeUserPatches(list: UserPatch[]): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(list));
  } catch {
    /* quota/privado: a sessão continua, só não persiste */
  }
}

/** Snapshot do board atual — a origem de um patch de usuário. `name` já vem
 *  resolvido pelo chamador (MSG.userPatchDefaultName quando ficou em branco). */
export function snapshotOf(board: BoardView, name: string, count: number): UserPatch {
  return {
    id: `u${Date.now().toString(36)}${count}`,
    name,
    fromLabel: board.ppLabel,
    savedAt: new Date().toISOString(),
    slots: cloneSlots(board.slots),
  };
}

/** Board do patch de usuário (mesmo shape do device; sem pp de fábrica).
 *  `index` é a posição na lista do dono (0-based) — o rótulo U## acompanha a
 *  ordem em que os patches aparecem, como o P## acompanha a biblioteca. */
export function boardOfUserPatch(p: UserPatch, index: number): BoardView {
  return {
    pp: -1,
    name: p.name,
    ppType: 0,
    ppTypeName: MSG.userBankType,
    slots: cloneSlots(p.slots),
    bank: "user",
    ppLabel: `U${String(index + 1).padStart(2, "0")}`,
  };
}