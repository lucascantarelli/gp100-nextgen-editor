/**
 * useBoot — dispara o `device_boot` e consome `device://progress`
 * (barra de progresso; boot nunca trava a UI).
 *
 * Estado canônico (UI_DESIGN §5): o boot vive num ScreenState próprio; o
 * progresso é um NUMBER % (throttle a ~30 fps por rAF) — 2297 beats por
 * boot não podem renderizar 2297 vezes (ui-ux-practices: feedback <100 ms
 * e zero jank). O pp corrente do beat alimenta o display da ConnectionBar.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { deviceBoot, onBootProgress } from "../ipc/device";
import type { BootProgress, BootReport, ScreenState } from "../ipc/types";

/** Etapa em PT-BR para exibição (literais estáveis do backend). */
export const BOOT_STAGE_LABEL: Record<BootProgress["stage"], string> = {
  tables: "Tabelas de IR",
  scan: "Scan de presets",
  probe: "Sonda banco 02",
  setlist: "Setlist",
  names: "Nomes",
  keepalive: "Keepalive",
};

export function useBoot() {
  const [state, setState] = useState<ScreenState<BootReport>>({ kind: "idle" });
  const [progress, setProgress] = useState<number | null>(null);
  const [stage, setStage] = useState<BootProgress["stage"] | null>(null);
  const [bootPp, setBootPp] = useState<number | null>(null);
  // refs do throttle: último frame agendado + beat pendente
  const raf = useRef<number | null>(null);
  const pending = useRef<BootProgress | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    onBootProgress((p) => {
      pending.current = p;
      if (raf.current === null) {
        raf.current = requestAnimationFrame(() => {
          raf.current = null;
          const last = pending.current;
          if (!last) return;
          setProgress(Math.round((last.done / last.total) * 100));
          setStage(last.stage);
          setBootPp(last.currentPp);
        });
      }
    }).then((un) => {
      if (disposed) un();
      else unlisten = un;
    });
    return () => {
      disposed = true;
      unlisten?.();
      if (raf.current !== null) cancelAnimationFrame(raf.current);
    };
  }, []);

  const startBoot = useCallback(() => {
    setState({ kind: "loading" });
    setProgress(0);
    deviceBoot()
      .then((report) => {
        setProgress(100);
        setState({ kind: "ready", data: report });
      })
      .catch((e: unknown) => {
        setProgress(null);
        setState({
          kind: "error",
          message: e instanceof Error ? e.message : String(e),
          retry: startBoot,
        });
      });
  }, []);

  const reset = useCallback(() => {
    setState({ kind: "idle" });
    setProgress(null);
    setStage(null);
    setBootPp(null);
  }, []);

  return { state, progress, stage, bootPp, startBoot, reset };
}
