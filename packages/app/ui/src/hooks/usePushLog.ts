/**
 * usePushLog — consome `device://push` (backlog D7 do device, hex cru
 * reemitido pelo actor) e mantém um LOG limitado ("pushes
 * visíveis em log da UI"). Fora do Tauri o listener local permite que o
 * app dev/teste exiba pushes injetados.
 */
import { useEffect, useState } from "react";
import { onDevicePush } from "../ipc/device";
import type { PushLogEntry } from "../ipc/types";

const MAX_ENTRIES = 100;

export function usePushLog() {
  const [log, setLog] = useState<PushLogEntry[]>([]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    onDevicePush((hex) => {
      setLog((prev) => {
        const next = [...prev, { hex, at: Date.now() }];
        return next.length > MAX_ENTRIES ? next.slice(-MAX_ENTRIES) : next;
      });
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
