/**
 * VuPanel — painel de LED/equalizador REATIVO (V-7), morando no CABEÇALHO
 * DO PALCO (ao lado da trava ⇄ mover) com a identidade visual do
 * pedalboard: caixa escura de hardware, mono âmbar e fileira de LEDs
 * verde → âmbar → vermelho, como um VU de rack analógico. Dança quando há
 * som (drum tocando ou loop em PLAY/REC) e fica em repouso fora disso.
 *
 * Os dois modos têm animações DIFERENTES de verdade:
 *  - "led": escada de LED — 12 colunas × 6 segmentos que ACENDEM de baixo
 *    para cima conforme o nível; cor por faixa do segmento (verde → âmbar
 *    → vermelho), como o medidor de hardware;
 *  - "eq": equalizador gráfico — 12 barras contínuas com envelope de
 *    espectro (graves altas e lentas → agudos curtas e rápidas).
 *
 * Limites honestos da prévia local: o palco NÃO tem áudio real (sem
 * WebAudio/Analyser no mock) — a animação é um visualizador de CONTEXTO
 * (estado do drum/looper), com envelope determinístico (sem Math.random,
 * para o frame ser reprodutível em captura de tela). Quando o canal de
 * áudio do device existir, o mesmo componente recebe espectro real via prop.
 *
 * Performance: requestAnimationFrame escrevendo DIRETO no estilo dos
 * elementos — nenhum re-render do React por frame; o loop pausa com a aba
 * oculta e não existe com prefers-reduced-motion (LEDs estáticos apagados).
 */
import { useEffect, useRef } from "react";
import type { CSSProperties } from "react";
import { MSG } from "../i18n/messages";

export type VuMode = "led" | "eq";

interface Props {
  mode: VuMode;
  onModeChange: (m: VuMode) => void;
  /** há fonte de som ativa? (drum tocando ou loop em PLAY/REC) */
  active: boolean;
}

/* paleta do PALCO (mesma do pedalboard artístico — âmbar/madeira) */
const box: CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: 6,
  minHeight: 32,
  padding: "3px 8px",
  background: "#0a0805",
  border: "1px solid #3d3220",
  borderRadius: 6,
  boxShadow: "inset 0 0 10px #000",
};
const modeBtn = (on: boolean): CSSProperties => ({
  background: "#141009",
  border: `1px solid ${on ? "#8a6a24" : "#3d3220"}`,
  color: on ? "#ffb85c" : "#d8c9a8",
  fontWeight: on ? 700 : 400,
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-xs)",
  padding: "3px 7px",
  cursor: "pointer",
  borderRadius: 4,
  minHeight: 32, /* piso de alvo da casa (R6 a11y) */
});
const window_: CSSProperties = {
  display: "inline-flex",
  alignItems: "flex-end",
  gap: 2,
  height: 20,
  padding: "1px 3px",
  borderRadius: 4,
  background: "#050403",
  border: "1px solid #3d3220",
  overflow: "hidden",
};

const BARS = 12;
const SEGS = 6;
/* níveis de pico por coluna (frações) — DETERMINÍSTICOS: as baselines
   visuais comparam screenshots e a animação é congelada nelas. */
const PEAKS = [0.42, 0.68, 0.35, 0.9, 0.52, 0.74, 0.3, 0.85, 0.48, 0.66, 0.38, 0.58];

/** envelope determinístico (0..1) por coluna/tempo — compartilhado pelos
    dois modos (sem Math.random p/ manter o frame reprodutível em captura) */
function levelAt(t: number, i: number): number {
  const peak = PEAKS[i] ?? 0.5;
  const slow = Math.sin(t * 0.11 + i * 1.7) * 0.12;
  const fast = Math.sin(t * 0.31 + i * 0.9) * 0.06;
  return Math.max(0.12, Math.min(1, peak + slow + fast));
}

export function VuPanel({ mode, onModeChange, active }: Props) {
  /* LED: segmentos [coluna][segmento]; EQ: barra contínua por coluna */
  const segRefs = useRef<Array<Array<HTMLSpanElement | null>>>([]);
  const barRefs = useRef<Array<HTMLSpanElement | null>>([]);

  useEffect(() => {
    if (!active) return;
    // matchMedia não existe em jsdom/teste (e o efeito é só decorativo)
    if (typeof window.matchMedia !== "function") return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    let raf = 0;
    let t = 0;
    const tick = () => {
      t += 1;
      if (mode === "eq") {
        for (let i = 0; i < BARS; i += 1) {
          const el = barRefs.current[i];
          if (!el) continue;
          // espectro: graves (i baixo) altos e lentos; agudos curtos e rápidos
          const tilt = 1 - (i / BARS) * 0.45;
          const h = Math.max(0.1, levelAt(t, i) * tilt);
          el.style.height = `${Math.round(h * 100)}%`;
        }
      } else {
        for (let i = 0; i < BARS; i += 1) {
          const lit = Math.round(levelAt(t, i) * SEGS);
          const col = segRefs.current[i] ?? [];
          for (let j = 0; j < SEGS; j += 1) {
            const seg = col[j];
            if (!seg) continue;
            seg.dataset.lit = j < lit ? "1" : "0";
          }
        }
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    const onVis = () => {
      if (document.hidden) {
        cancelAnimationFrame(raf);
        raf = 0;
      } else if (!raf) {
        raf = requestAnimationFrame(tick);
      }
    };
    document.addEventListener("visibilitychange", onVis);
    return () => {
      if (raf) cancelAnimationFrame(raf);
      document.removeEventListener("visibilitychange", onVis);
    };
  }, [mode, active]);

  return (
    <div style={box} role="group" aria-label={MSG.vuAria}>
      <button
        style={modeBtn(mode === "led")}
        onClick={() => onModeChange("led")}
        aria-pressed={mode === "led"}
        aria-label={MSG.vuModeLed}
        title={MSG.vuModeLed}
      >
        ▁▃▅
      </button>
      <button
        style={modeBtn(mode === "eq")}
        onClick={() => onModeChange("eq")}
        aria-pressed={mode === "eq"}
        aria-label={MSG.vuModeEq}
        title={MSG.vuModeEq}
      >
        ▚▞▚
      </button>
      {mode === "eq" ? (
        <span data-vu="eq" style={window_} aria-hidden="true">
          {Array.from({ length: BARS }, (_, i) => (
            <span
              key={i}
              ref={(el) => {
                barRefs.current[i] = el;
              }}
              className="vu-bar"
              data-level={i < 7 ? "low" : i < 10 ? "mid" : "high"}
              style={{ width: 4, height: active ? "20%" : "12%", borderRadius: 1 }}
            />
          ))}
        </span>
      ) : (
        <span data-vu="led" style={{ ...window_, alignItems: "flex-end" }} aria-hidden="true">
          {Array.from({ length: BARS }, (_, i) => (
            <span
              key={i}
              style={{ display: "flex", flexDirection: "column-reverse", gap: 1 }}
            >
              {Array.from({ length: SEGS }, (_, j) => (
                <span
                  key={j}
                  ref={(el) => {
                    (segRefs.current[i] ??= [])[j] = el;
                  }}
                  className="vu-seg"
                  data-lit="0"
                  data-band={j < 4 ? "low" : j < 5 ? "mid" : "high"}
                  style={{ width: 4, height: 2, borderRadius: 1 }}
                />
              ))}
            </span>
          ))}
        </span>
      )}
    </div>
  );
}
