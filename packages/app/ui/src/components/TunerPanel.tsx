/**
 * TunerPanel — o afinador do palco (V-7), SEMPRE VISÍVEL no cabeçalho,
 * ocupando o lugar do antigo visualizador VU.
 *
 * Display grande com o MESMO estilo de hardware do LED (caixa escura
 * `#0a0d10`, iluminação superior, sombra interna, reflexo espectral).
 *
 * Comportamento espelha o tuner do GP-100 (manual V1.8, p.7 "Using The
 * TUNER"): a nota aparece no CENTRO da escala, flat à esquerda (♭) e sharp
 * à direita (♯), e a cor acompanha a aproximação — verde (no ponto),
 * âmbar (perto), vermelho (fora).
 *
 * MONITOR: o afinador pode ficar ligado durante o uso da pedaleira para
 * monitorar a afinação em tempo real — o display nunca colapsa.
 *
 * LED PRÓPRIO (sem conflito): ponto de estado DEDICADO.
 * O display LED do palco (pp/nome/preset) NUNCA é tomado pelo afinador.
 *
 * Controles (como no device): MONITOR on/off (botão visual com LED),
 * MODO Bypass/Thru/Mute (TunerMode 0..2) e REF PITCH 435–445 Hz.
 *
 * Demo: alimenta o MOTOR REAL com senoide sintética (±30 cents em A2).
 * Quando o canal de áudio existir, a leitura chega via prop `reading`.
 */
import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { MSG } from "../i18n/messages";
import { ANALYSIS_WINDOW, REF_PITCH_DEFAULT, REF_PITCH_MAX, REF_PITCH_MIN, TunerEngine } from "../tuner/pitch";
import type { TunerReading } from "../tuner/pitch";

export type TunerMode = "bypass" | "thru" | "mute";

export interface TunerSettings {
  on: boolean;
  mode: TunerMode;
  refPitch: number;
}

export const TUNER_KEY = "gp100.tuner.v1";

export function loadTuner(): TunerSettings {
  try {
    const raw = localStorage.getItem(TUNER_KEY);
    if (raw) {
      const p = JSON.parse(raw) as Partial<TunerSettings>;
      return {
        on: p.on ?? false,
        mode: p.mode ?? "mute",
        refPitch: Math.min(REF_PITCH_MAX, Math.max(REF_PITCH_MIN, p.refPitch ?? REF_PITCH_DEFAULT)),
      };
    }
  } catch {
    /* localStorage indisponível (teste) — defaults */
  }
  return { on: false, mode: "mute", refPitch: REF_PITCH_DEFAULT };
}

/* MESMA identidade da caixa LED do palco (#0a0d10 + inset preto) */
const box: CSSProperties = {
  display: "grid",
  gap: 6,
  flex: "1 1 300px",
  minWidth: 280,
  maxWidth: 430,
  padding: "8px 12px",
  background: "#0a0d10",
  border: "1px solid #1d242c",
  borderRadius: 6,
  boxShadow: "inset 0 0 12px #000",
  fontFamily: "var(--font-mono)",
};

/* botão de controle com iluminação premium (superior mais claro) */
const ctrl = (on: boolean): CSSProperties => ({
  background: `linear-gradient(180deg, ${on ? "#2a1f0a" : "#1a1510"} 0%, ${on ? "#140f05" : "#0f0c08"} 100%)`,
  border: `1px solid ${on ? "#8a6a24" : "#3d3220"}`,
  color: on ? "#ffb85c" : "#d8c9a8",
  fontWeight: on ? 700 : 400,
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-xs)",
  padding: "4px 10px",
  cursor: "pointer",
  borderRadius: 6,
  minHeight: 32,
  whiteSpace: "nowrap",
  boxShadow: on
    ? "0 2px 0 #0a0703, inset 0 1px 0 rgba(255,255,255,0.08), inset 0 -1px 0 rgba(0,0,0,0.4)"
    : "0 2px 0 #080604, inset 0 1px 0 rgba(255,255,255,0.03)",
  transition: "all var(--motion-fast) var(--ease-out)",
});

const meter: CSSProperties = {
  position: "relative",
  flex: 1,
  height: 20,
  borderRadius: 3,
  background: "#050403",
  border: "1px solid #3d3220",
};

/* escala central: gradientes fixos por faixa (verde no centro, âmbar em
   volta, vermelho nas pontas) — identidade VU de rack do palco */
