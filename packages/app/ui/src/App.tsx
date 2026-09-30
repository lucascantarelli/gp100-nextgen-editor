/**
 * App: ConnectionBar
 * com boot + barra de progresso (eventos `device://progress`), painel de
 * info do device (mock) e PushLog (pushes visíveis em log da UI).
 * Padrões da skill ui-ux-practices: estados de tela canônicos (ScreenState),
 * erro sempre com retry, foco visível, feedback <100ms, tokens Fibonacci,
 * i18n-ready e a11y (dl/dt/dd, role="status", aria-valuenow na barra).
 *
 * Linguagem visual "pedalboard ao vivo no palco" (docs/UI_DESIGN.md §2):
 * LED pulsando quando conectado; ações primárias em âmbar Valeton.
 */
import type { CSSProperties } from "react";
import { useDevice } from "./hooks/useDevice";
import { useBoot } from "./hooks/useBoot";
import { usePushLog } from "./hooks/usePushLog";
import { ConnectionBar } from "./components/ConnectionBar";
import { PushLog } from "./components/PushLog";

export default function App() {
  const { state, refresh } = useDevice();
  const boot = useBoot();
  const { log, clear } = usePushLog();

  const booting = boot.state.kind === "loading";

  return (
    <main style={styles.page}>
      <header style={styles.header}>
        <div style={styles.brandRow}>
          <span style={styles.logoDot} aria-hidden="true" />
          <h1 style={styles.title}>GP-100 NextGen Editor</h1>
        </div>
        <p style={styles.subtitle}>
          conexão + boot — backend <strong>mock</strong> (política de
          hardware: nenhum byte vai ao device sem gate)
        </p>
      </header>

      <ConnectionBar
        info={state.kind === "ready" ? state.data : null}
        bootState={booting ? "booting" : boot.state.kind === "ready" ? "done" : boot.state.kind === "error" ? "error" : "idle"}
        progress={boot.progress}
        stage={boot.stage}
        bootPp={boot.bootPp}
        onBoot={boot.startBoot}
      />

      {boot.state.kind === "error" && (
        <div role="alert" style={styles.error}>
          <strong>Erro no boot:</strong> {boot.state.message}
          <button type="button" onClick={boot.state.retry} style={styles.button}>
            Tentar de novo
          </button>
        </div>
      )}

      <section aria-label="Device" style={styles.panel}>
        {state.kind === "idle" && <p>Nada feito ainda.</p>}
        {state.kind === "loading" && (
          <p role="status" aria-live="polite">
            Conectando ao device (mock)…
          </p>
        )}
        {state.kind === "error" && (
          <div role="alert" style={styles.error}>
            <strong>Erro:</strong> {state.message}
            <button type="button" onClick={state.retry} style={styles.button}>
              Tentar de novo
            </button>
          </div>
        )}
        {state.kind === "ready" && (
          <dl style={styles.grid}>
            <dt style={styles.dt}>Backend</dt>
            <dd style={styles.dd}>{state.data.backend}</dd>

            <dt style={styles.dt}>Presets</dt>
            <dd style={styles.ddMono}>{state.data.presetCount}</dd>

            <dt style={styles.dt}>Nome</dt>
            <dd style={styles.dd}>{state.data.currentName}</dd>

            <dt style={styles.dt}>Tipo (ppType)</dt>
            <dd style={styles.ddMono}>{state.data.currentPpType}</dd>

            <dt style={styles.dt}>IRs com CRC</dt>
            <dd style={styles.ddMono}>
              {state.data.irSlotsWithCrc}/20
            </dd>

            <dt style={styles.dt}>Boot</dt>
            <dd style={styles.dd}>
              {boot.state.kind === "ready"
                ? `completo — ${boot.state.data.transactions} transações`
                : booting
                  ? "em curso…"
                  : "não executado (mock já responde sem boot)"}
            </dd>
          </dl>
        )}
        <button type="button" onClick={refresh} style={styles.button}>
          Atualizar
        </button>
      </section>

      <PushLog log={log} onClear={clear} />

      <footer style={styles.footer}>
        <span style={styles.footerHint}>
          Paleta palco (âmbar Valeton) · Fibonacci 4·8·12·20·32·52·84 ·
          contraste AA medido · foco 2px — docs/UI_DESIGN.md
        </span>
      </footer>
    </main>
  );
}

/* Estilos só via tokens (lint + review guardam; zero px cru de espaço). */
const styles: Record<string, CSSProperties> = {
  page: {
    minHeight: "100vh",
    padding: "var(--space-32) var(--space-52)",
    display: "flex",
    flexDirection: "column",
    gap: "var(--space-32)",
    maxWidth: 720,
    margin: "0 auto",
  },
  header: { display: "flex", flexDirection: "column", gap: "var(--space-8)" },
  brandRow: {
    display: "flex",
    alignItems: "center",
    gap: "var(--space-12)",
  },
  logoDot: {
    width: 20,
    height: 20,
    borderRadius: "50%",
    background:
      "radial-gradient(circle at 35% 35%, var(--accent), color-mix(in srgb, var(--accent) 45%, #000))",
    boxShadow: "0 0 var(--space-12) color-mix(in srgb, var(--accent) 35%, transparent)",
  },
  title: { fontSize: "var(--text-lg)", margin: 0 },
  subtitle: {
    margin: 0,
    color: "var(--text-muted)",
    fontSize: "var(--text-sm)",
  },
  panel: {
    background: "var(--bg-raised)",
    border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
    borderRadius: "var(--space-12)",
    padding: "var(--space-20)",
    display: "flex",
    flexDirection: "column",
    gap: "var(--space-12)",
    boxShadow: "0 var(--space-4) var(--space-20) rgba(0,0,0,0.35)",
  },
  grid: {
    display: "grid",
    gridTemplateColumns: "auto 1fr",
    gap: "var(--space-8) var(--space-20)",
    margin: 0,
  },
  dt: { color: "var(--text-muted)" },
  dd: { margin: 0 },
  ddMono: {
    margin: 0,
    fontFamily: "var(--font-mono)",
    fontSize: "var(--text-sm)",
  },
  button: {
    alignSelf: "flex-start",
    minHeight: 32,
    padding: "var(--space-8) var(--space-20)",
    borderRadius: "var(--space-4)",
    border: "1px solid var(--accent)",
    background: "color-mix(in srgb, var(--accent) 12%, transparent)",
    color: "var(--accent)",
    cursor: "pointer",
    transition:
      "background var(--motion-fast) var(--ease-out), transform var(--motion-fast) var(--ease-out)",
  },
  error: {
    display: "flex",
    flexDirection: "column",
    gap: "var(--space-8)",
    color: "var(--error)",
  },
  footer: { marginTop: "auto" },
  footerHint: { color: "var(--text-muted)", fontSize: "var(--text-xs)" },
};
