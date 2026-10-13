/**
 * GainPanel — o assistente de gain staging (issue #115).
 *
 * **O que esta tela faz.** Mostra, módulo por módulo da cadeia, onde os
 * controles de ganho e de nível estão na faixa do dicionário, qual é a menor
 * folga da cadeia, o risco declarado e a ordem de ajuste sugerida — com a
 * ORIGEM de cada número (o nome do controle e o valor que está no preset)
 * impressa ao lado dele.
 *
 * **O que ela NÃO faz, e é a regra da issue.** Não escreve. Não há botão que
 * mande valor para o aparelho, nem "aplicar sugestão": o assistente aponta, e o
 * dono decide. Um assistente que ajusta sozinho é o oposto do que a #115 pede —
 * os coeficientes são estimativa, e uma estimativa não tem autoridade para
 * mexer no timbre de ninguém.
 *
 * **O método e a limitação aparecem NA TELA.** O modo de falha desta feature é
 * o número parecer preciso: `Gain = 70 · 71% da faixa` lê como medição, e não é.
 * Por isso o painel imprime a limitação junto do resultado (não só no `docs/`),
 * e a 'folga' é porcentagem de CONTROLE, nunca dB.
 *
 * **O risco é regra declarada, não medição:** um estágio no teto é candidato;
 * dois ganhos no teto, ou o AMP no teto com um CAB ligado atrás (o caso da
 * issue — gain alto com a IR no slot), sobe o risco para ALTO.
 *
 * **O lugar do slot é 1-based na tela** (como no palco: "1º lugar"), e o
 * relatório fala 0-based (é a ordem do sinal no arquivo). A tradução é do
 * painel, que é quem fala com o dono.
 */
import { useEffect, useRef } from "react";
import type { CSSProperties } from "react";
import { FACTORY_PRESETS } from "../artifacts/presetData";
import { MSG } from "../i18n/messages";
import type { Gain } from "../hooks/useGain";
import type { Modulo, Risco } from "../ipc/gain";

