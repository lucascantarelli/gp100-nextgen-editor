/**
 * Stage — o palco REAL da Fase 2 (U-3): mantém o cabeçalho do palco da
 * casca (display LED do patch, trava ⇄ mover e afinador) e desenha os
 * pedais REAIS do board do device nas posições cuja família já foi
 * validada (`PEDAL_FAMILIES_READY` — "1 efeito por vez", decisão do owner
 * em docs/UI_REFERENCE.md §3.1). As demais posições seguem como os 9
 * lugares marcados da Fase 1, para o owner validar o pedal em TODAS as 9
 * posições (roteiro R7): com a trava ATIVA, arrastar o pedal para outra
 * posição reordena LOCAL (prévia — o device ainda não tem comando de
 * reorder; o LED/footswitch e switch/combox também são prévia local).
 * O knob NUMÉRICO é o único que sai daqui para o device: o App escuta
 * `onKnobChange` e manda `device_set_param` (§13.11).
 */
import { useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties, DragEvent } from "react";
import type { BoardSlot, BoardView } from "../ipc/types";
import { CHAIN_FAMILIES } from "../ipc/types";
import { Pedal } from "./Pedal";
import { TunerPanel } from "./TunerPanel";
import type { TunerSettings } from "./TunerPanel";
import { MSG } from "../i18n/messages";

/**
 * Famílias com pedal REAL já validado no palco (fase "1 efeito por vez").
 * Só o PRE/COMP entrou até agora; cada rodada do R7 (docs/UI_TEST_PLAN.md)
 * adiciona a próxima família depois da aprovação do owner.
 */
export const PEDAL_FAMILIES_READY: ReadonlySet<BoardSlot["family"]> = new Set<
  BoardSlot["family"]
>(["PRE"]);

