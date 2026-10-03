/**
 * i18n (#30) — o contrato do catálogo de textos.
 *
 * O risco desta feature não é "traduzir errado", é o texto SUMIR: um idioma
 * incompleto que deixa `undefined` na tela, uma chave digitada com erro que o
 * TypeScript não pega (o `DictPatch` é todo opcional), ou um `<option>` vazio.
 * Por isso os testes abaixo atacam a completude e a paridade, não o gosto.
 */
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it } from "vitest";
import App from "../src/App";
import { LANGS, LANG_NAMES, MSG, getLanguage, merge, setLanguage, useLanguage } from "../src/i18n/messages";
import type { Dict, DictPatch, LangCode } from "../src/i18n/messages";
import { PT_BR } from "../src/i18n/dictionaries/pt-BR";
import { EN } from "../src/i18n/dictionaries/en";
import { ES } from "../src/i18n/dictionaries/es";
import { ZH } from "../src/i18n/dictionaries/zh";

const OVERLAYS: Record<Exclude<LangCode, "pt-BR">, DictPatch> = { en: EN, es: ES, zh: ZH };

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

beforeEach(() => {
  // o idioma é estado de MÓDULO (não de componente): sem resetar, um teste
  // deixaria o próximo montar o App já em inglês.
  setLanguage("pt-BR");
  localStorage.clear();
});

/** Percorre um valor do dicionário e devolve todas as strings que ele produz. */
function stringsOf(v: unknown, depth = 0): string[] {
  if (depth > 3) return [];
  if (typeof v === "string") return [v];
  if (typeof v === "function") {
    // Os "formatadores" do catálogo são `(…args) => string`: chamá-los com
    // placeholders é a única forma de provar que o texto interpolado também
    // não pode ser vazio.
    const fn = v as (...a: unknown[]) => unknown;
    return stringsOf(fn("Q", 7, "Q", true, 1, "Q"), depth + 1);
  }
  if (Array.isArray(v)) return v.flatMap((x) => stringsOf(x, depth + 1));
  if (v && typeof v === "object") return Object.values(v).flatMap((x) => stringsOf(x, depth + 1));
  return [];
}

/** Todos os caminhos de folha do catálogo (`settingsTabs.About`,
 *  `aboutData.0.1`, `tunerPowerTitle`…) — a "forma" que o app consome. */
function leafPaths(v: unknown, prefix = "", depth = 0): string[] {
  if (depth > 3) return [prefix];
  if (Array.isArray(v)) return v.flatMap((x, i) => leafPaths(x, `${prefix}.${i}`, depth + 1));
  if (v && typeof v === "object") {
    return Object.entries(v).flatMap(([k, x]) => leafPaths(x, prefix ? `${prefix}.${k}` : k, depth + 1));
  }
  return [prefix];
}

describe("i18n — troca de idioma", () => {
  it("setLanguage troca o catálogo inteiro e volta ao pt-BR", () => {
    expect(getLanguage()).toBe("pt-BR");
    expect(MSG.tagline).toBe(PT_BR.tagline);

    setLanguage("en");
    expect(getLanguage()).toBe("en");
    expect(MSG.tagline).toBe("unofficial editor");
    expect(MSG.openSettingsAria).toBe("Open settings");

    setLanguage("es");
    expect(MSG.tagline).toBe("editor no oficial");

    setLanguage("zh");
    expect(MSG.tagline).toBe("非官方编辑器");

    setLanguage("pt-BR");
    expect(MSG.tagline).toBe(PT_BR.tagline);
  });

  it("useLanguage re-renderiza só quando o idioma MUDA de verdade", () => {
    // setLanguage() idempotente: um selector que dispara change com o mesmo
    // valor não pode redesenhar a casca inteira à toa.
    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    let renders = 0;
    function Probe() {
      useLanguage();
      renders++;
      return null;
    }
    act(() => root.render(<Probe />));
    expect(renders).toBe(1);

    act(() => setLanguage("pt-BR"));
    expect(renders, "mesmo idioma: nada a redesenhar").toBe(1);

    act(() => setLanguage("en"));
    expect(renders).toBe(2);

    act(() => setLanguage("en"));
    expect(renders, "já está em en").toBe(2);

    act(() => root.unmount());
    host.remove();
  });

  it("os 4 idiomas estão registrados com nome no próprio idioma", () => {
    expect(LANGS).toEqual(["pt-BR", "en", "es", "zh"]);
    expect(Object.keys(LANG_NAMES)).toEqual([...LANGS]);
    expect(LANG_NAMES.zh).toBe("中文");
    expect(LANG_NAMES["pt-BR"]).toBe("Português (Brasil)");
  });

  it("os números dos textos (presets, ritmos, algoritmo) vêm do mesmo lugar nos 4", () => {
    // Se um idioma copiasse "99 presets" à mão, o catálogo mudando quebraria
    // a verdade sem nenhum teste reclamar — por isso os fatos são derivados.
    for (const lang of LANGS) {
      setLanguage(lang);
      expect(MSG.drumNote, `drumNote em ${lang}`).toContain(MSG.aboutData[1][1].split(" ")[0]);
    }
    setLanguage("pt-BR");
    expect(MSG.aboutData[0][1]).toMatch(/^99 /);
    expect(MSG.releaseNote).toContain("99");
  });
});

