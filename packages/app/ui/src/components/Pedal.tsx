/**
 * Pedal SVG — modelo POR VARIANTE (efeito real do dicionário). Layout
 * GRANDE: knob de 64px e VALOR EDITÁVEL (textbox) sob cada knob —
 * Enter/tab-out aplica o valor ao device, Esc restaura, ↑/↓ incrementa,
 * duplo-clique no knob reseta ao default.
 *
 * O board mostra UM efeito por vez (o 1º da cadeia — hoje, o COMP);
 * foco total em precisão de ajuste antes de voltar aos 9 slots.
 *
 * LEDs padronizados: VERDE = ligado, VERMELHO = desligado.
 * Modo engenheiro: tooltip do knob mostra o endereço de memória do comando SET.
 */
import { useState } from "react";
import type { BoardSlot } from "../ipc/types";
import { Knob } from "./Knob";
import { modelFor } from "../artifacts/fxModels";
import { MSG } from "../i18n/messages";

/** Paleta por família (fallback de cor; variantes podem sobrescrever). */
const FAMILY_STYLE: Record<
  BoardSlot["family"],
  { body: string; face: string; accent: string; kind: string }
> = {
  PRE: { body: "#5b6470", face: "#39404b", accent: "#8fd3ff", kind: "Pre" },
  DST: { body: "#c2452d", face: "#8f2d1c", accent: "#ffd23f", kind: "Drive" },
  AMP: { body: "#d8a437", face: "#a3742a", accent: "#fff1c9", kind: "Amp" },
  NR: { body: "#4f7942", face: "#37552e", accent: "#b7f7a1", kind: "Gate" },
  CAB: { body: "#5a4632", face: "#3e3021", accent: "#ffd9a0", kind: "Cabinet" },
  EQ: { body: "#3f7f8c", face: "#2a5a64", accent: "#9ef3ff", kind: "EQ" },
  MOD: { body: "#7b5ea7", face: "#574080", accent: "#e0c3ff", kind: "Mod" },
  DLY: { body: "#2f6f8f", face: "#1e4d64", accent: "#7fd4ff", kind: "Delay" },
  RVB: { body: "#a24a6d", face: "#7a3350", accent: "#ffb3d1", kind: "Reverb" },
};

const KNOB = 64; // knob grande
const PITCH = 116; // passo horizontal/vertical da grade (sem sobreposição)
const VALUE_W = 92; // caixa do valor editável

/** Dimensões determinísticas do pedal (board usa p/ layout). */
export function pedalDims(slot: BoardSlot): { w: number; h: number } {
  const m = modelFor(slot);
  const rows = Math.max(1, Math.ceil(Math.max(slot.knobs.length, 1) / m.cols));
  return { w: Math.max(m.w, 420), h: 150 + rows * PITCH + 110 };
}

interface Props {
  slot: BoardSlot;
  engineer?: boolean;
  onKnobChange: (slot: BoardSlot, pos: number, value: string) => void;
  onKnobReset: (slot: BoardSlot, pos: number) => void;
  onToggle: (slot: BoardSlot) => void;
}

const LED_ON = "#39d353";
const LED_OFF = "#ff4b4b";

