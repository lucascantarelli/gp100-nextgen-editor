/**
 * useDevice — estado de sessão do device como ScreenState canônico
 * (docs/UI_DESIGN.md §5): idle → loading → ready | error(retry).
 * Nada de booleanos soltos; erro sempre com ação de recuperação.
 */
import { useCallback, useEffect, useState } from "react";
import { deviceInfo } from "../ipc/device";
import type { DeviceInfo, ScreenState } from "../ipc/types";

export function useDevice() {
  const [state, setState] = useState<ScreenState<DeviceInfo>>({ kind: "idle" });

  const refresh = useCallback(() => {
    setState((prev) => ({ kind: "loading", from: prev }));
    deviceInfo()
      .then((info) => setState({ kind: "ready", data: info }))
      .catch((e: unknown) =>
        setState({
          kind: "error",
          message: e instanceof Error ? e.message : String(e),
          retry: refresh,
        }),
      );
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { state, refresh };
}