interface Props {
  /** O estado do assistente (o hook é o dono da leitura e do erro). */
  gain: Gain;
  /** preset de fábrica analisado (o do palco) */
  pp: number;
  /** fecha a tela */
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
const sheet: CSSProperties = {
  background: "var(--bg)",
  color: "var(--text)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  borderRadius: 12,
  padding: "var(--space-16)",
  width: "min(760px, 94vw)",
  maxHeight: "86vh",
  overflowY: "auto",
  display: "grid",
  gap: "var(--space-12)",
};
const head: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "var(--space-8)",
  flexWrap: "wrap",
};
const h2t: CSSProperties = { margin: 0, fontSize: "var(--text-lg)" };
const p: CSSProperties = { margin: 0, fontSize: "var(--text-sm)", color: "var(--text-muted)" };
const note: CSSProperties = { ...p, fontSize: "var(--text-xs)" };
const mono: CSSProperties = { fontFamily: "var(--font-mono)", fontSize: "var(--text-xs)" };
const resumo: CSSProperties = { display: "flex", alignItems: "center", gap: "var(--space-8)", flexWrap: "wrap" };
const badge = (risco: Risco): CSSProperties => ({
  ...mono,
  fontWeight: 700,
  padding: "2px 8px",
  borderRadius: 6,
  border: "1px solid",
  color:
    risco === "alto" ? "var(--danger, #f66)" : risco === "medio" ? "var(--accent-text)" : "var(--text-muted)",
  borderColor:
    risco === "alto"
      ? "color-mix(in srgb, var(--danger, #f66) 55%, transparent)"
      : "color-mix(in srgb, var(--text-muted) 40%, transparent)",
  whiteSpace: "nowrap",
});
const box: CSSProperties = {
  display: "grid",
  gap: 2,
  borderLeft: "2px solid color-mix(in srgb, var(--accent) 50%, transparent)",
  paddingLeft: "var(--space-12)",
};
const lin: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "minmax(0, 1fr) auto",
  gap: "var(--space-8)",
  alignItems: "baseline",
};
const close: CSSProperties = {
  background: "transparent",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  color: "var(--text)",
  borderRadius: 8,
  cursor: "pointer",
  minHeight: 32,
  minWidth: 32,
  marginLeft: "auto",
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

/** Porcentagem inteira — o dono lê "27%", não "0.2727272727272727". */
function pct(v: number): string {
  return `${Math.round(v * 100)}%`;
}

/** Um módulo: nome + estado + as leituras com a origem de cada número. */
function LinhaModulo({ m, limiar }: { m: Modulo; limiar: number }) {
  return (
    <div style={box}>
      <div style={lin}>
        <span style={{ ...mono, fontWeight: 700 }}>
          {MSG.gainModulo(m.slot + 1, m.familia, m.nome)}
        </span>
        <span style={note}>
          {m.ligado ? "" : MSG.gainDesligado}
          {m.ligado && m.folga != null ? ` · ${MSG.gainFolga(pct(m.folga))}` : ""}
        </span>
      </div>
      {m.leituras.length === 0 && <p style={note}>{MSG.gainSemControle}</p>}
      {m.leituras.map((l) => (
        <div key={`${l.knob}-${l.pos}`} style={lin}>
          <span style={mono}>{MSG.gainLeitura(l.knob, l.valor, l.faixa[0], l.faixa[1])}</span>
          <span style={note}>
            {MSG.gainPosicao(pct(l.posicao))}
            {m.ligado && 1 - l.folga >= limiar ? ` · ${MSG.gainNoTeto}` : ""}
          </span>
        </div>
      ))}
      {m.ignorados.length > 0 && <p style={note}>{MSG.gainIgnorados(m.ignorados.join(", "))}</p>}
    </div>
  );
}

export function GainPanel({ gain, pp, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    ref.current?.focus();
  }, []);

  const r = gain.relatorio;
  const nome = FACTORY_PRESETS.find((x) => x.pp === pp)?.name ?? "";

  return (
    <div style={overlay} onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-label={MSG.gainTitle}
        tabIndex={-1}
        className="gp-surface gp-surface--floating"
        style={sheet}
      >
        <div style={head}>
          <h2 style={h2t}>{MSG.gainTitle}</h2>
          <span style={note}>{MSG.gainTarget(MSG.libPp(pp), nome)}</span>
          <button style={close} onClick={onClose} aria-label={MSG.gainClose}>
            ✕
          </button>
        </div>

        <p style={p}>{MSG.gainHint}</p>

        {gain.err != null && (
          <div role="alert" style={banner}>
            <span>{gain.err.message}</span>
            <button style={btn} onClick={gain.err.retry}>
              {MSG.errRetry}
            </button>
          </div>
        )}
        {gain.carregando && (
          <p role="status" style={note}>
            {MSG.gainCarregando}
          </p>
        )}

        {r != null && (
          <>
            <div style={resumo}>
              <span style={note}>{MSG.gainRisco}</span>
              <span style={badge(r.risco)}>{MSG.gainRiscoNivel[r.risco]}</span>
              <span style={note}>{MSG.gainFolgaMenor}</span>
              <span style={mono}>
                {r.folgaMinima == null
                  ? MSG.gainSemFolga
                  : MSG.gainFolgaDe(r.folgaMinima[0] + 1, pct(r.folgaMinima[1]))}
              </span>
            </div>

            <div style={{ display: "grid", gap: "var(--space-8)" }}>
              {r.modulos.map((m) => (
                <LinhaModulo key={m.slot} m={m} limiar={r.metodo.limiarTeto} />
              ))}
            </div>

            {r.foraDoDicionario.length > 0 && (
              <p style={note}>
                {MSG.gainForaDoDicionario(r.foraDoDicionario.map((s) => s + 1).join(", "))}
              </p>
            )}

            <div style={box}>
              <span style={{ ...mono, fontWeight: 700 }}>{MSG.gainAjusteTitulo}</span>
              {r.ajuste.length === 0 ? (
                <p style={note}>{MSG.gainAjusteVazio}</p>
              ) : (
                <ol style={{ margin: 0, paddingLeft: "var(--space-16)", ...mono }}>
                  {r.ajuste.map((s) => (
                    <li key={`${s.slot}-${s.pos}`}>
                      {MSG.gainAjusteItem(s.familia, s.knob, s.valor)}
                    </li>
                  ))}
                </ol>
              )}
            </div>

            {/* O MÉTODO na tela: quais nomes entram na conta e o que ela não é.
                É o que separa "a ferramenta avisa" de "a ferramenta mede". */}
            <div style={box}>
              <span style={{ ...mono, fontWeight: 700 }}>{MSG.gainMetodoTitulo}</span>
              <p style={note}>
                {MSG.gainGanho} <span style={mono}>{r.metodo.nomesDeGanho.join(", ")}</span> ·{" "}
                {MSG.gainSaida} <span style={mono}>{r.metodo.nomesDeSaida.join(", ")}</span> ·{" "}
                {MSG.gainMix} <span style={mono}>{r.metodo.nomesDeMix.join(", ")}</span> ·{" "}
                {MSG.gainTetoEm(pct(r.metodo.limiarTeto))}
              </p>
              <p style={note}>{r.metodo.ordem}</p>
              <p style={note}>{r.metodo.limitacao}</p>
            </div>

            <p style={note}>{MSG.gainNaoEscreve}</p>
          </>
        )}
      </div>
    </div>
  );
}
