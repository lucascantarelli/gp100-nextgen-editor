/**
 * TopBar — navbar "painel da pedaleira" (referência visual do manual do
 * device): logo, cluster de conexão+boot, patch corrente, chip do DRUM
 * (play/stop e BPM DIRETO na navbar — sem abrir o painel; o chip abre o
 * DrumPanel com os 87 ritmos do firmware) e Master VOL + kill.
 *
 * Refino da navbar (issue #10):
 *  - alturas UNIFORMES: todo botão daqui tem 32px (`nbBtn`) → clusters iguais;
 *  - larguras RESPONSIVAS: sem minWidth rígido; textos encolhem com
 *    reticências (minWidth 0) em vez de quebrar a linha do banner;
 *  - cores por FUNÇÃO: perigo=kill, ok=play/stop, acento=boot/patch —
 *    relevo e gradiente continuam vindo do design system (.gp-btn);
 *  - hierarquia do chip: toggle (estado) · chip (identidade/abre o modal)
 *    · stepper de BPM (numérico tabular).
 * Nada aqui escreve no device: os fluxos globais ainda não têm escrita via USB.
 */
import { useEffect, useRef } from "react";
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
  /** aberto do MODAL do drum vive no App (precedência do Esc global) */
  drumOpen: boolean;
  /** settings é GLOBAL: mora na navbar (os modais do corpo cobrem a tela
   *  quando abertos — ⚙ no rodapé ficaria inalcançável) */
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
/* clusters da navbar: módulos do sistema (`gp-surface--flat` + reflexo
   especular + `.nb-cluster`, que carrega ritmo/padding em CSS — o media
   query ≤1024 aperta o gap SEM bater em estilo inline) */
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
  /* números tabulares: o valor não "treme" ao arrastar o slider */
  fontVariantNumeric: "tabular-nums",
};
/* nome do patch: encolhe com reticências quando a navbar aperta (nunca
   empurra a linha para 2 fileiras — expectShellAligned exige <70px) */
const patchName: CSSProperties = {
  fontFamily: "var(--font-mono)",
  color: "var(--text)",
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
  minWidth: 0,
  flex: "1 1 auto",
  textAlign: "center",
};
/* separador vertical: agrupa sem borda pesada (status|boot, master|kill) */
const divider: CSSProperties = { width: 1, height: 20, background: "var(--border)", flex: "0 0 auto" };

/* TODOS os botões da navbar com a MESMA caixa (32px, border-box) →
   clusters com altura idêntica; some com o 34px do .btn-glass */
const nbBtn: CSSProperties = { height: 32, boxSizing: "border-box" };

/* cores por FUNÇÃO (issue #10) — overrides inline por cima do .gp-btn:
   o gradiente/relevo de base é do design system; aqui só identidade */
const tone = {
  /* acento anodizado: boot ⟳ e passos ◀ ▶ (repouso; hover vive no CSS
     .nb-accent — estilo inline nunca perderia para :hover) */
  accent: {
    color: "var(--accent-text)",
    border: "1px solid color-mix(in srgb, var(--accent) 55%, var(--border))",
  },
  /* estado ligado (⚙ com settings aberto): acento com brilho */
  accentOn: {
    color: "var(--accent-text)",
    border: "1px solid color-mix(in srgb, var(--accent) 70%, var(--border))",
    background:
      "linear-gradient(180deg, color-mix(in srgb, var(--accent) 30%, var(--bg-raised)) 0%, color-mix(in srgb, var(--accent) 14%, var(--bg-raised)) 100%)",
    boxShadow: "inset 0 1px 0 var(--light-top), 0 2px 0 var(--shade-deep), 0 0 12px color-mix(in srgb, var(--accent-glow) 40%, transparent)",
  },
  /* energia/ok: play/stop do drum */
  ok: {
    color: "var(--ok)",
    border: "1px solid color-mix(in srgb, var(--ok) 50%, var(--border))",
  },
  okOn: {
    color: "var(--text)",
    border: "1px solid color-mix(in srgb, var(--ok) 65%, var(--border))",
    background:
      "linear-gradient(180deg, color-mix(in srgb, var(--ok) 26%, var(--bg-raised)) 0%, color-mix(in srgb, var(--ok) 12%, var(--bg-raised)) 100%)",
    boxShadow: "inset 0 1px 0 var(--light-top), 0 2px 0 var(--shade-deep), 0 0 12px color-mix(in srgb, var(--ok) 40%, transparent)",
  },
  /* perigo: kill (repouso) e killed (solto, vermelho sólido + brilho) */
  danger: {
    color: "var(--error)",
    border: "1px solid color-mix(in srgb, var(--error) 50%, var(--border))",
  },
  dangerOn: {
    color: "var(--text)",
    border: "1px solid color-mix(in srgb, var(--error) 70%, var(--border))",
    background:
      "linear-gradient(180deg, color-mix(in srgb, var(--error) 38%, var(--bg-raised)) 0%, color-mix(in srgb, var(--error) 20%, var(--bg-raised)) 100%)",
    boxShadow: "inset 0 1px 0 var(--light-top), 0 2px 0 var(--shade-deep), 0 0 12px color-mix(in srgb, var(--error) 45%, transparent)",
  },
} satisfies Record<string, CSSProperties>;

