/**
 * DrumPanel — gestão dos RITMOS da bateria (drawer inferior full-width,
 * opção A do alinhamento: mantém o palco do pedalboard 100% limpo). Os 87
 * estilos em 5 gêneros vêm do firmware V2.1 (drumData.ts, GERADO); os
 * compassos são os reais (2/4…9/8). Controle é PRÉVIA LOCAL (o drum é
 * estado do device; a escrita via USB ainda não está implementada) —
 * persistência localStorage.
 *
 * Esc: quem fecha é o atalho GLOBAL (useGlobalShortcuts no App), que conhece
 * a precedência Settings → Drum → pushes — este drawer é controlado puro.
 */
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

const drawer: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))",
  gap: "var(--space-12) var(--space-20)",
  padding: "var(--space-20) var(--space-32) var(--space-32)",
  alignItems: "start",
};
const head: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "var(--space-12)",
  padding: "var(--space-12) var(--space-32) 0",
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
const btn: CSSProperties = {
  background: "var(--accent)",
  border: "1px solid var(--accent-glow)",
  color: "var(--on-accent)",
  fontWeight: 700,
  borderRadius: 8,
  padding: "6px 14px",
  cursor: "pointer",
  fontFamily: "var(--font-mono)",
  minHeight: 32,
};
const btnOff: CSSProperties = { ...btn, background: "var(--bg)", color: "var(--text)", borderColor: "color-mix(in srgb, var(--text-muted) 30%, transparent)" };
const note: CSSProperties = { fontSize: "var(--text-xs)", color: "var(--text-muted)", gridColumn: "1 / -1", margin: 0 };

export function DrumPanel({ open, drum, onChange, onClose }: Props) {
  if (!open) return null;
  const genreEntry = DRUM_GENRES.find((g) => g.genre === drum.genre) ?? DRUM_GENRES[0];
  const set = (patch: Partial<DrumState>) => onChange({ ...drum, ...patch });

  return (
    <div className="drum-drawer" role="group" aria-label={MSG.drumPanelAria}>
      <div style={head}>
        <strong style={{ fontSize: "var(--text-sm)" }}>{MSG.drumTitle}</strong>
        <button style={drum.on ? btn : btnOff} onClick={() => set({ on: !drum.on })} aria-pressed={drum.on}>
          {drum.on ? MSG.drumStopLabel : MSG.drumPlayLabel}
        </button>
        <button style={{ ...btnOff, marginLeft: "auto" }} onClick={onClose} aria-label={MSG.drumCloseAria}>
          ✕
        </button>
      </div>

      <div style={drawer}>
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
  );
}