const meterScale: CSSProperties = {
  position: "absolute",
  inset: 3,
  borderRadius: 2,
  background:
    "linear-gradient(90deg, color-mix(in srgb, var(--error) 55%, #050403) 0%, color-mix(in srgb, var(--warn) 55%, #050403) 26%, color-mix(in srgb, var(--ok) 65%, #050403) 50%, color-mix(in srgb, var(--warn) 55%, #050403) 74%, color-mix(in srgb, var(--error) 55%, #050403) 100%)",
  opacity: 0.55,
};

const needle = (offset: number, color: string | null): CSSProperties => ({
  position: "absolute",
  top: 1,
  bottom: 1,
  width: 3,
  borderRadius: 2,
  /* desvio −50..+50 cents → −50%..+50% do medidor (centro = no ponto) */
  left: `calc(${50 + Math.max(-50, Math.min(50, offset))}% - 1.5px)`,
  background: color ?? "#8b97a6",
  boxShadow: color ? `0 0 7px ${color}` : undefined,
  transition: "left 90ms linear, background 120ms linear",
});

/* LED próprio do afinador (ponto de estado dedicado): cinza apagado,
   âmbar ouvindo, verde/vermelho pela banda da leitura — com brilho */
const ledDot = (color: string | null): CSSProperties => ({
  display: "inline-block",
  width: 10,
  height: 10,
  borderRadius: "50%",
  background: color ?? "#2c3440",
  boxShadow: color
    ? `0 0 10px ${color}, inset 0 0 4px rgba(255,255,255,0.3)`
    : "inset 0 0 3px rgba(0,0,0,0.5)",
  transition: "all 120ms linear",
});

/* marcadores das pontas da escala: esquerda flat, direita sharp (manual) */
const edgeMark: CSSProperties = {
  fontSize: 13,
  color: "#8b97a6",
  userSelect: "none",
};

const noteStyle = (color: string | null): CSSProperties => ({
  position: "absolute",
  left: "50%",
  top: "50%",
  transform: "translate(-50%, -50%)", /* centraliza exato no meio do medidor */
  minWidth: 48,
  textAlign: "center",
  fontSize: 18,
  fontWeight: 700,
  letterSpacing: 0.5,
  color: color ?? "#d8dce4",
  whiteSpace: "nowrap",
  textShadow: "0 1px 4px rgba(0,0,0,0.9)",
  background: "rgba(10,13,16,0.75)",
  borderRadius: 5,
  padding: "2px 6px",
  backdropFilter: "blur(3px)",
  border: color ? "1px solid color-mix(in srgb, currentColor 30%, transparent)" : "1px solid rgba(255,255,255,0.08)",
});

const MODES: TunerMode[] = ["bypass", "thru", "mute"];
const DEMO_MS = 8000; /* demo desliga sozinha — nunca vira enfeite permanente */
const DEMO_RATE = 22050; /* taxa da amostra sintética (barata e suficiente) */
const DEMO_BASE_HZ = 110; /* A2 — corda de referência do varredor */
const DEMO_CENTS = 30; /* varre ±30 cents: percorre flat → centro → sharp */

