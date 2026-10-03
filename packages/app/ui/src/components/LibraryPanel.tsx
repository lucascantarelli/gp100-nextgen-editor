/**
 * LibraryPanel — biblioteca de patches. Fábrica: os 99 REAIS do `all.prst`
 * (presetData.ts, GERADO). Usuário: patches que o dono salvou da cadeia
 * corrente (userPatches.ts, PRÉVIA LOCAL — a escrita no device ainda não
 * tem canal).
 *
 * Abrir um patch é o QUE FAZ O PALCO MUDAR: `onSelect`/`onOpenUser` devolvem
 * ao App, que relê o board e o Stage redesenha os 9 pedais com a cadeia
 * verdadeira daquele patch (artefato presetChains.ts no mock web, o
 * `device_board` do core no shell real).
 *
 * Largura: 248px (era 300px na #11) — o pedalboard precisa do espaço; as
 * linhas encolhem por ellipsis e a lista rola por dentro.
 */
import { useEffect, useMemo, useState } from "react";
import type { CSSProperties } from "react";
import { FACTORY_PRESETS } from "../artifacts/presetData";
import { MSG } from "../i18n/messages";
import type { UserPatch } from "../userPatches";

interface Props {
  /** preset de fábrica corrente (o chip/navbar navegam por aqui) */
  currentPp: number;
  /** banco aberto no palco — decide qual aba mostra o selecionado */
  bank: "factory" | "user";
  /** patches do dono (prévia local) */
  userPatches: UserPatch[];
  /** id do patch de usuário ABERTO no palco (null = nenhum) */
  currentUserId: string | null;
  onSelect: (pp: number) => void;
  onOpenUser: (patch: UserPatch, index: number) => void;
  /** salva o patch CORRENTE (qualquer banco) como patch de usuário */
  onSave: (name: string) => void;
  onDelete: (id: string) => void;
}

/* módulo elevado do sistema (`gp-surface`): luz e sombra pelos tokens */
const panel: CSSProperties = {
  padding: "var(--space-12)",
  display: "grid",
  gridTemplateRows: "auto auto 1fr auto",
  gap: "var(--space-8)",
  /* mesmo bloco do shell (248px ↔ pedalboard, align stretch): a altura
     acompanha o pedalboard SEM lacuna — a lista rola por dentro */
  minHeight: 0,
  minWidth: 0,
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
  minWidth: 0,
};
const list: CSSProperties = { overflowY: "auto", display: "grid", alignContent: "start", gap: 2, minWidth: 0 };
/* patches de usuário: a lista é um `list` (cada linha tem AÇÃO própria — abrir e
   excluir), não um listbox — o option embaixo do botão excluir ficaria órfão
   para o leitor de tela (filhos de option são presentacionais) */
const userList: CSSProperties = { display: "grid", alignContent: "start", gap: 2, minWidth: 0 };
const userRow: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "minmax(0, 1fr) auto",
  gap: "var(--space-4)",
  alignItems: "center",
  minWidth: 0,
};
const rowBtn = (on: boolean): CSSProperties => ({
  display: "grid",
  gridTemplateColumns: "40px minmax(0, 1fr) auto",
  gap: "var(--space-4)",
  alignItems: "center",
  textAlign: "left",
  padding: "5px 8px",
  borderRadius: 8,
  cursor: "pointer",
  minHeight: 32,
  minWidth: 0,
  background: on ? "color-mix(in srgb, var(--accent) 18%, transparent)" : "transparent",
  border: on ? "1px solid var(--accent)" : "1px solid transparent",
});
/* nome do patch: encolhe com reticências (coluna estreita, nome longo) */
const rowName: CSSProperties = {
  fontSize: "var(--text-sm)",
  minWidth: 0,
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
};
const ppCell: CSSProperties = { fontFamily: "var(--font-mono)", fontSize: "var(--text-xs)", color: "var(--text-muted)" };
const styleBadge: CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-xs)",
  color: "var(--text-muted)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  borderRadius: 6,
  padding: "1px 6px",
  whiteSpace: "nowrap",
};
/* ✕ de excluir patch do usuário: alvo ≥32px, perigo só no hover */
const delBtn: CSSProperties = {
  background: "transparent",
  border: "1px solid transparent",
  color: "var(--text-muted)",
  borderRadius: 8,
  cursor: "pointer",
  minHeight: 32,
  minWidth: 32,
  fontSize: "var(--text-sm)",
};
const saveRow: CSSProperties = { display: "grid", gridTemplateColumns: "minmax(0, 1fr) auto", gap: "var(--space-4)", alignItems: "center" };
const saveBtn: CSSProperties = {
  ...tab,
  padding: "6px 10px",
  minHeight: 32,
  border: "1px solid color-mix(in srgb, var(--accent) 45%, transparent)",
  color: "var(--accent-text)",
  fontWeight: 700,
  whiteSpace: "nowrap",
};
const note: CSSProperties = { fontSize: "var(--text-xs)", color: "var(--text-muted)", margin: 0, padding: "var(--space-4)" };
const empty: CSSProperties = { color: "var(--text-muted)", fontSize: "var(--text-sm)", padding: "var(--space-8)" };

