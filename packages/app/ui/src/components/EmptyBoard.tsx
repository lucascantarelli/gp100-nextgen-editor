/**
 * EmptyBoard — o palco: os 9 LUGARES da cadeia marcados
 * (PRE/DST/AMP/NR/CAB/EQ/MOD/DLY/RVB), sem pedais desenhados ainda.
 * Display LED estilo hardware com pp/nome (também na navbar).
 * A trava ⇄ mover já é respeitada aqui: com ela ativa, os slots aceitam
 * drop (feedback visual), preparando a reordenação quando os pedais chegarem.
 */
import { useRef, useState } from "react";
import type { CSSProperties } from "react";
import { CHAIN_FAMILIES } from "../ipc/types";
import { TunerPanel } from "./TunerPanel";
import type { TunerSettings } from "./TunerPanel";
import { MSG } from "../i18n/messages";

interface Props {
  pp: number;
  presetName: string;
  ppTypeName: string;
  celebrate?: boolean;
  arrangeMode: boolean;
  onToggleArrange: () => void;
  /** afinador (V-7): mora AQUI no cabeçalho, ao lado do LED e do ⇄ mover,
   *  ocupando o espaço do antigo visualizador VU — SEMPRE VISÍVEL */
  tuner: TunerSettings;
  onTunerChange: (s: TunerSettings) => void;
  onReorder: (fromFamily: string, toFamily: string) => void;
}

const stage: CSSProperties = {
  borderRadius: 16,
  padding: "var(--space-20) var(--space-20) var(--space-32)",
  background: "radial-gradient(140% 120% at 50% 0%, #2a241d 0%, #1c1712 42%, #120e0a 100%)",
  border: "1px solid #3a3128",
  boxShadow:
    "inset 0 2px 0 rgba(255,255,255,.06), inset 0 -18px 30px rgba(0,0,0,.55), 0 26px 44px rgba(0,0,0,.65)",
  position: "relative",
};
const ledBox: CSSProperties = {
  background: "#0a0d10",
  border: "1px solid #1d242c",
  borderRadius: 6,
  padding: "7px 16px",
  fontFamily: "var(--font-mono)",
  boxShadow: "inset 0 0 12px #000",
  display: "inline-flex",
  alignItems: "baseline",
  gap: "var(--space-12)",
};
/* grade responsiva via .board-slots (9-across no wide; 3×3 ≤1340px) */
const row: CSSProperties = {
  position: "relative",
  zIndex: 4,
};
const slot = (over: boolean, arrange: boolean): CSSProperties => ({
  borderRadius: 12,
  minHeight: 132,
  display: "grid",
  gridTemplateRows: "auto 1fr auto",
  justifyItems: "center",
  alignItems: "center",
  gap: "var(--space-4)",
  padding: "var(--space-12) var(--space-4)",
  background: over
    ? "color-mix(in srgb, var(--accent) 14%, #17130e)"
    : "linear-gradient(180deg, #241f19 0%, #1a1611 100%)",
  border: over ? "2px dashed var(--accent)" : "1px solid #3a3128",
  cursor: arrange ? "grab" : "default",
  transition: "border var(--motion-fast) var(--ease-out)",
});
const famTag: CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-xs)",
  fontWeight: 700,
  letterSpacing: 1.2,
  color: "var(--accent)",
  background: "#0d0b08",
  border: "1px solid #4a3d29",
  borderRadius: 6,
  padding: "2px 10px",
};
const hint: CSSProperties = { fontSize: "var(--text-xs)", color: "#6b6255", textAlign: "center" };

