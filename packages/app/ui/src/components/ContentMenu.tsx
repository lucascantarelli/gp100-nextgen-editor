/**
 * ContentMenu — a porta do conteúdo do dono (issues #25 e #24).
 *
 * **Por que um menu e não dois botões no rodapé.** A coluna da biblioteca tem
 * 248px e o rodapé já tem três botões (este + exportar + importar). Um quarto
 * botão estoura a linha, o rodapé vira duas fileiras, a coluna cresce por
 * `stretch` e o pedalboard desce junto — foi o que a #25 custou em baselines
 * visuais (18 PNGs renovadas). Uma porta com dois ambientes é o que mantém a
 * geometria intacta E diz a verdade: as duas telas guardam conteúdo do dono
 * no arquivo dele (o `.clo` convertido e o `.ir` do pedal).
 *
 * **É um modal, e não um popover.** Um popover ancorado no rodapé depende da
 * posição do botão e some quando o painel da biblioteca rola; o modal segue o
 * padrão do Settings/Drum/Tuner, fecha no ✕/Esc/clique-fora e é alcançável por
 * teclado sem nenhum CSS de posicionamento.
 */
import type { CSSProperties } from "react";
import { MSG } from "../i18n/messages";

const box: CSSProperties = {
  position: "fixed",
  inset: 0,
  background: "color-mix(in srgb, #000 60%, transparent)",
  display: "grid",
  placeItems: "center",
  zIndex: 50,
};
const sheet: CSSProperties = {
  background: "var(--bg)",
  color: "var(--text)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  borderRadius: 12,
  padding: "var(--space-16)",
  width: "min(420px, 92vw)",
  display: "grid",
  gap: "var(--space-12)",
};
const h2t: CSSProperties = { margin: 0, fontSize: "var(--text-lg)" };
const p: CSSProperties = { margin: 0, fontSize: "var(--text-sm)", color: "var(--text-muted)" };
const item: CSSProperties = {
  background: "transparent",
  border: "1px solid color-mix(in srgb, var(--accent) 45%, transparent)",
  color: "var(--accent-text)",
  borderRadius: 8,
  padding: "var(--space-8) var(--space-12)",
  minHeight: 32,
  cursor: "pointer",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  textAlign: "left",
};
const fecha: CSSProperties = { ...item, borderColor: "transparent", color: "var(--text-muted)" };

interface Props {
  /** Abre o gestor de tons SnapTone/NAM. */
  onTones: () => void;
  /** Abre o laboratório de IRs. */
  onIrs: () => void;
  /** Fecha este menu. */
  onClose: () => void;
}

export function ContentMenu({ onTones, onIrs, onClose }: Props) {
  return (
    <div style={box} onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div role="dialog" aria-modal="true" aria-label={MSG.contentMenuTitle} style={sheet}>
        <h2 style={h2t}>{MSG.contentMenuTitle}</h2>
        <p style={p}>{MSG.contentMenuHint}</p>
        <button style={item} onClick={onTones}>
          {MSG.toneTitle}
        </button>
        <button style={item} onClick={onIrs}>
          {MSG.irTitle}
        </button>
        <button style={fecha} onClick={onClose}>
          {MSG.contentMenuClose}
        </button>
      </div>
    </div>
  );
}
