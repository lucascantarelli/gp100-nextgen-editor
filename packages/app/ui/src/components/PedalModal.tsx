/**
 * PedalModal — edição ampliada do pedal (U-3, passo 9 do R7): o MESMO
 * `Pedal` do palco na escala de EDIÇÃO (knob 64 → 80px com o 1.25× — base do
 * checklist §3.2), com os mesmos handlers — o estado é único, então o que se
 * ajusta aqui aparece no palco na hora (o pedal do palco é compacto e só
 * LEITURA: mostra os valores, quem ajusta é este modal). Fecha com Esc, com o
 * ✕ ou clicando fora; o App mantém o transporte global inerte enquanto o
 * modal está aberto (mesma regra do Settings) e dá precedência ao Esc
 * (painel do topo).
 *
 * À DIREITA mora a "Effects List" (issue #19): a lista de algoritmos que o
 * módulo oferece, com busca e o efeito atual marcado. Trocar ali troca o
 * efeito no PALCO na hora (estado único, como os knobs) e já devolve os
 * controles com os valores do novo efeito. É prévia local — o `change-effect`
 * (`0x47`) ainda não tem formato validado no fio (BLOCKERS 10b).
 */
import { useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties } from "react";
import type { BoardSlot } from "../ipc/types";
import { algorithmsOf, filterAlgorithms } from "../effects";
import type { FxAlgorithm } from "../artifacts/fxData";
import { Pedal } from "./Pedal";
import { pedalDims } from "../design/geometry";
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
  /** troca o algoritmo do slot (prévia local; o App só reflete no palco) */
  onChangeEffect: (slot: BoardSlot, alg: FxAlgorithm) => void;
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
/* modal = superfície FLUTUANTE do sistema (luz/sombra dos tokens) */
const dialog: CSSProperties = {
  width: "min(880px, 96vw)",
  maxHeight: "92vh",
  overflow: "hidden",
  display: "grid",
  gridTemplateRows: "auto 1fr",
};
/* corpo em DUAS colunas: pedal ampliado à esquerda, Effects List à direita
   (issue #19). Em janela estreita as colunas empilham — o pedal é o que
   encolhe primeiro em largura. */
const body: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "minmax(0, 1fr) minmax(220px, 264px)",
  gap: "var(--space-12)",
  padding: "0 var(--space-12) var(--space-12)",
  minHeight: 0,
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

