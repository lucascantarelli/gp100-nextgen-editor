/**
 * LooperPanel — o loop da GP-100 como MÁQUINA DE FITA REEL-TO-REEL
 * (referências: Studer A807 / Nagra / Tandberg — rolos girando, VU meters
 * com luz âmbar, contador mecânico e transporte com símbolos clássicos).
 *
 * Dados REAIS extraídos:
 *  - firmware V2.1 (menu LOOPER): Rec VOL · Play VOL · Pre/Post · P-VOL;
 *  - specs oficiais Valeton: 90 s em PRE / 45 s em POST (stereo com efeitos),
 *    24-bit · 44.1 kHz · SNR 110 dB; Rec level 0–99.
 *
 * PRÉVIA LOCAL: o looper é estado do device (a escrita via USB ainda não
 * está implementada); volumes/rota ficam em localStorage, transporte é de sessão.
 */
import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { MSG } from "../i18n/messages";

export interface LooperSettings {
  recVol: number;
  playVol: number;
  pVol: number;
  pre: boolean; // true = PRE (90 s) · false = POST (45 s, com efeitos)
}

const KEY = "gp100.looper.v1";
export const LOOP_SECONDS_PRE = 90;
export const LOOP_SECONDS_POST = 45;

export function loadLooper(): LooperSettings {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) return { ...{ recVol: 80, playVol: 80, pVol: 80, pre: true }, ...(JSON.parse(raw) as Partial<LooperSettings>) };
  } catch {
    /* teste */
  }
  return { recVol: 80, playVol: 80, pVol: 80, pre: true };
}

type Mode = "idle" | "rec" | "play" | "dub" | "stop";

const card: CSSProperties = {
  background: "linear-gradient(180deg, #241d15 0%, #17120c 100%)",
  border: "1px solid #3a3128",
  borderRadius: 16,
  padding: "var(--space-20)",
  display: "grid",
  // deck | VU/contador/transporte | rack — empilha em telas estreitas
  gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))",
  gap: "var(--space-20)",
  alignItems: "center",
  boxShadow: "inset 0 2px 0 rgba(255,255,255,.05), 0 18px 34px rgba(0,0,0,.5)",
};
const plate: CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-xs)",
  letterSpacing: 2,
  color: "#d8c9a8",
  border: "1px solid #5a4a2f",
  borderRadius: 6,
  padding: "2px 10px",
  background: "#141009",
  justifySelf: "start",
};
const lbl: CSSProperties = {
  fontSize: "var(--text-xs)",
  color: "#b9a888",
  fontFamily: "var(--font-mono)",
  textTransform: "uppercase",
  letterSpacing: 0.8,
};
const counterBox: CSSProperties = {
  background: "#0a0805",
  border: "1px solid #3d3220",
  borderRadius: 6,
  padding: "6px 12px",
  fontFamily: "var(--font-mono)",
  color: "#ffb85c",
  fontSize: 22,
  fontWeight: 700,
  letterSpacing: 3,
  boxShadow: "inset 0 0 10px #000",
};
const btn = (active: boolean, danger = false): CSSProperties => ({
  minWidth: 52,
  minHeight: 40,
  borderRadius: 8,
  border: "1px solid #0a0c0f",
  background: active
    ? danger
      ? "linear-gradient(180deg,#ff6b6b,#c23636)"
      : "linear-gradient(180deg,#3a4150,#232830)"
    : "linear-gradient(180deg,#2c313a,#1b1f26)",
  color: active ? "#fff" : "#c9ced6",
  cursor: "pointer",
  fontFamily: "var(--font-mono)",
  fontSize: 16,
  fontWeight: 700,
  boxShadow: "0 3px 0 #0a0c0f",
});
const sliderRow: CSSProperties = { display: "grid", gridTemplateColumns: "74px 1fr 34px", gap: "var(--space-8)", alignItems: "center", minHeight: 32 };

