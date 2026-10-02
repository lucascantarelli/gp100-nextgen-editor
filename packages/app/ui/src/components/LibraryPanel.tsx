/**
 * LibraryPanel — biblioteca de presets (fábrica real: presetData.ts, gerado
 * do all.prst). Busca por nome/número/tipo, lista navegável por teclado e
 * abrir = deviceSelectPreset (select real; fallback dev local). Layout segue
 * o oficial: Factory Patch | User Patch (user ainda vazio).
 */
import { useMemo, useState } from "react";
import type { CSSProperties } from "react";
import { FACTORY_PRESETS } from "../artifacts/presetData";
import { MSG } from "../i18n/messages";

interface Props {
  currentPp: number;
  onSelect: (pp: number) => void;
}

/* módulo elevado do sistema (`gp-surface`): luz e sombra pelos tokens */
const panel: CSSProperties = {
  padding: "var(--space-12)",
  display: "grid",
  gridTemplateRows: "auto auto 1fr",
  gap: "var(--space-8)",
  /* mesmo bloco do shell (300px ↔ pedalboard, align stretch): a altura
     acompanha o pedalboard SEM lacuna — a lista rola por dentro */
  minHeight: 0,
};
const tab: CSSProperties = {
  background: "transparent",
  border: "none",
  color: "var(--text-muted)",
  padding: "6px 10px",
  cursor: "pointer",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  borderRadius: 8,
  minHeight: 32,
};
const tabOn: CSSProperties = { ...tab, color: "var(--on-accent)", background: "var(--accent)", fontWeight: 700 };
const input: CSSProperties = {
  background: "var(--bg)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  color: "var(--text)",
  borderRadius: 8,
  padding: "7px 10px",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  minHeight: 32,
};
const list: CSSProperties = { overflowY: "auto", display: "grid", alignContent: "start", gap: 2 };
const rowBtn = (on: boolean): CSSProperties => ({
  display: "grid",
  gridTemplateColumns: "44px 1fr auto",
  gap: "var(--space-8)",
  alignItems: "center",
  textAlign: "left",
  padding: "6px 8px",
  borderRadius: 8,
  cursor: "pointer",
  minHeight: 32,
  background: on ? "color-mix(in srgb, var(--accent) 18%, transparent)" : "transparent",
  border: on ? "1px solid var(--accent)" : "1px solid transparent",
});
const ppCell: CSSProperties = { fontFamily: "var(--font-mono)", fontSize: "var(--text-xs)", color: "var(--text-muted)" };
const styleBadge: CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-xs)",
  color: "var(--text-muted)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  borderRadius: 6,
  padding: "1px 6px",
};

export function LibraryPanel({ currentPp, onSelect }: Props) {
  const [tabKind, setTabKind] = useState<"factory" | "user">("factory");
  const [q, setQ] = useState("");

  const filtered = useMemo(() => {
    const needle = q.trim().toLowerCase();
    if (!needle) return FACTORY_PRESETS;
    return FACTORY_PRESETS.filter(
      (p) =>
        p.name.toLowerCase().includes(needle) ||
        p.ppTypeName.toLowerCase().includes(needle) ||
        String(p.pp) === needle ||
        String(p.pp + 1).padStart(2, "0") === needle, // display é 1-based (P01..P99)
    );
  }, [q]);

  return (
    <aside className="gp-surface" style={panel} aria-label={MSG.libAria}>
      <div role="tablist" aria-label={MSG.libTabsAria} style={{ display: "flex", gap: "var(--space-4)" }}>
        <button role="tab" aria-selected={tabKind === "factory"} style={tabKind === "factory" ? tabOn : tab} onClick={() => setTabKind("factory")}>
          {MSG.libTabFactory}
        </button>
        <button role="tab" aria-selected={tabKind === "user"} style={tabKind === "user" ? tabOn : tab} onClick={() => setTabKind("user")} disabled>
          {MSG.libTabUser}
        </button>
      </div>

      <input
        style={input}
        value={q}
        onChange={(e) => setQ(e.target.value)}
        placeholder={MSG.searchPlaceholder}
        aria-label={MSG.searchAria}
      />

      {tabKind === "factory" ? (
        <div style={list} role="listbox" aria-label={`${MSG.libListAria} (${FACTORY_PRESETS.length})`}>
          {filtered.length === 0 && (
            <p style={{ color: "var(--text-muted)", fontSize: "var(--text-sm)", padding: "var(--space-8)" }}>
              {MSG.searchEmpty(q)}
            </p>
          )}
          {filtered.map((p) => {
            const on = p.pp === currentPp;
            return (
              <button
                key={p.pp}
                role="option"
                aria-selected={on}
                style={rowBtn(on)}
                onClick={() => onSelect(p.pp)}
                title={MSG.libRowTitle(p.name, p.ppTypeName)}
              >
                {/* oficial exibe 1-based (P25 Mist = ppID 24 do all.prst) */}
                <span style={ppCell}>{MSG.libPp(p.pp)}</span>
                <span style={{ fontSize: "var(--text-sm)", fontWeight: on ? 700 : 400 }}>{p.name}</span>
                <span style={styleBadge}>{p.ppTypeName}</span>
              </button>
            );
          })}
        </div>
      ) : (
        <p style={{ color: "var(--text-muted)", fontSize: "var(--text-sm)", padding: "var(--space-8)" }}>
          {MSG.userPatchEmpty}
        </p>
      )}
    </aside>
  );
}
