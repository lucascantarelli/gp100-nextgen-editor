/**
 * SettingsModal — modal de configurações com as 6 abas do app oficial.
 * General persiste LOCAL (localStorage) com badge "prévia local": a escrita
 * global no device entra quando o canal de configuração via USB existir
 * (política de hardware — nunca escrever sem canal verificado). Esc fecha,
 * foco começa no diálogo (WCAG), alvos ≥32px.
 */
import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { MSG, LANGS, LANG_NAMES } from "../i18n/messages";

export interface GeneralSettings {
  inputLevel: number;
  usbAudio: boolean;
  normalLevel: number;
  hintMode: "left" | "right";
  /** Tooltip dos knobs com addr/code/ctrl do SET (U-3 — checklist §3.2). */
  engineerMode: boolean;
  tapTempo: { pre: boolean; mod: boolean; dly: boolean };
  language: "pt-BR" | "en" | "es" | "zh";
}

const DEFAULTS: GeneralSettings = {
  inputLevel: 100,
  usbAudio: false,
  normalLevel: 100,
  hintMode: "left",
  engineerMode: false,
  tapTempo: { pre: true, mod: false, dly: false },
  language: "pt-BR",
};

const KEY = "gp100.settings.general.v1";

export function loadGeneral(): GeneralSettings {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) return { ...DEFAULTS, ...(JSON.parse(raw) as Partial<GeneralSettings>) };
  } catch {
    /* localStorage indisponível (teste) — defaults */
  }
  return DEFAULTS;
}

type Tab = "General" | "Global EQ" | "About" | "Info Frame" | "Help" | "Release Note";
const TABS: Tab[] = ["General", "Global EQ", "About", "Info Frame", "Help", "Release Note"];

interface Props {
  open: boolean;
  general: GeneralSettings;
  onChangeGeneral: (s: GeneralSettings) => void;
  onClose: () => void;
}

const overlay: CSSProperties = {
  position: "fixed",
  inset: 0,
  background: "rgba(0,0,0,.55)",
  display: "grid",
  placeItems: "center",
  zIndex: 50,
};
/* modal = superfície FLUTUANTE do sistema (gp-surface--floating): a luz e a
   sombra vêm dos tokens; aqui só geometria/estrutura */
const dialog: CSSProperties = {
  width: "min(640px, 92vw)",
  maxHeight: "86vh",
  overflow: "hidden",
  display: "grid",
  gridTemplateRows: "auto auto 1fr",
};
const tabRow: CSSProperties = { display: "flex", gap: "var(--space-4)", padding: "var(--space-12) var(--space-12) 0", flexWrap: "wrap" };
const tabStyle = (on: boolean): CSSProperties => ({
  background: "transparent",
  border: "1px solid transparent",
  borderBottom: on ? "2px solid var(--accent)" : "2px solid transparent",
  color: on ? "var(--accent)" : "var(--text-muted)",
  padding: "8px 10px",
  cursor: "pointer",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  borderRadius: "8px 8px 0 0",
  minHeight: 32,
});
const body: CSSProperties = { overflowY: "auto", padding: "var(--space-12) var(--space-20) var(--space-20)", display: "grid", gap: "var(--space-12)", alignContent: "start" };
const line: CSSProperties = { display: "flex", alignItems: "center", justifyContent: "space-between", gap: "var(--space-12)", minHeight: 32 };
const lbl: CSSProperties = { fontSize: "var(--text-sm)", color: "var(--text)" };
const badge: CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-xs)",
  color: "var(--warn)",
  border: "1px solid color-mix(in srgb, var(--warn) 50%, transparent)",
  borderRadius: 6,
  padding: "2px 8px",
};
const input: CSSProperties = {
  background: "var(--bg)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  color: "var(--text)",
  borderRadius: 8,
  padding: "6px 10px",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  minHeight: 32,
};
const p: CSSProperties = { fontSize: "var(--text-sm)", color: "var(--text-muted)", lineHeight: 1.6 };
const kbd: CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-xs)",
  color: "var(--text)",
  background: "var(--bg)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 40%, transparent)",
  borderBottomWidth: 2,
  borderRadius: 6,
  padding: "2px 8px",
  whiteSpace: "nowrap",
};
const helpTable: CSSProperties = {
  width: "100%",
  borderCollapse: "collapse",
  fontSize: "var(--text-sm)",
  color: "var(--text)",
};
const helpTd: CSSProperties = {
  padding: "6px 8px",
  borderBottom: "1px solid color-mix(in srgb, var(--text-muted) 18%, transparent)",
  verticalAlign: "top",
};

