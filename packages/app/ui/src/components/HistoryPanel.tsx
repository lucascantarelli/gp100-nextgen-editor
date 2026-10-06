/**
 * HistoryPanel — o histórico do patch, o diff e as restaurações (issue #113).
 *
 * **O que esta tela FAZ.** Mostra as versões que o patch já teve, o que mudou
 * entre duas delas — no nível do knob, agrupado por slot da cadeia — e deixa
 * o dono voltar atrás: a versão inteira, ou um knob só.
 *
 * **O que a tela NÃO faz, e é uma regra.** Ela não apaga. Não existe botão de
 * "descartar versão" aqui, e não existe na porta: restaurar ACRECENTA uma
 * versão nova, e a antiga continua na lista. Um "desfazer" que perde o presente
 * é um "desfazer" que mente — então o botão diz "virou a versão N", que é o
 * que aconteceu de verdade.
 *
 * **O par comparado é uma escolha visível, não um cálculo escondido.** Dois
 * seletores ("antes" e "depois") na cabeça da tela: quem lê a resposta precisa
 * saber sobre o quê ela é. O default é a última contra a penúltima, que é a
 * pergunta que a pessoa quase sempre tem ("o que eu mudei desde a última
 * vez"), mas o default é visível e trocável.
 *
 * **A troca de algoritmo é um evento SEPARADO dos knobs.** Trocar o AMP troca
 * todos os knobs dele; sem a linha própria, a tela diria "você mexeu em 12
 * controles" quando o dono trocou um pedal.
 *
 * **Nada aqui escreve no aparelho.** O histórico é do arquivo do dono
 * (`gp100-library`, ADR-9); restaurar não manda byte para o GP-100 — o que
 * importa porque, no build de campo sem `write-verified`, a escrita no device
 * é recusada antes do driver.
 */
import { useEffect, useRef } from "react";
import type { CSSProperties } from "react";
import { MSG } from "../i18n/messages";
import type { History } from "../hooks/useHistory";
import type { LibraryVersionRow } from "../ipc/history";

interface Props {
  /** estado do histórico (o hook é o dono da política) */
  hist: History;
  /** nome do patch aberto — a tela diz QUAL patch este histórico é */
  nome: string;
  /** fecha o painel */
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
const dialog: CSSProperties = {
  width: "min(760px, 94vw)",
  maxHeight: "86vh",
  overflow: "hidden",
  display: "grid",
  gridTemplateRows: "auto 1fr",
};
const head: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "var(--space-8)",
  padding: "var(--space-12) var(--space-20)",
  flexWrap: "wrap",
};
const body: CSSProperties = {
  overflowY: "auto",
  padding: "0 var(--space-20) var(--space-20)",
  display: "grid",
  gap: "var(--space-12)",
  alignContent: "start",
};
const close: CSSProperties = {
  background: "transparent",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  color: "var(--text)",
  borderRadius: 8,
  cursor: "pointer",
  minHeight: 32,
  minWidth: 32,
};
const select: CSSProperties = {
  background: "var(--bg)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  color: "var(--text)",
  borderRadius: 8,
  padding: "7px 10px",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  minHeight: 32,
};
const btn: CSSProperties = {
  background: "transparent",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  color: "var(--text)",
  borderRadius: 8,
  padding: "5px 10px",
  cursor: "pointer",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-xs)",
  minHeight: 32,
  whiteSpace: "nowrap",
};
const note: CSSProperties = { fontSize: "var(--text-xs)", color: "var(--text-muted)", margin: 0 };
const slotBox: CSSProperties = {
  display: "grid",
  gap: "var(--space-4)",
  borderLeft: "2px solid color-mix(in srgb, var(--accent) 50%, transparent)",
  paddingLeft: "var(--space-12)",
};
const knobRow: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "minmax(0, 1fr) auto auto",
  gap: "var(--space-8)",
  alignItems: "center",
  minHeight: 32,
};
const versRow: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "minmax(0, 1fr) auto",
  gap: "var(--space-8)",
  alignItems: "center",
  minHeight: 32,
};
const banner: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "var(--space-8)",
  border: "1px solid color-mix(in srgb, var(--danger, #f66) 45%, transparent)",
  borderRadius: 8,
  padding: "var(--space-8) var(--space-12)",
  fontSize: "var(--text-sm)",
};

