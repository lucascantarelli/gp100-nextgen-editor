/**
 * Stage — o palco REAL da Fase 2 (U-3): mantém o cabeçalho do palco da
 * casca (display LED do patch, trava ⇄ mover e afinador) e desenha PEDAIS
 * REAIS nas 9 posições da cadeia (decisão do owner: visão completa — os
 * "lugares marcados" da Fase 1 só aparecem antes da 1ª leitura do board).
 * Com a trava ATIVA, arrastar o pedal para outra
 * posição reordena LOCAL (prévia — o device ainda não tem comando de
 * reorder; o LED/footswitch também é prévia local). O pedal do palco é
 * COMPACTO e os knobs são SÓ LEITURA (mostram o valor de cada controle):
 * quem ajusta é o modal de edição, aberto com clique/Enter em QUALQUER ponto
 * do pedal — e o App escuta `onKnobChange` e manda `device_set_param`
 * (§13.11).
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
 * Famílias que desenham pedal REAL no palco: a CADEIA INTEIRA (visão
 * completa das 9 posições — cada família tem modelo próprio em fxModels).
 * A validação de ESCRITA por família (roteiro R7, docs/UI_TEST_PLAN.md)
 * segue manual e NÃO esconde mais o pedal: a escrita é a mesma cadeia
 * genérica `device_set_param` (§13.11).
 */
export const PEDAL_FAMILIES_READY: ReadonlySet<BoardSlot["family"]> = new Set<
  BoardSlot["family"]
>(CHAIN_FAMILIES);

interface Props {
  /** Board do preset corrente (device_board); null antes da 1ª leitura. */
  board: BoardView | null;
  celebrate?: boolean;
  /** Modo engenheiro (Settings → General): tooltip do knob com addr/code/ctrl. */
  engineer?: boolean;
  arrangeMode: boolean;
  onToggleArrange: () => void;
  /** afinador (V-7): mora AQUI no cabeçalho, ao lado do LED e do cadeado */
  tuner: TunerSettings;
  onTunerChange: (s: TunerSettings) => void;
  onToggle: (slot: BoardSlot) => void;
  onKnobChange: (slot: BoardSlot, pos: number, value: string) => void;
  onKnobReset: (slot: BoardSlot, pos: number) => void;
  /** Abre o modal de edição (clique/Enter no pedal; trava OFF). */
  onEdit: (slot: BoardSlot) => void;
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
/* display do patch: superfície ESCAVADA do sistema (gp-sunken cuida do
   vidro escuro, do fio de luz e da sombra interna) */
const ledBox: CSSProperties = {
  padding: "7px 16px",
  fontFamily: "var(--font-mono)",
  display: "inline-flex",
  alignItems: "baseline",
  gap: "var(--space-12)",
};
/* as 9 posições numa ÚNICA fileira: 9 colunas fluidas (o desenho do pedal
   acompanha a coluna); abaixo do piso de largura, a faixa rola na horizontal */
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
  width: "100%", // acompanha a coluna fluida do .board-slots
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
/* cadeado (desenho puro): trava/destrava o arrastar-e-soltar dos pedais.
   Geometria/realce vêm do botão do sistema (`gp-btn`) — com a trava ATIVA
   ele fica no estado ligado (aria-pressed). */
const lockBtn = (on: boolean): CSSProperties => ({
  fontFamily: "var(--font-mono)",
  fontSize: 13,
  lineHeight: 1,
  color: on ? undefined : "#8b97a6",
  padding: "5px 8px",
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
  engineer = false,
  arrangeMode,
  onToggleArrange,
  tuner,
  onTunerChange,
  onToggle,
  onKnobChange,
  onKnobReset,
  onEdit,
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
          <div className="gp-sunken" style={ledBox} role="status">
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
            className="gp-btn"
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

      {/* as 9 posições da cadeia: TODAS com pedal real (visão completa);
          "lugar marcado" só antes da 1ª leitura do board */}
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
            // clique/Enter em QUALQUER ponto do pedal abre o modal de edição:
            // no palco os knobs são SÓ LEITURA (não disputam o gesto) — o
            // único controle com clique próprio é o footswitch (role button);
            // com a trava ativa o clique pertence ao drag-and-drop
            const isControl = (t: EventTarget | null) =>
              t instanceof Element &&
              t.closest('input, select, textarea, [role="button"], [data-control]') != null;
            return (
              <div
                key={chainIdx}
                aria-label={MSG.slotAria(pos + 1, fam)}
                role="group"
                tabIndex={0}
                title={MSG.pedalExpandTitle}
                draggable={arrangeMode}
                onDragStart={(e) => {
                  if (!arrangeMode) return;
                  dragFrom.current = pos;
                  if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
                }}
                onClick={(e) => {
                  if (arrangeMode || isControl(e.target)) return;
                  onEdit(s);
                }}
                onKeyDown={(e) => {
                  if (e.key !== "Enter" && e.key !== " ") return;
                  if (arrangeMode || isControl(e.target)) return;
                  // não roubar o atalho global do Espaço (drum) quando o foco
                  // está no PRÓPRIO grupo do pedal
                  e.preventDefault();
                  e.stopPropagation();
                  onEdit(s);
                }}
                {...dropHandlers}
                style={socketPedal(isOver, arrangeMode)}
              >
                <Pedal
                  slot={s}
                  engineer={engineer}
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

      {/* rodapé do palco (IN/OUT marcam o RODAPÉ da página, no App): com a
          trava ⇄ destravada ele vira a dica do arrasto — os pedais reais não
          têm mais o hint dos antigos lugares vazios */}
      <div
        aria-hidden="true"
        style={{
          textAlign: "center",
          padding: "var(--space-12) var(--space-4) 0",
          fontFamily: "var(--font-mono)",
          fontSize: 10,
          color: arrangeMode ? "var(--accent-text)" : "#6b6255",
        }}
      >
        {arrangeMode ? MSG.stageFooterArrange : MSG.stageFooter}
      </div>
    </section>
  );
}
