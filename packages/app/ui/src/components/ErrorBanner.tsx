/**
 * ErrorBanner — a faixa de erro transitória com AÇÃO de recuperação.
 *
 * Tirada do `App.tsx` pela #82, junto com o `BootProgressBar`: as duas faixas
 * do topo são o mesmo objeto com dois conteúdos (erro e progresso), e ambas
 * estavam enterradas em `style` inline de três níveis dentro do componente que
 * só devia orquestrar.
 *
 * **Por que a ação é obrigatória** (issue #20): o banner sem `retry` é um
 * spinner eterno — o usuário vê que algo falhou e não tem como sair. O
 * `retry` chega PRONTO do hook (`useStage`), que sabe reexecutar exatamente a
 * operação que falhou; este componente não sabe (e não deve saber) o que é
 * "reabrir o preset" nem "reaplicar o knob".
 */
import { MSG } from "../i18n/messages";

interface ErrorBannerProps {
  message: string;
  /** Reexecuta a operação que falhou — vem do hook, não é inventado aqui. */
  onRetry: () => void;
}

const retryBtn: React.CSSProperties = {
  // alvo ≥32px — WCAG 2.5.8 (a11y)
  minHeight: 32,
  padding: "var(--space-4) var(--space-12)",
  borderRadius: "var(--space-4)",
  border: "1px solid currentColor",
  background: "transparent",
  color: "inherit",
  cursor: "pointer",
  font: "inherit",
  whiteSpace: "nowrap",
};

const banner: React.CSSProperties = {
  background: "#2a1414",
  border: "1px solid #5b2626",
  color: "#ffb3b3",
  padding: "8px 12px",
  borderRadius: 8,
  fontSize: 13,
  display: "flex",
  alignItems: "center",
  gap: "var(--space-12)",
};

export function ErrorBanner({ message, onRetry }: ErrorBannerProps) {
  return (
    <div role="alert" style={banner}>
      <span>{message}</span>
      <button type="button" onClick={onRetry} aria-label={MSG.errRetryAria} style={retryBtn}>
        {MSG.errRetry}
      </button>
    </div>
  );
}
