/**
 * useGlobalShortcuts — atalhos globais da casca:
 *   Espaço = drum play/stop · R = REC do looper · Esc = fecha painéis.
 *
 * Regras (documentadas na aba Help do Settings):
 *  - campos de texto/busca/select NUNCA disparam atalhos de transporte;
 *  - Espaço não rouba a ativação nativa de botões/switches (a11y: Espaço
 *    sobre um botão focado ativa O BOTÃO, não o drum);
 *  - Ctrl/Alt/⌘ + tecla é ignorado; auto-repeat (manter pressionada) é ignorado;
 *  - Esc fecha o painel DO TOPO (precedência decidida no App).
 *
 * Os handlers são relidos a cada render (ref) — closures sempre atuais sem
 * re-registrar o listener.
 */
import { useEffect, useRef } from "react";

type ShortcutKind = "space" | "letter" | "escape";

/** campos onde o usuário digita — atalhos de transporte ficam inativos */
const TEXT_ENTRY = [
  "input",
  "textarea",
  "select",
  '[contenteditable="true"]',
  '[contenteditable=""]',
  '[role="textbox"]',
  '[role="combobox"]',
  '[role="searchbox"]',
  '[role="spinbutton"]',
].join(", ");

/** controles cuja tecla Espaço tem comportamento nativo (ativação) */
const SPACE_NATIVE = [
  "button",
  "a",
  "summary",
  '[role="button"]',
  '[role="switch"]',
  '[role="option"]',
  '[role="tab"]',
  '[role="menuitem"]',
].join(", ");

function matches(target: EventTarget | null, selector: string): boolean {
  return target instanceof Element && target.closest(selector) != null;
}

/** alvo é campo de texto/busca (R e Espaço ignoram) */
export function isTextEntryTarget(target: EventTarget | null): boolean {
  return matches(target, TEXT_ENTRY);
}

/** Espaço deve respeitar o controle focado (a11y: ativação nativa) */
export function isSpaceNativeTarget(target: EventTarget | null): boolean {
  return matches(target, `${TEXT_ENTRY}, ${SPACE_NATIVE}`);
}

/** filtro único aplicado antes de cada ação */
export function shouldHandleShortcut(e: KeyboardEvent, kind: ShortcutKind): boolean {
  if (e.ctrlKey || e.metaKey || e.altKey) return false;
  if (kind === "escape") return true; // Esc sempre disponível (idempotente)
  if (e.repeat) return false; // manter pressionada não alterna transporte
  if (kind === "space") return !isSpaceNativeTarget(e.target);
  return !isTextEntryTarget(e.target); // letter (R)
}

interface GlobalShortcutHandlers {
  onDrumToggle: () => void; // Espaço
  onLooperRec: () => void; // R
  onEscape: () => void; // Esc (painel do topo)
}

export function useGlobalShortcuts(h: GlobalShortcutHandlers): void {
  const ref = useRef<GlobalShortcutHandlers>(h);
  useEffect(() => {
    ref.current = h;
  });

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && shouldHandleShortcut(e, "escape")) {
        ref.current.onEscape();
        return;
      }
      if (e.key === " " && shouldHandleShortcut(e, "space")) {
        ref.current.onDrumToggle();
        return;
      }
      if ((e.key === "r" || e.key === "R") && shouldHandleShortcut(e, "letter")) {
        ref.current.onLooperRec();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