/** Caixa de valor EDITÁVEL (textbox) — commit em Enter/blur, Esc restaura. */
function ValueBox({
  text,
  frac,
  accent,
  onCommit,
}: {
  text: string;
  frac: boolean;
  accent: string;
  onCommit: (raw: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState<string | null>(null);
  if (!editing) {
    return (
      <input
        readOnly
        value={text}
        aria-label={MSG.pedalValueAria}
        onFocus={() => {
          setDraft(null);
          setEditing(true);
        }}
        onClick={() => {
          setDraft(null);
          setEditing(true);
        }}
        style={{
          width: VALUE_W,
          height: 30,
          borderRadius: 8,
          border: "1px solid #3a424e",
          background: "#10141a",
          color: accent,
          fontFamily: "ui-monospace, monospace",
          fontSize: 16,
          fontWeight: 700,
          textAlign: "center",
          cursor: "text",
        }}
      />
    );
  }
  return (
    <input
      autoFocus
      value={draft ?? text}
      inputMode="decimal"
      aria-label={MSG.pedalValueEditAria}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={() => {
        const raw = (draft ?? text).trim();
        if (raw !== "" && raw !== text) {
          const v = Number(raw);
          if (!Number.isNaN(v)) onCommit(frac ? v.toFixed(1) : String(Math.round(v)));
        }
        setDraft(null);
        setEditing(false);
      }}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          const raw = (draft ?? text).trim();
          if (raw !== "" && raw !== text) {
            const v = Number(raw);
            if (!Number.isNaN(v)) onCommit(frac ? v.toFixed(1) : String(Math.round(v)));
          }
          setDraft(null);
          setEditing(false);
        }
        if (e.key === "Escape") {
          setDraft(null);
          setEditing(false);
        }
      }}
      style={{
        width: VALUE_W,
        height: 30,
        borderRadius: 8,
        border: `1.5px solid ${accent}`,
        background: "#10141a",
        color: accent,
        fontFamily: "ui-monospace, monospace",
        fontSize: 16,
        fontWeight: 700,
        textAlign: "center",
      }}
    />
  );
}

