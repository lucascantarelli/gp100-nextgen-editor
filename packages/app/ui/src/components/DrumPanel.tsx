/**
 * DrumPanel — MODAL de gestão dos RITMOS (87 estilos em 5 gêneros do
 * firmware V2.1, drumData.ts, GERADO); os compassos são os reais (2/4…9/8).
 * Controle é PRÉVIA LOCAL (o drum é estado do device; a escrita via USB
 * ainda não está implementada) — persistência localStorage.
 *
 * Padrão idêntico ao SettingsModal (overlay + superfície FLUTUANTE do
 * sistema): aqui só MODIFICAÇÃO de estilo — play/stop e BPM ficam direto
 * na navbar (TopBar), sem exigir este modal aberto.
 *
 * Esc: quem fecha é o atalho GLOBAL (useGlobalShortcuts no App, que conhece
 * a precedência Settings → Drum → pushes) — este modal é controlado puro,
 * sem listener próprio de Esc (com os dois, uma tecla fechava dois painéis).
 */
import { useEffect, useRef } from "react";
import type { CSSProperties } from "react";
import { DRUM_BEATS, DRUM_GENRES } from "../artifacts/drumData";
import { MSG } from "../i18n/messages";

export interface DrumState {
  on: boolean;
  genre: string;
  style: string;
  bpm: number;
  beat: string;
  volume: number;
  speed: number;
}

interface Props {
  open: boolean;
  drum: DrumState;
  onChange: (d: DrumState) => void;
  onClose: () => void;
}

const overlay: CSSProperties = {
  position: "fixed",
  inset: 0,
  background: "rgba(0,0,0,.55)",
  display: "grid",
  placeItems: "center",
  zIndex: 50,
};
/* modal = superfície FLUTUANTE do sistema (gp-surface--floating): a luz e a
   sombra vêm dos tokens; aqui só geometria/estrutura */
const dialog: CSSProperties = {
  width: "min(720px, 94vw)",
  maxHeight: "86vh",
  overflow: "hidden",
  display: "grid",
  gridTemplateRows: "auto 1fr",
};
const head: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "var(--space-12)",
  padding: "var(--space-12) var(--space-20) 0",
};
const body: CSSProperties = {
  overflowY: "auto",
  padding: "var(--space-12) var(--space-20) var(--space-20)",
  display: "grid",
  gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))",
  gap: "var(--space-12) var(--space-20)",
  alignContent: "start",
};
const line: CSSProperties = { display: "grid", gridTemplateColumns: "92px 1fr", alignItems: "center", gap: "var(--space-8)", minHeight: 32 };
const lbl: CSSProperties = { fontSize: "var(--text-xs)", color: "var(--text-muted)", fontFamily: "var(--font-mono)", textTransform: "uppercase", letterSpacing: 0.6 };
const input: CSSProperties = {
  background: "var(--bg)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  color: "var(--text)",
  borderRadius: 8,
  padding: "6px 8px",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  minHeight: 32,
  width: "100%",
};
const close: CSSProperties = {
  background: "var(--bg)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  color: "var(--text)",
  borderRadius: 8,
  padding: "6px 10px",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  minHeight: 32,
  minWidth: 32,
  cursor: "pointer",
};
const note: CSSProperties = { fontSize: "var(--text-xs)", color: "var(--text-muted)", gridColumn: "1 / -1", margin: 0 };

export function DrumPanel({ open, drum, onChange, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  // WCAG: o foco entra no diálogo ao abrir (idem PedalModal); o retorno do
  // foco ao chip é do clique do usuário (ele já está onde clicou)
  useEffect(() => {
    if (open) ref.current?.focus();
  }, [open]);

  if (!open) return null;
  const genreEntry = DRUM_GENRES.find((g) => g.genre === drum.genre) ?? DRUM_GENRES[0];
  const set = (patch: Partial<DrumState>) => onChange({ ...drum, ...patch });

  return (
    <div style={overlay} onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-label={MSG.drumPanelAria}
        tabIndex={-1}
        className="gp-surface gp-surface--floating"
        style={dialog}
      >
        <div style={head}>
          <strong style={{ fontSize: "var(--text-md)" }}>{MSG.drumTitle}</strong>
          <button style={{ ...close, marginLeft: "auto" }} onClick={onClose} aria-label={MSG.drumCloseAria}>
            ✕
          </button>
        </div>

        <div style={body}>
          <div style={line}>
            <label style={lbl} htmlFor="drum-genre">{MSG.drumGenreLabel}</label>
            <select
              id="drum-genre"
              style={input}
              value={genreEntry.genre}
              onChange={(e) => {
                const g = DRUM_GENRES.find((x) => x.genre === e.target.value)!;
                set({ genre: g.genre, style: g.styles[0] });
              }}
            >
              {DRUM_GENRES.map((g) => (
                <option key={g.genre} value={g.genre}>
                  {g.genre} ({g.styles.length})
                </option>
              ))}
            </select>
          </div>

          <div style={line}>
            <label style={lbl} htmlFor="drum-style">{MSG.drumStyleLabel}</label>
            <select id="drum-style" style={input} value={drum.style} onChange={(e) => set({ style: e.target.value })}>
              {genreEntry.styles.map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
            </select>
          </div>

          <div style={line}>
            <label style={lbl} htmlFor="drum-bpm">{MSG.drumBpmLabel}</label>
            <input
              id="drum-bpm"
              type="number"
              className="gp-num"
              min={40}
              max={240}
              style={input}
              value={drum.bpm}
              onChange={(e) => set({ bpm: Math.min(240, Math.max(40, Number(e.target.value) || 40)) })}
            />
          </div>

          <div style={line}>
            <label style={lbl} htmlFor="drum-beat">{MSG.drumBeatLabel}</label>
            <select id="drum-beat" style={input} value={drum.beat} onChange={(e) => set({ beat: e.target.value })}>
              {DRUM_BEATS.map((b) => (
                <option key={b} value={b}>
                  {b}
                </option>
              ))}
            </select>
          </div>

          <div style={line}>
            <label style={lbl} htmlFor="drum-vol">{MSG.drumVolLabel}</label>
            <input id="drum-vol" type="range" min={0} max={99} value={drum.volume} style={{ width: "100%", accentColor: "var(--accent)", minHeight: 32 }} onChange={(e) => set({ volume: Number(e.target.value) })} />
          </div>

          <div style={line}>
            <label style={lbl} htmlFor="drum-speed">{MSG.drumSpeedLabel}</label>
            <input id="drum-speed" type="range" min={0} max={99} value={drum.speed} style={{ width: "100%", accentColor: "var(--accent)", minHeight: 32 }} onChange={(e) => set({ speed: Number(e.target.value) })} />
          </div>

          <p style={note}>{MSG.drumNote}</p>
        </div>
      </div>
    </div>
  );
}
