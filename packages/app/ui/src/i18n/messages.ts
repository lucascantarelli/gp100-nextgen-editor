/**
 * i18n/messages — o catálogo que a casca inteira lê.
 *
 * Por que um Proxy e não um contexto React: ~200 textos em ~15 componentes
 * já fazem `import { MSG } from "../i18n/messages"`. Passar a exigir um
 * provider/`useT()` em cada um deles seria uma migração que só pagaria na
 * próxima issue. O Proxy mantém a assinatura atual e troca o idioma inteiro
 * com UMA linha: `setLanguage("en")` + o App re-renderizar.
 *
 * Como o fallback funciona: cada idioma é `Partial<Dict>`, e o que falta nele
 * (ou num sub-objeto como `bootStages`) é preenchido pelo pt-BR no momento em
 * que o registro é montado. Um idioma incompleto NUNCA mostra chave crua nem
 * string vazia — degrada para português, que é a língua garantida.
 */
import { useSyncExternalStore } from "react";
import { PT_BR } from "./dictionaries/pt-BR";
import type { Dict, DictPatch } from "./dictionaries/pt-BR";
import { EN } from "./dictionaries/en";
import { ES } from "./dictionaries/es";
import { ZH } from "./dictionaries/zh";

export type { Dict, DictPatch };
export type LangCode = "pt-BR" | "en" | "es" | "zh";

export const LANGS: readonly LangCode[] = ["pt-BR", "en", "es", "zh"] as const;
/** nome do idioma no próprio idioma (nunca traduzido) */
export const LANG_NAMES: Record<LangCode, string> = {
  "pt-BR": "Português (Brasil)",
  en: "English",
  es: "Español",
  zh: "中文",
};

type Loose = Record<string, unknown>;
const isPlainObject = (v: unknown): v is Loose =>
  typeof v === "object" && v !== null && !Array.isArray(v);

/**
 * Merge PROFUNDO sobre o pt-BR. Só combina objetos simples: função continua
 * função (traduzir a função é substituir a função inteira, não mesclar
 * campos) e array é substituído (metade de uma lista de dados traduzida é
 * pior do que a lista inteira em pt-BR).
 */
export function merge(base: Dict, overlay: DictPatch): Dict {
  const out: Loose = { ...base };
  for (const [k, v] of Object.entries(overlay as Loose)) {
    const b = out[k];
    out[k] = isPlainObject(b) && isPlainObject(v) ? { ...b, ...v } : v;
  }
  return out as Dict;
}

const DICTS: Record<LangCode, Dict> = {
  "pt-BR": PT_BR,
  en: merge(PT_BR, EN),
  es: merge(PT_BR, ES),
  zh: merge(PT_BR, ZH),
};

let active: LangCode = "pt-BR";
const listeners = new Set<() => void>();

export function getLanguage(): LangCode {
  return active;
}

/** Troca o idioma e avisa quem está assinado (o App, via `useLanguage`). */
export function setLanguage(next: LangCode): void {
  if (next === active || !DICTS[next]) return;
  active = next;
  listeners.forEach((fn) => fn());
}

/**
 * O catálogo ativo. A assinatura é a de sempre (`MSG.algo`), o tipo é o do
 * pt-BR — então trocar o idioma não pode quebrar a tipagem de nenhum
 * componente, e nenhuma string pode sumir do tipo.
 */
export const MSG: Dict = new Proxy(PT_BR, {
  get(_t, prop: string) {
    return (DICTS[active] as Loose)[prop];
  },
}) as Dict;

/**
 * Assina a troca de idioma. Só o App precisa: ele é a raiz da árvore, e um
 * re-render dele redesenha todos os `MSG.*` sem que nenhum outro componente
 * saiba que existe idioma.
 */
export function useLanguage(): LangCode {
  return useSyncExternalStore(
    (fn) => {
      listeners.add(fn);
      return () => listeners.delete(fn);
    },
    getLanguage,
    () => "pt-BR" as LangCode,
  );
}