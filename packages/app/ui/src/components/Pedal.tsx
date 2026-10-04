/**
 * Pedal SVG — modelo POR VARIANTE (efeito real do dicionário) em DUAS
 * escalas:
 *
 *   - `board` (palco): enclosure COMPACTO com a largura real do catálogo
 *     (fxModels) — knob de 32px, valor em TEXTO (leitura) sob o rótulo e
 *     footswitch/jacks pequenos. Cabe no espaçamento definido do
 *     `.board-slots` (118–132px), então os 9 pedais convivem no board;
 *   - `modal` (edição): layout GRANDE — knob de 64px e VALOR EDITÁVEL
 *     (textbox) sob cada knob, com Enter aplicando ao device, Esc
 *     restaurando, ↑/↓ incrementando e duplo-clique resetando ao default.
 *
 * Os DOIS usam o MESMO estado: o que se ajusta no modal aparece no palco.
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

/** Escala do pedal: palco (compacto) ou modal de edição (grande). */
type PedalVariant = "board" | "modal";

/** Métricas de layout por escala (arte do palco — não é dado do device). */
const SCALES = {
  board: {
    knob: 32,
    pitchX: 46, // passo horizontal (knob + respiro)
    rowH: 54, // passo vertical (knob + rótulo + valor)
    cy0: 78, // centro da 1ª linha de knobs
    deckTop: 58,
    footR: 12,
    valueBox: false, // palco: valor em texto (a edição mora no modal)
    plate: false,
  },
  modal: {
    knob: 64,
    pitchX: 116,
    rowH: 116,
    cy0: 200,
    deckTop: 154,
    footR: 19,
    valueBox: true,
    plate: true,
  },
} as const;

const VALUE_W = 92; // caixa do valor editável (modal)
const LED_ON = "#39d353";
const LED_OFF = "#ff4b4b";

/** Corte de texto por largura disponível (nunca estoura o enclosure). */
function clip(text: string, max: number): string {
  return text.length > max ? `${text.slice(0, Math.max(1, max - 1))}…` : text;
}

interface Layout {
  W: number;
  H: number;
  cols: number;
  rows: number;
  /** centro da 1ª linha de knobs */
  cy0: number;
  /** recorte do deck (fundo dos controles) */
  deckTop: number;
  deckH: number;
  footY: number;
  jackY: number;
  rowH: number;
  /** passo horizontal dos knobs (pode encolher p/ caber no enclosure) */
  pitchX: number;
  nameMax: number;
  labelMax: number;
}

/** Geometria determinística do pedal para um slot e uma escala. */
function layoutFor(slot: BoardSlot, variant: PedalVariant): Layout {
  const m = modelFor(slot);
  const s = SCALES[variant];
  const cols = Math.min(modelCols(m.cols), 4);
  const rows = Math.max(1, Math.ceil(Math.max(slot.knobs.length, 1) / cols));
  if (variant === "modal") {
    // escala de EDIÇÃO: a geometria grande de sempre (deck com as caixas de
    // valor + footswitch e placa no pé, sem nenhuma sobreposição)
    const W = Math.max(m.w, 420);
    const H = 150 + rows * s.rowH + 110;
    return {
      W,
      H,
      cols,
      rows,
      cy0: s.cy0,
      deckTop: s.cy0 - 46,
      deckH: rows * s.rowH + 52,
      footY: H - 66,
      jackY: H - 22,
      rowH: s.rowH,
      pitchX: s.pitchX,
      nameMax: 26,
      labelMax: 16,
    };
  }
  // escala do PALCO: a largura fica DENTRO do espaçamento do .board-slots
  // (118–132px) — o pedal mais largo do catálogo nunca estoura a coluna, então
  // os 9 convivem no board. O passo dos knobs encolhe p/ caber no enclosure.
  const W = Math.max(118, Math.min(m.w, 132));
  let boardCols = Math.max(1, Math.min(modelCols(m.cols), 4));
  while (boardCols > 1 && (W - 12) / boardCols < 34) boardCols -= 1; // nunca sobrepõe
  const pitchX = Math.min(s.pitchX, (W - 12) / boardCols);
  const boardRows = Math.max(1, Math.ceil(Math.max(slot.knobs.length, 1) / boardCols));
  const cy0 = s.cy0;
  const lastVal = cy0 + (boardRows - 1) * s.rowH + (s.knob / 2 + 9 + 10);
  const deckTop = s.deckTop;
  const deckH = lastVal + 4 - deckTop;
  const footY = deckTop + deckH + 26;
  const H = footY + s.footR + 19;
  return {
    W,
    H,
    cols: boardCols,
    rows: boardRows,
    cy0,
    deckTop,
    deckH,
    footY,
    jackY: H - 11,
    rowH: s.rowH,
    pitchX,
    nameMax: Math.max(6, Math.floor((W - 22) / 6.4)),
    labelMax: Math.max(3, Math.floor((pitchX - 8) / 5.4)),
  };
}

