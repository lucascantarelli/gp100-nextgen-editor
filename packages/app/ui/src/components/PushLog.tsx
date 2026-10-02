/**
 * PushLog — log dos pushes não solicitados do device: evento
 * `device://push` → hex cru (F0…F7). Lista em ordem de chegada, com botão
 * limpar; mono e truncado
 * (hover/focus revela a linha inteira — overflow-x).
 *
 * Repetições CONSECUTIVAS do mesmo hex (o boot repete a resposta de tabela)
 * ficam na MESMA linha com contador `×N` — parsing/dedupe no ipc/push.
 */
import type { PushLogEntry } from "../ipc/types";
import { MSG } from "../i18n/messages";

interface Props {
  log: PushLogEntry[];
  onClear: () => void;
}

export function PushLog({ log, onClear }: Props) {
  return (
    <section aria-label={MSG.pushTitle} style={styles.panel}>
      <div style={styles.header}>
        <h2 style={styles.title}>{MSG.pushTitle}</h2>
        <button type="button" onClick={onClear} style={styles.clear}>
          {MSG.pushClear}
        </button>
      </div>
      {log.length === 0 ? (
        <p style={styles.empty} role="status">
          {MSG.pushEmpty}
        </p>
      ) : (
        <ol style={styles.list} aria-label={MSG.pushListAria}>
          {log.map((e, i) => (
            <li key={`${e.at}-${i}`} style={styles.item}>
              <span style={styles.hex}>{e.hex}</span>
              {e.count > 1 && (
                <span style={styles.repeat} title={MSG.pushRepeats}>
                  {MSG.pushRepeatMark}{e.count}
                </span>
              )}
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}

const styles: Record<string, React.CSSProperties> = {
  panel: {
    background: "var(--bg-raised)",
    border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
    borderRadius: "var(--space-12)",
    padding: "var(--space-20)",
    display: "flex",
    flexDirection: "column",
    gap: "var(--space-12)",
  },
  header: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    gap: "var(--space-12)",
  },
  title: {
    margin: 0,
    fontSize: "var(--text-sm)",
    color: "var(--text-muted)",
    textTransform: "uppercase",
    letterSpacing: "0.08em",
  },
  clear: {
    minHeight: 32, // alvo clicável mínimo de acessibilidade (32×32)
    padding: "var(--space-8) var(--space-12)",
    borderRadius: "var(--space-4)",
    border: "1px solid color-mix(in srgb, var(--text-muted) 40%, transparent)",
    background: "transparent",
    color: "var(--text-muted)",
    cursor: "pointer",
  },
  empty: { margin: 0, color: "var(--text-muted)", fontSize: "var(--text-sm)" },
  list: {
    margin: 0,
    padding: 0,
    listStyle: "none",
    display: "flex",
    flexDirection: "column",
    gap: "var(--space-4)",
    maxHeight: 180,
    overflowY: "auto",
  },
  item: {
    fontFamily: "var(--font-mono)",
    fontSize: "var(--text-xs)",
    color: "var(--text-muted)",
    overflowX: "auto",
    whiteSpace: "nowrap",
    padding: "var(--space-4) 0",
    borderBottom: "1px solid color-mix(in srgb, var(--text-muted) 15%, transparent)",
  },
  hex: { letterSpacing: "0.02em" },
  repeat: {
    marginLeft: "var(--space-8)",
    color: "var(--accent-text)",
    fontWeight: 600,
  },
};
