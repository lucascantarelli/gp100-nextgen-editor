/**
 * usePushLog — consome `device://push` (pushes não solicitados do device,
 * hex cru reemitido pelo actor) e mantém um LOG limitado ("pushes
 * visíveis em log da UI"). Fora do Tauri o listener local permite que o
 * app dev/teste exiba pushes injetados.
 *
 * Parsing/dedupe vivem no `ipc/push` (puro, testável — issue #20): push
 * inválido NÃO entra no log e repetição consecutiva do mesmo hex vira
 * contador na MESMA linha (o boot repete a resposta de tabela dezenas de
 * vezes; 61 linhas idênticas não são diagnóstico).
 */
import { useEffect, useState } from "react";
import { onDevicePush } from "../ipc/device";
import { pushLogReducer } from "../ipc/push";
import type { PushLogEntry } from "../ipc/types";

export function usePushLog() {
  const [log, setLog] = useState<PushLogEntry[]>([]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    onDevicePush((hex) => {
      setLog((prev) => pushLogReducer(prev, hex, Date.now()));
    }).then((un) => {
      if (disposed) un();
      else unlisten = un;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const clear = () => setLog([]);

  return { log, clear };
}
