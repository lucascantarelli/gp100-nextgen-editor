/**
 * Pedalboard — o index artístico: DUAS linhas de pedais (tamanhos reais
 * por variante), cabos jack-a-jack (saem do jack DIREITO de um pedal e
 * entram no ESQUERDO do próximo, com curva catenária curta — patch cable),
 * cabo entre linhas, textura de PALCO, display LED com flip no fim do
 * boot e drag-and-drop para reordenar (prévia local).
 */
import { useMemo, useRef, useState } from "react";
import type { BoardView, BoardSlot } from "../ipc/types";
import { Pedal } from "./Pedal";
import { pedalDims } from "../design/geometry";
import { MSG } from "../i18n/messages";

interface Props {
  board: BoardView;
  states: Record<number, boolean>;
  order?: number[];
  engineer?: boolean;
  celebrate?: boolean;
  /** TRAVA do drag-and-drop: só reordena com ela ATIVA (knobs primeiro). */
  arrangeMode?: boolean;
  onToggle: (slot: BoardSlot) => void;
  onKnobChange: (slot: BoardSlot, pos: number, value: string) => void;
  onKnobReset: (slot: BoardSlot, pos: number) => void;
  onReorder: (fromSlot: number, toSlot: number) => void;
}

const GAP = 18; // patch cables curtos (respiro mínimo)
const PAD = 22;
const ROW_GAP = 30;

/** Geometria de uma linha: posição x de cada pedal + jack y. */
interface RowGeo {
  items: Array<{ slot: BoardSlot; x: number; w: number; h: number }>;
  width: number;
  height: number;
  jackY: number;
}

function rowGeo(list: BoardSlot[], jackY: number): RowGeo {
  let x = PAD;
  const items = list.map((slot) => {
    const d = pedalDims(slot);
    const item = { slot, x, w: d.w, h: d.h };
    x += d.w + GAP;
    return item;
  });
  return {
    items,
    width: Math.max(x - GAP + PAD, 720),
    height: Math.max(...list.map((s) => pedalDims(s).h), 220),
    jackY,
  };
}