describe("i18n — fallback e completude", () => {
  it("merge preenche sub-objeto parcial com o pt-BR (nunca deixa buraco)", () => {
    const m = merge(PT_BR, { looperMode: { ready: "LISTO" } });
    expect(m.looperMode.ready).toBe("LISTO");
    expect(m.looperMode.rec).toBe(PT_BR.looperMode.rec);
    expect(m.looperMode.stop).toBe(PT_BR.looperMode.stop);
  });

  it("merge substitui array inteiro e função inteira (nada de metade)", () => {
    const soUm = merge(PT_BR, { helpControls: ["só um"] });
    expect(soUm.helpControls).toHaveLength(1);

    const soRetorno = merge(PT_BR, { tunerPowerTitle: () => "Monitor ENCENDIDO" });
    expect(soRetorno.tunerPowerTitle(true)).toBe("Monitor ENCENDIDO");
  });

  it("nenhum idioma deixa valor indefinido ou string vazia", () => {
    for (const lang of LANGS) {
      setLanguage(lang);
      for (const key of Object.keys(PT_BR) as Array<keyof Dict>) {
        const v = MSG[key];
        expect(v, `${lang}.${key} não pode ser undefined`).toBeDefined();
        const texts = stringsOf(v);
        expect(texts.length, `${lang}.${key} não produz texto nenhum`).toBeGreaterThan(0);
        for (const t of texts) {
          expect(t.trim().length, `${lang}.${key} tem texto vazio`).toBeGreaterThan(0);
        }
      }
    }
    setLanguage("pt-BR");
  });

  it("nenhum idioma imprime a chave crua como texto", () => {
    // Uma chave perdida apareceria na tela com o NOME DELA no lugar do texto.
    for (const lang of LANGS) {
      setLanguage(lang);
      for (const key of Object.keys(PT_BR)) {
        const v = (MSG as unknown as Record<string, unknown>)[key];
        for (const t of stringsOf(v)) {
          expect(t, `${lang}.${key} virou chave crua`).not.toBe(key);
        }
      }
    }
    setLanguage("pt-BR");
  });

  it("nenhum idioma deixa buraco nem muda a FORMA do catálogo", () => {
    // Comparar a ÁRVORE inteira (não só o topo) é o que pega um sub-objeto
    // traduzido pela metade: sem isto, `bootStages.scan` cairia no pt-BR sem
    // ninguém notar — ou pior, sumiria se o merge deixasse de ser profundo.
    const base = leafPaths(PT_BR).sort();
    expect(base.length, "o catálogo base tem folhas").toBeGreaterThan(100);
    for (const lang of LANGS) {
      setLanguage(lang);
      expect(leafPaths(MSG as unknown).sort(), `a forma do catálogo mudou em ${lang}`).toEqual(base);
    }
    setLanguage("pt-BR");
  });

  it("os dicionários traduzidos só usam chaves que existem no pt-BR", () => {
    // `DictPatch` é todo opcional: `reelTakeupB: "..."` compila e não faz nada.
    for (const [lang, dict] of Object.entries(OVERLAYS)) {
      for (const key of Object.keys(dict)) {
        expect(Object.keys(PT_BR), `${lang}.${key} não existe no dicionário base`).toContain(key);
      }
      // e nos sub-objetos (`bootStages.scan`, `settingsTabs.About`, …)
      for (const [key, val] of Object.entries(dict)) {
        const base = (PT_BR as unknown as Record<string, unknown>)[key];
        if (!base || typeof base !== "object" || Array.isArray(base) || typeof val === "function") continue;
        for (const sub of Object.keys(val as object)) {
          expect(Object.keys(base as object), `${lang}.${key}.${sub} não existe`).toContain(sub);
        }
      }
    }
  });

  it("os 3 idiomas traduzidos cobrem todas as chaves de topo do pt-BR", () => {
    for (const [lang, dict] of Object.entries(OVERLAYS)) {
      const missing = (Object.keys(PT_BR) as string[]).filter((k) => !(k in dict));
      expect(missing, `faltou traduzir em ${lang}: ${missing.join(", ")}`).toEqual([]);
    }
  });

  it("os termos que o device imprime ficam IGUAIS nos 4 idiomas", () => {
    const deviceTerms = [
      MSG.looperMode.rec, MSG.looperMode.play, MSG.looperMode.dub, MSG.looperMode.stop,
      "PRE", "POST", "REW", "BPM", "L-CUT FREQ", "H-CUT FREQ", "Effects List",
      "Factory Patch", "User Patch", "Supply", "Take-up", "Input Level",
    ];
    setLanguage("pt-BR");
    const base = deviceTerms.map((t) => stringsOf(t).join("|"));
    for (const lang of LANGS) {
      setLanguage(lang);
      expect(deviceTerms.map((t) => stringsOf(t).join("|")), `termos do device em ${lang}`).toEqual(base);
    }
    setLanguage("pt-BR");
  });
});

