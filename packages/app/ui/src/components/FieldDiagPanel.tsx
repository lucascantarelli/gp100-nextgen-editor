/**
 * FieldDiagPanel — o diagnóstico de campo que só o utilitário de linha de
 * comando fazia, em forma de tela.
 *
 * **O que esta tela é.** Quatro capacidades que o editor não tinha e o
 * utilitário de terminal tinha: GRAVAR o patch na pedaleira, LER o que está lá
 * (o dump que a validação de campo usou para provar a escrita), GRAVAR o
 * tráfego USB num arquivo e VER o que seria enviado sem enviar. Com elas na
 * tela, a sessão de campo acontece dentro do editor — o arquivo de log vai
 * direto para quem analisa, sem depender do binário de terminal.
 *
 * **Por que o painel é honesto antes de ser bonito.** O build de campo de
 * leitura não escreve na pedaleira (a trava é do transporte, antes do fio), e
 * sem aparelho é tudo simulação. Esconder isso atrás de um botão que falha
 * depois do clique seria pior que mostrar: o operador de campo descobre o tipo
 * de instalação que tem na MÃO, e o botão travado vem com a razão escrita ao
 * lado.
 *
 * **O hex vem do BACKEND, nunca é montado aqui.** O front não fala SysEx, e é
 * por isso que a prévia do envio e o envio real não podem divergir — inclusive
 * na trava de valor por faixa, que vale igual para os dois.
 *
 * A prévia descreve a GRAVAÇÃO porque é a operação que este painel tem em
 * mãos. A prévia de um knob pertence ao painel do pedal, que é quem sabe o
 * slot, o código e o controle.
 */
import type { CSSProperties } from "react";
import { useEffect, useState } from "react";
import { MSG } from "../i18n/messages";
import { useFieldDiag } from "../hooks/useFieldDiag";
import type { DeviceInfo } from "../ipc/types";

interface Props {
  info: DeviceInfo | null;
}

/** Quanto tempo o botão fica dizendo "copiado" antes de voltar a "copiar". */
const COPIADO_MS = 1_500;

/** Teto dos campos numéricos: inteiro sem sinal, 0..65535 (o u16 do fio). */
function inteiro(bruto: string, max = 65535): number {
  const n = Number(bruto);
  return Number.isInteger(n) && n >= 0 && n <= max ? n : 0;
}

