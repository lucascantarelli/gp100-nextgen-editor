/**
 * LibraryPanel — a biblioteca de patches (issue #26).
 *
 * **A lista é do BANCO, não do artefato.** Antes a aba de fábrica filtrava
 * `FACTORY_PRESETS` na memória; agora o texto e o filtro de estilo vão para o
 * SQLite (`library_search`) e o que volta são linhas. A diferença aparece
 * quando a lista deixar de caber na memória: filtrar no front seria mentir
 * sobre o custo. Fora do webview do Tauri a porta tem um fallback em memória
 * com o mesmo shape, então esta UI é a mesma nos dois lugares.
 *
 * Os patches de usuário também moram no banco agora (o `localStorage` foi
 * migrado no boot por `useLibrary`), então as duas abas saem do mesmo lugar —
 * e é por isso que existe o botão de apagar em cada linha da aba do dono.
 *
 * Largura: 248px (era 300px na #11) — o pedalboard precisa do espaço; as
 * linhas encolhem por ellipsis e a lista rola por dentro.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { FACTORY_PRESETS } from "../artifacts/presetData";
import { MSG } from "../i18n/messages";
import type { Banco, Library } from "../hooks/useLibrary";

interface Props {
  /** preset de fábrica corrente (o chip/navbar navegam por aqui) */
  currentPp: number;
  /** banco ABERTO no palco — a aba o segue (abrir patch troca os dois) */
  bankDoPalco: "factory" | "user";
  /** id do patch de usuário ABERTO no palco (null = nenhum) */
  currentUserId: string | null;
  /** estado da biblioteca (busca no banco, números, erros, migração) */
  lib: Library;
  /** abre um preset de fábrica (o palco troca para o banco de fábrica) */
  onOpenFactory: (pp: number) => void;
  /** abre um patch de usuário pelo id + posição na lista */
  onOpenUser: (id: string, index: number) => void;
  /** salva o patch CORRENTE (qualquer banco) como patch de usuário */
  onSave: (name: string) => void;
  /** apaga um patch de usuário pelo id */
  onDelete: (id: string) => void;
}

/* módulo elevado do sistema (`gp-surface`): luz e sombra pelos tokens */
const panel: CSSProperties = {
  padding: "var(--space-12)",
  display: "grid",
  gridTemplateRows: "auto auto auto 1fr auto",
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
/* linha de busca: a caixa cresce e o ✕ fica de tamanho fixo (alvo ≥32px) */
const searchRow: CSSProperties = { display: "grid", gridTemplateColumns: "minmax(0, 1fr) auto", gap: "var(--space-4)", alignItems: "center" };
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
/* ✕ de apagar patch do usuário: alvo ≥32px, perigo só no hover */
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
/* botões de arquivo (export/import): mesma família do salvar, sem o destaque */
const ioBtn: CSSProperties = { ...tab, padding: "4px 8px", minHeight: 32, border: "1px solid transparent", whiteSpace: "nowrap" };
const ioRow: CSSProperties = { display: "flex", gap: "var(--space-4)", justifyContent: "space-between", alignItems: "center" };
const note: CSSProperties = { fontSize: "var(--text-xs)", color: "var(--text-muted)", margin: 0, padding: "var(--space-4)" };
const empty: CSSProperties = { color: "var(--text-muted)", fontSize: "var(--text-sm)", padding: "var(--space-8)" };
/* banner de erro do ARQUIVO: a barra do App é a do device, esta é do banco —
   mesma forma (mensagem + ação) do #20, tokens diferentes */
const banner: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "minmax(0, 1fr) auto",
  gap: "var(--space-4)",
  alignItems: "center",
  fontSize: "var(--text-xs)",
  color: "var(--danger)",
  border: "1px solid color-mix(in srgb, var(--danger) 45%, transparent)",
  borderRadius: 8,
  padding: "var(--space-4)",
};
const infoBanner: CSSProperties = {
  ...banner,
  color: "var(--text-muted)",
  borderColor: "color-mix(in srgb, var(--text-muted) 30%, transparent)",
};

/**
 * Os estilos disponíveis vêm do artefato de fábrica, não do banco: a lista pode
 * estar filtrada por "Bl", e um filtro que mostra só os estilos que sobraram é
 * um filtro que muda de conteúdo enquanto o dono digita.
 */
const ESTILOS: ReadonlyArray<{ ppType: number; nome: string }> = (() => {
  const vistos = new Map<number, string>();
  for (const p of FACTORY_PRESETS) vistos.set(p.ppType, p.ppTypeName);
  return [...vistos.entries()]
    .map(([ppType, nome]) => ({ ppType, nome }))
    .sort((a, b) => a.nome.localeCompare(b.nome));
})();

