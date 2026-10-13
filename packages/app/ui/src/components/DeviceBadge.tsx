/**
 * DeviceBadge — indicador de CONEXÃO do aparelho na navbar: um dot de
 * estado (`.live-dot` pulsando quando conectado, `.idle-dot` apagado quando
 * não) + a linha de status honesta.
 *
 * **(dono 12/10) a miniatura em SVG foi REMOVIDA** — "está ruim com svg,
 * remova": o chassi desenhado saiu; ficou o essencial (estado + informação),
 * que é o que os seletores do e2e já esperavam (`.live-dot`/`.idle-dot`
 * dentro do cluster `role="status"` de conexão).
 *
 * **Honestidade de dado (contrato #150).** Só se afirma o que o `device_info`
 * expõe: `backend`, `presetCount` e `writeVerified`. O nome/serial do aparelho
 * NÃO vem do fio (o `currentName` é vazio no real — `REAL_DEVICE_GAP.md`),
 * então a linha de status nunca inventa um nome de aparelho.
 *
 * **`conectado` vem de FORA (App) com o gating do boot (#161)** — a fórmula
 * é `pronto && info != null && info.backend !== "none"`: device_info
 * respondendo NÃO é conexão validada (boot falho mantém o LED off, senão a
 * navbar mentiria sobre um device que o gate recusou).
 *
 * - ON (real): dot verde pulsante, "198 presets · escrita liberada";
 * - ON (mock): dot verde pulsante + badge "Mock Device" (estado simulado);
 * - OFF (none): dot apagado, "sem aparelho" — sem glow, sem animação;
 * - OFF (boot não validado): dot apagado e SÓ o "off" — sem linha que
 *   afirme presença/ausência (o alerta do gate explica o motivo);
 * - null (sem device_info — carregou ou falhou): "off" sem linha extra —
 *   "sem device_info não há conexão" (spec do e2e estados), e sem afirmar
 *   "sem aparelho" (null não prova ausência, só falta de dado).
 *
 * Acessibilidade: o cluster inteiro é `role=\"status\"` (no TopBar — o leitor
 * de tela anuncia mudança de conexão sem roubar foco); o dot é `aria-hidden`
 * porque o texto ao lado carrega o significado. Toda animação respeita
 * `prefers-reduced-motion` (regra do projeto — o CSS do `.live-dot` desliga o
 * pulso).
 */
import type { CSSProperties } from "react";
import type { DeviceInfo } from "../ipc/types";
import { MSG } from "../i18n/messages";

interface Props {
  /** `device_info` corrente; `null` = ainda carregando (dot em repouso). */
  info: DeviceInfo | null;
  /**
   * Conexão VALIDADA pelo boot — `pronto && info != null && backend !== "none"`
   * (o App calcula, mesma fórmula do cluster antigo). Sem este gating o badge
   * mostrava "on" com o boot falho (e2e mid-boot: LED de conectado num device
   * que o gate recusou — mentira de estado).
   */
  conectado: boolean;
}

/* O badge inteiro é compacto (linha da navbar, ~32px de altura). */
const wrap: CSSProperties = { display: "inline-flex", alignItems: "center", gap: "var(--space-8)" };

/* Linha de status: mono, uppercase e tabular — o número não "treme" quando o
 * inventário muda entre um boot e outro. */
const status: CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-xs)",
  letterSpacing: 0.4,
  textTransform: "uppercase",
  color: "var(--text-muted)",
  whiteSpace: "nowrap",
};

/** Números tabulares no meio do status (presetCount). */
const num: CSSProperties = { fontVariantNumeric: "tabular-nums", color: "var(--text)" };

/* badge "Mock Device": chip de estado simulado (mesmo desenho do cluster
   commitado — o CSS só o ESCONDE em telas estreitas, o estilo é daqui) */
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

/* ── o que cada estado do dot significa ────────────────────────────────── */
type Luz = "on" | "mock" | "off";

export function DeviceBadge({ info, conectado }: Props) {
  // luz: SÓ o backend validado pelo boot acende — sem boot pronto, off.
  const luz: Luz = !conectado ? "off" : info?.backend === "real" ? "on" : "mock";
  const carregando = info == null;

  /** A linha de status: o que o device_info GARANTE, sem inventar. */
  const linha = () => {
    if (carregando) return null; // o span de estado acima já diz "lendo…"
    if (!conectado) {
      // sem boot validado a linha não afirma presença ("sem aparelho" só
      // quando o device_info CONFIRMA backend none; o motivo do erro fica
      // no alerta do gate, logo abaixo da navbar)
      return info!.backend === "none" ? MSG.connBadgeNone : null;
    }
    const presets = <span style={num}>{info!.presetCount}</span>;
    const escrita = info!.writeVerified ? MSG.connBadgeWriteOk : MSG.connBadgeWriteLocked;
    return (
      <>
        {presets} {MSG.connBadgePresets} · {escrita}
      </>
    );
  };

  return (
    <span style={wrap} data-conectado={conectado} data-luz={luz}>
      {/* dot de estado: verde pulsando quando o device responde, apagado caso
          contrário (aria-hidden — o texto ao lado carrega o significado) */}
      <span className={conectado ? "live-dot" : "idle-dot"} aria-hidden="true" />
      <span style={{ display: "inline-flex", flexDirection: "column", lineHeight: 1.15, minWidth: 0 }}>
        {/* estado ON/OFF curto — em span PRÓPRIO: o e2e ancora `getByText("on",
            { exact: true })` no cluster de conexão */}
        <span style={{ ...status, color: conectado ? "var(--text)" : "var(--text-muted)" }}>
          {conectado ? MSG.connShortOn : MSG.connShortOff}
        </span>
        {/* a linha informativa (presets + escrita, ou o motivo de estar off) */}
        <span style={status}>{linha()}</span>
      </span>
      {conectado && info?.backend === "mock" && (
        <span className="mock-badge nb-push" style={badge}>
          {MSG.mockBadge}
        </span>
      )}
    </span>
  );
}