function Reel({ label, spinning, fast }: { label: string; spinning: boolean; fast: boolean }) {
  return (
    <figure style={{ margin: 0, display: "grid", justifyItems: "center", gap: 6 }}>
      <svg
        width="132"
        height="132"
        viewBox="0 0 132 132"
        role="img"
        aria-label={MSG.reelAria(label, spinning)}
        className={spinning ? (fast ? "reel-spin reel-fast" : "reel-spin") : undefined}
        style={{ filter: "drop-shadow(0 8px 12px rgba(0,0,0,.6))" }}
      >
        <circle cx="66" cy="66" r="62" fill="#101216" stroke="#3d3220" strokeWidth="3" />
        <circle cx="66" cy="66" r="52" fill="none" stroke="#232830" strokeWidth="1.4" />
        {/* fita acumulada */}
        <circle cx="66" cy="66" r="40" fill="#1d2128" stroke="#2c313a" strokeWidth="1" />
        <circle cx="66" cy="66" r="30" fill="#141821" />
        {/* cubo + 3 raios */}
        <circle cx="66" cy="66" r="13" fill="#0c0e12" stroke="#454c56" strokeWidth="2" />
        {[0, 120, 240].map((a) => {
          const rad = (a * Math.PI) / 180;
          return (
            <line
              key={a}
              x1={66 + Math.sin(rad) * 12}
              y1={66 - Math.cos(rad) * 12}
              x2={66 + Math.sin(rad) * 34}
              y2={66 - Math.cos(rad) * 34}
              stroke="#454c56"
              strokeWidth="4"
              strokeLinecap="round"
            />
          );
        })}
        <circle cx="66" cy="66" r="5" fill="#0a0c0f" />
      </svg>
      <figcaption style={lbl}>{label}</figcaption>
    </figure>
  );
}