/* botões da navbar usam o botão do SISTEMA (`gp-btn`) com a geometria
   própria de cada um; `btn-glass` é o modificador translúcido do mock */
const btn = (on = false): string => `gp-btn${on ? " gp-btn--on" : ""}`;
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
/* stepper de BPM da navbar: − valor + (tabular; clamp 40–240 igual ao modal).
   As medidas vivem no CSS (`.nb-step`/`.nb-bpm`) para o media query ≤1024
   poder encolher o bloco e devolver largura ao chip do ritmo. */
const stepper: CSSProperties = { display: "inline-flex", alignItems: "center", gap: "var(--space-4)" };
const bpmVal: CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  fontWeight: 700,
  color: "var(--text)",
  textAlign: "center",
  fontVariantNumeric: "tabular-nums",
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
      /* minWidth 40: o slider encolhe junto com a navbar em ≤1024; margin 0
         mata a margem de 2px do UA (sem ela o cluster do master fica 4px
         mais alto que os outros — alturas precisam ser IDENTICAS) */
      style={{ width: w, minWidth: 40, margin: 0, boxSizing: "border-box", accentColor: "var(--accent)", minHeight: 32 }}
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
  // botões de ação global usam a classe glass (32px + blur) quando há mock;
  // sem mock, ficam só com a base (visual idêntico ao resto da casca)
  const glassCls = mock ? "btn-glass" : undefined;

  // KILL (mute global, como o footswitch do hardware): silencia o master e
  // para o drum. Guarda o último master audível para o toggle ser reversível
  // — clicar de novo restaura o volume anterior em vez de virar botão morto.
  const lastMaster = useRef(masterVol > 0 ? masterVol : 99);
  useEffect(() => {
    if (masterVol > 0) lastMaster.current = masterVol;
  }, [masterVol]);
  const killed = masterVol === 0 && !drum.on;
  const onKill = () => {
    if (killed) onMasterVol(lastMaster.current);
    else {
      onMasterVol(0);
      onDrum({ ...drum, on: false });
    }
  };

  return (
    <header
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: "var(--space-8)",
        flexWrap: "wrap",
        padding: "var(--space-4) 0",
      }}
      role="banner"
    >
      {/* LOGO ancorada à esquerda */}
      <div style={{ ...row, gap: "var(--space-8)" }}>
        <span
          aria-hidden="true"
          style={{
            width: 32,
            height: 32,
            borderRadius: "50%",
            background: `linear-gradient(135deg, var(--accent) 0%, var(--accent-glow) 100%)`,
            display: "grid",
            placeItems: "center",
            fontFamily: "var(--font-mono)",
            fontWeight: 800,
            color: "var(--on-accent)",
            fontSize: 11,
            boxShadow: `0 0 12px color-mix(in srgb, var(--accent-glow) 50%, transparent), inset 0 1px 0 rgba(255,255,255,0.2)`,
          }}
        >
          {MSG.brandMono}
        </span>
        <strong style={{ fontSize: "var(--text-md)", letterSpacing: 0.5, color: "var(--text)" }}>{MSG.brand}</strong>
      </div>

      {/* CONT roles à direita — `.nb-controls` (nowrap + min-width 0): os
          clusters ENCOLHEM com ellipsis em vez de quebrar o banner */}
      <div className="nb-controls">

      {/* conexão + boot: status, badge de backend e botão de boot num
          cluster só (a seção de conexão da página foi removida — duplicava
          esta navbar); separador + ⟳ acento dão o ar de painel */}
      <div className="gp-surface gp-surface--flat gp-specular nb-cluster" role="status" aria-label={MSG.connClusterAria}>
        <span className={connected ? "live-dot" : "idle-dot"} aria-hidden="true" />
        <span style={label}>{connected ? MSG.connShortOn : MSG.connShortOff}</span>
        {mock && <span className="mock-badge nb-push" style={badge}>{MSG.mockBadge}</span>}
        <span className="nb-div" style={divider} aria-hidden="true" />
        <button
          className="gp-btn nb-icon-btn"
          style={{ ...nbBtn, fontFamily: "var(--font-mono)", fontSize: "var(--text-md)" }}
          onClick={onBoot}
          disabled={booting}
          aria-busy={booting}
          aria-label={MSG.rescanAria}
          title={MSG.rescanTitle}
        >
          <span className="nb-spin" aria-hidden="true">
            ⟳
          </span>
        </button>
      </div>

      {/* patch corrente (navbar) — Q2: também há display LED no board;
          ◀ ▶ acento com hover refinado (`.nb-accent`) e nome que encolhe */}
      <div className="gp-surface gp-surface--flat gp-specular nb-cluster">
        <span className="nb-cap" style={label}>{MSG.patchLabel}</span>
        <button className="gp-btn nb-accent" style={{ ...nbBtn, padding: "4px 10px" }} onClick={onPrevPatch} aria-label={MSG.patchPrevAria} title={MSG.patchPrevTitle}>
          ◀
        </button>
        <strong style={patchName}>{presetLabel}</strong>
        <button className="gp-btn nb-accent" style={{ ...nbBtn, padding: "4px 10px" }} onClick={onNextPatch} aria-label={MSG.patchNextAria} title={MSG.patchNextTitle}>
          ▶
        </button>
      </div>

      {/* DRUM — play/stop e BPM DIRETO na navbar (UX da issue #10: não
          exige o painel aberto); chip = identidade, abre o modal de gestão */}
      <div className="gp-surface gp-surface--flat gp-specular nb-cluster">
        <button
          className={btn(drum.on)}
          style={{ ...nbBtn, ...(drum.on ? tone.okOn : tone.ok), padding: "4px 10px" }}
          onClick={() => onDrum({ ...drum, on: !drum.on })}
          aria-pressed={drum.on}
          aria-label={MSG.drumToggleAria}
        >
          {drum.on ? "⏹" : "⏵"}
        </button>
        <button
          className={btn(drumOpen)}
          style={{ ...nbBtn, flex: "1 1 auto", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis", minWidth: 0, padding: "4px 10px" }}
          onClick={() => onDrumOpenChange(!drumOpen)}
          aria-expanded={drumOpen}
          aria-haspopup="dialog"
          aria-label={MSG.drumChipAria(drum.style, drum.bpm, drum.beat)}
        >
          <span className="nb-chip-pre">{MSG.drumChipPre}</span>
          {drum.style}
        </button>
        <span style={stepper}>
          <button
            className={`${btn()} nb-step`}
            style={nbBtn}
            aria-label={MSG.drumBpmDownAria}
            onClick={() => onDrum({ ...drum, bpm: Math.max(40, drum.bpm - 1) })}
          >
            −
          </button>
          <span className="nb-bpm" style={bpmVal}>
            {drum.bpm}
          </span>
          <button
            className={`${btn()} nb-step`}
            style={nbBtn}
            aria-label={MSG.drumBpmUpAria}
            onClick={() => onDrum({ ...drum, bpm: Math.min(240, drum.bpm + 1) })}
          >
            +
          </button>
        </span>
        <DrumPanel open={drumOpen} drum={drum} onChange={onDrum} onClose={() => onDrumOpenChange(false)} />
      </div>

      {/* Master VOL (prévia local) + separador + kill (mute junto do
          volume, como no painel do hardware) + ⚙ (global, sempre
          alcançável) */}
      <div className="gp-surface gp-surface--flat gp-specular nb-cluster">
        <span className="nb-cap" style={label}>{MSG.masterLabel}</span>
        <Slider value={masterVol} onChange={onMasterVol} ariaLabel={MSG.masterAria} w={68} />
        <span style={val}>{masterVol}</span>
        <span className="nb-div nb-push" style={divider} aria-hidden="true" />
        <button
          className={[btn(killed), glassCls].filter(Boolean).join(" ")}
          style={{
            ...nbBtn,
            ...(killed ? tone.dangerOn : tone.danger),
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
            gap: 4,
          }}
          onClick={onKill}
          aria-pressed={killed}
          aria-label={MSG.killAria}
          title={MSG.killTitle}
        >
          {killed ? MSG.killLabelOn : MSG.killLabel}
        </button>
        <button
          className={[btn(settingsOpen), glassCls].filter(Boolean).join(" ")}
          style={{ ...nbBtn, ...(settingsOpen ? tone.accentOn : {}) }}
          onClick={onOpenSettings}
          aria-label={MSG.openSettingsAria}
          aria-haspopup="dialog"
        >
          ⚙
        </button>
      </div>
      </div>
    </header>
  );
}
