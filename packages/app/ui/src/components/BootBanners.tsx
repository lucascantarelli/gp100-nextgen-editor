/**
 * BootBanners — as faixas transitórias coladas no topo da página (#20/#150):
 *
 *   1. barra de progresso do boot (só enquanto boota);
 *   2. falha do boot (device não respondeu);
 *   3. **app sem aparelho (#150):** o motivo que o BACKEND declarou
 *      (`detail`) + a ação `device_conectar` — o mesmo contrato
 *      {mensagem + ação} do #20. Nada de lista de fábrica disfarçada de
 *      aparelho;
 *   4. erro de operação do palco com retry (o `ErrorBanner`).
 *
 * Saiu do `App.tsx` na #150 porque a casca voltou a passar do teto de 300
 * (docs/ARCHITECTURE.md §4): as faixas são apresentação pura — o estado
 * (boot, info, err) continua sendo de quem as aciona.
 */
import type { BootProgress } from "../ipc/types";
import { deviceConectar } from "../ipc/device";
import { MSG } from "../i18n/messages";
import { BOOT_STAGE_LABEL } from "../hooks/useBoot";
import { BootProgressBar } from "./BootProgressBar";
import { ErrorBanner } from "./ErrorBanner";

type Props = {
  /** `null` fora do boot — a barra some (não ocupa layout permanente). */
  progress: number | null;
  /** Estágio do boot; só tem valor com `progress` presente. */
  stage: BootProgress["stage"] | null;
  /** Falha do boot (device não respondeu). */
  bootFailed: boolean;
  /** `backend: "none"` → o motivo declarado pelo backend; `null` = conectado. */
  offDetail: string | null;
  /** Erro de operação do palco com ação de retry (issue #20). */
  err: { message: string; retry: () => void } | null;
  /** Reconecta e re-roda o boot manual (a falha fica no `deviceInfo`). */
  onReconnected: () => void;
};

export function BootBanners({
  progress,
  stage,
  bootFailed,
  offDetail,
  err,
  onReconnected,
}: Props) {
  return (
    <>
      {progress !== null && (
        <BootProgressBar
          progress={progress}
          stageLabel={stage ? BOOT_STAGE_LABEL[stage] : MSG.bootStarting}
        />
      )}
      {bootFailed && (
        <div
          role="alert"
          style={{
            background: "color-mix(in srgb, var(--error) 12%, transparent)",
            border: "1px solid color-mix(in srgb, var(--error) 45%, transparent)",
            color: "var(--error)",
            borderRadius: "var(--space-8)",
            padding: "var(--space-8) var(--space-12)",
            fontSize: "var(--text-sm)",
          }}
        >
          {MSG.connBootError}
        </div>
      )}
      {/* **(#150) app sem aparelho:** o aviso é o motivo que o BACKEND
          declarou (`detail`), e a ação é o device_conectar — o mesmo
          contrato {mensagem + ação} do #20. Nada de lista de fábrica
          disfarçada de aparelho. */}
      {offDetail != null && (
        <div
          role="alert"
          style={{
            display: "flex",
            flexWrap: "wrap",
            alignItems: "center",
            gap: "var(--space-8)",
            background: "color-mix(in srgb, var(--error) 12%, transparent)",
            border: "1px solid color-mix(in srgb, var(--error) 45%, transparent)",
            color: "var(--error)",
            borderRadius: "var(--space-8)",
            padding: "var(--space-8) var(--space-12)",
            fontSize: "var(--text-sm)",
          }}
        >
          <span style={{ flex: "1 1 auto", minWidth: 0 }}>
            {`${MSG.connOffTitle}: ${offDetail || MSG.connBootError}`}
          </span>
          <button
            onClick={() => {
              void deviceConectar()
                .then(onReconnected)
                .catch(() => undefined); // a falha fica no info/deviceInfo
            }}
            aria-label={MSG.connOffRetryAria}
            title={MSG.connOffRetryAria}
            style={{
              border: "1px solid color-mix(in srgb, var(--error) 45%, transparent)",
              background: "transparent",
              color: "inherit",
              borderRadius: "var(--space-8)",
              padding: "6px 10px",
              cursor: "pointer",
              fontFamily: "var(--font-mono)",
              fontSize: "var(--text-sm)",
              minHeight: 32,
            }}
          >
            {MSG.connOffConnect}
          </button>
        </div>
      )}
      {err != null && <ErrorBanner message={err.message} onRetry={err.retry} />}
    </>
  );
}