function VuMeter({ channel, active }: { channel: "L" | "R"; active: boolean }) {
  return (
    <svg width="104" height="64" viewBox="0 0 104 64" role="img" aria-label={MSG.looperVuAria(channel, active)}>
      <defs>
        <linearGradient id={`vu-face-${channel}`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#ffe9b8" />
          <stop offset="100%" stopColor="#e8c46a" />
        </linearGradient>
      </defs>
      <rect x="2" y="2" width="100" height="60" rx="6" fill={`url(#vu-face-${channel})`} stroke="#5a4a2f" strokeWidth="2" />
      {/* escala */}
      {[-45, -25, -5, 15, 35, 45].map((a) => {
        const rad = ((a - 90) * Math.PI) / 180;
        const r1 = 46;
        const r2 = 40;
        return (
          <line
            key={a}
            x1={52 + Math.sin(rad) * r1}
            y1={58 - Math.cos(rad) * r1}
            x2={52 + Math.sin(rad) * r2}
            y2={58 - Math.cos(rad) * r2}
            stroke="#8a6a24"
            strokeWidth="1.4"
          />
        );
      })}
      <text x="84" y="26" fontSize="7" fill="#8a6a24" fontFamily="var(--font-mono)">+3</text>
      <text x="18" y="26" fontSize="7" fill="#8a6a24" fontFamily="var(--font-mono)">-20</text>
      <g
        className={active ? "vu-needle" : undefined}
        style={{ transformOrigin: "52px 58px" }}
      >
        <line x1="52" y1="58" x2="52" y2="16" stroke="#1b1f26" strokeWidth="2.2" strokeLinecap="round" />
      </g>
      <circle cx="52" cy="58" r="4" fill="#1b1f26" />
      <text x="52" y="52" textAnchor="middle" fontSize="7" fill="#8a6a24" fontFamily="var(--font-mono)">{channel}</text>
    </svg>
  );
}

export function LooperPanel({
  settings,
  onChange,
  recRequest = 0,
  onPlayingChange,
}: {
  settings: LooperSettings;
  onChange: (s: LooperSettings) => void;
  /** pulso do atalho global R (useGlobalShortcuts no App): incrementa p/ disparar REC */
  recRequest?: number;
  /** VU da navbar: true quando o loop está tocando/gravando (há "som") */
  onPlayingChange?: (playing: boolean) => void;
}) {
  const [mode, setMode] = useState<Mode>("idle");

  // VU reativo da navbar: qualquer transporte que produza som anima as barras
  useEffect(() => {
    onPlayingChange?.(mode === "rec" || mode === "play" || mode === "dub");
  }, [mode, onPlayingChange]);
  const [hasTape, setHasTape] = useState(false);
  const [secs, setSecs] = useState(0);
  const [confirmClear, setConfirmClear] = useState(false);
  const timer = useRef<number | null>(null);

  const maxSecs = settings.pre ? LOOP_SECONDS_PRE : LOOP_SECONDS_POST;
  const running = mode === "rec" || mode === "play" || mode === "dub";
  const set = (patch: Partial<LooperSettings>) => onChange({ ...settings, ...patch });

  useEffect(() => {
    if (!running) {
      if (timer.current != null) window.clearInterval(timer.current);
      timer.current = null;
      return;
    }
    timer.current = window.setInterval(() => {
      setSecs((s) => {
        if (mode === "rec" && !hasTape) {
          if (s + 1 >= maxSecs) {
            setHasTape(true);
            setMode("play");
            return 0;
          }
          return s + 1;
        }
        return (s + 1) % maxSecs;
      });
    }, 1000);
    return () => {
      if (timer.current != null) window.clearInterval(timer.current);
      timer.current = null;
    };
  }, [running, mode, hasTape, maxSecs]);

  const onRec = () => {
    setConfirmClear(false);
    if (mode === "rec") {
      setHasTape(true);
      setMode("play");
    } else if (mode === "play" || mode === "dub") {
      setMode("dub");
    } else {
      setMode("rec");
    }
  };
  // Atalho global R (App): dispara o MESMO onRec do botão ● (rec→play→dub).
  // ref mantém o handler sempre atual sem re-registrar o efeito.
  const onRecRef = useRef(onRec);
  useEffect(() => {
    onRecRef.current = onRec;
  });
  useEffect(() => {
    if (recRequest > 0) onRecRef.current();
  }, [recRequest]);

  const onPlay = () => {
    if (!hasTape) return;
    setMode(mode === "play" ? "stop" : "play");
  };
  const onStop = () => setMode("stop");
  const onRew = () => setSecs(0);
  const onClear = () => {
    if (!confirmClear) {
      setConfirmClear(true);
      return;
    }
    setConfirmClear(false);
    setHasTape(false);
    setMode("idle");
    setSecs(0);
  };

  const modeLabel =
    mode === "rec" ? MSG.looperMode.rec : mode === "play" ? MSG.looperMode.play : mode === "dub" ? MSG.looperMode.dub : mode === "stop" ? MSG.looperMode.stop : hasTape ? MSG.looperMode.ready : MSG.looperMode.empty;
  const mm = String(Math.floor(secs / 60)).padStart(2, "0");
  const ss = String(secs % 60).padStart(2, "0");

  return (
    <section style={card} aria-label={MSG.looperAria}>
      {/* deck de fitas */}
      <div style={{ display: "grid", gap: "var(--space-12)", justifyItems: "center" }}>
        <span style={plate}>{MSG.looperPlate}</span>
        {/* estado da fita (único lugar — era duplicado no rack e na linha do transporte) */}
        <span style={lbl} role="status">
          {hasTape ? MSG.tapeState(maxSecs, settings.pre ? "PRE" : "POST") : MSG.tapeEmpty}
        </span>
        <div style={{ display: "flex", gap: "var(--space-12)", alignItems: "center" }}>
          <Reel label={MSG.reelSupply} spinning={running} fast={mode === "rec" || mode === "dub"} />
          {/* caminho da fita pelos cabeçotes */}
          <div aria-hidden="true" style={{ display: "grid", gap: 6, justifyItems: "center" }}>
            {["REC", "PB"].map((h) => (
              <div key={h} style={{ width: 54, height: 18, borderRadius: 4, background: "linear-gradient(180deg,#2c313a,#14171c)", border: "1px solid #0a0c0f", display: "grid", placeItems: "center", fontFamily: "var(--font-mono)", fontSize: 8, color: "#8b939e" }}>
                {h}
              </div>
            ))}
            <div style={{ width: 6, height: 34, borderRadius: 3, background: "#454c56" }} title={MSG.capstanTitle} />
          </div>
          <Reel label={MSG.reelTakeup} spinning={running} fast={mode === "rec" || mode === "dub"} />
        </div>
      </div>

      {/* VU + contador + transporte */}
      <div style={{ display: "grid", gap: "var(--space-12)", justifyItems: "center" }}>
        <div style={{ display: "flex", gap: "var(--space-12)" }}>
          <VuMeter channel="L" active={running} />
          <VuMeter channel="R" active={running} />
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-12)" }}>
          <span style={counterBox} role="timer" aria-label={MSG.looperTimerAria}>
            {mm}:{ss}
          </span>
          <span
            style={{ ...lbl, color: mode === "rec" || mode === "dub" ? "#ff6b6b" : mode === "play" ? "#3ecf8e" : "#b9a888" }}
            role="status"
          >
            ● {modeLabel}
          </span>
        </div>
        <div style={{ display: "flex", gap: "var(--space-8)" }} role="group" aria-label={MSG.looperTransportAria}>
          <button style={btn(false)} onClick={onRew} aria-label={MSG.looperRewAria}>
            ⏪
          </button>
          <button style={btn(mode === "stop" || mode === "idle")} onClick={onStop} aria-label={MSG.looperStopAria}>
            ■
          </button>
          <button style={btn(mode === "play")} onClick={onPlay} aria-label={MSG.looperPlayAria} aria-pressed={mode === "play"} disabled={!hasTape}>
            ▶
          </button>
          <button style={btn(mode === "rec" || mode === "dub", true)} onClick={onRec} aria-label={mode === "rec" ? MSG.looperRecStopAria : mode === "play" || mode === "dub" ? MSG.looperDubAria : MSG.looperRecAria} aria-pressed={mode === "rec" || mode === "dub"}>
            ●
          </button>
          <button style={btn(confirmClear, true)} onClick={onClear} aria-label={confirmClear ? MSG.looperClearConfirmAria : MSG.looperClearAria}>
            {confirmClear ? MSG.looperClearConfirmBtn : MSG.looperClearBtn}
          </button>
        </div>
      </div>

      {/* rack de parâmetros do firmware */}
      <div style={{ display: "grid", gap: "var(--space-12)", minWidth: 240 }}>
        <div style={{ ...sliderRow }}>
          <label style={lbl} htmlFor="loop-rec">{MSG.looperRecVol}</label>
          <input id="loop-rec" type="range" min={0} max={99} value={settings.recVol} onChange={(e) => set({ recVol: Number(e.target.value) })} style={{ width: "100%", accentColor: "var(--accent)", minHeight: 32 }} />
          <span style={{ ...lbl, textAlign: "right", color: "var(--accent)" }}>{settings.recVol}</span>
        </div>
        <div style={{ ...sliderRow }}>
          <label style={lbl} htmlFor="loop-play">{MSG.looperPlayVol}</label>
          <input id="loop-play" type="range" min={0} max={99} value={settings.playVol} onChange={(e) => set({ playVol: Number(e.target.value) })} style={{ width: "100%", accentColor: "var(--accent)", minHeight: 32 }} />
          <span style={{ ...lbl, textAlign: "right", color: "var(--accent)" }}>{settings.playVol}</span>
        </div>
        <div style={{ ...sliderRow }}>
          <label style={lbl} htmlFor="loop-pvol">{MSG.looperPVol}</label>
          <input id="loop-pvol" type="range" min={0} max={99} value={settings.pVol} onChange={(e) => set({ pVol: Number(e.target.value) })} style={{ width: "100%", accentColor: "var(--accent)", minHeight: 32 }} />
          <span style={{ ...lbl, textAlign: "right", color: "var(--accent)" }}>{settings.pVol}</span>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-8)" }}>
          <span style={lbl}>{MSG.looperRoute}</span>
          <button style={{ ...btn(settings.pre), fontSize: 12, minWidth: 64 }} onClick={() => set({ pre: true })} aria-pressed={settings.pre} aria-label={MSG.looperPreAria}>
            {MSG.looperPreBtn}
          </button>
          <button style={{ ...btn(!settings.pre), fontSize: 12, minWidth: 64 }} onClick={() => set({ pre: false })} aria-pressed={!settings.pre} aria-label={MSG.looperPostAria}>
            {MSG.looperPostBtn}
          </button>
        </div>
        <p style={{ ...lbl, margin: 0, lineHeight: 1.5 }}>
          {MSG.looperSpecs}
        </p>
      </div>
    </section>
  );
}