/** Grava o arquivo que o dono pediu (o input de arquivo é invisível por isso). */
function baixa(nome: string, texto: string): void {
  const url = URL.createObjectURL(new Blob([texto], { type: "application/json" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = nome;
  a.click();
  URL.revokeObjectURL(url);
}

export function LibraryPanel({
  currentPp,
  bankDoPalco,
  currentUserId,
  lib,
  onOpenFactory,
  onOpenUser,
  onSave,
  onDelete,
}: Props) {
  const [draft, setDraft] = useState("");
  /** Relato do import: contabilidade ou recusa. `null` = nada aconteceu. */
  const [relato, setRelato] = useState<string | null>(null);
  const arquivoRef = useRef<HTMLInputElement>(null);

  /**
   * A aba VIVE no hook (`lib.banco`), e é ela que diz de onde vem a lista.
   *
   * Quando a aba morava no painel, a busca continuava sendo a do banco do
   * PALCO: clicar na aba do dono buscava entre os 99 de fábrica e a lista
   * aparecia vazia — sem mensagem, sem nada quebrado, só "nada encontrado".
   * Um estado só para a aba e um para a consulta é como isso acontece.
   *
   * E o texto da busca é do BANCO QUE ESTÁ NA TELA, não um texto solto: buscar
   * "06" na fábrica e clicar na aba do dono procuraria 06 entre patches que são
   * nomeados, não numerados — e o dono leria isso como "o meu patch sumiu".
   */
  const { banco: tabKind, setBanco, setTexto } = lib;
  const vaiPara = useCallback(
    (b: Banco) => {
      setBanco(b);
      setTexto("");
    },
    [setBanco, setTexto],
  );

  // A aba segue o BANCO ABERTO no palco: abrir um patch (de fábrica ou de
  // usuário) tem que refletir aqui, senão a biblioteca mente sobre o que está
  // tocando. Clicar numa aba é só navegação de VISÃO — não muda o palco.
  //
  // O `ref` é o que separa os dois sentidos. Sem ele, este efeito volta atrás
  // do clique do dono sempre que a aba estiver diferente do banco do palco — e
  // a aba de User Patch saltaria de volta para Factory assim que o dono
  // clicasse nela, já que abrir patch no palco não muda o banco visível.
  const bancoVisto = useRef(bankDoPalco);
  useEffect(() => {
    if (bancoVisto.current !== bankDoPalco) {
      bancoVisto.current = bankDoPalco;
      vaiPara(bankDoPalco);
    }
  }, [bankDoPalco, vaiPara]);

  // Importar é uma ação do DONO e uma ação de arquivo: o clique precisa dizer
  // se gravou, quanto gravou, ou se o arquivo não era uma biblioteca. Sem este
  // texto o botão seria um tiro no escuro com botão de retry.
  const escolheArquivo = async (f: File | undefined): Promise<void> => {
    if (!f) return;
    const rel = await lib.importar(await f.text(), false);
    setRelato(rel != null ? MSG.libImportDone(rel.inserted, rel.replaced, rel.skipped) : MSG.libImportRejected);
  };

  // a lista JÁ vem do banco visivel (`lib.banco`): aqui não há filtro por
  // banco, porque um filtro em memória de um conjunto que o banco já filtrou
  // é a forma de a tela e o arquivo discordarem sem nenhum sinal
  const lista = lib.rows;

  return (
    <aside className="gp-surface" style={panel} aria-label={MSG.libAria}>
      <div role="tablist" aria-label={MSG.libTabsAria} style={{ display: "flex", gap: "var(--space-4)" }}>
        <button role="tab" aria-selected={tabKind === "factory"} style={tabKind === "factory" ? tabOn : tab} onClick={() => vaiPara("factory")}>
          {MSG.libTabFactory}
        </button>
        <button role="tab" aria-selected={tabKind === "user"} style={tabKind === "user" ? tabOn : tab} onClick={() => vaiPara("user")}>
          {`${MSG.libTabUser}${(lib.stats?.user ?? 0) > 0 ? ` (${lib.stats?.user})` : ""}`}
        </button>
      </div>

      {/* busca: o texto vai para o BANCO (debounce no hook), não para um .filter */}
      <div style={searchRow}>
        <input
          style={input}
          value={lib.texto}
          onChange={(e) => lib.setTexto(e.target.value)}
          placeholder={MSG.libSearchPlaceholder}
          aria-label={MSG.libSearchAria}
        />
        {lib.texto.length > 0 && (
          <button style={delBtn} onClick={() => lib.setTexto("")} aria-label={MSG.libSearchClear} title={MSG.libSearchClear}>
            ✕
          </button>
        )}
      </div>

      {tabKind === "factory" ? (
        <select
          style={input}
          value={lib.estilo ?? ""}
          onChange={(e) => lib.setEstilo(e.target.value === "" ? null : Number(e.target.value))}
          aria-label={MSG.libFilterAria}
        >
          <option value="">{MSG.libFilterAll}</option>
          {ESTILOS.map((e) => (
            <option key={e.ppType} value={e.ppType}>
              {e.nome}
            </option>
          ))}
        </select>
      ) : (
        /* salvar o patch CORRENTE: o retrato é a cadeia que está no palco
           agora (o que o dono ajusta é o que volta ao abrir) */
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

      {lib.err != null && (
        <div role="alert" style={banner}>
          <span>{lib.err.message}</span>
          <button style={ioBtn} onClick={lib.err.retry}>
            {MSG.errRetry}
          </button>
        </div>
      )}

      {tabKind === "factory" ? (
        <div style={list} role="listbox" aria-label={`${MSG.libListAria} (${lib.stats?.factory ?? 0})`}>
          {lista.length === 0 && (
            <p style={empty}>{lib.texto.trim() ? MSG.libEmptySearch(lib.texto.trim()) : MSG.libEmpty}</p>
          )}
          {lista.map((p) => {
            const on = bankDoPalco === "factory" && p.pp === currentPp;
            return (
              <button
                key={p.id}
                role="option"
                aria-selected={on}
                style={rowBtn(on)}
                onClick={() => p.pp != null && onOpenFactory(p.pp)}
                title={MSG.libRowTitle(p.name, p.ppTypeName)}
              >
                {/* oficial exibe 1-based (P25 Mist = ppID 24 do all.prst) */}
                <span style={ppCell}>{p.pp != null ? MSG.libPp(p.pp) : ""}</span>
                <span style={{ ...rowName, fontWeight: on ? 700 : 400 }}>{p.name}</span>
                <span style={styleBadge}>{p.ppTypeName}</span>
              </button>
            );
          })}
        </div>
      ) : (
        <div style={list}>
          {lista.length === 0 ? (
            <p style={empty}>{lib.texto.trim() ? MSG.libEmptySearch(lib.texto.trim()) : MSG.userPatchEmpty}</p>
          ) : (
            <div role="list" aria-label={MSG.userPatchListAria} style={userList}>
              {lista.map((p, i) => {
                const on = bankDoPalco === "user" && p.id === currentUserId;
                return (
                  <div key={p.id} role="listitem" style={userRow}>
                    <button
                      aria-current={on}
                      style={rowBtn(on)}
                      onClick={() => onOpenUser(p.id, i)}
                      title={MSG.libRowTitle(p.name, p.ppTypeName)}
                    >
                      <span style={ppCell}>{MSG.libUserPp(i)}</span>
                      <span style={{ ...rowName, fontWeight: on ? 700 : 400 }}>{p.name}</span>
                      <span style={styleBadge}>{p.ppTypeName}</span>
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
          {/* onde o dado mora: depois da #26 o patch do dono está num ARQUIVO,
              e não no navegador — dizer o contrário é o tipo de texto que o
              dono acredita e perde o dado quando limpa o storage do navegador */}
          <p style={note}>{MSG.userPatchNote}</p>
        </div>
      )}

      {/* rodapé: os números do arquivo + o backup em arquivo. O export é a rede
          de segurança do banco — sem ele, um arquivo corrompido é um banco
          corrompido. */}
      <div style={ioRow}>
        <p style={note} role="status">
          {lib.stats != null ? MSG.libStats(lib.stats.total, lib.stats.user, lib.stats.schema) : ""}
        </p>
        <div style={{ display: "flex", gap: "var(--space-4)" }}>
          <button
            style={ioBtn}
            aria-label={MSG.libExportAria}
            title={MSG.libExportAria}
            onClick={() => void lib.exportar().then((json) => { if (json != null) baixa("gp100.library.json", json); })}
          >
            {MSG.libExport}
          </button>
          <button style={ioBtn} aria-label={MSG.libImportAria} title={MSG.libImportAria} onClick={() => arquivoRef.current?.click()}>
            {MSG.libImport}
          </button>
        </div>
      </div>

      {/* o input de arquivo fica fora da tela: o botão é o alvo visível e o
          input continua alcançável por teclado/leitor de tela */}
      <input
        ref={arquivoRef}
        type="file"
        accept="application/json,.json"
        style={{ display: "none" }}
        aria-label={MSG.libImportAria}
        onChange={(e) => void escolheArquivo(e.target.files?.[0])}
      />

      {relato != null && (
        <div role="status" style={infoBanner}>
          <span>{relato}</span>
          <button style={ioBtn} onClick={() => setRelato(null)} aria-label={MSG.libSearchClear}>
            ✕
          </button>
        </div>
      )}
      {lib.migrouLegado && relato == null && (
        <div role="status" style={infoBanner}>
          <span>{MSG.libMigrated(lib.stats?.user ?? 0)}</span>
        </div>
      )}
    </aside>
  );
}