describe("i18n — o seletor em Settings", () => {
  function mountApp(): { root: Root; host: HTMLElement } {
    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    act(() => root.render(<App />));
    return { root, host };
  }
  async function settle() {
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
  }
  function setSelect(el: HTMLSelectElement, value: string) {
    const setter = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, "value")!.set!;
    setter.call(el, value);
    el.dispatchEvent(new Event("change", { bubbles: true }));
  }
  const teardown = (root: Root, host: HTMLElement) => {
    act(() => root.unmount());
    host.remove();
  };

  it("lista os 4 idiomas, habilitada, e trocar redesenha a casca inteira", async () => {
    const { root, host } = mountApp();
    await settle();

    // a casca começa em português
    expect(host.querySelector('[aria-label="Abrir configurações"]'), "topbar em pt-BR").toBeTruthy();

    act(() => host.querySelector<HTMLElement>('[aria-label="Abrir configurações"]')!
      .dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();

    const dialog = host.querySelector("[role='dialog']") as HTMLElement;
    const sel = dialog.querySelector<HTMLSelectElement>(`select[aria-label="${MSG.languageAria}"]`);
    expect(sel, "seletor de idioma na aba General").toBeTruthy();
    expect(sel!.disabled, "idioma NÃO depende de canal USB — tem que estar habilitada").toBe(false);
    expect(Array.from(sel!.options).map((o) => o.value)).toEqual([...LANGS]);
    expect(Array.from(sel!.options).map((o) => o.textContent)).toEqual(LANGS.map((l) => LANG_NAMES[l]));

    // troca para inglês: o MESMO componente passa a falar inglês
    act(() => setSelect(sel!, "en"));
    await settle();

    expect(MSG.tagline).toBe("unofficial editor");
    expect(
      host.querySelector('[aria-label="Open settings"]'),
      "a topbar inteira redesenhou em inglês",
    ).toBeTruthy();
    expect(JSON.parse(localStorage.getItem("gp100.settings.general.v1")!).language).toBe("en");
    // e o próprio seletor continua lá, agora traduzido
    expect(dialog.querySelector(`select[aria-label="${MSG.languageAria}"]`)!.getAttribute("aria-label")).toBe(
      "App language",
    );

    // volta ao português e a casca volta junto (nada fica "presa" em inglês)
    act(() => setSelect(dialog.querySelector<HTMLSelectElement>(`select[aria-label="${MSG.languageAria}"]`)!, "pt-BR"));
    await settle();
    expect(host.querySelector('[aria-label="Abrir configurações"]')).toBeTruthy();

    teardown(root, host);
  });

  it("reabre o app já no idioma persistido (sem voltar ao pt-BR)", async () => {
    localStorage.setItem(
      "gp100.settings.general.v1",
      JSON.stringify({ language: "zh" }),
    );
    const { root, host } = mountApp();
    await settle();

    expect(getLanguage()).toBe("zh");
    expect(host.querySelector(".pg-tagline")!.textContent).toBe(MSG.tagline);
    expect(MSG.tagline).toBe("非官方编辑器");

    teardown(root, host);
    setLanguage("pt-BR");
  });
});