/** Data curta e honesta: dia/mês e hora, no fuso do navegador. */
function quando(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return `${String(d.getDate()).padStart(2, "0")}/${String(d.getMonth() + 1).padStart(2, "0")} ${String(
    d.getHours(),
  ).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

/** Rótulo de uma versão nos seletores: "v3 · 05/10 21:40". */
function rotulo(v: LibraryVersionRow): string {
  return `v${v.seq} · ${quando(v.savedAt)}`;
}

export function HistoryPanel({ hist, nome, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    ref.current?.focus();
  }, []);

  const { versoes, diff, antes, depois, ocupado } = hist;

  return (
    <div style={overlay} onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-label={MSG.histAria}
        tabIndex={-1}
        className="gp-surface gp-surface--floating"
        style={dialog}
      >
        <div style={head}>
          <strong style={{ fontSize: "var(--text-md)" }}>{MSG.histTitle}</strong>
          <span style={note}>{nome}</span>
          <button style={{ ...close, marginLeft: "auto" }} onClick={onClose} aria-label={MSG.histClose}>
            ✕
          </button>
        </div>

        <div style={body}>
          {hist.err != null && (
            <div role="alert" style={banner}>
              <span>{hist.err.message}</span>
              <button style={btn} onClick={hist.err.retry}>
                {MSG.errRetry}
              </button>
            </div>
          )}
          {hist.relato != null && (
            <p role="status" style={note}>
              {hist.relato}
            </p>
          )}

          {versoes.length === 0 ? (
            <p style={note}>{MSG.histEmpty}</p>
          ) : (
            <>
              {/* o par comparado é VISÍVEL: quem lê a resposta precisa saber
                  sobre o quê ela é */}
              <div style={{ display: "grid", gridTemplateColumns: "auto 1fr auto 1fr", gap: "var(--space-8)", alignItems: "center" }}>
                <label htmlFor="hist-antes" style={note}>
                  {MSG.histBefore}
                </label>
                <select
                  id="hist-antes"
                  style={select}
                  value={antes ?? ""}
                  onChange={(e) => hist.compara(e.target.value ? Number(e.target.value) : null, depois)}
                >
                  <option value="">{MSG.histPickVersion}</option>
                  {versoes.map((v) => (
                    <option key={v.id} value={v.id}>
                      {rotulo(v)}
                    </option>
                  ))}
                </select>
                <label htmlFor="hist-depois" style={note}>
                  {MSG.histAfter}
                </label>
                <select
                  id="hist-depois"
                  style={select}
                  value={depois ?? ""}
                  onChange={(e) => hist.compara(antes, e.target.value ? Number(e.target.value) : null)}
                >
                  <option value="">{MSG.histPickVersion}</option>
                  {versoes.map((v) => (
                    <option key={v.id} value={v.id}>
                      {rotulo(v)}
                    </option>
                  ))}
                </select>
              </div>

              {/* a lista de versões + a restauração TOTAL de cada uma */}
              <div style={slotBox}>
                {versoes.map((v) => (
                  <div key={v.id} style={versRow}>
                    <span>
                      {rotulo(v)} · {v.name}
                      {v.current ? ` ${MSG.histCurrent}` : ""}
                    </span>
                    <button
                      style={btn}
                      disabled={ocupado}
                      onClick={() => void hist.restaura(v.id)}
                      aria-label={MSG.histRestoreAll(v.seq)}
                    >
                      {MSG.histRestoreAllAction}
                    </button>
                  </div>
                ))}
              </div>

              {diff == null ? (
                <p style={note}>{MSG.histPickTwo}</p>
              ) : diff.identical ? (
                <p style={note}>{MSG.histIdentical}</p>
              ) : (
                <>
                  <p style={note}>{MSG.histDiffTitle}</p>
                  {diff.slots.map((s) => (
                    <div key={s.slot} style={slotBox}>
                      <strong style={{ fontFamily: "var(--font-mono)", fontSize: "var(--text-sm)" }}>
                        {MSG.histSlot(s.slot, s.nameAfter)}
                      </strong>
                      {s.algorithmChanged && (
                        <p style={note}>{MSG.histAlgorithm(s.nameBefore, s.nameAfter)}</p>
                      )}
                      {s.onBefore !== s.onAfter && (
                        <p style={note}>{MSG.histSlotState(s.onAfter)}</p>
                      )}
                      {s.knobs.map((k) => (
                        <div key={k.pos} style={knobRow}>
                          <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--text-sm)", minWidth: 0 }}>
                            {k.knob}
                          </span>
                          <span style={note}>{MSG.histArrow(k.from, k.to)}</span>
                          <button
                            style={btn}
                            disabled={ocupado}
                            onClick={() => void hist.restauraKnob(antes ?? -1, s.slot, k.pos)}
                            aria-label={MSG.histRestoreKnob(k.knob, k.from, k.to)}
                          >
                            {MSG.histRestoreKnobAction}
                          </button>
                        </div>
                      ))}
                    </div>
                  ))}
                  {diff.slotsAdded.length > 0 && <p style={note}>{MSG.histSlotsAdded(diff.slotsAdded.join(", "))}</p>}
                  {diff.slotsRemoved.length > 0 && <p style={note}>{MSG.histSlotsRemoved(diff.slotsRemoved.join(", "))}</p>}
                </>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
