/**
 * BootGate — a casca SÓ existe depois do boot validado (#161): enquanto o
 * aparelho não responde com inventário e nomes coerentes, a página mostra
 * SOMENTE a navbar + este gate. Duas caras:
 *
 *   - `idle|loading` → painel de progresso: mensagem + faixa + estágio
 *     (`role="status"`: o leitor de tela avisa sem roubar o foco);
 *   - `error` → `role="alert"` com o MOTIVO — o `detail` do backend quando
 *     não há aparelho (o contrato {mensagem + ação} do #150: a ação é o
 *     `device_conectar`) ou o texto amigável vindo do useBoot (validação
 *     do relatório ou falha de transporte, #161) — e as ações:
 *     **Refazer o boot** (sempre) e **Reconectar** (só sem aparelho).
 *     O detalhe técnico NUNCA chega aqui: fica no console no useBoot
 *     (contrato do e2e: nada de "debug:" no alert).
 *
 * Acessibilidade (#161): o alert recebe FOCO ao virar erro — sem isso o
 * teclado ficaria perdido na navbar, sem caminho até a ação de
 * recuperação.
 *
 * Saiu do App.tsx porque a casca vive no teto de 300 linhas (ARCHITECTURE
 * §4): aqui é apresentação pura — o estado (boot, info) é de quem aciona.
 */
import { useEffect, useRef } from "react";
import type { DeviceInfo } from "../ipc/types";
import { deviceConectar } from "../ipc/device";
import { MSG } from "../i18n/messages";
import { BOOT_STAGE_LABEL } from "../hooks/useBoot";
import type { useBoot } from "../hooks/useBoot";
import { BootProgressBar } from "./BootProgressBar";

type Boot = ReturnType<typeof useBoot>;

/** Caixa de erro: MESMA face das faixas de erro do app (var(--error)). */
const erroBox: React.CSSProperties = {
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
};

const botao: React.CSSProperties = {
  border: "1px solid color-mix(in srgb, var(--error) 45%, transparent)",
  background: "transparent",
  color: "inherit",
  borderRadius: "var(--space-8)",
  padding: "6px 10px",
  cursor: "pointer",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  minHeight: 32,
};

interface BootGateProps {
  /** Estado canônico do boot (useBoot) — o gate é apresentação dele. */
  boot: Boot;
  /** `deviceInfo` corrente — o motivo do backend quando `backend: "none"`. */
  info: DeviceInfo | null;
}

export function BootGate({ boot, info }: BootGateProps) {
  const alertRef = useRef<HTMLDivElement>(null);
  const estado = boot.state;
  // A11y (#161): virou erro → o alert é o foco (ação de recuperação à mão).
  useEffect(() => {
    if (estado.kind === "error") alertRef.current?.focus();
  }, [estado.kind]);

  if (estado.kind === "error") {
    // **Sem aparelho (#150):** o motivo é o que o BACKEND declarou
    // (`detail`) e a ação é o `device_conectar` — nada de lista de fábrica
    // disfarçada de aparelho. Com aparelho: o texto amigável do useBoot.
    const semAparelho = info?.backend === "none";
    const motivo = semAparelho
      ? `${MSG.connOffTitle}: ${info?.detail || MSG.connBootError}`
      : estado.message;
    return (
      <div role="alert" ref={alertRef} tabIndex={-1} style={erroBox}>
        <span style={{ flex: "1 1 auto", minWidth: 0 }}>{motivo}</span>
        <button onClick={estado.retry} style={botao}>
          {MSG.bootRetry}
        </button>
        {semAparelho && (
          <button
            onClick={() => {
              void deviceConectar()
                .then(() => boot.startBoot("manual"))
                .catch(() => undefined); // a falha fica no info/deviceInfo
            }}
            aria-label={MSG.connOffRetryAria}
            title={MSG.connOffRetryAria}
            style={botao}
          >
            {MSG.connOffConnect}
          </button>
        )}
      </div>
    );
  }

  // idle|loading — a faixa é transitória (não ocupa layout permanente).
  return (
    <div
      role="status"
      aria-live="polite"
      style={{ display: "grid", gap: "var(--space-8)" }}
    >
      <span style={{ fontSize: "var(--text-sm)", color: "var(--text-muted)" }}>
        {MSG.bootPanelMsg}
      </span>
      <BootProgressBar
        progress={boot.progress ?? 0}
        stageLabel={boot.stage ? BOOT_STAGE_LABEL[boot.stage] : MSG.bootStarting}
      />
    </div>
  );
}
