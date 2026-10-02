/**
 * PedalModal — edição ampliada do pedal (U-3, passo 9 do R7): o MESMO
 * `Pedal` do palco, escalado 1.25× (knob 64 → 80px — base do checklist
 * §3.2), com os mesmos handlers — o estado é único, então o que se ajusta
 * aqui aparece no palco na hora. Fecha com Esc, com o ✕ ou clicando fora;
 * o App mantém o transporte global inerte enquanto o modal está aberto
 * (mesma regra do Settings) e dá precedência ao Esc (painel do topo).
 */
import { useEffect, useRef } from "react";
import type { CSSProperties } from "react";
import type { BoardSlot } from "../ipc/types";
import { Pedal, pedalDims } from "./Pedal";
import { MSG } from "../i18n/messages";

/** Fator do modal: knob 64 → 80px (faixa 72–80 do §3.2). */
export const MODAL_SCALE = 1.25;

interface Props {
  /** Slot em edição (null = fechado); o objeto vem do board VIVO do App. */
  slot: BoardSlot | null;
  engineer?: boolean;
  onToggle: (slot: BoardSlot) => void;
  onKnobChange: (slot: BoardSlot, pos: number, value: string) => void;
  onKnobReset: (slot: BoardSlot, pos: number) => void;
  onClose: () => void;
}

const overlay: CSSProperties = {
  position: "fixed",
  inset: 0,
  background: "rgba(0,0,0,.55)",
  display: "grid",
  placeItems: "center",
  zIndex: 60,
};
const dialog: CSSProperties = {
  width: "min(640px, 96vw)",
  maxHeight: "92vh",
  overflow: "hidden",
  display: "grid",
  gridTemplateRows: "auto 1fr",
  background: "var(--bg-raised)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  borderRadius: 14,
  boxShadow: "0 24px 60px rgba(0,0,0,.6)",
};
const close: CSSProperties = {
  background: "var(--bg)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  color: "var(--text)",
  borderRadius: 8,
  padding: "6px 10px",
  cursor: "pointer",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  minHeight: 32,
};

export function PedalModal({
  slot,
  engineer = false,
  onToggle,
  onKnobChange,
  onKnobReset,
  onClose,
}: Props) {
  const ref = useRef<HTMLDivElement>(null);

  // WCAG: o foco entra no diálogo ao abrir (Esc/✕ sempre alcançáveis).
  useEffect(() => {
    if (slot != null) ref.current?.focus();
  }, [slot]);

  if (slot == null) return null;
  const dims = pedalDims(slot);

  return (
    <div style={overlay} onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-label={MSG.pedalModalAria(slot.name)}
        tabIndex={-1}
        style={dialog}
        onKeyDown={(e) => e.key === "Escape" && onClose()}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "var(--space-12)",
            padding: "var(--space-12) var(--space-12) var(--space-8)",
          }}
        >
          <strong style={{ fontSize: "var(--text-md)" }}>{MSG.pedalModalAria(slot.name)}</strong>
          <span style={{ fontSize: "var(--text-xs)", color: "var(--text-muted)" }}>
            {MSG.pedalModalHint}
          </span>
          <button
            type="button"
            onClick={onClose}
            aria-label={MSG.pedalModalCloseAria}
            style={{ ...close, marginLeft: "auto" }}
          >
            ✕
          </button>
        </div>

        {/* pedal AMPLIADO: wrapper com o tamanho escalado + transform no
            conteúdo (os inputs/knobs continuam interativos na área certa) */}
        <div style={{ overflow: "auto", padding: "0 var(--space-12) var(--space-12)", display: "grid", placeItems: "center" }}>
          <div style={{ width: dims.w * MODAL_SCALE, height: dims.h * MODAL_SCALE }}>
            <div style={{ transform: `scale(${MODAL_SCALE})`, transformOrigin: "top left" }}>
              <Pedal
                slot={slot}
                engineer={engineer}
                onToggle={onToggle}
                onKnobChange={onKnobChange}
                onKnobReset={onKnobReset}
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
