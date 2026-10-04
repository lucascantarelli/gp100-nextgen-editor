/**
 * userPatches — patches de USUÁRIO (patches salvos pelo dono).
 *
 * A #26 muda o ONDE esse patch mora: sai do `localStorage` e vai para o banco
 * SQLite (crate `gp100-library`, ADR-9). O que NÃO muda é o formato do
 * retrato: a cadeia salva é o SNAPSHOT do board no momento do save (os 9
 * slots com nome/estado/knobs), e é ela que volta ao pedalboard quando o
 * patch é aberto.
 *
 * Este arquivo é a FRONTEIRA entre o retrato (`UserPatch`, o que o palco
 * desenha) e o registro do banco (`LibraryRecord`, o que o SQLite guarda).
 * As duas direções ficam aqui — no mesmo lugar de propósito: uma fronteira em
 * dois arquivos é o jeito garantido de elas divergirem, e foi o que a migração
 * do `localStorage` teve de repetir na mão.
 *
 * A escrita no DEVICE continua sem canal USB — o que muda é que o patch não
 * depende mais do navegador para sobreviver ao fechar do app.
 */
import type { BoardSlot, BoardView } from "./ipc/types";
import type { LibraryRecord } from "./ipc/library";
import { MSG } from "./i18n/messages";

export interface UserPatch {
  /** id estavel na biblioteca (não é índice: apagar não renumera o resto). */
  id: string;
  name: string;
  /** ISO do instante do save (a lista mostra a data). */
  savedAt: string;
  /** estilo numérico do preset de onde o patch saiu (o badge da lista). */
  ppType: number;
  /** rótulo do estilo ("Rock", "User"...). */
  ppTypeName: string;
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

/** Snapshot do board atual — a origem de um patch de usuario. `name` já vem
 *  resolvido pelo chamador (MSG.userPatchDefaultName quando ficou em branco). */
export function snapshotOf(board: BoardView, name: string, count: number): UserPatch {
  return {
    id: `u${Date.now().toString(36)}${count}`,
    name,
    savedAt: new Date().toISOString(),
    ppType: board.ppType,
    ppTypeName: board.ppTypeName,
    slots: cloneSlots(board.slots),
  };
}

/** Board do patch de usuario (mesmo shape do device; sem pp de fabrica).
 *  `index` é a posição na lista do dono (0-based) — o rótulo U## acompanha a
 *  ordem em que os patches aparecem, como o P## acompanha a biblioteca. */
export function boardOfUserPatch(p: UserPatch, index: number): BoardView {
  return {
    pp: -1,
    name: p.name,
    ppType: p.ppType,
    ppTypeName: MSG.userBankType,
    slots: cloneSlots(p.slots),
    bank: "user",
    ppLabel: `U${String(index + 1).padStart(2, "0")}`,
  };
}

/**
 * Retrato -> registro do banco. É a gravação: o `payload` é o snapshot
 * SERIALIZADO, e não um objeto, porque quem vai ler isso depois é o SQLite e
 * não o React.
 */
export function registroDePatch(p: UserPatch): LibraryRecord {
  return {
    id: p.id,
    bank: "user",
    pp: null,
    name: p.name,
    ppType: p.ppType,
    ppTypeName: p.ppTypeName,
    savedAt: p.savedAt,
    hasPayload: true,
    payload: JSON.stringify(p.slots),
  };
}

/**
 * Registro do banco -> retrato. `null` quando o `payload` não volta a ser
 * slots: registro gravado por outra versao do app, ou arquivo editado a mao.
 *
 * O motivo de devolver `null` em vez de lançar e o mesmo do `lerLegadoCru`: o
 * dado que o dono tem é precioso, então quem chama decide o que fazer (a UI
 * avisa e mantém o preset aberto) em vez de um `throw` virar tela branca no
 * meio do palco.
 */
export function patchDeRegistro(rec: LibraryRecord): UserPatch | null {
  if (!rec.payload) return null;
  let slots: unknown;
  try {
    slots = JSON.parse(rec.payload);
  } catch {
    return null;
  }
  if (!Array.isArray(slots)) return null;
  return {
    id: rec.id,
    name: rec.name,
    savedAt: rec.savedAt,
    ppType: rec.ppType,
    ppTypeName: rec.ppTypeName,
    slots: slots as BoardSlot[],
  };
}