/* ── Effects List (o que o módulo oferece; issue #19) ── */
const listCol: CSSProperties = {
  display: "grid",
  gridTemplateRows: "auto auto minmax(0, 1fr) auto",
  gap: "var(--space-8)",
  minHeight: 0,
  minWidth: 0,
  paddingLeft: "var(--space-12)",
  borderLeft: "1px solid color-mix(in srgb, var(--text-muted) 22%, transparent)",
};
const listHead: CSSProperties = {
  display: "flex",
  alignItems: "baseline",
  justifyContent: "space-between",
  gap: "var(--space-8)",
  minWidth: 0,
};
const listTitle: CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  fontWeight: 700,
  minWidth: 0,
  overflow: "hidden",
  textOverflow: "ellipsis",
  whiteSpace: "nowrap",
};
const searchInput: CSSProperties = {
  background: "var(--bg)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  color: "var(--text)",
  borderRadius: 8,
  padding: "7px 10px",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  minHeight: 32,
  minWidth: 0,
  width: "100%",
};
const algList: CSSProperties = { overflowY: "auto", display: "grid", alignContent: "start", gap: 2, minWidth: 0 };
const algRow = (on: boolean): CSSProperties => ({
  display: "flex",
  alignItems: "center",
  gap: "var(--space-4)",
  textAlign: "left",
  padding: "5px 8px",
  borderRadius: 8,
  cursor: "pointer",
  minHeight: 32,
  minWidth: 0,
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  background: on ? "color-mix(in srgb, var(--accent) 18%, transparent)" : "transparent",
  border: on ? "1px solid var(--accent)" : "1px solid transparent",
  color: on ? "var(--text)" : "var(--text-muted)",
  fontWeight: on ? 700 : 400,
});
const algName: CSSProperties = { minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" };
const algCheck: CSSProperties = { color: "var(--accent-text)", flex: "0 0 auto" };
const emptyNote: CSSProperties = { color: "var(--text-muted)", fontSize: "var(--text-sm)", padding: "var(--space-8)", margin: 0 };
const note: CSSProperties = { fontSize: "var(--text-xs)", color: "var(--text-muted)", margin: 0 };

export function PedalModal({
  slot,
  engineer = false,
  onToggle,
  onKnobChange,
  onKnobReset,
  onChangeEffect,
  onClose,
}: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const [q, setQ] = useState("");

  // WCAG: o foco entra no diálogo ao abrir (Esc/✕ sempre alcançáveis).
  useEffect(() => {
    if (slot != null) ref.current?.focus();
  }, [slot]);

  // trocar de efeito reescreve os controles: a busca antiga não tem mais a quem
  // servir (o efeito novo tem nome e quantidade de knobs diferentes).
  useEffect(() => {
    setQ("");
  }, [slot?.code]);

  // a lista é do MÓDULO (PRE oferece PREs, DST oferece DSTs): a família do
  // slot é fixa na cadeia — o que muda é o algoritmo dentro dela.
  const algs = useMemo(() => (slot == null ? [] : algorithmsOf(slot.family)), [slot?.family]);
  const shown = useMemo(() => filterAlgorithms(algs, q), [algs, q]);

  if (slot == null) return null;
  const dims = pedalDims(slot, "modal");
  const pick = (alg: FxAlgorithm) => onChangeEffect(slot, alg);

  return (
    <div style={overlay} onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-label={MSG.pedalModalAria(slot.name)}
        tabIndex={-1}
        className="gp-surface gp-surface--floating"
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

        <div style={body}>
          {/* pedal AMPLIADO: wrapper com o tamanho escalado + transform no
              conteúdo (os inputs/knobs continuam interativos na área certa).
              `safe center`: em janela estreita o pedal NÃO é cortado — o
              alinhamento vira start e a faixa rola na horizontal. */}
          <div
            style={{
              overflow: "auto",
              minWidth: 0,
              display: "grid",
              justifyItems: "safe center",
              alignContent: "safe center",
            }}
          >
            <div style={{ width: dims.w * MODAL_SCALE, height: dims.h * MODAL_SCALE }}>
              {/* o interno PRECISA da medida da escala (sem ela o bloco herda
                  a largura do wrapper e o 1.25× vira 1.5625× — pedal mais largo
                  que o corpo, conteúdo fora de centro e barra de rolagem) */}
              <div
                style={{
                  width: dims.w,
                  height: dims.h,
                  transform: `scale(${MODAL_SCALE})`,
                  transformOrigin: "top left",
                }}
              >
                <Pedal
                  slot={slot}
                  variant="modal"
                  engineer={engineer}
                  onToggle={onToggle}
                  onKnobChange={onKnobChange}
                  onKnobReset={onKnobReset}
                />
              </div>
            </div>
          </div>

          {/* EFFECTS LIST — os algoritmos do módulo (issue #19). O efeito
              atual fica marcado com ✓ e em destaque; clicar troca no palco
              na hora (estado único, igual aos knobs). */}
          <div style={listCol}>
            <div style={listHead}>
              <span style={listTitle}>{slot.family}</span>
              <span style={{ ...listTitle, fontWeight: 400 }}>{MSG.effectListCount(algs.length)}</span>
            </div>
            <input
              style={searchInput}
              value={q}
              onChange={(e) => setQ(e.target.value)}
              placeholder={MSG.effectListSearchPlaceholder}
              aria-label={MSG.effectListSearchAria}
            />
            <div style={algList} role="listbox" aria-label={MSG.effectListAria(slot.family)}>
              {shown.length === 0 && <p style={emptyNote}>{MSG.effectListEmpty(q)}</p>}
              {shown.map((a) => {
                const on = a.name === slot.name && (a.nibble << 24 | a.index) === slot.code;
                return (
                  <button
                    key={`${a.nibble}-${a.index}`}
                    role="option"
                    aria-selected={on}
                    style={algRow(on)}
                    onClick={() => pick(a)}
                    title={on ? MSG.effectListCurrent(a.name) : MSG.effectListPick(a.name)}
                  >
                    {on && <span style={algCheck} aria-hidden="true">✓</span>}
                    <span style={algName}>{a.name}</span>
                  </button>
                );
              })}
            </div>
            <p style={note}>{MSG.effectListNote}</p>
          </div>
        </div>
      </div>
    </div>
  );
}