export function FieldDiagPanel({ info }: Props) {
  const d = useFieldDiag(info);
  const real = info?.backend === "real";
  const podeGravar = info?.writeVerified === true;
  // Três estados, não dois: sem aparelho nenhum o que o operador mexe fica só
  // na tela. Chamar isso de "simulação" seria mentira — a simulação é quando
  // existe um aparelho de mentira.
  const backend = info == null ? MSG.diagBackendNone : real ? MSG.diagBackendReal : MSG.diagBackendMock;
  // O botão de cópia tem dois rótulos: o que FAZ ("copiar") e o que ACONTECEU
  // ("copiado"). Mostrar o passado em repouso faria a lista parecer já copiada.
  const [copiado, setCopiado] = useState<number | null>(null);
  useEffect(() => {
    if (copiado == null) return;
    const t = setTimeout(() => setCopiado(null), COPIADO_MS);
    return () => clearTimeout(t);
  }, [copiado]);

  return (
    <details className="gp-surface gp-surface--flat" style={styles.wrap}>
      <summary style={styles.summary}>{MSG.diagSummary}</summary>
      <section aria-label={MSG.diagAria} style={styles.panel}>
        {/* QUEM responde: simulação ou aparelho. A diferença muda o que todo o
            resto do painel significa, então fica no topo e não num rodapé. */}
        <div style={styles.header}>
          <h2 style={styles.title}>{MSG.diagTitle}</h2>
          <span style={{ ...styles.badge, ...(real ? styles.badgeReal : styles.badgeMock) }}>
            {backend}
          </span>
        </div>
        <p style={styles.hint} role="status">
          {podeGravar ? MSG.diagWriteOpen : MSG.diagWriteLocked}
        </p>
        {info == null && <p style={styles.hint}>{MSG.diagNoDeviceHint}</p>}

        {/* o patch de destino: gravar, ler e descrever usam OS MESMOS campos */}
        <div style={styles.row}>
          <label style={styles.field}>
            <span style={styles.label}>{MSG.diagPpLabel}</span>
            <input
              style={styles.input}
              type="number"
              min={0}
              max={65535}
              value={d.pp}
              aria-label={MSG.diagPpLabel}
              onChange={(e) => d.setPp(inteiro(e.target.value))}
            />
          </label>
          <label style={styles.field}>
            <span style={styles.label}>{MSG.diagPpTypeLabel}</span>
            <input
              style={styles.input}
              type="number"
              min={0}
              max={65535}
              value={d.ppType}
              aria-label={MSG.diagPpTypeLabel}
              onChange={(e) => d.setPpType(inteiro(e.target.value))}
            />
          </label>
          <label style={styles.grow}>
            <span style={styles.label}>{MSG.diagNameLabel}</span>
            <input
              style={styles.input}
              value={d.name}
              aria-label={MSG.diagNameLabel}
              onChange={(e) => d.setName(e.target.value)}
            />
          </label>
        </div>

        <div style={styles.actions}>
          <button
            type="button"
            style={podeGravar ? styles.btn : styles.btnOff}
            disabled={!podeGravar || d.busy != null}
            onClick={() => void d.save()}
          >
            {MSG.diagSave}
          </button>
          <button
            type="button"
            style={styles.btnOff}
            disabled={d.busy != null}
            onClick={() => void d.dumpPreset()}
          >
            {MSG.diagDump}
          </button>
          <button
            type="button"
            style={styles.btnOff}
            disabled={d.busy != null}
            onClick={() => void d.previa()}
          >
            {MSG.diagPreviewBtn}
          </button>
        </div>

        {d.saved && (
          <p style={styles.ok} role="status">
            {MSG.diagSaveDone}
          </p>
        )}
        {d.error != null && (
          <p style={styles.err} role="alert">
            {d.error}
          </p>
        )}

        {/* o log de tráfego: a sessão pode ligar e desligar sem reiniciar o
            aparelho, e o arquivo é o que o operador entrega depois */}
        <div style={styles.block}>
          <div style={styles.row}>
            <label style={styles.grow}>
              <span style={styles.label}>{MSG.diagLogPathLabel}</span>
              <input
                style={styles.input}
                value={d.logFile}
                aria-label={MSG.diagLogPathLabel}
                onChange={(e) => d.setLogFile(e.target.value)}
              />
            </label>
            <button
              type="button"
              style={d.logging ? styles.btn : styles.btnOff}
              disabled={d.busy != null}
              onClick={() => void d.toggleLog()}
            >
              {d.logging ? MSG.diagLogStop : MSG.diagLogStart}
            </button>
          </div>
          <p style={styles.hint}>{d.logging && d.logPath != null ? MSG.diagLogOn(d.logPath) : MSG.diagLogHint}</p>
        </div>

        {/* o dump fica recolhido: são 9 blocos de hexadecimal, e o operador só
            olha para eles quando algo está errado */}
        {d.dump != null && (
          <details style={styles.block}>
            <summary style={styles.subSummary}>
              {`${MSG.diagDumpTitle} · ${MSG.diagDumpPages(d.dump.pages.length)}`}
            </summary>
            <div style={styles.block}>
              <span style={styles.label}>{MSG.diagDumpMeta6}</span>
              <code style={styles.hex}>{d.dump.meta6}</code>
              {d.dump.pages.map((pg, i) => (
                <code key={i} style={styles.hex}>
                  {pg}
                </code>
              ))}
            </div>
          </details>
        )}

        {/* a prévia: o que SAIRIA pelo fio, sem sair */}
        {d.preview != null && (
          <details style={styles.block}>
            <summary style={styles.subSummary}>
              {`${MSG.diagPreviewTitle} · ${MSG.diagPreviewCount(d.preview.length)}`}
            </summary>
            <ol style={styles.list}>
              {d.preview.length === 0 ? (
                <li style={styles.hint}>{MSG.diagPreviewEmpty}</li>
              ) : (
                d.preview.map((f, i) => (
                  <li key={`${f.label}-${i}`} style={styles.item}>
                    <span style={styles.label}>{f.label}</span>
                    <code style={styles.hex}>{f.hex}</code>
                    <button
                      type="button"
                      style={styles.copy}
                      aria-label={MSG.diagCopyAria}
                      title={MSG.diagCopyAria}
                      onClick={() => {
                        setCopiado(i);
                        void copiar(f.hex);
                      }}
                    >
                      {copiado === i ? MSG.diagCopied : MSG.diagCopyBtn}
                    </button>
                  </li>
                ))
              )}
            </ol>
          </details>
        )}

        {(d.dump != null || d.preview != null) && (
          <button type="button" style={styles.btnOff} onClick={d.clear}>
            {MSG.pushClear}
          </button>
        )}
      </section>
    </details>
  );
}

