/**
 * useBoot — dispara o `device_boot` e consome `device://progress`
 * (barra de progresso; boot nunca trava a UI).
 *
 * Estado canônico: o boot vive num ScreenState próprio; o
 * progresso é um NUMBER % (throttle a ~30 fps por rAF) — 2297 beats por
 * boot não podem renderizar 2297 vezes (ui-ux-practices: feedback <100 ms
 * e zero jank).
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { deviceBoot, onBootProgress } from "../ipc/device";
import type { BootProgress, BootReport, ScreenState } from "../ipc/types";
import { MSG } from "../i18n/messages";

/** Etapa em PT-BR para exibição (textos do catálogo central de mensagens). */
export const BOOT_STAGE_LABEL: Record<BootProgress["stage"], string> = {
  tables: MSG.bootStages.tables,
  scan: MSG.bootStages.scan,
  probe: MSG.bootStages.probe,
  setlist: MSG.bootStages.setlist,
  names: MSG.bootStages.names,
  keepalive: MSG.bootStages.keepalive,
};

export function useBoot() {
  const [state, setState] = useState<ScreenState<BootReport>>({ kind: "idle" });
  const [progress, setProgress] = useState<number | null>(null);
  const [stage, setStage] = useState<BootProgress["stage"] | null>(null);
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

  const [origin, setOrigin] = useState<"auto" | "manual">("auto");

  const startBoot = useCallback((src: "auto" | "manual" = "manual") => {
    setOrigin(src);
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

  // boot AUTOMÁTICO no mount: o app detecta o device sozinho ao abrir
 // (o estado "off" da navbar já mostra o aguardo); o botão Boot da
  // navbar continua disponível para re-escanear explicitamente. A ref
  // evita rodar duas vezes sob StrictMode (mesma instância, 2 efeitos).
  const autoRan = useRef(false);
  useEffect(() => {
    if (autoRan.current) return;
    autoRan.current = true;
    startBoot("auto");
  }, [startBoot]);

  const reset = useCallback(() => {
    setState({ kind: "idle" });
    setProgress(null);
    setStage(null);
  }, []);

  return { state, progress, stage, origin, startBoot, reset };
}
