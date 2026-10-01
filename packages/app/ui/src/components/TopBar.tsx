/**
 * TopBar — navbar "painel da pedaleira" (referência visual do manual do
 * device): logo, cluster de conexão+boot, patch corrente, chip do DRUM
 * (abre o DrumPanel com os 87 ritmos do firmware) e Master VOL + kill.
 * Nada aqui escreve no device: os fluxos globais ainda não têm escrita via USB.
 */
import type { CSSProperties } from "react";
import { DrumPanel } from "./DrumPanel";
import type { DrumState } from "./DrumPanel";
import { MSG } from "../i18n/messages";

interface Props {
  connected: boolean;
  /** backend simulado (mock): badge de estado, não faixa de aviso */
  mock: boolean;
  /** boot em andamento (botão desabilitado + ocupado) */
  booting: boolean;
  /** dispara o script de boot (estava na seção de conexão, removida por
   *  duplicar a navbar — o boot agora mora aqui, junto do status) */
  onBoot: () => void;
  presetLabel: string;
  masterVol: number;
  drum: DrumState;
  /** aberto do drawer do drum vive no App (precedência do Esc global) */
  drumOpen: boolean;
  /** settings é GLOBAL: mora na navbar (o drawer do drum cobre o rodapé
   *  quando aberto — ⚙ no rodapé ficaria inalcançável) */
  settingsOpen: boolean;
  onOpenSettings: () => void;
  onMasterVol: (v: number) => void;
  onDrum: (d: DrumState) => void;
  onDrumOpenChange: (open: boolean) => void;
  /** ◀ ▶: preset anterior/seguinte (manual do device, ciclo P01→P99) */
  onPrevPatch: () => void;
  onNextPatch: () => void;
}
/* ⇄ mover migrou para o PALCO (EmptyBoard: controla o arrastar dos
 * slots) e o VU reativo (V-7) também — identidade visual do pedalboard;
 * Boot é o ícone ⟳ de re-escanear; kill/⚙ ficam na navbar — globais. */

const row: CSSProperties = { display: "flex", alignItems: "center", gap: "var(--space-12)" };
const cluster: CSSProperties = {
  ...row,
  background: "var(--bg-raised)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 25%, transparent)",
  borderRadius: 12,
  padding: "6px var(--space-12)",
  position: "relative",
};
const label: CSSProperties = {
  fontSize: "var(--text-xs)",
  color: "var(--text-muted)",
  fontFamily: "var(--font-mono)",
  letterSpacing: 0.8,
  textTransform: "uppercase",
};
const val: CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  fontWeight: 700,
  color: "var(--accent)",
  minWidth: 30,
  textAlign: "right",
};
const btnBase: CSSProperties = {
  background: "var(--bg-raised)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 25%, transparent)",
  color: "var(--text)",
  borderRadius: 8,
  padding: "7px 12px",
  cursor: "pointer",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  minHeight: 32,
};
const btnActive: CSSProperties = {
  background: "var(--accent)",
  border: "1px solid var(--accent-glow)",
  color: "var(--on-accent)",
  fontWeight: 700,
};
const badge: CSSProperties = {
  fontSize: "var(--text-xs)",
  fontFamily: "var(--font-mono)",
  padding: "1px 8px",
  borderRadius: 8,
  color: "var(--accent-text)",
  border: "1px solid color-mix(in srgb, var(--accent) 45%, transparent)",
  background: "color-mix(in srgb, var(--accent) 12%, transparent)",
  whiteSpace: "nowrap",
};

function Slider({
  value,
  onChange,
  ariaLabel,
  w = 90,
}: {
  value: number;
  onChange: (v: number) => void;
  ariaLabel: string;
  w?: number;
}) {
  return (
    <input
      type="range"
      min={0}
      max={99}
      value={value}
      aria-label={ariaLabel}
      onChange={(e) => onChange(Number(e.target.value))}
      style={{ width: w, accentColor: "var(--accent)", minHeight: 32 }}
    />
  );
}

