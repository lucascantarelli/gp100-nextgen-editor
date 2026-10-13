/**
 * ContentMenu — a porta do conteúdo (issues #25, #24, #114 e #115).
 *
 * **Por que um menu e não mais botões no rodapé.** A coluna da biblioteca tem
 * 248px e o rodapé já tem três botões (este + exportar + importar). Um quarto
 * botão estoura a linha, o rodapé vira duas fileiras, a coluna cresce por
 * `stretch` e o pedalboard desce junto — foi o que a #25 custou em baselines
 * visuais (18 PNGs renovadas). Uma porta com os ambientes é o que mantém a
 * geometria intacta E diz a verdade: três telas levam conteúdo do pedal para um
 * ARQUIVO (o `.clo` convertido, o `.ir` do pedal, e o preset em JSON/folha) e a
 * quarta — o assistente de gain staging (#115) — não leva arquivo nenhum: ela
 * LÊ a cadeia e devolve um relatório. A quinta — o A/B com blind (#116) —
 * também não: compara duas versões do MESMO patch e troca uma pela outra. O
 * rótulo do item é o TÍTULO da tela que ele abre, e é isso que o dono lê nos
 * dois lugares.
 *
 * A navbar não é alternativa: ela tem ~11px de folga em 1280 e qualquer botão
 * novo a estoura (o achado da #25). O modal é o único lugar que cresce sem
 * mexer em geometria de repouso.
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
  /** Abre o preset em arquivo (JSON + folha de timbre). */
  onPreset: () => void;
  /** Abre o assistente de gain staging (#115). */
  onGain: () => void;
  /** Abre o A/B com blind test entre versões do patch (#116). */
  onAb: () => void;
  /** Fecha este menu. */
  onClose: () => void;
}

export function ContentMenu({ onTones, onIrs, onPreset, onGain, onAb, onClose }: Props) {
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
        <button style={item} onClick={onPreset}>
          {MSG.presetFileTitle}
        </button>
        <button style={item} onClick={onGain}>
          {MSG.gainTitle}
        </button>
        <button style={item} onClick={onAb}>
          {MSG.abTitle}
        </button>
        <button style={fecha} onClick={onClose}>
          {MSG.contentMenuClose}
        </button>
      </div>
    </div>
  );
}
