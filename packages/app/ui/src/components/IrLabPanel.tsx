/**
 * IrLabPanel — o laboratório de IRs (issue #24).
 *
 * **O que esta tela FAZ.** Importa o `.ir` que o dono escolheu no disco,
 * nomeia, escolhe um dos 20 slots do aparelho, manda para o aparelho e mostra
 * o que o próprio aparelho tem gravado agora.
 *
 * **A tela tem DUAS listas de slots, e elas não podem ser confundidas.** A
 * primeira é o que o DONO guardou no arquivo; a segunda é o que o APARELHO
 * relata (`list_user_irs`, §13.12). Elas divergem — o dono importa um IR pelo
 * painel de hardware do GP-100, ou outra sessão do app faz isso, e o arquivo
 * local não fica sabendo. Sem a segunda lista, o botão "enviar" para o slot 3
 * sobrescreveria em silêncio um IR que o dono gravou na semana passada.
 *
 * **O envio é longo de verdade.** Na captura, 296 chunks levaram ~5 s; um IR
 * de 300 KB são ~20.000 chunks e quase 6 minutos. Por isso o botão de enviar
 * passa por uma confirmação que mostra o NOME que o aparelho já tem naquele
 * slot — não um "tem certeza?" genérico, que é o que se responde sem ler.
 *
 * **Apagar da biblioteca não apaga do aparelho.** O `erase` do slot é um
 * comando de fio que o projeto ainda não tem fechado (§13.12). A tela diz isso
 * no aviso de confirmação em vez de deixar o dono acreditar que o slot ficou
 * livre.
 */
import { useCallback, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { MSG } from "../i18n/messages";
import type { Irs } from "../hooks/useIrs";
import { irEnviavel } from "../ipc/ir";

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
  width: "min(760px, 92vw)",
  maxHeight: "88vh",
  overflowY: "auto",
  display: "grid",
  gap: "var(--space-12)",
};
const h2t: CSSProperties = { margin: 0, fontSize: "var(--text-lg)" };
const p: CSSProperties = { margin: 0, fontSize: "var(--text-sm)", color: "var(--text-muted)" };
const btn: CSSProperties = {
  background: "transparent",
  border: "1px solid color-mix(in srgb, var(--accent) 45%, transparent)",
  color: "var(--accent-text)",
  borderRadius: 8,
  padding: "6px 10px",
  minHeight: 32,
  cursor: "pointer",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  whiteSpace: "nowrap",
};
const btnOff: CSSProperties = { ...btn, borderColor: "transparent", color: "var(--text-muted)" };
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
const linha: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "minmax(0, 1fr) auto",
  gap: "var(--space-8)",
  alignItems: "center",
  borderTop: "1px solid color-mix(in srgb, var(--text-muted) 18%, transparent)",
  paddingTop: "var(--space-8)",
};
const nomeCell: CSSProperties = {
  fontSize: "var(--text-sm)",
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
};
/* O aviso é a única coisa AMARELA da tela — é para chamar o olho sem parecer
   erro (não é erro: é uma consequência que o dono escolhe). */
const aviso: CSSProperties = {
  ...p,
  color: "var(--accent-text)",
  border: "1px solid color-mix(in srgb, var(--accent) 45%, transparent)",
  borderRadius: 8,
  padding: "var(--space-8)",
  display: "grid",
  gap: "var(--space-4)",
};
const erro: CSSProperties = {
  ...p,
  color: "var(--danger)",
  border: "1px solid color-mix(in srgb, var(--danger) 45%, transparent)",
  borderRadius: 8,
  padding: "var(--space-8)",
};
const relatorio: CSSProperties = { ...p, color: "var(--text)" };
/* A grade de slots do aparelho: 20 números pequenos, um por linha de 10. */
const gradeSlots: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "repeat(10, minmax(0, 1fr))",
  gap: "var(--space-4)",
};
const slotVazio: CSSProperties = {
  ...p,
  padding: "var(--space-4)",
  borderRadius: 6,
  border: "1px solid color-mix(in srgb, var(--text-muted) 22%, transparent)",
  textAlign: "center",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-xs)",
  overflow: "hidden",
  textOverflow: "ellipsis",
  whiteSpace: "nowrap",
};
const slotCheio: CSSProperties = {
  ...slotVazio,
  color: "var(--accent-text)",
  // `border` inteiro e não `borderColor`: misturar o atalho com a propriedade
  // longa no mesmo objeto é o que o React avisa (e o que deixa o estilo do
  // slot ocupado depender da ordem de aplicação).
  border: "1px solid color-mix(in srgb, var(--accent) 45%, transparent)",
};