export function TunerPanel({
  settings,
  onChange,
  reading: deviceReading = null,
}: {
  settings: TunerSettings;
  onChange: (s: TunerSettings) => void;
  /** leitura real do device/canal de áudio (quando existir) */
  reading?: TunerReading | null;
}) {
  const [demo, setDemo] = useState(false);
  const [demoReading, setDemoReading] = useState<TunerReading | null>(null);
  const engine = useRef<TunerEngine | null>(null);

  /* demo: gera a senoide por frame e empurra no MOTOR REAL (tuner/pitch) */
  useEffect(() => {
    if (!demo) {
      setDemoReading(null);
      return;
    }
    /* ambiente pode estar destruído quando o efeito reexecuta (teste) */
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    engine.current ??= new TunerEngine(settings.refPitch);
    engine.current.setRefPitch(settings.refPitch);
    let raf = 0;
    let phase = 0;
    const t0 = performance.now();
    const tick = () => {
      const el = performance.now() - t0;
      const cents = Math.sin((el / 2000) * Math.PI * 2) * DEMO_CENTS;
      const freq = DEMO_BASE_HZ * Math.pow(2, cents / 1200);
      /* janela do MOTOR (2048): menos que isso o detectPitch devolve null */
      const frame = new Float32Array(ANALYSIS_WINDOW);
      for (let i = 0; i < frame.length; i += 1) {
        phase += (2 * Math.PI * freq) / DEMO_RATE;
        frame[i] = 0.4 * Math.sin(phase) + 0.15 * Math.sin(2 * phase); /* +harmônico */
      }
      setDemoReading(engine.current?.push(frame, DEMO_RATE) ?? null);
      if (el < DEMO_MS) raf = requestAnimationFrame(tick);
      else setDemo(false);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [settings.refPitch, demo]);

  const set = (patch: Partial<TunerSettings>) => onChange({ ...settings, ...patch });
  const active = settings.on;
  const reading = deviceReading ?? demoReading;
  const colorToken =
    reading === null ? null : reading.band === "ok" ? "var(--ok)" : reading.band === "warn" ? "var(--warn)" : "var(--error)";
  /* LED PRÓPRIO: cinza = monitor desligado; âmbar = ligado/ouvindo; banda
     com leitura. O LED do palco (pp/nome) segue intocado — zero conflito. */
  const tunerLed = !active ? null : colorToken ?? "var(--warn)";

  return (
    <div style={box} role="group" aria-label={MSG.tunerAria}>
      {/* DISPLAY: LED + escala ♭→♯ + nota centralizada (sempre visível) */}
      <div style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: "var(--space-8)", alignItems: "center", width: "100%" }}>
        {/* LED próprio (esquerda, fixo) */}
        <span data-tuner-led style={ledDot(tunerLed)} aria-hidden="true" />

        {/* escala com nota centralizada sobre a agulha */}
        <div style={{ position: "relative", display: "flex", alignItems: "center", gap: 0 }}>
          <span style={edgeMark} aria-hidden="true">{MSG.tunerFlatMark}</span>
          <span style={meter}>
            <span style={meterScale} />
            <span style={{ position: "absolute", left: "50%", top: 0, bottom: 0, width: 1, background: "#4a3d29" }} />
            <span data-tuner-needle style={needle(reading?.cents ?? 0, colorToken)} />
          </span>
          <span style={edgeMark} aria-hidden="true">{MSG.tunerSharpMark}</span>
          {/* nota centralizada sobre a agulha */}
          <span data-tuner-note style={noteStyle(colorToken)}>
            {reading ? `${reading.note}${reading.octave}` : MSG.tunerIdleNote}
          </span>
        </div>
      </div>

      {/* controles: monitor (visual) + modo + ref pitch + demo */}
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-8)", width: "100%", minWidth: 0, flexWrap: "wrap" }}>
        {/* botão monitorar: visual on/off com LED integrado */}
        <button
          style={{
            ...ctrl(active),
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
          }}
          onClick={() => set({ on: !active })}
          aria-pressed={active}
          aria-label={MSG.tunerPowerAria}
          title={MSG.tunerPowerTitle}
        >
          <span
            style={{
              width: 8,
              height: 8,
              borderRadius: "50%",
              background: active ? "var(--ok)" : "var(--error)",
              boxShadow: active ? "0 0 8px var(--ok)" : "0 0 8px var(--error)",
              transition: "all 150ms linear",
            }}
            aria-hidden="true"
          />
          {active ? MSG.tunerPowerOn : MSG.tunerPowerOff}
        </button>

        {/* quando ativo: modo + ref pitch */}
        {active && (
          <>
            <button
              style={ctrl(settings.mode === "bypass")}
              onClick={() => set({ mode: MODES[(MODES.indexOf(settings.mode) + 1) % MODES.length] })}
              aria-label={MSG.tunerModeAria}
              title={MSG.tunerModeTitle}
            >
              {MSG.tunerModeLabel(settings.mode)}
            </button>
            <label style={{ display: "inline-flex", alignItems: "center", gap: 4, minHeight: 32, minWidth: 0 }}>
              <span style={{ fontSize: "var(--text-xs)", color: "#d8c9a8" }}>{MSG.tunerRefLabel}</span>
              <input
                type="range"
                min={REF_PITCH_MIN}
                max={REF_PITCH_MAX}
                step={1}
                value={settings.refPitch}
                onChange={(e) => set({ refPitch: Number(e.target.value) })}
                aria-label={MSG.tunerRefAria}
                title={MSG.tunerRefTitle}
                style={{ width: 68, accentColor: "#ffb85c", minHeight: 32 }}
              />
              <span data-tuner-ref style={{ fontSize: "var(--text-xs)", color: "#ffb85c", minWidth: 40 }}>
                {MSG.tunerRefValue(settings.refPitch)}
              </span>
            </label>
          </>
        )}

        {/* botão demo (sempre visível) */}
        <button
          style={ctrl(demo)}
          onClick={() => setDemo((v) => !v)}
          aria-label={MSG.tunerDemoAria}
          title={MSG.tunerDemoTitle}
        >
          {MSG.tunerDemoLabel}
        </button>
      </div>
    </div>
  );
}