/**
 * Copia o hexadecimal para a área de transferência. Falhou (sem permissão, ou
 * um `webview` sem a área), o botão apenas não confirma: perder um "copiado"
 * silencioso seria pior do que perder o atalho.
 */
async function copiar(hex: string): Promise<void> {
  try {
    await navigator.clipboard?.writeText(hex);
  } catch {
    /* sem área de transferência: o hex continua visível na lista para copiar à mão */
  }
}

const styles: Record<string, CSSProperties> = {
  wrap: { overflow: "hidden" },
  summary: {
    cursor: "pointer",
    padding: "8px 12px",
    fontSize: 12,
    color: "var(--text-muted)",
    fontFamily: "var(--font-mono)",
  },
  panel: {
    padding: "var(--space-12) var(--space-20) var(--space-20)",
    display: "flex",
    flexDirection: "column",
    gap: "var(--space-12)",
  },
  header: { display: "flex", alignItems: "center", gap: "var(--space-8)", flexWrap: "wrap" },
  title: { margin: 0, fontSize: "var(--text-sm)", letterSpacing: "0.08em", textTransform: "uppercase" },
  badge: {
    padding: "2px 8px",
    borderRadius: 999,
    fontSize: "var(--text-xs)",
    fontFamily: "var(--font-mono)",
    border: "1px solid transparent",
  },
  badgeMock: {
    color: "var(--text-muted)",
    borderColor: "color-mix(in srgb, var(--text-muted) 40%, transparent)",
  },
  badgeReal: {
    color: "var(--accent-text)",
    borderColor: "color-mix(in srgb, var(--accent) 45%, transparent)",
  },
  hint: { margin: 0, fontSize: "var(--text-sm)", color: "var(--text-muted)" },
  ok: { margin: 0, fontSize: "var(--text-sm)", color: "var(--accent-text)" },
  err: { margin: 0, fontSize: "var(--text-sm)", color: "var(--error)" },
  row: { display: "flex", gap: "var(--space-8)", alignItems: "flex-end", flexWrap: "wrap" },
  field: { display: "grid", gap: "var(--space-4)" },
  grow: { display: "grid", gap: "var(--space-4)", flex: "1 1 160px", minWidth: 0 },
  label: { fontSize: "var(--text-xs)", color: "var(--text-muted)" },
  input: {
    background: "var(--bg)",
    border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
    color: "var(--text)",
    borderRadius: "var(--space-4)",
    padding: "6px 8px",
    minHeight: 32,
    minWidth: 0,
    fontFamily: "var(--font-mono)",
    fontSize: "var(--text-sm)",
  },
  actions: { display: "flex", gap: "var(--space-8)", flexWrap: "wrap" },
  btn: {
    minHeight: 32,
    padding: "6px 10px",
    borderRadius: "var(--space-4)",
    border: "1px solid color-mix(in srgb, var(--accent) 45%, transparent)",
    background: "transparent",
    color: "var(--accent-text)",
    fontSize: "var(--text-sm)",
    cursor: "pointer",
  },
  btnOff: {
    minHeight: 32,
    padding: "6px 10px",
    borderRadius: "var(--space-4)",
    border: "1px solid color-mix(in srgb, var(--text-muted) 40%, transparent)",
    background: "transparent",
    color: "var(--text-muted)",
    fontSize: "var(--text-sm)",
    cursor: "pointer",
  },
  block: { display: "grid", gap: "var(--space-4)" },
  subSummary: {
    cursor: "pointer",
    fontSize: "var(--text-sm)",
    color: "var(--text-muted)",
  },
  list: { margin: 0, padding: 0, listStyle: "none", display: "grid", gap: "var(--space-4)", maxHeight: 180, overflowY: "auto" },
  item: { display: "flex", gap: "var(--space-8)", alignItems: "center", flexWrap: "wrap" },
  hex: {
    fontFamily: "var(--font-mono)",
    fontSize: "var(--text-xs)",
    color: "var(--text-muted)",
    overflowX: "auto",
    whiteSpace: "nowrap",
  },
  copy: {
    minHeight: 32,
    padding: "4px 8px",
    borderRadius: "var(--space-4)",
    border: "1px solid color-mix(in srgb, var(--text-muted) 40%, transparent)",
    background: "transparent",
    color: "var(--text-muted)",
    fontSize: "var(--text-xs)",
    cursor: "pointer",
  },
};