export function LibraryPanel({
  currentPp,
  bank,
  userPatches,
  currentUserId,
  onSelect,
  onOpenUser,
  onSave,
  onDelete,
}: Props) {
  const [tabKind, setTabKind] = useState<"factory" | "user">(bank);
  const [q, setQ] = useState("");
  const [draft, setDraft] = useState("");

  // A aba segue o BANCO ABERTO no palco: abrir um patch (de fábrica ou de
  // usuário) tem que refletir aqui, senão a biblioteca mente sobre o que está
  // tocando. Clicar numa aba é só navegação de VISÃO — não muda o palco.
  useEffect(() => {
    setTabKind(bank);
  }, [bank]);

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
        <button role="tab" aria-selected={tabKind === "user"} style={tabKind === "user" ? tabOn : tab} onClick={() => setTabKind("user")}>
          {`${MSG.libTabUser}${userPatches.length > 0 ? ` (${userPatches.length})` : ""}`}
        </button>
      </div>

      {tabKind === "factory" ? (
        <input
          style={input}
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder={MSG.searchPlaceholder}
          aria-label={MSG.searchAria}
        />
      ) : (
        /* salvar o patch CORRENTE: o snapshot é a cadeia que está no palco
           agora (patches de usuário = prévia local, sem canal de escrita) */
        <div style={saveRow}>
          <input
            style={input}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            placeholder={MSG.userPatchNewPlaceholder}
            aria-label={MSG.userPatchNameAria}
          />
          <button style={saveBtn} onClick={() => { onSave(draft); setDraft(""); }}>
            {MSG.userPatchSave}
          </button>
        </div>
      )}

      {tabKind === "factory" ? (
        <div style={list} role="listbox" aria-label={`${MSG.libListAria} (${FACTORY_PRESETS.length})`}>
          {filtered.length === 0 && <p style={empty}>{MSG.searchEmpty(q)}</p>}
          {filtered.map((p) => {
            const on = bank === "factory" && p.pp === currentPp;
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
                <span style={{ ...rowName, fontWeight: on ? 700 : 400 }}>{p.name}</span>
                <span style={styleBadge}>{p.ppTypeName}</span>
              </button>
            );
          })}
        </div>
      ) : (
        <>
          <div style={list}>
            {userPatches.length === 0 ? (
              <p style={empty}>{MSG.userPatchEmpty}</p>
            ) : (
              <div role="list" aria-label={MSG.userPatchListAria} style={userList}>
                {userPatches.map((p, i) => {
                  const on = bank === "user" && p.id === currentUserId;
                  return (
                    <div key={p.id} role="listitem" style={userRow}>
                      <button
                        aria-current={on}
                        style={rowBtn(on)}
                        onClick={() => onOpenUser(p, i)}
                        title={MSG.userPatchRowTitle(p.name, p.fromLabel)}
                      >
                        <span style={ppCell}>{MSG.libUserPp(i)}</span>
                        <span style={{ ...rowName, fontWeight: on ? 700 : 400 }}>{p.name}</span>
                        <span style={styleBadge}>{p.fromLabel}</span>
                      </button>
                      <button
                        style={delBtn}
                        onClick={() => onDelete(p.id)}
                        aria-label={MSG.userPatchDeleteAria(p.name)}
                        title={MSG.userPatchDeleteAria(p.name)}
                      >
                        ✕
                      </button>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
          <p style={note}>{MSG.userPatchNote}</p>
        </>
      )}
    </aside>
  );
}