/**
 * AbPanel — A/B instantâneo com blind test entre duas versões do patch (#116).
 *
 * **O que esta tela é.** O comparador: escolhe dois lados (as versões do
 * histórico do patch aberto), troca um pelo outro com um clique, mede a
 * diferença de nível entre eles e — quando o ouvido é o juiz — esconde o
 * resultado até a resposta.
 *
 * **Por que o BLIND esconde mais do que o rótulo.** Armado, a tela não mostra
 * nem o lado ativo, nem as versões, nem o relatório: um relatório com o nome
 * do lado ("calibrado no B") entrega a resposta do mesmo jeito que um botão
 * que diz "agora é B". É por isso que o bloco inteiro some e só restam a
 * pergunta, "trocar de lado" (que não nomeia para onde vai) e os dois palpites.
 * O teste de UI é o que prova que nada vaza antes da resposta.
 *
 * **O nível é POSIÇÃO, e a tela diz.** `nivel()`/`deltaNivel()` vêm de
 * `abLevel.ts`: média das posições dos controles de saída, 0..100 — o
 * aparelho não expõe dB por SysEx e inventar a conversão seria pior que não
 * medir. Método e limitação ficam NA TELA, junto do número, como no
 * assistente de gain (#115).
 *
 * **Escrita:** trocar knobs no aparelho e calibrar são escrita (ADR-5). Numa
 * build de leitura os botões nascem desabilitados com o motivo — o mesmo
 * `writeLockedHint` do knob, do IR e do SnapTone (#126) — e o relatório diz
 * que a troca aconteceu só na tela.
 */
import type { CSSProperties } from "react";
import { MSG } from "../i18n/messages";
import { deltaNivel, nivel } from "../abLevel";
import type { Ab } from "../hooks/useAb";

interface Props {
  ab: Ab;
  /** Escrita liberada nesta build? (ADR-5; `false` = build de leitura). */
  podeGravar: boolean;
  onClose: () => void;
}

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
  width: "min(720px, 92vw)",
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
const btnOff: CSSProperties = { ...btn, border: "1px solid transparent", color: "var(--text-muted)" };
const lado: CSSProperties = {
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  borderRadius: 8,
  padding: "var(--space-8)",
  display: "grid",
  gap: 2,
};
const ladoAtivo: CSSProperties = {
  ...lado,
  // `border` no singular em vez de `borderColor`: misturar os dois faz o
  // React avisar que a propriedade shorthand some no re-render.
  border: "1px solid color-mix(in srgb, var(--accent) 55%, transparent)",
  background: "color-mix(in srgb, var(--accent) 8%, transparent)",
};
const erro: CSSProperties = {
  ...p,
  color: "var(--danger)",
  border: "1px solid color-mix(in srgb, var(--danger) 45%, transparent)",
  borderRadius: 8,
  padding: "var(--space-8)",
};
const numero: CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-md)",
  color: "var(--text)",
};