/** `cols` do catálogo (sempre ≥1 — modelo nunca vem vazio). */
function modelCols(cols: number): number {
  return cols > 0 ? cols : 1;
}

/** Dimensões determinísticas do pedal (board/modal usam p/ layout). */
export function pedalDims(slot: BoardSlot, variant: PedalVariant = "board"): { w: number; h: number } {
  const l = layoutFor(slot, variant);
  return { w: l.W, h: l.H };
}

interface Props {
  slot: BoardSlot;
  /** Escala do desenho (default: o palco, compacto). */
  variant?: PedalVariant;
  engineer?: boolean;
  onKnobChange: (slot: BoardSlot, pos: number, value: string) => void;
  onKnobReset: (slot: BoardSlot, pos: number) => void;
  onToggle: (slot: BoardSlot) => void;
}

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

export function Pedal({
  slot,
  variant = "board",
  engineer = false,
  onKnobChange,
  onKnobReset,
  onToggle,
}: Props) {
  const fam = FAMILY_STYLE[slot.family];
  const model = modelFor(slot);
  const body = model.body ?? fam.body;
  const face = model.face ?? fam.face;
  const scale = SCALES[variant];
  const compact = variant === "board";
  const { W, H, cols, cy0, deckTop, deckH, footY, jackY, pitchX, nameMax, labelMax } =
    layoutFor(slot, variant);
  const on = slot.state;
  const led = on ? LED_ON : LED_OFF;
  const codeHex = `0x${(slot.code >>> 0).toString(16).padStart(8, "0")}`;
  const addr = `10 ${(slot.slot + 1).toString(16).padStart(2, "0")} 00 02`;

  const knobs = slot.knobs; // TODOS os controles do algoritmo
  const gridW = cols * pitchX;
  const gridLeft = W / 2 - gridW / 2 + pitchX / 2;

  const fracOf = (r?: [number, number]) =>
    r != null && (!Number.isInteger(r[0]) || !Number.isInteger(r[1]));

  const nameText = clip(slot.name, nameMax);
  // O TIPO do pedal é texto de usuário (#30): vem do catálogo, com a sigla
  // como fallback para uma família que o device ganhe depois.
  const kindText = (MSG.pedalKind as Record<string, string>)[fam.kind] ?? fam.kind;
  const refText = clip(`${compact ? "" : `${kindText} · `}${model.ref}`, nameMax + 6);

  return (
    <svg
      width={W}
      height={H}
      viewBox={`0 0 ${W} ${H}`}
      role="group"
      aria-label={MSG.pedalGroupAria(kindText, slot.name, on)}
      style={{
        // PALCO: o desenho acompanha a coluna do .board-slots (100% da
        // célula, nunca além da largura real do catálogo) — o aspect do
        // viewBox é preservado, então os 9 cabem numa fileira em qualquer
        // largura; o MODAL mantém a escala de edição (W×H atributos)
        width: compact ? "100%" : W,
        maxWidth: compact ? W : undefined,
        height: compact ? "auto" : H,
        // nunca deixa o flex do board ENCOLHER o desenho (escala distorcida)
        flexShrink: 0,
        filter: compact
          ? "drop-shadow(0 6px 8px rgba(0,0,0,.55))"
          : "drop-shadow(0 12px 16px rgba(0,0,0,.6))",
      }}
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
        {/* LUZ DO SISTEMA (#9): direção única cima/esquerda — risco
            especular na diagonal + sombreado na ponta oposta */}
        <linearGradient id={`spec-${slot.slot}`} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="#fff" stopOpacity="0.2" />
          <stop offset="30%" stopColor="#fff" stopOpacity="0.04" />
          <stop offset="62%" stopColor="#fff" stopOpacity="0" />
          <stop offset="100%" stopColor="#000" stopOpacity="0.1" />
        </linearGradient>
        <radialGradient id={`led-${slot.slot}`} cx="35%" cy="30%">
          <stop offset="0%" stopColor="#fff" />
          <stop offset="40%" stopColor={led} />
          <stop offset="100%" stopColor={led} stopOpacity="0.85" />
        </radialGradient>
      </defs>

      <ellipse
        cx={W / 2}
        cy={H - 4}
        rx={W / 2 - (compact ? 8 : 10)}
        ry={compact ? 4 : 6}
        fill="#000"
        opacity="0.5"
      />

      {/* corpo */}
      <rect
        x={compact ? 4 : 6}
        y={compact ? 6 : 8}
        width={W - (compact ? 8 : 12)}
        height={H - (compact ? 12 : 20)}
        rx={compact ? 10 : 14}
        fill={`url(#body-${slot.slot})`}
        stroke="#0a0c0f"
        strokeWidth={compact ? 1.4 : 1.8}
      />
      <rect
        x={compact ? 4 : 6}
        y={compact ? 6 : 8}
        width={W - (compact ? 8 : 12)}
        height={H - (compact ? 12 : 20)}
        rx={compact ? 10 : 14}
        fill={`url(#sheen-${slot.slot})`}
      />
      {/* brilho especular + fio de luz na aresta de cima do enclosure */}
      <rect
        x={compact ? 4 : 6}
        y={compact ? 6 : 8}
        width={W - (compact ? 8 : 12)}
        height={H - (compact ? 12 : 20)}
        rx={compact ? 10 : 14}
        fill={`url(#spec-${slot.slot})`}
      />
      <rect
        x={compact ? 10 : 13}
        y={compact ? 7.5 : 9.5}
        width={W - (compact ? 20 : 26)}
        height="1.6"
        rx="0.8"
        fill="#ffffff"
        opacity="0.14"
      />

      {/* LED + parafusos */}
      <circle
        cx={W / 2}
        cy={compact ? 18 : 34}
        r={compact ? 6 : 8}
        fill={`url(#led-${slot.slot})`}
        stroke="#0a0c0f"
        strokeWidth="1.1"
        data-led={on ? "on" : "off"}
      />
      <circle
        cx={W / 2}
        cy={compact ? 18 : 34}
        r={compact ? 10 : 14}
        fill={led}
        opacity={on ? 0.18 : 0.1}
        aria-hidden="true"
      />
      {[compact ? 13 : 18, W - (compact ? 13 : 18)].map((x) => (
        <g key={x}>
          <circle
            cx={x}
            cy={compact ? 15 : 22}
            r={compact ? 2.6 : 3.6}
            fill="#c8ccd2"
            stroke="#5b6068"
            strokeWidth="0.8"
          />
          <line
            x1={x - (compact ? 1.7 : 2.4)}
            y1={compact ? 15 : 22}
            x2={x + (compact ? 1.7 : 2.4)}
            y2={compact ? 15 : 22}
            stroke="#5b6068"
            strokeWidth="0.8"
          />
        </g>
      ))}

      {/* nome do efeito (nome REAL do algoritmo) */}
      <text
        x={W / 2}
        y={compact ? 40 : 68}
        textAnchor="middle"
        fontSize={compact ? 11.5 : 16}
        fontWeight="800"
        fill="#f3f5f7"
        fontFamily="ui-sans-serif, system-ui"
      >
        {nameText}
      </text>
      <text
        x={W / 2}
        y={compact ? 51 : 84}
        textAnchor="middle"
        fontSize={compact ? 7.5 : 9}
        fill="#9aa3ad"
        fontFamily="ui-sans-serif, system-ui"
      >
        {refText}
      </text>

      {/* deck de controles */}
      <rect
        x={compact ? 8 : 14}
        y={deckTop}
        width={W - (compact ? 16 : 28)}
        height={deckH}
        rx={compact ? 8 : 10}
        fill="#000"
        opacity="0.28"
      />

      {/* knobs (+ valor: texto no palco, textbox no modal) */}
      {knobs.map((k, i) => {
        const row = Math.floor(i / cols);
        const colIdx = i % cols;
        const inRow = Math.min(knobs.length - row * cols, cols);
        const offset = ((cols - inRow) / 2) * pitchX; // última linha centralizada
        const cx = gridLeft + colIdx * pitchX + offset;
        const cy = cy0 + row * scale.rowH;
        const label = clip(k.name, labelMax);
        return (
          <g key={k.pos}>
            <foreignObject
              x={cx - scale.knob / 2}
              y={cy - scale.knob / 2}
              width={scale.knob}
              height={scale.knob}
            >
              <Knob
                knob={k}
                size={scale.knob}
                locked={compact}
                accent={fam.accent}
                engineer={engineer}
                addr={addr}
                codeHex={codeHex}
                onChange={(pos, value) => onKnobChange(slot, pos, value)}
                onReset={(pos) => onKnobReset(slot, pos)}
              />
            </foreignObject>
            <text
              x={cx}
              y={cy + scale.knob / 2 + (compact ? 9 : 18)}
              textAnchor="middle"
              fontSize={compact ? 8 : 11}
              fontWeight="700"
              fill="#dfe4ea"
              fontFamily="ui-sans-serif, system-ui"
            >
              {label}
            </text>
            {scale.valueBox ? (
              <foreignObject
                x={cx - VALUE_W / 2}
                y={cy + scale.knob / 2 + 24}
                width={VALUE_W}
                height="32"
              >
                <ValueBox
                  text={k.value ?? "—"}
                  frac={fracOf(k.range)}
                  accent={fam.accent}
                  onCommit={(raw) => onKnobChange(slot, k.pos, raw)}
                />
              </foreignObject>
            ) : (
              <text
                x={cx}
                y={cy + scale.knob / 2 + 19}
                textAnchor="middle"
                fontSize="8.5"
                fontWeight="700"
                fill={fam.accent}
                fontFamily="var(--font-mono)"
                data-value={k.value ?? "—"}
              >
                {k.value ?? "—"}
              </text>
            )}
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
        <circle
          cx={W / 2}
          cy={footY}
          r={scale.footR}
          fill="#20242b"
          stroke="#0a0c0f"
          strokeWidth={compact ? 1.2 : 1.6}
        />
        <circle
          cx={W / 2}
          cy={footY}
          r={scale.footR - (compact ? 3.5 : 5)}
          fill={on ? "#3a4150" : "#2a2f38"}
          stroke="#0a0c0f"
        />
        <circle
          cx={W / 2}
          cy={footY - 1}
          r={scale.footR - (compact ? 5 : 7)}
          fill="#565f6e"
          opacity="0.5"
        />
      </g>

      {/* jacks */}
      <circle
        cx={compact ? 18 : 26}
        cy={jackY}
        r={compact ? 4 : 5.6}
        fill="#0b0d10"
        stroke="#454c56"
        strokeWidth={compact ? 1.2 : 1.6}
      />
      <circle
        cx={W - (compact ? 18 : 26)}
        cy={jackY}
        r={compact ? 4 : 5.6}
        fill="#0b0d10"
        stroke="#454c56"
        strokeWidth={compact ? 1.2 : 1.6}
      />

      {/* placa GP-100 (só no modal: no palco o espaço é dos controles) */}
      {scale.plate && (
        <>
          <rect
            x={W / 2 - 30}
            y={H - 32}
            width="60"
            height="14"
            rx="3"
            fill="#14171c"
            stroke="#0a0c0f"
            strokeWidth="0.8"
          />
          <text
            x={W / 2}
            y={H - 22}
            textAnchor="middle"
            fontSize="9"
            fontWeight="700"
            letterSpacing="1.4"
            fill={fam.accent}
            fontFamily="ui-monospace, monospace"
          >
            {MSG.brandPlate}
          </text>
        </>
      )}
    </svg>
  );
}