export function Pedalboard({
  board,
  states,
  order,
  engineer = false,
  celebrate = false,
  arrangeMode = false,
  onToggle,
  onKnobChange,
  onKnobReset,
  onReorder,
}: Props) {
  const ordered = useMemo(() => {
    if (!order) return board.slots;
    const bySlot = new Map(board.slots.map((s) => [s.slot, s]));
    const arranged = order.map((s) => bySlot.get(s)).filter((s): s is BoardSlot => !!s);
    const missing = board.slots.filter((s) => !order.includes(s.slot));
    return [...arranged, ...missing];
  }, [board.slots, order]);

  // 1ª linha: 4 pedais; 2ª linha: os demais + o AMP encaixado no board
  // (pedalboard livre na tela inteira — o amp é item do palco, não coluna).
  // 1ª linha começa ABAIXO do display LED (não sobrepõe o 1º pedal).
  const LED_ZONE = 52;
  // TRÊS linhas de 3 pedais: knobs grandes, zero sobreposição, amp incluído.
  const rowsGeo = [ordered.slice(0, 3), ordered.slice(3, 6), ordered.slice(6, 9)]
    .filter((r) => r.length > 0)
    .map((r) => rowGeo(r, 0));
  const width = Math.max(...rowsGeo.map((r) => r.width), 720);

  // ── drag-and-drop (prévia local) ──
  const dragFrom = useRef<number | null>(null);
  const [dragOver, setDragOver] = useState<number | null>(null);

  /** Patch cable entre dois jacks (y = base dos pedais). */
  const jackCable = (x1: number, x2: number, y: number) =>
    `M ${x1} ${y} C ${x1 + 12} ${y + 16}, ${x2 - 12} ${y + 16}, ${x2} ${y}`;

  /** SVG de cabos da linha: OUT jack (direita, H-16) → IN jack (esquerda). */
  const rowCables = (geo: RowGeo, rowTop: number) => {
    const y = rowTop + geo.height - 16;
    const segs: string[] = [];
    for (let i = 0; i + 1 < geo.items.length; i += 1) {
      const a = geo.items[i];
      const b = geo.items[i + 1];
      segs.push(jackCable(a.x + a.w - 20, b.x + 20, y));
    }
    return segs.join(" ");
  };

  /** Pulsos de sinal por segmento (só quando AMBOS os pedais estão ON). */
  const pulses = (geo: RowGeo, rowTop: number) => {
    const y = rowTop + geo.height - 16;
    const out: Array<{ d: string; key: string; begin: number }> = [];
    for (let i = 0; i + 1 < geo.items.length; i += 1) {
      const a = geo.items[i];
      const b = geo.items[i + 1];
      const on = (states[a.slot.slot] ?? a.slot.state) && (states[b.slot.slot] ?? b.slot.state);
      if (on) {
        out.push({
          d: jackCable(a.x + a.w - 20, b.x + 20, y),
          key: `${a.slot.slot}-${b.slot.slot}`,
          begin: (i * 0.24) % 1.2,
        });
      }
    }
    return out;
  };

  const renderRow = (geo: RowGeo, rowTop: number, rowIdx: number) => (
    <>
      <svg
        aria-hidden="true"
        width={width}
        height={geo.height + 40}
        style={{ position: "absolute", left: 0, top: rowTop, pointerEvents: "none", zIndex: 3 }}
      >
        <path d={rowCables(geo, rowTop)} fill="none" stroke="#08090b" strokeWidth="7" strokeLinecap="round" />
        <path d={rowCables(geo, rowTop)} fill="none" stroke="#454d57" strokeWidth="4" strokeLinecap="round" />
        {pulses(geo, rowTop).map((p) => (
          <circle key={p.key} r="3.2" fill="#ffd23f" opacity="0.95">
            <animateMotion dur="1.2s" begin={`${p.begin}s`} repeatCount="indefinite" path={p.d} />
          </circle>
        ))}
        {/* salto entre linhas: fim da linha de cima → começo da de baixo */}
        {rowIdx === 0 && geo.items.length > 0 && (
          <>
            <path
              d={`M ${geo.items[geo.items.length - 1].x + geo.items[geo.items.length - 1].w - 20} ${rowTop + geo.height - 16} C ${width - 6} ${rowTop + geo.height + 4}, ${width - 6} ${rowTop + geo.height + ROW_GAP - 8}, ${PAD + 20} ${rowTop + geo.height + ROW_GAP - 16}`}
              fill="none"
              stroke="#08090b"
              strokeWidth="7"
              strokeLinecap="round"
            />
            <path
              d={`M ${geo.items[geo.items.length - 1].x + geo.items[geo.items.length - 1].w - 20} ${rowTop + geo.height - 16} C ${width - 6} ${rowTop + geo.height + 4}, ${width - 6} ${rowTop + geo.height + ROW_GAP - 8}, ${PAD + 20} ${rowTop + geo.height + ROW_GAP - 16}`}
              fill="none"
              stroke="#454d57"
              strokeWidth="4"
              strokeLinecap="round"
            />
          </>
        )}
      </svg>

      <div
        style={{
          position: "absolute",
          left: 0,
          top: rowTop,
          zIndex: 4,
          display: "flex",
          gap: GAP,
          padding: `0 ${PAD}px`,
          alignItems: "flex-start",
        }}
      >
        {geo.items.map(({ slot: s }) => {
          const dim = states[s.slot] ?? s.state;
          return (
            <div
              key={s.slot}
              draggable={arrangeMode}
              onDragStart={(e) => {
                if (!arrangeMode) return;
                dragFrom.current = s.slot;
                e.dataTransfer.effectAllowed = "move";
              }}
              onDragOver={(e) => {
                if (!arrangeMode) return;
                e.preventDefault();
                setDragOver(s.slot);
              }}
              onDragLeave={() => setDragOver((cur) => (cur === s.slot ? null : cur))}
              onDrop={(e) => {
                e.preventDefault();
                setDragOver(null);
                if (dragFrom.current != null && dragFrom.current !== s.slot) {
                  onReorder(dragFrom.current, s.slot);
                }
                dragFrom.current = null;
              }}
              style={{
                cursor: arrangeMode ? "grab" : undefined,
                filter: dragOver === s.slot ? "brightness(1.25)" : undefined,
                outline: dragOver === s.slot ? "2px dashed #ffb020" : undefined,
                outlineOffset: 4,
                borderRadius: 12,
                opacity: dim ? 1 : 0.85,
                transition: "filter .15s ease, opacity .2s ease",
              }}
            >
              <Pedal
                slot={{ ...s, state: dim }}
                engineer={engineer}
                onToggle={onToggle}
                onKnobChange={onKnobChange}
                onKnobReset={onKnobReset}
              />
            </div>
          );
        })}
      </div>
    </>
  );

  const totalH =
    LED_ZONE + rowsGeo.reduce((acc, geo) => acc + geo.height + ROW_GAP, 0) - ROW_GAP + 40;

  return (
    <div style={{ position: "relative", overflowX: "auto", paddingBottom: 8 }}>
      <div style={{ position: "relative", minWidth: width, padding: "0 10px 6px" }}>
        {/* display LED (alinhado à esquerda, junto ao header do board) */}
        <div
          style={{
            position: "absolute",
            top: 6,
            left: 14,
            background: "#0a0d10",
            border: "1px solid #1d242c",
            borderRadius: 6,
            padding: "7px 16px",
            fontFamily: "ui-monospace, monospace",
            boxShadow: "inset 0 0 12px #000",
            zIndex: 6,
            display: "flex",
            alignItems: "baseline",
            gap: 10,
          }}
          role="status"
        >
          <span
            className={celebrate ? "led-flip" : undefined}
            style={{ color: "#ff9d2e", fontSize: 24, fontWeight: 700, letterSpacing: 1.5, display: "inline-block" }}
          >
            {String(board.pp).padStart(2, "0")}
          </span>
          <span style={{ color: "#5c6774", fontSize: 11, marginLeft: 10 }}>{board.name}</span>
          <span style={{ color: "#3d4653", fontSize: 10, marginLeft: 10 }}>
            {board.ppTypeName} · {ordered.filter((s) => states[s.slot] ?? s.state).length}/{ordered.length}{MSG.onSuffix}
          </span>
        </div>

        {/* o board — palco */}
        <div
          style={{
            marginTop: 48,
            borderRadius: 16,
            padding: "20px 0 26px",
            background: "radial-gradient(140% 120% at 50% 0%, #2a241d 0%, #1c1712 42%, #120e0a 100%)",
            border: "1px solid #3a3128",
            boxShadow:
              "inset 0 2px 0 rgba(255,255,255,.06), inset 0 -18px 30px rgba(0,0,0,.55), 0 26px 44px rgba(0,0,0,.65)",
            position: "relative",
            height: totalH,
          }}
        >
          {/* spots de palco */}
          <div
            aria-hidden="true"
            style={{
              position: "absolute",
              inset: 0,
              borderRadius: 16,
              background:
                "radial-gradient(60% 40% at 20% 0%, rgba(255,176,32,.10), transparent 60%), radial-gradient(60% 40% at 80% 0%, rgba(255,176,32,.08), transparent 60%)",
              pointerEvents: "none",
              zIndex: 1,
            }}
          />
          {/* velcro por linha */}
          {rowsGeo
            .map((geo, i) => ({
              top: LED_ZONE - 4 + i * (geo.height + ROW_GAP),
              h: geo.height + 20,
            }))
            .map((v, r) => (
              <div
                key={r}
                aria-hidden="true"
                style={{
                  position: "absolute",
                  left: 20,
                  right: 20,
                  top: v.top,
                  height: v.h,
                  borderRadius: 10,
                  backgroundImage: "repeating-linear-gradient(45deg,#242831 0 3px,#1d2128 3px 6px)",
                  opacity: 0.9,
                  boxShadow: "inset 0 0 18px rgba(0,0,0,.65)",
                  zIndex: 2,
                }}
              />
            ))}

          {rowsGeo.map((geo, i) => renderRow(geo, LED_ZONE + i * (geo.height + ROW_GAP), i))}

          {/* rótulos de saída */}
          <div
            style={{
              position: "absolute",
              bottom: 4,
              left: 0,
              right: 0,
              display: "flex",
              justifyContent: "space-between",
              padding: "0 26px",
              fontFamily: "ui-monospace, monospace",
              fontSize: 10,
              color: "#6b6255",
              zIndex: 4,
            }}
            aria-hidden="true"
          >
            <span>{MSG.stageIn}</span>
            <span>{MSG.stageOut}</span>
          </div>
        </div>
      </div>
    </div>
  );
}