/** `v3 · 06/10 14:32` — o que distingue um lado do outro na tela. */
function rotulo(seq: number, savedAt: string): string {
  const d = new Date(savedAt);
  const data = Number.isNaN(d.getTime()) ? savedAt : d.toLocaleString("pt-BR", { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" });
  return `${MSG.abVersao(seq)} · ${data}`;
}

export function AbPanel({ ab, podeGravar, onClose }: Props) {
  // O BLIND é o interruptor de TUDO que nomeia um lado.
  const revelado = !ab.blind || ab.resposta != null;
  const a = ab.a;
  const b = ab.b;
  const na = a == null ? null : nivel(a.board);
  const nb = b == null ? null : nivel(b.board);
  const delta = a == null || b == null ? null : deltaNivel(a.board, b.board);

  if (!ab.disponivel) {
    return (
      <div style={box} onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
        <div role="dialog" aria-modal="true" aria-label={MSG.abTitle} style={sheet}>
          <h2 style={h2t}>{MSG.abTitle}</h2>
          <p style={p}>{MSG.abIntro}</p>
          <p style={p} role="status">{MSG.abEmpty}</p>
          <div>
            <button type="button" style={btnOff} onClick={onClose}>{MSG.irCancel}</button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div style={box} onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div role="dialog" aria-modal="true" aria-label={MSG.abTitle} style={sheet}>
        <h2 style={h2t}>{MSG.abTitle}</h2>
        <p style={p}>{MSG.abIntro}</p>
        {!podeGravar && (
          <p style={{ ...p, color: "var(--accent-text)" }} role="status">{MSG.writeLockedHint}</p>
        )}

        {/* ── o comparador ── */}
        <div style={{ display: "grid", gap: "var(--space-8)" }}>
          {revelado ? (
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-8)" }}>
              <div style={ab.lado === "a" ? ladoAtivo : lado}>
                <strong style={numero}>{MSG.abLado("A")}</strong>
                <span style={p}>{a != null ? rotulo(a.row.seq, a.row.savedAt) : ""}</span>
                {ab.lado === "a" && <span style={p}>{MSG.abSoando}</span>}
              </div>
              <div style={ab.lado === "b" ? ladoAtivo : lado}>
                <strong style={numero}>{MSG.abLado("B")}</strong>
                <span style={p}>{b != null ? rotulo(b.row.seq, b.row.savedAt) : ""}</span>
                {ab.lado === "b" && <span style={p}>{MSG.abSoando}</span>}
              </div>
            </div>
          ) : (
            <p style={p} role="status">{MSG.abPergunta}</p>
          )}

          <div style={{ display: "flex", gap: "var(--space-8)", flexWrap: "wrap" }}>
            {revelado ? (
              <>
                <button type="button" style={btn} disabled={ab.ocupado} onClick={() => void ab.escolhe("a")}>
                  {MSG.abOuvir("A")}
                </button>
                <button type="button" style={btn} disabled={ab.ocupado} onClick={() => void ab.escolhe("b")}>
                  {MSG.abOuvir("B")}
                </button>
              </>
            ) : (
              <>
                <button type="button" style={btnOff} disabled={ab.ocupado} onClick={ab.trocaLado}>
                  {MSG.abTrocar}
                </button>
                <button type="button" style={btn} disabled={ab.ocupado} onClick={() => ab.responde("a")}>
                  {MSG.abPalpite("A")}
                </button>
                <button type="button" style={btn} disabled={ab.ocupado} onClick={() => ab.responde("b")}>
                  {MSG.abPalpite("B")}
                </button>
              </>
            )}
            <label style={{ ...p, display: "flex", alignItems: "center", gap: 6 }}>
              <input
                type="checkbox"
                checked={ab.blind}
                aria-label={MSG.abBlind}
                onChange={(e) => ab.armaBlind(e.target.checked)}
              />
              {MSG.abBlind}
            </label>
            {ab.blind && ab.resposta != null && (
              <button type="button" style={btnOff} onClick={ab.reinicia}>{MSG.abDeNovo}</button>
            )}
          </div>

          {ab.blind && ab.resposta != null && (
            <p style={numero} role="status">{MSG.abResposta(ab.lado.toUpperCase(), ab.acertou)}</p>
          )}
        </div>

        {/* ── o nível: número + método + limitação, juntos ── */}
        {revelado && (
          <div style={{ display: "grid", gap: "var(--space-4)" }}>
            <div style={{ display: "flex", gap: "var(--space-16)", flexWrap: "wrap" }}>
              <span style={p}>{MSG.abNivel("A", na == null ? "—" : na.toFixed(1))}</span>
              <span style={p}>{MSG.abNivel("B", nb == null ? "—" : nb.toFixed(1))}</span>
              <span style={{ ...p, color: "var(--text)" }}>
                {MSG.abDelta(delta == null ? "—" : (delta > 0 ? "+" : "") + delta.toFixed(1))}
              </span>
            </div>
            <span style={p}>{MSG.abNivelMetodo}</span>
            {(na == null || nb == null) && (
              <span style={{ ...p, color: "var(--accent-text)" }} role="status">{MSG.abNivelSemControle}</span>
            )}
            <div>
              <button
                type="button"
                style={podeGravar && delta != null && delta !== 0 ? btn : btnOff}
                disabled={!podeGravar || ab.ocupado || delta == null || delta === 0}
                title={podeGravar ? undefined : MSG.writeLockedHint}
                onClick={() => void ab.calibra()}
              >
                {MSG.abCalibrar}
              </button>
            </div>
          </div>
        )}

        {revelado && ab.relato != null && (
          <p style={p} role="status">{ab.relato}</p>
        )}
        {ab.err != null && (
          <div role="alert" style={erro}>
            <span>{ab.err.message}</span>
            <button type="button" style={btnOff} onClick={ab.err.retry}>{MSG.errRetry}</button>
          </div>
        )}

        <div>
          <button type="button" style={btnOff} onClick={onClose}>{MSG.irCancel}</button>
        </div>
      </div>
    </div>
  );
}