interface Props {
  /** Board do preset corrente (device_board); null antes da 1ª leitura. */
  board: BoardView | null;
  celebrate?: boolean;
  arrangeMode: boolean;
  onToggleArrange: () => void;
  /** afinador (V-7): mora AQUI no cabeçalho, ao lado do LED e do cadeado */
  tuner: TunerSettings;
  onTunerChange: (s: TunerSettings) => void;
  onToggle: (slot: BoardSlot) => void;
  onKnobChange: (slot: BoardSlot, pos: number, value: string) => void;
  onKnobReset: (slot: BoardSlot, pos: number) => void;
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
/* grade das posições da cadeia: colunas por CONTEÚDO (o pedal real é mais
   largo que um lugar vazio); em telas largas as 9 posições cabem numa linha */
const row: CSSProperties = {
  position: "relative",
  zIndex: 4,
};
const socketPedal = (over: boolean, arrange: boolean): CSSProperties => ({
  borderRadius: 12,
  cursor: arrange ? "grab" : undefined,
  outline: over ? "2px dashed var(--accent)" : undefined,
  outlineOffset: 4,
  filter: over ? "brightness(1.2)" : undefined,
  alignSelf: "start",
  display: "flex",
  justifyContent: "center",
});
const slot = (over: boolean, arrange: boolean): CSSProperties => ({
  borderRadius: 12,
  minHeight: 132,
  width: 132,
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
const rightSlot: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "var(--space-8)",
  flex: 1,
  justifyContent: "flex-end",
  minWidth: 0,
};

/** Ordem de exibição padrão: cada posição com o slot da MESMA família. */
const BASE_ORDER = CHAIN_FAMILIES.map((_, i) => i);

export function Stage({
  board,
  celebrate = false,
  arrangeMode,
  onToggleArrange,
  tuner,
  onTunerChange,
  onToggle,
  onKnobChange,
  onKnobReset,
}: Props) {
  // Reordenação LOCAL (prévia): ordem[display] = slot do board na posição.
  // Reset ao trocar de preset — editar um knob NÃO pode reordenar o palco.
  const [order, setOrder] = useState<number[] | null>(null);
  const dragFrom = useRef<number | null>(null); // posição de ORIGEM do arrasto
  const [over, setOver] = useState<number | null>(null);

  const pp = board?.pp ?? null;
  useEffect(() => {
    setOrder(null);
  }, [pp]);

  const display = useMemo(() => order ?? BASE_ORDER, [order]);
  const byChain = useMemo(() => {
    const m = new Map<number, BoardSlot>();
    for (const s of board?.slots ?? []) m.set(s.slot, s);
    return m;
  }, [board]);

  /** Move o item da posição `from` para a posição `to` (prévia local). */
  const moveTo = (from: number, to: number) => {
    setOrder(() => {
      const next = [...display];
      const [moved] = next.splice(from, 1);
      next.splice(to, 0, moved);
      return next;
    });
  };

  const name = board?.name ?? "…";
  const ppLabel = String((pp ?? 0) + 1).padStart(2, "0");
  const typeName = board?.ppTypeName ?? "…";

  return (
    <section style={stage} aria-label={MSG.boardAria}>
      {/* cabeçalho do palco: display LED do patch (com o cadeado de mover
          logo ABAIXO dele, alinhado) à esquerda; AFINADOR à direita */}
      <div
        style={{
          display: "flex",
          alignItems: "flex-start",
          justifyContent: "space-between",
          gap: "var(--space-12)",
          flexWrap: "wrap",
          marginBottom: "var(--space-20)",
        }}
      >
        <div style={leftCol}>
          <div style={ledBox} role="status">
            <span
              className={celebrate ? "led-flip" : undefined}
              style={{
                color: "var(--accent)",
                fontSize: 26,
                fontWeight: 700,
                letterSpacing: 2,
                display: "inline-block",
              }}
            >
              {ppLabel}
            </span>
            <span style={{ color: "var(--text)", fontSize: 14, fontWeight: 700 }}>{name}</span>
            <span style={{ color: "#5c6774", fontSize: 11 }}>{typeName}</span>
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

      {/* as 9 posições da cadeia: pedal real onde a família já foi validada,
          placeholders nas demais (1 efeito por vez — R7) */}
      <div className="board-slots" style={row}>
        {display.map((chainIdx, pos) => {
          const s = byChain.get(chainIdx);
          const fam = s?.family ?? CHAIN_FAMILIES[chainIdx];
          const ready = s != null && PEDAL_FAMILIES_READY.has(s.family);
          const isOver = over === pos;
          const dropHandlers = {
            onDragOver: (e: DragEvent) => {
              if (!arrangeMode) return;
              e.preventDefault();
              setOver(pos);
            },
            onDragLeave: () => setOver((cur) => (cur === pos ? null : cur)),
            onDrop: (e: DragEvent) => {
              e.preventDefault();
              setOver(null);
              if (dragFrom.current != null && dragFrom.current !== pos) {
                moveTo(dragFrom.current, pos);
              }
              dragFrom.current = null;
            },
          };
          if (s != null && ready) {
            return (
              <div
                key={chainIdx}
                aria-label={MSG.slotAria(pos + 1, fam)}
                draggable={arrangeMode}
                onDragStart={(e) => {
                  if (!arrangeMode) return;
                  dragFrom.current = pos;
                  if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
                }}
                {...dropHandlers}
                style={socketPedal(isOver, arrangeMode)}
              >
                <Pedal
                  slot={s}
                  onToggle={onToggle}
                  onKnobChange={onKnobChange}
                  onKnobReset={onKnobReset}
                />
              </div>
            );
          }
          return (
            <div
              key={chainIdx}
              aria-label={MSG.slotAria(pos + 1, fam)}
              {...dropHandlers}
              style={slot(isOver, arrangeMode)}
            >
              <span style={famTag}>{fam}</span>
              <span aria-hidden="true" style={{ fontSize: 30, color: "#3d3352" }}>
                ◇
              </span>
              <span style={hint}>{arrangeMode ? MSG.slotArrange : MSG.slotEmpty}</span>
            </div>
          );
        })}
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
