/**
 * ipc/push — parsing e dedupe dos pushes não solicitados do device
 * (issue #20, edge cases de IPC nível 2).
 *
 * O evento `device://push` chega como HEX CRU do device. No fio real ele
 * pode vir truncado, com tamanho ímpar ou fora do envelope SysEx — e o boot
 * REPETE a mesma resposta de tabela dezenas de vezes (backlog D7: 61 pushes
 * idênticos no inventário default). O log da UI é a superfície de
 * diagnóstico: exibir lixo ou 61 linhas iguais não ajuda ninguém.
 *
 * Contrato (puro, testável sem hook/DOM):
 *   - `parsePushHex` — aceita SÓ `F0…F7` com tamanho par e só hex; devolve
 *     `null` para qualquer outra coisa (o log ignora, nunca renderiza lixo);
 *   - `pushLogReducer` — dedupa repetição CONSECUTIVA na mesma linha
 *     (`count`) e aplica o cap de entradas (as mais novas ficam).
 */

import type { PushLogEntry } from "./types";

/** Cap do log: as entradas mais NOVAS permanecem (contrato do usePushLog). */
export const PUSH_LOG_MAX = 100;

/**
 * Valida/normaliza o hex cru de um push.
 *
 * @param raw Payload do evento (não confiável: pode não ser string).
 * @returns Hex em MAIÚSCULAS quando é um SysEx bem formado; `null` caso
 * contrário (não-SysEx, tamanho ímpar/curto demais, caractere não-hex).
 */
export function parsePushHex(raw: unknown): string | null {
  if (typeof raw !== "string") return null;
  const hex = raw.trim().toUpperCase();
  // F0 + F7 já são 2 bytes; menos que isso não é mensagem.
  if (hex.length < 4 || hex.length % 2 !== 0) return null;
  if (!/^[0-9A-F]+$/.test(hex)) return null;
  if (!hex.startsWith("F0") || !hex.endsWith("F7")) return null;
  return hex;
}

/**
 * Reducer do log de pushes: valida → dedupa consecutivos → aplica o cap.
 *
 * @param prev Log atual (ordem de chegada).
 * @param raw Payload cru do evento `device://push`.
 * @param now Instante da chegada (o chamador injeta — hora é dependência).
 * @returns Novo log; `prev` inalterado quando o payload é inválido.
 */
export function pushLogReducer(
  prev: PushLogEntry[],
  raw: unknown,
  now: number,
): PushLogEntry[] {
  const hex = parsePushHex(raw);
  if (hex === null) return prev;
  const last = prev[prev.length - 1];
  if (last !== undefined && last.hex === hex) {
    // Repetição CONSECUTIVA: mesma linha, contador incrementado (o `at`
    // acompanha a última chegada — é o que o usuário quer ver).
    return [...prev.slice(0, -1), { hex, at: now, count: last.count + 1 }];
  }
  const next = [...prev, { hex, at: now, count: 1 }];
  return next.length > PUSH_LOG_MAX ? next.slice(-PUSH_LOG_MAX) : next;
}
