/**
 * ConnectionBar — barra superior "pedalboard ao vivo" (docs/UI_DESIGN.md §6): backend/status à esquerda, pp corrente no centro e o
 * botão de boot à direita. Durante o boot, a barra de progresso ocupa a
 * linha inferior (âmbito âmbar Valeton; nunca trava a UI — eventos).
 */
import type { BootProgress, DeviceInfo } from "../ipc/types";
import { BOOT_STAGE_LABEL } from "../hooks/useBoot";

interface Props {
  info: DeviceInfo | null;
  bootState: "idle" | "booting" | "done" | "error";
  progress: number | null;
  stage: BootProgress["stage"] | null;
  bootPp: number | null;
  onBoot: () => void;
}

export function ConnectionBar({
  info,
  bootState,
  progress,
  stage,
  bootPp,
  onBoot,
}: Props) {
  const connected = info !== null;
  const booting = bootState === "booting";
  return (
    <section aria-label="Conexão" style={styles.bar}>
      <div style={styles.row}>
        <span style={styles.status} role="status">
          <span
            className={connected ? "live-dot" : "idle-dot"}
            aria-hidden="true"
          />
          {connected ? "Device conectado" : "Device desconectado"}
          <span style={styles.backend}>
            {" "}
            · mock (política de hardware: nenhum byte ao device na Fase M)
          </span>
        </span>

        <span style={styles.pp}>
          pp{" "}
          <strong style={styles.ppMono}>
            0x{(bootPp ?? info?.currentPp ?? 0).toString(16).padStart(4, "0")}
          </strong>
          {bootPp !== null && booting ? " (scan)" : ""}
        </span>

        <button
          type="button"
          onClick={onBoot}
          disabled={booting}
          aria-busy={booting}
          style={{
            ...styles.button,
            ...(booting ? styles.buttonBusy : {}),
          }}
        >
          {booting ? "Booting…" : "Boot"}
        </button>
      </div>

      {booting && progress !== null && (
        <div
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={progress}
          aria-label="Progresso do boot"
          style={styles.track}
        >
          <div style={{ ...styles.fill, width: `${progress}%` }} />
          <span style={styles.trackLabel}>
            {stage ? BOOT_STAGE_LABEL[stage] : "Iniciando"} · {progress}%
          </span>
        </div>
      )}
    </section>
  );
}

/* Estilos só via tokens (lint + review guardam; zero px cru de espaço). */
const styles: Record<string, React.CSSProperties> = {
  bar: {
    background: "var(--bg-raised)",
    border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
    borderRadius: "var(--space-12)",
    padding: "var(--space-12) var(--space-20)",
    display: "flex",
    flexDirection: "column",
    gap: "var(--space-8)",
  },
  row: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    gap: "var(--space-12)",
    flexWrap: "wrap",
  },
  status: {
    display: "inline-flex",
    alignItems: "center",
    gap: "var(--space-8)",
    fontSize: "var(--text-sm)",
    color: "var(--ok)",
  },
  backend: { color: "var(--text-muted)" },
  pp: { fontSize: "var(--text-sm)", color: "var(--text-muted)" },
  ppMono: {
    fontFamily: "var(--font-mono)",
    color: "var(--text)",
  },
  button: {
    minHeight: 32,
    padding: "var(--space-8) var(--space-20)",
    borderRadius: "var(--space-4)",
    border: "1px solid var(--accent)",
    background: "color-mix(in srgb, var(--accent) 12%, transparent)",
    color: "var(--accent)",
    cursor: "pointer",
    fontWeight: 600,
    transition:
      "background var(--motion-fast) var(--ease-out), transform var(--motion-fast) var(--ease-out)",
  },
  buttonBusy: {
    opacity: 0.7,
    cursor: "progress",
  },
  track: {
    position: "relative",
    height: 20,
    borderRadius: "var(--space-4)",
    background: "color-mix(in srgb, var(--text-muted) 18%, transparent)",
    overflow: "hidden",
  },
  fill: {
    height: "100%",
    background:
      "linear-gradient(90deg, color-mix(in srgb, var(--accent) 65%, transparent), var(--accent))",
    transition: "width var(--motion-fast) var(--ease-out)",
  },
  trackLabel: {
    position: "absolute",
    inset: 0,
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    fontSize: "var(--text-xs)",
    color: "var(--text)",
    textShadow: "0 1px 2px rgba(0,0,0,0.6)",
  },
};