/* coluna esquerda: display LED do patch + cadeado de mover ABAIXO dele */
const leftCol: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  alignItems: "flex-start",
  gap: 8,
  minWidth: 0,
};
/* cadeado (desenho puro): trava/destrava o arrastar-e-soltar dos pedais */
const lockBtn = (on: boolean): CSSProperties => ({
  fontFamily: "var(--font-mono)",
  fontSize: 13,
  lineHeight: 1,
  color: on ? "var(--accent)" : "#8b97a6",
  background: "transparent",
  border: "1px solid #4a3d29",
  borderRadius: 6,
  padding: "5px 8px",
  cursor: "pointer",
  minHeight: 32,
  minWidth: 36,
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "center",
});
/* afinador ocupa a DIREITA, no lugar onde era o botão ⇄ mover */
const rightSlot: CSSProperties = { display: "flex", alignItems: "center", gap: "var(--space-8)", flex: 1, justifyContent: "flex-end", minWidth: 0 };

export function EmptyBoard({ pp, presetName, ppTypeName, celebrate = false, arrangeMode, onToggleArrange, tuner, onTunerChange, onReorder }: Props) {
  const dragFrom = useRef<string | null>(null);
  const [over, setOver] = useState<string | null>(null);

  return (
    <section style={stage} aria-label={MSG.boardAria}>
      {/* cabeçalho do palco: display LED do patch (com o cadeado de mover
          logo ABAIXO dele, alinhado) à esquerda; AFINADOR à direita — ocupa
          o lugar onde era o botão ⇄ mover */}
      <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: "var(--space-12)", flexWrap: "wrap", marginBottom: "var(--space-20)" }}>
        <div style={leftCol}>
          <div style={ledBox} role="status">
            <span className={celebrate ? "led-flip" : undefined} style={{ color: "var(--accent)", fontSize: 26, fontWeight: 700, letterSpacing: 2, display: "inline-block" }}>
              {String(pp + 1).padStart(2, "0")}
            </span>
            <span style={{ color: "var(--text)", fontSize: 14, fontWeight: 700 }}>{presetName}</span>
            <span style={{ color: "#5c6774", fontSize: 11 }}>{ppTypeName}</span>
          </div>
          <button
            style={lockBtn(arrangeMode)}
            onClick={onToggleArrange}
            aria-pressed={arrangeMode}
            aria-label={MSG.arrangeAria}
            title={MSG.arrangeTitle}
          >
            {arrangeMode ? MSG.arrangeLabelOpen : MSG.arrangeLabel}
          </button>
        </div>
        <div style={rightSlot}>
          <TunerPanel settings={tuner} onChange={onTunerChange} />
        </div>
      </div>

      {/* os 9 lugares */}
      <div className="board-slots" style={row}>
        {CHAIN_FAMILIES.map((fam, i) => (
          <div
            key={fam}
            style={slot(over === fam, arrangeMode)}
            draggable={arrangeMode}
            onDragStart={() => {
              dragFrom.current = fam;
            }}
            onDragOver={(e) => {
              if (!arrangeMode) return;
              e.preventDefault();
              setOver(fam);
            }}
            onDragLeave={() => setOver((cur) => (cur === fam ? null : cur))}
            onDrop={(e) => {
              e.preventDefault();
              setOver(null);
              if (dragFrom.current && dragFrom.current !== fam) onReorder(dragFrom.current, fam);
              dragFrom.current = null;
            }}
            aria-label={MSG.slotAria(i + 1, fam)}
          >
            <span style={famTag}>{fam}</span>
            <span aria-hidden="true" style={{ fontSize: 30, color: "#3d3352" }}>◇</span>
            <span style={hint}>{arrangeMode ? MSG.slotArrange : MSG.slotEmpty}</span>
          </div>
        ))}
      </div>

      {/* rodapé do palco (IN/OUT marcam o RODAPÉ da página, no App) */}
      <div
        aria-hidden="true"
        style={{
          textAlign: "center",
          padding: "var(--space-12) var(--space-4) 0",
          fontFamily: "var(--font-mono)",
          fontSize: 10,
          color: "#6b6255",
        }}
      >
        {MSG.stageFooter}
      </div>
    </section>
  );
}