export function TopBar({
  connected,
  mock,
  booting,
  onBoot,
  presetLabel,
  masterVol,
  drum,
  drumOpen,
  settingsOpen,
  onOpenSettings,
  onMasterVol,
  onDrum,
  onDrumOpenChange,
  onPrevPatch,
  onNextPatch,
}: Props) {
  // botões de ação global usam a classe glass (34px + blur) quando há mock;
  // sem mock, ficam só com a base (visual idêntico ao resto da casca)
  const glassCls = mock ? "btn-glass" : undefined;

  return (
    <header
      style={{
        display: "flex",
        alignItems: "center",
        gap: "var(--space-8)",
        flexWrap: "wrap",
        padding: "var(--space-4) 0",
      }}
      role="banner"
    >
      {/* logo: UMA linha (a tagline mora no rodapé da página) — a navbar
          não pode quebrar em 2 fileiras (defeito travado no responsivo) */}
      <div style={{ ...row, gap: "var(--space-8)" }}>
        <span
          aria-hidden="true"
          style={{
            width: 30,
            height: 30,
            borderRadius: "50%",
            border: "2px solid var(--accent)",
            display: "grid",
            placeItems: "center",
            fontFamily: "var(--font-mono)",
            fontWeight: 800,
            color: "var(--accent)",
            fontSize: 11,
          }}
        >
          {MSG.brandMono}
        </span>
        <strong style={{ fontSize: "var(--text-sm)", letterSpacing: 0.5 }}>{MSG.brand}</strong>
      </div>

      {/* conexão + boot: status, badge de backend e botão de boot num
          cluster só (a seção de conexão da página foi removida — duplicava
          esta navbar) */}
      <div style={cluster} role="status" aria-label={MSG.connClusterAria}>
        <span className={connected ? "live-dot" : "idle-dot"} aria-hidden="true" />
        <span style={label}>{connected ? MSG.connShortOn : MSG.connShortOff}</span>
        {mock && <span className="mock-badge" style={badge}>{MSG.mockBadge}</span>}
        <button
          style={{
            ...btnBase,
            padding: "4px 9px",
            fontFamily: "var(--font-mono)",
            fontSize: "var(--text-md)",
            ...(booting ? { opacity: 0.7, cursor: "progress" } : {}),
          }}
          onClick={onBoot}
          disabled={booting}
          aria-busy={booting}
          aria-label={MSG.rescanAria}
          title={MSG.rescanTitle}
        >
          ⟳
        </button>
      </div>

      {/* patch corrente (navbar) — Q2: também há display LED no board;
          ◀ ▶ reproduzem a coluna do patch do app oficial */}
      <div style={cluster}>
        <span className="nb-cap" style={label}>{MSG.patchLabel}</span>
        <button style={{ ...btnBase, padding: "4px 10px" }} onClick={onPrevPatch} aria-label={MSG.patchPrevAria} title={MSG.patchPrevTitle}>
          ◀
        </button>
        <strong style={{ fontFamily: "var(--font-mono)", color: "var(--text)", minWidth: 108, textAlign: "center" }}>{presetLabel}</strong>
        <button style={{ ...btnBase, padding: "4px 10px" }} onClick={onNextPatch} aria-label={MSG.patchNextAria} title={MSG.patchNextTitle}>
          ▶
        </button>
      </div>

      {/* DRUM — chip abre o painel de RITMOS (87 do firmware); info
          EMPILHADA (BPM sobre compasso) para a navbar caber em 1 linha */}
      <div style={cluster}>
        <button
          style={drum.on ? btnActive : btnBase}
          onClick={() => onDrumOpenChange(!drumOpen)}
          aria-expanded={drumOpen}
          aria-haspopup="dialog"
          aria-label={MSG.drumChipAria(drum.style, drum.bpm, drum.beat)}
        >
          {drum.on ? MSG.drumChipOn(drum.style) : MSG.drumChipOff(drum.style)}
        </button>
        <span
          style={{
            ...label,
            whiteSpace: "pre-line",
            lineHeight: 1.15,
            textAlign: "center",
          }}
        >
          {MSG.drumInfo(drum.bpm, drum.beat)}
        </span>
        <DrumPanel open={drumOpen} drum={drum} onChange={onDrum} onClose={() => onDrumOpenChange(false)} />
      </div>

      {/* Master VOL (prévia local) + kill (mute junto do volume, como no
          painel do hardware) + ⚙ (global, sempre alcançável) */}
      <div style={cluster}>
        <span className="nb-cap" style={label}>{MSG.masterLabel}</span>
        <Slider value={masterVol} onChange={onMasterVol} ariaLabel={MSG.masterAria} w={72} />
        <span style={val}>{masterVol}</span>
        <button className={glassCls} style={btnBase} aria-label={MSG.killAria}>
          {MSG.killLabel}
        </button>
        <button
          className={glassCls}
          style={settingsOpen ? btnActive : btnBase}
          onClick={onOpenSettings}
          aria-label={MSG.openSettingsAria}
          aria-haspopup="dialog"
        >
          ⚙
        </button>
      </div>
    </header>
  );
}