export function Pedal({ slot, engineer = false, onKnobChange, onKnobReset, onToggle }: Props) {
  const fam = FAMILY_STYLE[slot.family];
  const model = modelFor(slot);
  const body = model.body ?? fam.body;
  const face = model.face ?? fam.face;
  const W = pedalDims(slot).w;
  const H = pedalDims(slot).h;
  const on = slot.state;
  const led = on ? LED_ON : LED_OFF;
  const codeHex = `0x${(slot.code >>> 0).toString(16).padStart(8, "0")}`;
  const addr = `10 ${(slot.slot + 1).toString(16).padStart(2, "0")} 00 02`;

  const knobs = slot.knobs; // TODOS os controles do algoritmo
  const cols = Math.min(model.cols, 4);
  const rows = Math.max(1, Math.ceil(Math.max(knobs.length, 1) / cols));
  const gridW = cols * PITCH;
  const gridLeft = W / 2 - gridW / 2 + PITCH / 2;
  const knobTop = 200;

  const fracOf = (r?: [number, number]) =>
    r != null && (!Number.isInteger(r[0]) || !Number.isInteger(r[1]));

  const footY = H - 66;

  return (
    <svg
      width={W}
      height={H}
      viewBox={`0 0 ${W} ${H}`}
      role="group"
      aria-label={MSG.pedalGroupAria(fam.kind, slot.name, on)}
      style={{ filter: "drop-shadow(0 12px 16px rgba(0,0,0,.6))" }}
    >
      <defs>
        <linearGradient id={`body-${slot.slot}`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={body} />
          <stop offset="72%" stopColor={face} />
          <stop offset="100%" stopColor="#0f1216" />
        </linearGradient>
        <linearGradient id={`sheen-${slot.slot}`} x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stopColor="#fff" stopOpacity="0.16" />
          <stop offset="45%" stopColor="#fff" stopOpacity="0.04" />
          <stop offset="100%" stopColor="#000" stopOpacity="0.12" />
        </linearGradient>
        <radialGradient id={`led-${slot.slot}`} cx="35%" cy="30%">
          <stop offset="0%" stopColor="#fff" />
          <stop offset="40%" stopColor={led} />
          <stop offset="100%" stopColor={led} stopOpacity="0.85" />
        </radialGradient>
      </defs>

      <ellipse cx={W / 2} cy={H - 4} rx={W / 2 - 10} ry={6} fill="#000" opacity="0.5" />

      {/* corpo */}
      <rect x="6" y="8" width={W - 12} height={H - 20} rx="14" fill={`url(#body-${slot.slot})`} stroke="#0a0c0f" strokeWidth="1.8" />
      <rect x="6" y="8" width={W - 12} height={H - 20} rx="14" fill={`url(#sheen-${slot.slot})`} />

      {/* LED grande + parafusos */}
      <circle cx={W / 2} cy={34} r="8" fill={`url(#led-${slot.slot})`} stroke="#0a0c0f" strokeWidth="1.1" data-led={on ? "on" : "off"} />
      <circle cx={W / 2} cy={34} r="14" fill={led} opacity={on ? 0.18 : 0.1} aria-hidden="true" />
      {[18, W - 18].map((x) => (
        <g key={x}>
          <circle cx={x} cy={22} r="3.6" fill="#c8ccd2" stroke="#5b6068" strokeWidth="0.8" />
          <line x1={x - 2.4} y1={22} x2={x + 2.4} y2={22} stroke="#5b6068" strokeWidth="0.8" />
        </g>
      ))}

      {/* nome do efeito (nome REAL do algoritmo) */}
      <text x={W / 2} y={68} textAnchor="middle" fontSize="16" fontWeight="800" fill="#f3f5f7" fontFamily="ui-sans-serif, system-ui">
        {slot.name.length > 26 ? `${slot.name.slice(0, 25)}…` : slot.name}
      </text>
      <text x={W / 2} y={84} textAnchor="middle" fontSize="9" fill="#9aa3ad" fontFamily="ui-sans-serif, system-ui">
        {`${fam.kind} · ${model.ref}`}
      </text>

      {/* deck de controles */}
      <rect x="14" y={knobTop - 46} width={W - 28} height={rows * PITCH + 52} rx="10" fill="#000" opacity="0.28" />

      {/* knobs grandes + valor editável sob cada um */}
      {knobs.map((k, i) => {
        const row = Math.floor(i / cols);
        const colIdx = i % cols;
        const inRow = Math.min(knobs.length - row * cols, cols);
        const offset = ((cols - inRow) / 2) * PITCH; // última linha centralizada
        const cx = gridLeft + colIdx * PITCH + offset;
        const cy = knobTop + row * PITCH;
        return (
          <g key={k.pos}>
            <foreignObject x={cx - KNOB / 2} y={cy - KNOB / 2} width={KNOB} height={KNOB}>
              <Knob
                knob={k}
                size={KNOB}
                accent={fam.accent}
                engineer={engineer}
                addr={addr}
                codeHex={codeHex}
                onChange={(pos, value) => onKnobChange(slot, pos, value)}
                onReset={(pos) => onKnobReset(slot, pos)}
              />
            </foreignObject>
            <text x={cx} y={cy + KNOB / 2 + 18} textAnchor="middle" fontSize="11" fontWeight="700" fill="#dfe4ea" fontFamily="ui-sans-serif, system-ui">
              {k.name}
            </text>
            <foreignObject x={cx - VALUE_W / 2} y={cy + KNOB / 2 + 24} width={VALUE_W} height="32">
              <ValueBox
                text={k.value ?? "—"}
                frac={fracOf(k.range)}
                accent={fam.accent}
                onCommit={(raw) => onKnobChange(slot, k.pos, raw)}
              />
            </foreignObject>
          </g>
        );
      })}

      {/* footswitch */}
      <g
        onClick={() => onToggle(slot)}
        style={{ cursor: "pointer" }}
        role="button"
        aria-label={on ? MSG.pedalToggleOff : MSG.pedalToggleOn}
      >
        <circle cx={W / 2} cy={footY} r="19" fill="#20242b" stroke="#0a0c0f" strokeWidth="1.6" />
        <circle cx={W / 2} cy={footY} r="14" fill={on ? "#3a4150" : "#2a2f38"} stroke="#0a0c0f" />
        <circle cx={W / 2} cy={footY - 1} r="12" fill="#565f6e" opacity="0.5" />
      </g>

      {/* jacks */}
      <circle cx="26" cy={H - 22} r="5.6" fill="#0b0d10" stroke="#454c56" strokeWidth="1.6" />
      <circle cx={W - 26} cy={H - 22} r="5.6" fill="#0b0d10" stroke="#454c56" strokeWidth="1.6" />

      {/* placa GP-100 */}
      <rect x={W / 2 - 30} y={H - 32} width="60" height="14" rx="3" fill="#14171c" stroke="#0a0c0f" strokeWidth="0.8" />
      <text x={W / 2} y={H - 22} textAnchor="middle" fontSize="9" fontWeight="700" letterSpacing="1.4" fill={fam.accent} fontFamily="ui-monospace, monospace">
        {MSG.brandPlate}
      </text>
    </svg>
  );
}