/** KB arredondados — o `.ir` tem centenas de KB e o dono não quer bytes crus. */
function kb(bytes: number): number {
  return Math.round(bytes / 1024);
}

/** Lê um arquivo do disco como array de bytes (o que o serde espera). */
async function bytesDoArquivo(f: File): Promise<number[]> {
  return Array.from(new Uint8Array(await f.arrayBuffer()));
}

interface Props {
  /** O estado do laboratório (o hook é o dono). */
  irs: Irs;
}

export function IrLabPanel({ irs }: Props) {
  const [nome, setNome] = useState("");
  const [renomeando, setRenomeando] = useState<string | null>(null);
  const [confirmando, setConfirmando] = useState<string | null>(null);
  const [confirmandoDelete, setConfirmandoDelete] = useState<string | null>(null);
  const irRef = useRef<HTMLInputElement>(null);

  const board = irs.board;
  const slots = board?.slots ?? 20;
  const lista = board?.irs ?? [];

  /**
   * O nome que o APARELHO tem naquele slot agora (`list_user_irs`).
   *
   * É o que a confirmação de sobrescrita mostra: o dono precisa ver o nome que
   * vai sumir, não um "tem certeza?".
   */
  const nomeNoDevice = useCallback(
    (slot: number | null): string | null => {
      if (slot == null || irs.deviceSlots == null) return null;
      const achado = irs.deviceSlots.find((d) => d.slot === slot);
      return achado != null && achado.name !== "" ? achado.name : null;
    },
    [irs.deviceSlots],
  );

  /**
   * Importa o `.ir` escolhido.
   *
   * O nome nasce do nome do ARQUIVO sem a extensão: pedir o nome de novo para
   * cada importação é um campo vazio a cada vez, e `vintage_4x12.ir` já é um
   * nome utilizável — o dono renomeia depois se quiser.
   */
  const escolheArquivo = useCallback(
    async (f: File | undefined) => {
      if (!f) return;
      const bytes = await bytesDoArquivo(f);
      const linha = await irs.importa(nome.trim() || f.name.replace(/\.ir$/i, ""), bytes);
      // O input é limpo para permitir reescolher o MESMO arquivo: sem isso a
      // segunda escolha do mesmo `.ir` não dispara `change` e o dono acha que
      // o botão quebrou.
      if (irRef.current) irRef.current.value = "";
      if (linha != null) setNome("");
    },
    [irs, nome],
  );

  /** Envia ao aparelho — com a confirmação de sobrescrita na frente. */
  const envia = useCallback(
    async (id: string) => {
      if (confirmando === id) {
        setConfirmando(null);
        await irs.envia(id);
        return;
      }
      setConfirmando(id);
    },
    [confirmando, irs],
  );

  /** Apaga da biblioteca — com o aviso do que NÃO acontece na frente. */
  const apaga = useCallback(
    async (id: string) => {
      if (confirmandoDelete === id) {
        setConfirmandoDelete(null);
        await irs.apaga(id);
        return;
      }
      setConfirmandoDelete(id);
    },
    [confirmandoDelete, irs],
  );

  if (!irs.aberto) return null;

  const irDoSlot = (slot: number) => lista.find((i) => i.slot === slot);
  const emEnvio = lista.find((i) => i.id === confirmando);
  // O slot e o nome que o aparelho tem nele saem ANTES do JSX: narrowing em
  // `emEnvio.slot` dentro da árvore do React não atravessa a arrow function do
  // onclick, e um `as` esconderia o caso real (um IR sem slot não tem o que
  // sobrescrever).
  const slotConfirmado = emEnvio?.slot ?? null;
  const nomeConfirmado = slotConfirmado != null ? nomeNoDevice(slotConfirmado) : null;

  return (
    <div style={box} onClick={(e) => { if (e.target === e.currentTarget) irs.fechar(); }}>
      <div role="dialog" aria-modal="true" aria-label={MSG.irTitle} style={sheet}>
        <h2 style={h2t}>{MSG.irTitle}</h2>
        <p style={p}>{MSG.irIntro}</p>

        {/* ── o que o APARELHO tem: a leitura do fio, não do arquivo ── */}
        <div style={{ display: "grid", gap: "var(--space-8)" }}>
          <h3 style={{ ...h2t, fontSize: "var(--text-md)" }}>{MSG.irDeviceTitle}</h3>
          <p style={p}>{MSG.irDeviceHint}</p>
          <div role="list" aria-label={MSG.irDeviceAria} style={gradeSlots}>
            {Array.from({ length: slots }, (_, slot) => {
              const nome = irs.deviceSlots?.find((d) => d.slot === slot)?.name ?? "";
              return (
                <span
                  key={slot}
                  role="listitem"
                  style={nome === "" ? slotVazio : slotCheio}
                  title={nome === "" ? MSG.irDeviceEmpty : MSG.irDeviceSlotTitle(slot, nome)}
                >
                  {nome === "" ? `${slot}` : `${slot}·${nome}`}
                </span>
              );
            })}
          </div>
          <button style={btnOff} disabled={irs.relendoDevice} onClick={() => void irs.relêDevice()}>
            {irs.relendoDevice ? MSG.irReadingDevice : MSG.irReadDevice}
          </button>
        </div>

        {irs.err != null && (
          <div role="alert" style={erro}>
            <span>{irs.err.message}</span>
            <button style={btnOff} onClick={irs.err.retry}>
              {MSG.errRetry}
            </button>
          </div>
        )}

        {/* importar: o nome é opcional — sem ele nasce do nome do arquivo */}
        <div style={{ display: "grid", gridTemplateColumns: "minmax(0, 1fr) auto", gap: "var(--space-8)", alignItems: "center" }}>
          <input
            style={input}
            value={nome}
            onChange={(e) => setNome(e.target.value)}
            placeholder={MSG.irNamePlaceholder}
            aria-label={MSG.irNameAria}
          />
          <button style={btn} aria-label={MSG.irImportAria} title={MSG.irImportAria} onClick={() => irRef.current?.click()}>
            {MSG.irImport}
          </button>
        </div>
        <input
          ref={irRef}
          type="file"
          accept=".ir"
          style={{ display: "none" }}
          aria-label={MSG.irImportAria}
          onChange={(e) => void escolheArquivo(e.target.files?.[0])}
        />

        {/* a lista da BIBLIOTECA */}
        <div role="list" aria-label={MSG.irListAria} style={{ display: "grid", gap: "var(--space-8)" }}>
          {lista.length === 0 && <p style={p}>{MSG.irEmpty}</p>}
          {lista.map((ir) => {
            const nomeAparelho = nomeNoDevice(ir.slot);
            const enviavel = irEnviavel(ir);
            return (
              <div key={ir.id} role="listitem" style={linha}>
                <div style={{ display: "grid", gap: 2, minWidth: 0 }}>
                  {renomeando === ir.id ? (
                    <input
                      style={input}
                      autoFocus
                      defaultValue={ir.name}
                      aria-label={MSG.irRenameAria}
                      onBlur={(e) => {
                        void irs.renomeia(ir.id, e.target.value);
                        setRenomeando(null);
                      }}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                      }}
                    />
                  ) : (
                    <span style={{ ...nomeCell, fontWeight: 700 }}>{ir.name}</span>
                  )}
                  <span style={p}>
                    {MSG.irRowTitle(ir.name, kb(ir.bytes))}
                    {nomeAparelho != null ? ` · ${MSG.irDeviceHas(nomeAparelho)}` : ""}
                  </span>
                </div>

                <div style={{ display: "flex", gap: "var(--space-4)", alignItems: "center", flexWrap: "wrap", justifyContent: "flex-end" }}>
                  <select
                    style={input}
                    value={ir.slot ?? ""}
                    aria-label={MSG.irSlotAria(ir.slot ?? 0)}
                    onChange={(e) => void irs.atribui(ir.id, e.target.value === "" ? null : Number(e.target.value))}
                  >
                    <option value="">{MSG.irSlotNone}</option>
                    {Array.from({ length: slots }, (_, i) => i).map((n) => (
                      <option key={n} value={n} disabled={irDoSlot(n) != null && irDoSlot(n)?.id !== ir.id}>
                        {MSG.irSlotLabel(n)}
                      </option>
                    ))}
                  </select>
                  <button style={btnOff} aria-label={MSG.irRenameAria} title={MSG.irRenameAria} onClick={() => setRenomeando(ir.id)}>
                    ✎
                  </button>
                  <button
                    style={enviavel && ir.slot != null ? btn : btnOff}
                    disabled={!enviavel || ir.slot == null || irs.enviando != null}
                    aria-label={MSG.irSendAria(ir.name)}
                    title={enviavel && ir.slot != null ? MSG.irSendAria(ir.name) : MSG.irNotEnviable}
                    onClick={() => void envia(ir.id)}
                  >
                    {irs.enviando === ir.id ? MSG.irSending : MSG.irSend}
                  </button>
                  <button style={btnOff} aria-label={MSG.irDeleteAria(ir.name)} title={MSG.irDeleteAria(ir.name)} onClick={() => void apaga(ir.id)}>
                    ✕
                  </button>
                </div>
              </div>
            );
          })}
        </div>

        {/* a confirmação de sobrescrita fica NO LUGAR do clique: o dono leu e
            agora decide, com o nome que vai sumir à vista. */}
        {emEnvio != null && slotConfirmado != null && (
          <div role="alertdialog" aria-label={MSG.irOverwriteTitle} style={aviso}>
            <strong>{MSG.irOverwriteTitle}</strong>
            <span>
              {nomeConfirmado != null
                ? MSG.irOverwriteText(nomeConfirmado, slotConfirmado)
                : MSG.irOverwriteEmpty}
            </span>
            <div style={{ display: "flex", gap: "var(--space-8)" }}>
              <button style={btn} onClick={() => { if (emEnvio != null) void envia(emEnvio.id); }}>
                {MSG.irSend}
              </button>
              <button style={btnOff} onClick={() => setConfirmando(null)}>
                {MSG.irCancel}
              </button>
            </div>
          </div>
        )}

        {confirmandoDelete != null && (
          <div role="alertdialog" aria-label={MSG.irDeleteWarnTitle} style={aviso}>
            <strong>{MSG.irDeleteWarnTitle}</strong>
            <span>{MSG.irDeleteWarnText}</span>
            <div style={{ display: "flex", gap: "var(--space-8)" }}>
              <button style={btn} onClick={() => { if (confirmandoDelete != null) void apaga(confirmandoDelete); }}>
                {MSG.irDelete}
              </button>
              <button style={btnOff} onClick={() => setConfirmandoDelete(null)}>
                {MSG.irCancel}
              </button>
            </div>
          </div>
        )}

        {irs.relatorio != null && (
          <p role="status" style={relatorio}>
            {MSG.irSent(irs.relatorio.chunks, irs.relatorio.bytes)}
          </p>
        )}

        <footer style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "var(--space-8)" }}>
          <p role="status" style={p}>
            {board != null ? MSG.irStats(board.irs.length, board.slots, board.usados) : ""}
          </p>
          <button style={btnOff} onClick={irs.fechar}>
            {MSG.libSearchClear}
          </button>
        </footer>
      </div>
    </div>
  );
}