function Toggle({ on, onChange, label }: { on: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <button
      role="switch"
      aria-checked={on}
      aria-label={label}
      onClick={() => onChange(!on)}
      style={{
        width: 44,
        height: 26,
        borderRadius: 13,
        border: "1px solid color-mix(in srgb, var(--text-muted) 40%, transparent)",
        background: on ? "var(--ok)" : "var(--bg)",
        position: "relative",
        cursor: "pointer",
        minHeight: 32,
      }}
    >
      <span
        aria-hidden="true"
        style={{
          position: "absolute",
          top: 3,
          left: on ? 21 : 3,
          width: 18,
          height: 18,
          borderRadius: "50%",
          background: "#fff",
          transition: "left var(--motion-fast) var(--ease-out)",
        }}
      />
    </button>
  );
}

export function SettingsModal({ open, general, onChangeGeneral, onClose }: Props) {
  const [tab, setTab] = useState<Tab>("General");
  const [globalEqPage, setGlobalEqPage] = useState(1);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (open) setTab("General");
  }, [open]);

  if (!open) return null;
  const set = (patch: Partial<GeneralSettings>) => onChangeGeneral({ ...general, ...patch });

  return (
    <div style={overlay} onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-label={MSG.settingsDialogAria}
        tabIndex={-1}
        className="gp-surface gp-surface--floating"
        style={dialog}
        onKeyDown={(e) => e.key === "Escape" && onClose()}
      >
        <div style={{ display: "flex", alignItems: "center", padding: "var(--space-12) var(--space-12) 0", gap: "var(--space-8)" }}>
          <strong style={{ fontSize: "var(--text-md)" }}>{MSG.settingsTitle}</strong>
          <span style={badge}>{MSG.previewBadge}</span>
          <button style={{ ...input, marginLeft: "auto", cursor: "pointer" }} onClick={onClose} aria-label={MSG.settingsCloseAria}>
            ✕
          </button>
        </div>

        <div role="tablist" aria-label={MSG.settingsTabsAria} style={tabRow}>
          {TABS.map((t) => (
            <button key={t} role="tab" aria-selected={tab === t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>
              {MSG.settingsTabs[t]}
            </button>
          ))}
        </div>

        <div style={body}>
          {tab === "General" && (
            <>
              <div style={line}>
                <span style={lbl}>{MSG.inputLevelLabel}</span>
                <span style={{ display: "flex", alignItems: "center", gap: "var(--space-8)" }}>
                  <input type="range" min={0} max={100} value={general.inputLevel} aria-label={MSG.inputLevelAria} onChange={(e) => set({ inputLevel: Number(e.target.value) })} style={{ width: 160, accentColor: "var(--accent)", minHeight: 32 }} />
                  <span style={{ fontFamily: "var(--font-mono)", minWidth: 34, textAlign: "right" }}>{general.inputLevel}</span>
                </span>
              </div>
              <div style={line}>
                <span style={lbl}>{MSG.usbAudioLabel}</span>
                <Toggle on={general.usbAudio} onChange={(v) => set({ usbAudio: v })} label={MSG.usbAudioAria} />
              </div>
              <div style={line}>
                <span style={lbl}>{MSG.normalLevelLabel}</span>
                <span style={{ display: "flex", alignItems: "center", gap: "var(--space-8)" }}>
                  <input type="range" min={0} max={100} value={general.normalLevel} aria-label={MSG.normalLevelAria} onChange={(e) => set({ normalLevel: Number(e.target.value) })} style={{ width: 160, accentColor: "var(--accent)", minHeight: 32 }} />
                  <span style={{ fontFamily: "var(--font-mono)", minWidth: 34, textAlign: "right" }}>{general.normalLevel}</span>
                </span>
              </div>
              <div style={line}>
                <span style={lbl}>{MSG.hintModeLabel}</span>
                <select aria-label={MSG.hintModeAria} value={general.hintMode} onChange={(e) => set({ hintMode: e.target.value as GeneralSettings["hintMode"] })} style={input}>
                  <option value="left">{MSG.hintLeft}</option>
                  <option value="right">{MSG.hintRight}</option>
                </select>
              </div>
              <div style={line}>
                <span style={lbl}>
                  {MSG.engineerModeLabel}
                  <span
                    style={{ display: "block", fontSize: "var(--text-xs)", color: "var(--text-muted)" }}
                  >
                    {MSG.engineerModeSub}
                  </span>
                </span>
                <Toggle
                  on={general.engineerMode}
                  onChange={(v) => set({ engineerMode: v })}
                  label={MSG.engineerModeAria}
                />
              </div>
              <div style={line}>
                <span style={lbl}>{MSG.tapTempoLabel}</span>
                <span style={{ display: "flex", gap: "var(--space-12)" }}>
                  {(["pre", "mod", "dly"] as const).map((k) => (
                    <label key={k} style={{ ...lbl, display: "flex", gap: 6, alignItems: "center", cursor: "pointer" }}>
                      <input type="checkbox" checked={general.tapTempo[k]} onChange={(e) => set({ tapTempo: { ...general.tapTempo, [k]: e.target.checked } })} style={{ accentColor: "var(--accent)", width: 18, height: 18 }} />
                      {MSG.tapTempoLabels[k]}
                    </label>
                  ))}
                </span>
              </div>
              <div style={line}>
                <span style={lbl}>
                  {MSG.footswitchLabel}
                  <span style={{ display: "block", fontSize: "var(--text-xs)", color: "var(--text-muted)" }}>
                    {MSG.footswitchSub}
                  </span>
                </span>
                <select aria-label={MSG.footswitchAria} disabled style={{ ...input, opacity: 0.55 }}>
                  <option>{MSG.footswitchSoon}</option>
                </select>
              </div>
              <div style={line}>
                <span style={lbl}>{MSG.languageLabel}</span>
                {/* O ÚNICO select sem canal USB que funciona: é software
                    local puro. Trocar aqui chama `set()` → o App aplica via
                    `setLanguage` e toda a casca redesenha. */}
                <select
                  aria-label={MSG.languageAria}
                  value={general.language}
                  onChange={(e) => set({ language: e.target.value as GeneralSettings["language"] })}
                  style={input}
                >
                  {LANGS.map((l) => (
                    <option key={l} value={l}>
                      {LANG_NAMES[l]}
                    </option>
                  ))}
                </select>
              </div>
            </>
          )}

          {tab === "Global EQ" && (
            <>
              <p style={p}>{MSG.globalEqIntro}</p>
              <div style={{ display: "flex", gap: "var(--space-8)" }} role="group" aria-label={MSG.eqBandGroupAria}>
                {[1, 2, 3, 4, 5].map((n) => (
                  <button key={n} style={tabStyle(globalEqPage === n)} onClick={() => setGlobalEqPage(n)} aria-pressed={globalEqPage === n}>
                    {MSG.eqBandBtn(n)}
                  </button>
                ))}
              </div>
              {([
                [MSG.eqFreqLabel(globalEqPage), MSG.eqFreqAria(globalEqPage)],
                [MSG.eqQLabel(globalEqPage), MSG.eqQAria(globalEqPage)],
                [MSG.eqGainLabel(globalEqPage), MSG.eqGainAria(globalEqPage)],
              ] as const).map(([labelText, aria]) => (
                <div key={labelText} style={line}>
                  <span style={lbl}>{labelText}</span>
                  <input type="range" min={0} max={100} defaultValue={50} aria-label={aria} style={{ width: 200, accentColor: "var(--accent)", minHeight: 32 }} disabled />
                </div>
              ))}
              <div style={line}>
                <span style={lbl}>{MSG.lcutLabel}</span>
                <input type="range" min={0} max={100} defaultValue={0} aria-label={MSG.lcutAria} style={{ width: 200, accentColor: "var(--accent)", minHeight: 32 }} disabled />
              </div>
              <div style={line}>
                <span style={lbl}>{MSG.hcutLabel}</span>
                <input type="range" min={0} max={100} defaultValue={100} aria-label={MSG.hcutAria} style={{ width: 200, accentColor: "var(--accent)", minHeight: 32 }} disabled />
              </div>
            </>
          )}

          {tab === "About" && (
            <>
              <p style={p}>{MSG.aboutIntro}</p>
              <table style={helpTable} aria-label={MSG.aboutDataAria}>
                <tbody>
                  {MSG.aboutData.map(([k, v]) => (
                    <tr key={k}>
                      <td style={helpTd}>{k}</td>
                      <td style={helpTd}>{v}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </>
          )}

          {tab === "Info Frame" && (
            <>
              <p style={p}>{MSG.infoFrameIntro}</p>
              <table style={helpTable} aria-label={MSG.infoFrameAria}>
                <tbody>
                  {MSG.infoFrame.map(([k, v]) => (
                    <tr key={k}>
                      <td style={helpTd}>{k}</td>
                      <td style={helpTd}>{v}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </>
          )}

          {tab === "Help" && (
            <>
              <p style={p}>{MSG.helpShortcutsIntro}</p>
              <table style={helpTable} aria-label={MSG.helpTableAria}>
                <tbody>
                  <tr>
                    <td style={helpTd}>
                      <kbd style={kbd}>{MSG.helpKeySpace}</kbd>
                    </td>
                    <td style={helpTd}>{MSG.helpShortcutSpace}</td>
                  </tr>
                  <tr>
                    <td style={helpTd}>
                      <kbd style={kbd}>{MSG.helpKeyR}</kbd>
                    </td>
                    <td style={helpTd}>{MSG.helpShortcutR}</td>
                  </tr>
                  <tr>
                    <td style={helpTd}>
                      <kbd style={kbd}>{MSG.helpKeyEsc}</kbd>
                    </td>
                    <td style={helpTd}>{MSG.helpShortcutEsc}</td>
                  </tr>
                </tbody>
              </table>
              <p style={p}>{MSG.helpNotes}</p>
              <p style={p}>{MSG.helpControlsIntro}</p>
              <ul style={{ ...p, paddingLeft: "var(--space-20)", margin: 0 }}>
                {MSG.helpControls.map((c) => (
                  <li key={c}>{c}</li>
                ))}
              </ul>

              {/* Informações do sistema (V-7): versões reais do ambiente */}
              <p style={{ ...p, fontWeight: 700, color: "var(--text)" }}>{MSG.sysInfoTitle}</p>
              <p style={p}>{MSG.sysInfoHint}</p>
              <table style={helpTable} aria-label={MSG.sysInfoAria}>
                <tbody>
                  {MSG.sysInfo.map(([k, v]) => (
                    <tr key={k}>
                      <td style={{ ...helpTd, whiteSpace: "nowrap" }}>{k}</td>
                      <td style={{ ...helpTd, wordBreak: "break-word" }}>{v}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
              <p style={p}>{MSG.sysNote}</p>
            </>
          )}

          {tab === "Release Note" && <p style={p}>{MSG.releaseNote}</p>}
        </div>
      </div>
    </div>
  );
}
