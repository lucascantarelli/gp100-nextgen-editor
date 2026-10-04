/**
 * BootProgressBar — a faixa de progresso do boot, colada no topo da mesa.
 *
 * Tirada do `App.tsx` pela #82: era um bloco de ~10 linhas de JSX com
 * `style` inline de três níveis, dentro de um componente que já tem outros
 * nove blocos. Componente de apresentação puro — recebe o texto do estágio já
 * resolvido e não sabe de boot, device ou nada do domain.
 *
 * A barra é **transitória**: `App` só a monta enquanto `booting` é verdade,
 * então ela nunca ocupa layout permanente nem "congela" em 100% depois do fim.
 */
import { MSG } from "../i18n/messages";

interface BootProgressBarProps {
  /** 0–100, ou `null` quando ainda não há número para mostrar. */
  progress: number;
  /** Etapa já traduzida (o App resolve via `BOOT_STAGE_LABEL`). */
  stageLabel: string;
}

const track: React.CSSProperties = {
  position: "relative",
  height: 18,
  borderRadius: 9,
  overflow: "hidden",
  background: "color-mix(in srgb, var(--text-muted) 18%, transparent)",
};

const fill: React.CSSProperties = {
  position: "absolute",
  inset: 0,
  background:
    "linear-gradient(90deg, color-mix(in srgb, var(--accent) 65%, transparent), var(--accent))",
  transition: "width var(--motion-fast) var(--ease-out)",
};

const readout: React.CSSProperties = {
  position: "absolute",
  inset: 0,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  fontSize: "var(--text-xs)",
  color: "var(--text)",
  textShadow: "0 1px 2px rgba(0,0,0,.6)",
};

export function BootProgressBar({ progress, stageLabel }: BootProgressBarProps) {
  return (
    <div
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={progress}
      aria-label={MSG.bootProgressAria}
      style={track}
    >
      <div style={{ ...fill, width: `${progress}%` }} />
      <span style={readout}>
        {stageLabel} · {progress}%
      </span>
    </div>
  );
}
