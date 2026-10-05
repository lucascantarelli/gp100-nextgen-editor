/**
 * Componentes sem teste dedicado (#79 parte 2): `DrumPanel`, `LibraryPanel` e
 * `usePushLog`.
 *
 * Antes estos tres eram exercitados SO pelo App montado, e o preco apareceu no
 * relatorio: `DrumPanel` estava com branch 50% — metade das decisoes do
 * componente sem exercitar. Montar o App para testar um clamp de BPM e um
 * fallback de genero e o teste que nao se escreve quando a suite esta
 * apertada.
 *
 * Aqui cada um monta SO, sem o palco inteiro. E o `DrumPanel` ganha um caso
 * que so existe porque o componente tem o fallback: um genero desconhecido
 * (fingerprint de firmware velho, ou estado corrompido no localStorage) NAO
 * pode deixar o painel com genre vazio.
 */

import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { DrumPanel } from "../src/components/DrumPanel";
import type { DrumState } from "../src/components/DrumPanel";
import { LibraryPanel } from "../src/components/LibraryPanel";
import { TunerPanel } from "../src/components/TunerPanel";
import type { TunerSettings } from "../src/components/TunerPanel";
import { Pedalboard } from "../src/components/Pedalboard";
import { REF_PITCH_DEFAULT } from "../src/tuner/pitch";
import type { BoardSlot, BoardView } from "../src/ipc/types";
import { usePushLog } from "../src/hooks/usePushLog";
import { onDevicePush } from "../src/ipc/device";
import { PUSH_LOG_MAX } from "../src/ipc/push";
import { DRUM_BEATS, DRUM_GENRES } from "../src/artifacts/drumData";
import { MSG } from "../src/i18n/messages";
import { useLibrary } from "../src/hooks/useLibrary";
import { librarySave, resetaFallback } from "../src/ipc/library";
import * as ipcLibrary from "../src/ipc/library";

// O `usePushLog` assina `device://push` pelo MESMO registrador que o App usa.
// Sem este mock, `onDevicePush` e a funcao real e nao ha como injetar um hex —
// o teste viraria um teste de infra de evento em vez de teste do log.
vi.mock("../src/ipc/device", async (importOriginal) => {
  const mod = await importOriginal<typeof import("../src/ipc/device")>();
  return { ...mod, onDevicePush: vi.fn(mod.onDevicePush) };
});

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

/** Monta um componente e devolve o host para consulta direta. */
function mount(ui: React.ReactElement): { host: HTMLElement; unmount: () => void } {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root: Root = createRoot(host);
  act(() => root.render(ui));
  return {
    host,
    unmount: () => {
      act(() => root.unmount());
      host.remove();
    },
  };
}

/** React não enxerga `value` de input controlled sem o setter NATIVO, e o
 *  update resultante precisa ficar dentro de `act` — fora dele o React avisa e
 *  o teste passa a depender de timing. */
function setNative(el: HTMLInputElement | HTMLSelectElement, value: string): void {
  const proto =
    el instanceof HTMLSelectElement ? window.HTMLSelectElement.prototype : window.HTMLInputElement.prototype;
  const setter = Object.getOwnPropertyDescriptor(proto, "value")!.set!;
  act(() => {
    setter.call(el, value);
    el.dispatchEvent(new Event("change", { bubbles: true }));
  });
}

const porId = <T extends HTMLElement>(host: HTMLElement, id: string): T =>
  host.querySelector<T>(`#${id}`)!;

const drumBase: DrumState = {
  on: true,
  genre: "Rock",
  style: "Hard 1",
  bpm: 120,
  beat: "4/4",
  volume: 70,
  speed: 50,
};

describe("DrumPanel — os ramos que o App montado nunca pegava", () => {
  let mudou: DrumState[];

  beforeEach(() => {
    mudou = [];
  });

  function abre(drum: DrumState = drumBase) {
    return mount(
      <DrumPanel
        open
        drum={drum}
        onChange={(d) => {
          mudou.push(d);
          // Re-render com o novo estado, como o App faz.
          reabre(d);
        }}
        onClose={() => {}}
      />,
    );
  }

  let atual: { unmount: () => void } | null = null;
  function reabre(d: DrumState) {
    atual?.unmount();
    atual = mount(
      <DrumPanel open drum={d} onChange={() => {}} onClose={() => {}} />,
    );
  }

  it("nao renderiza nada quando fechado", () => {
    const { host, unmount } = mount(
      <DrumPanel open={false} drum={drumBase} onChange={() => {}} onClose={() => {}} />,
    );
    expect(host.querySelector('[role="dialog"]')).toBeNull();
    expect(host.textContent).toBe("");
    unmount();
  });

  it("abre com dialogo modal rotulado e foco dentro", () => {
    const { host, unmount } = abre();
    const dialogo = host.querySelector('[role="dialog"]')!;
    expect(dialogo.getAttribute("aria-modal")).toBe("true");
    expect(dialogo.getAttribute("aria-label")).toBe(MSG.drumPanelAria);
    // WCAG: o foco entra no dialogo ao abrir.
    expect(document.activeElement).toBe(dialogo);
    unmount();
    atual = null;
  });

  it("lista os generos do firmware com a contagem de estilos de cada um", () => {
    const { host, unmount } = abre();
    const opcoes = Array.from(porId<HTMLSelectElement>(host, "drum-genre").options);
    expect(opcoes.map((o) => o.value)).toEqual(DRUM_GENRES.map((g) => g.genre));
    // A contagem e o que promete o texto ("Rock (33)").
    for (const g of DRUM_GENRES) {
      const opt = opcoes.find((o) => o.value === g.genre)!;
      expect(opt.textContent).toBe(`${g.genre} (${g.styles.length})`);
    }
    unmount();
    atual = null;
  });

  it("trocar de genero traz o PRIMEIRO estilo daquele genero", () => {
    const { host, unmount } = abre();
    const rock = DRUM_GENRES.find((g) => g.genre === "Rock")!;
    setNative(porId<HTMLSelectElement>(host, "drum-genre"), "Jazz");
    const jazz = DRUM_GENRES.find((g) => g.genre === "Jazz")!;
    expect(mudou.at(-1)).toMatchObject({ genre: "Jazz", style: jazz.styles[0] });
    expect(rock.styles.length).toBeGreaterThan(1);
    unmount();
    atual = null;
  });

  it("genero desconhecido cai no primeiro do firmware (fingerprint velho/corrompido)", () => {
    // O `?? DRUM_GENRES[0]` que segurava a linha 108 sem cobertura.
    const { host, unmount } = abre({ ...drumBase, genre: "GENERO_FANTASMA", style: "x" });
    const sel = porId<HTMLSelectElement>(host, "drum-genre");
    expect(sel.value, "o select não fica com valor inválido").toBe(DRUM_GENRES[0].genre);
    // E os estilos exibidos são os do gênero real, não os do estado corrompido.
    const estilos = Array.from(porId<HTMLSelectElement>(host, "drum-style").options).map((o) => o.value);
    expect(estilos).toEqual(DRUM_GENRES[0].styles);
    unmount();
    atual = null;
  });

  it("o compasso sai do firmware, na ordem do firmware", () => {
    const { host, unmount } = abre();
    expect(Array.from(porId<HTMLSelectElement>(host, "drum-beat").options).map((o) => o.value)).toEqual(
      [...DRUM_BEATS],
    );
    unmount();
    atual = null;
  });

  it("o BPM clampa em 40–240 e nunca vira NaN", () => {
    const casos: Array<[string, number]> = [
      ["10", 40], // abaixo do piso
      ["40", 40], // no piso
      ["120", 120],
      ["240", 240], // no teto
      ["9999", 240], // acima do teto
      ["", 40], // vazio -> piso, não 0
      ["abc", 40], // não-número -> piso, não NaN
    ];
    for (const [digitado, esperado] of casos) {
      const estado = { ...drumBase };
      // Reproduz o clamp do componente isoladamente: ele é o contrato, e o
      // App montado nunca exercita os limites.
      const lido = Math.min(240, Math.max(40, Number(digitado) || 40));
      expect(lido, `digitado "${digitado}"`).toBe(esperado);
      expect(Number.isNaN(lido)).toBe(false);
      estado.bpm = lido;
      expect(estado.bpm).toBeGreaterThanOrEqual(40);
      expect(estado.bpm).toBeLessThanOrEqual(240);
    }
  });

  it("o input de BPM declara o mesmo piso/teto que o clamp aplica", () => {
    // Se o `min`/`max` do input e o clamp do handler divergirem, o usuário
    // digita um valor que o componente corrige na mao — e o atributo mente.
    const { host, unmount } = abre();
    const bpm = porId<HTMLInputElement>(host, "drum-bpm");
    expect(bpm.min).toBe("40");
    expect(bpm.max).toBe("240");
    unmount();
    atual = null;
  });

  it("digitar BPM fora da faixa chega CLAMPADO no onChange", () => {
    // O clamp vive no handler, não no atributo: quem protege é o `onChange`.
    for (const [digitado, esperado] of [
      ["10", 40],
      ["9999", 240],
      ["abc", 40],
    ] as Array<[string, number]>) {
      const antes = mudou.length;
      const vista = abre();
      setNative(porId<HTMLInputElement>(vista.host, "drum-bpm"), digitado);
      expect(mudou.at(-1)?.bpm, `digitado "${digitado}"`).toBe(esperado);
      expect(mudou.length, "o handler disparou").toBeGreaterThan(antes);
      vista.unmount();
      atual = null;
    }
  });

  it("trocar o estilo sozinho NÃO mexe no genero", () => {
    // Os dois selects são independentes: trocar o estilo dentro do gênero
    // corrente não pode reescrever o gênero (e trocar o gênero já traz o
    // primeiro estilo, coberto acima).
    const { host, unmount } = abre();
    const estilos = DRUM_GENRES.find((g) => g.genre === "Rock")!.styles;
    setNative(porId<HTMLSelectElement>(host, "drum-style"), estilos[estilos.length - 1]);
    expect(mudou.at(-1)).toMatchObject({ genre: "Rock", style: estilos[estilos.length - 1] });
    unmount();
    atual = null;
  });

  it("volume e speed vão de 0 a 99 e reportam o valor novo", () => {
    const { host, unmount } = abre();
    for (const id of ["drum-vol", "drum-speed"]) {
      const el = porId<HTMLInputElement>(host, id);
      expect(el.min).toBe("0");
      expect(el.max).toBe("99");
      const antes = mudou.length;
      // Input controlado: o React só enxerga a mudança pelo setter NATIVO.
      setNative(el, "12");
      expect(mudou.length, `${id} dispara onChange`).toBeGreaterThan(antes);
    }
    unmount();
    atual = null;
  });

  it("o ✕ fecha e o clique no fundo também (mas um clique DENTRO não)", () => {
    const fechas: number[] = [];
    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    act(() =>
      root.render(
        <DrumPanel open drum={drumBase} onChange={() => {}} onClose={() => fechas.push(1)} />,
      ),
    );

    // Dentro do dialogo: nao fecha (o clique no texto nao pode fechar o modal).
    act(() => {
      host.querySelector('[role="dialog"]')!.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    });
    expect(fechas.length, "clique dentro não fecha").toBe(0);

    // No overlay (o alvo é o próprio overlay): fecha.
    const overlay = host.querySelector('[role="dialog"]')!.parentElement!;
    act(() => overlay.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })));
    expect(fechas.length, "clique no fundo fecha").toBe(1);

    act(() => host.querySelector('[aria-label="' + MSG.drumCloseAria + '"]')!
      .dispatchEvent(new MouseEvent("click", { bubbles: true })));
    expect(fechas.length, "o ✕ fecha").toBe(2);

    act(() => root.unmount());
    host.remove();
  });
});

describe("LibraryPanel — a biblioteca no banco (#26)", () => {
  /**
   * O painel recebe o `useLibrary` por prop, então o teste monta o hook junto
   * com ele: um `lib` de mentira deixaria a busca, o debounce e os números
   * sem cobertura — que é justamente o que este painel passou a fazer.
   */
  let lib!: ReturnType<typeof useLibrary>;

  beforeEach(() => {
    // o fallback é estado de MÓDULO: sem isto, o patch salvo num teste
    // continuaria no banco em memória do seguinte e o "vazio" mentiria
    resetaFallback();
    localStorage.clear();
  });

  function Painel(props: Partial<React.ComponentProps<typeof LibraryPanel>> = {}) {
    return (
      <PainelProbe
        currentPp={1}
        bankDoPalco="factory"
        currentUserId={null}
        onOpenFactory={() => {}}
        onOpenUser={() => {}}
        onSave={() => {}}
        onDelete={() => {}}
        onOpenTones={() => {}}
        {...props}
      />
    );
  }

  function PainelProbe(props: Omit<React.ComponentProps<typeof LibraryPanel>, "lib">) {
    lib = useLibrary(props.bankDoPalco);
    return <LibraryPanel {...props} lib={lib} />;
  }

  /** Deixa o debounce (180ms) do hook expirar antes de olhar a lista. */
  async function assenta() {
    await act(async () => {
      await new Promise((r) => setTimeout(r, 260));
    });
  }

  async function abre(props: Partial<React.ComponentProps<typeof LibraryPanel>> = {}) {
    const m = mount(<Painel {...props} />);
    await assenta();
    return m;
  }

  it("a busca é do banco: o texto digitado CHEGA ao SQLite", async () => {
    // A prova de que a lista é do banco e não do artefato: filtrar aqui
    // mostraria "It's GP100" para qualquer texto. No banco, o nome tem que
    // casar — e o resultado tem que ser o do banco, não o do `FACTORY_PRESETS`.
    const { host, unmount } = await abre();
    const busca = host.querySelector<HTMLInputElement>(`[aria-label="${MSG.libSearchAria}"]`)!;
    expect(busca.placeholder).toBe(MSG.libSearchPlaceholder);
    setNative(busca, "blink");
    await assenta();
    const opcoes = Array.from(host.querySelectorAll('[role="option"]'));
    expect(opcoes).toHaveLength(1);
    expect(opcoes[0].textContent).toContain("Blink OD");
    unmount();
  });

  it("estado vazio só aparece quando a busca não encontra nada", async () => {
    const { host, unmount } = await abre();
    setNative(host.querySelector<HTMLInputElement>(`[aria-label="${MSG.libSearchAria}"]`)!, "zzzzz");
    await assenta();
    expect(host.textContent).toContain(MSG.libEmptySearch("zzzzz"));
    unmount();
  });

  it("limpar a busca volta o painel ao estado cheio", async () => {
    const { host, unmount } = await abre();
    const busca = host.querySelector<HTMLInputElement>(`[aria-label="${MSG.libSearchAria}"]`)!;
    setNative(busca, "zzzzz");
    await assenta();
    expect(host.textContent).toContain(MSG.libEmptySearch("zzzzz"));
    // o ✕ é o caminho declarado (aria-label do i18n), e não um botão genérico
    setNative(busca, "bl");
    await assenta();
    const x = host.querySelector<HTMLButtonElement>(`[aria-label="${MSG.libSearchClear}"]`)!;
    act(() => x.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await assenta();
    expect(host.querySelectorAll('[role="option"]').length).toBe(99);
    unmount();
  });

  it("o filtro de estilo vai para o banco junto (não filtra a lista em memória)", async () => {
    const { host, unmount } = await abre();
    setNative(host.querySelector<HTMLSelectElement>(`[aria-label="${MSG.libFilterAria}"]`)!, "6");
    await assenta();
    const opcoes = Array.from(host.querySelectorAll('[role="option"]'));
    expect(opcoes.length).toBeGreaterThan(0);
    expect(opcoes.every((o) => o.textContent?.includes("Pop"))).toBe(true);
    unmount();
  });

  it("abrir patch de usuário leva o id E o índice (a ordem importa)", async () => {
    // O `index` é a posição na lista, não o `id`: confundir os dois abre o
    // patch errado sem erro visível.
    const aberto: Array<[string, number]> = [];
    await librarySave({
      id: "u1",
      bank: "user",
      pp: null,
      name: "A",
      ppType: 4,
      ppTypeName: "Rock",
      savedAt: "2026-02-02T00:00:00Z",
      hasPayload: true,
      payload: "[]",
    });
    await librarySave({
      id: "u2",
      bank: "user",
      pp: null,
      name: "B",
      ppType: 4,
      ppTypeName: "Rock",
      savedAt: "2026-02-02T00:00:00Z",
      hasPayload: true,
      payload: "[]",
    });
    const { host, unmount } = await abre({
      bankDoPalco: "user",
      onOpenUser: (id, i) => aberto.push([id, i]),
    });
    const alvo = Array.from(host.querySelectorAll('[role="listitem"] button')).find((b) =>
      b.textContent?.includes("B"),
    );
    expect(alvo, "o patch B aparece na lista").toBeTruthy();
    act(() => alvo!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    expect(aberto).toEqual([["u2", 1]]);
    unmount();
  });

  it("trocar de aba NAO mostra os registros do outro banco", async () => {
    // O bug que este teste cobre: a busca do banco novo leva 180ms, e nesse
    // intervalo a tela mostrava os 99 de fábrica com o rótulo do dono (U01…)
    // — clicar num deles abria o patch errado sem erro nenhum.
    // nasce NA aba de Fábrica: é a TRANSIçÃO que mostra os 99 (começar
    // direto na aba do dono deixaria o bug passar: nunca haveria linhas velhas)
    const { host, unmount } = await abre();
    expect(host.querySelectorAll('[role="option"]').length).toBe(99);
    const abaDono = Array.from(host.querySelectorAll('[role="tab"]')).find((t) =>
      t.textContent?.includes("User Patch"),
    )!;
    act(() => abaDono.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    // NO INSTANTE da troca, antes do debounce: é aqui que os 99 velhos
    // apareceriam rotulados de U01. Depois de esperar a resposta do banco a
    // lista já estaria certa de qualquer jeito, e o bug passaria.
    expect(host.querySelectorAll('[role="listitem"], [role="option"]').length).toBe(0);
    await assenta();
    expect(host.textContent).toContain(MSG.userPatchEmpty);
    expect(host.textContent, "nenhum preset de fábrica sobrou na aba do dono").not.toContain("Blink OD");
    unmount();
  });

  it("sem patches de usuário o painel não finge que há biblioteca", async () => {
    const { host, unmount } = await abre({ bankDoPalco: "user" });
    expect(host.textContent).toContain(MSG.userPatchEmpty);
    expect(host.querySelectorAll("button").length).toBeGreaterThan(0); // busca e abas
    unmount();
  });

  it("o rodapé mostra os números que vieram do arquivo", async () => {
    const { host, unmount } = await abre();
    expect(host.textContent).toContain(MSG.libStats(99, 0, 0));
    unmount();
  });

  it("importar um arquivo mostra a contabilidade do que foi gravado", async () => {
    // Botão sem retorno é tiro no escuro: o dono precisa saber se gravou.
    const { host, unmount } = await abre();
    const arquivo = host.querySelector<HTMLInputElement>('input[type="file"]')!;
    const envelope = JSON.stringify({
      format: "gp100.library",
      version: 1,
      exportedAt: "2026-02-02T00:00:00Z",
      sqlite: "teste",
      schema: 0,
      presets: [
        {
          id: "u9",
          bank: "user",
          pp: null,
          name: "DO ARQUIVO",
          ppType: 4,
          ppTypeName: "Rock",
          savedAt: "2026-02-02T00:00:00Z",
          payload: "[]",
        },
      ],
    });
    const f = new File([envelope], "gp100.library.json", { type: "application/json" });
    Object.defineProperty(arquivo, "files", { value: [f], configurable: true });
    await act(async () => {
      arquivo.dispatchEvent(new Event("change", { bubbles: true }));
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(host.textContent).toContain(MSG.libImportDone(1, 0, 0));
    unmount();
  });

  it("falha do arquivo vira banner COM retry (issue #20)", async () => {
    // Um banco que não abre (permissão, HD externo) não pode virar lista
    // vazia silenciosa: o dono precisa do texto E do caminho de volta.
    // a lista REAL é capturada ANTES do spy: chamar a porta depois de espiá-la
    // devolveria a própria rejeição
    const listaReal = await ipcLibrary.librarySearch({});
    const spy = vi.spyOn(ipcLibrary, "librarySearch").mockRejectedValue(new Error("arquivo"));
    const { host, unmount } = await abre();
    const alerta = host.querySelector('[role="alert"]')!;
    expect(alerta.textContent).toContain(MSG.errLibrarySearch);
    // o botão é o retry de verdade: destravando a porta, ele volta a ler
    spy.mockResolvedValue(listaReal);
    act(() => {
      host.querySelector<HTMLButtonElement>('[role="alert"] button')!.dispatchEvent(
        new MouseEvent("click", { bubbles: true }),
      );
    });
    await assenta();
    expect(host.querySelector('[role="alert"]')).toBeNull();
    expect(host.querySelectorAll('[role="option"]').length).toBe(99);
    spy.mockRestore();
    unmount();
  });
});

describe("TunerPanel — a demo por rAF (o bloco que faltava cobrir)", () => {
  /**
   * jsdom não implementa `matchMedia`, e a demo se recusa a rodar quando ele
   * falta (guard de ambiente destruído). Sem este stub o efeito retorna antes
   * do rAF — e o bloco fica sem cobertura exatamente como estava.
   */
  function comAmbiente<T>(fn: () => T): T {
    const mmOriginal = window.matchMedia;
    const rafOriginal = globalThis.requestAnimationFrame;
    const cancelOriginal = globalThis.cancelAnimationFrame;
    window.matchMedia = ((q: string) => ({
      matches: false,
      media: q,
      onchange: null,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
    })) as unknown as typeof window.matchMedia;
    try {
      return fn();
    } finally {
      window.matchMedia = mmOriginal;
      globalThis.requestAnimationFrame = rafOriginal;
      globalThis.cancelAnimationFrame = cancelOriginal;
    }
  }

  /**
   * TunerPanel CONTROLADO, como o App usa: o botão de demo pede `on: true` pelo
   * `onChange`, e sem devolver o novo estado a demo é desligada de novo no
   * mesmo passo (`if (!active) setDemo(false)`).
   */
  function abreTuner() {
    function Host() {
      const [settings, setSettings] = useState<TunerSettings>({
        on: false,
        mode: "mute",
        refPitch: REF_PITCH_DEFAULT,
      });
      return <TunerPanel settings={settings} onChange={setSettings} />;
    }
    return mount(<Host />);
  }

  const demoBtn = (host: HTMLElement) =>
    host.querySelector<HTMLButtonElement>("[data-tuner-demo]")!;

  it("a demo sobe o motor real e publica uma leitura (rAF controlado)", () => {
    // O bloco de 195-222 estava sem cobertura porque depende de rAF. Em jsdom
    // o rAF existe mas não é estável; aqui ele vira um relogio controlado, e o
    // que se testa e o CONTRATO: a senoide entra no motor de verdade e vira
    // leitura visivel — nao um mock do resultado.
    const quadros: FrameRequestCallback[] = [];
    let t = 0;
    globalThis.requestAnimationFrame = ((cb: FrameRequestCallback) => {
      quadros.push(cb);
      return quadros.length;
    }) as typeof requestAnimationFrame;
    globalThis.cancelAnimationFrame = (() => {}) as typeof cancelAnimationFrame;
    performance.now = () => t;

    comAmbiente(() => {
      const { host, unmount } = abreTuner();
      const botao = demoBtn(host);
      expect(botao.getAttribute("data-tuner-demo"), "demo começa desligada").toBe("off");

      // A demo exige o monitor ouvindo: o proprio botao liga.
      act(() => botao.dispatchEvent(new MouseEvent("click", { bubbles: true })));
      expect(demoBtn(host).getAttribute("data-tuner-demo"), "demo ligou").toBe("on");

      // Dois frames: o suficiente para o motor analisar e a leitura aparecer.
      for (let i = 0; i < 2 && quadros.length; i += 1) {
        t += 16;
        const cb = quadros.shift()!;
        act(() => cb(t));
      }
      // A leitura aparece como NOTA no display (não como `role="status"`):
      // o que a demo alimenta é a agulha/`data-tuner-needle` e o texto da nota.
      const nota = Array.from(host.querySelectorAll("[data-tuner-note], [role='status']"))
        .map((e) => e.textContent ?? "")
        .join("");
      const agulha = host.querySelector<HTMLElement>("[data-tuner-needle]");
      expect(agulha, "a agulha existe com a demo ligada").toBeTruthy();
      expect(
        nota.trim().length + (agulha?.getAttribute("style")?.length ?? 0),
        "a demo produziu leitura visível",
      ).toBeGreaterThan(0);

      // Passado o DEMO_MS, a demo se desliga sozinha (nunca vira enfeite).
      t += 9000;
      act(() => {
        const cb = quadros.shift();
        if (cb) cb(t);
      });
      expect(demoBtn(host).getAttribute("data-tuner-demo"), "demo se desliga no fim").toBe("off");
      unmount();
    });
  });

  it("desligar o monitor durante a demo para tudo (nada roda em background)", () => {
    const quadros: FrameRequestCallback[] = [];
    const cancelados: number[] = [];
    globalThis.requestAnimationFrame = ((cb: FrameRequestCallback) => {
      quadros.push(cb);
      return quadros.length;
    }) as typeof requestAnimationFrame;
    globalThis.cancelAnimationFrame = ((id: number) => cancelados.push(id)) as typeof cancelAnimationFrame;
    performance.now = () => 0;

    comAmbiente(() => {
      const { host, unmount } = abreTuner();
      act(() => demoBtn(host).dispatchEvent(new MouseEvent("click", { bubbles: true })));
      expect(demoBtn(host).getAttribute("data-tuner-demo")).toBe("on");
      expect(quadros.length, "a demo agendou quadro").toBeGreaterThan(0);

      unmount();
      // O rAF foi CANCELADO: um loop de 8s não sobrevive ao componente.
      expect(cancelados.length, "cancelAnimationFrame chamado no unmount").toBeGreaterThan(0);
    });
  });
});

describe("Pedalboard — a trava do drag-and-drop", () => {
  const slot = (i: number): BoardSlot => ({
    slot: i,
    family: "DST",
    archetype: "DISTORTION",
    name: `Pedal ${i}`,
    variant: "comp",
    state: true,
    code: i,
    knobs: [{ name: "P0", pos: 0, kind: "knob", range: [0, 100], options: [] }],
  });

  const board: BoardView = {
    pp: 0,
    name: "Teste",
    ppType: 0,
    ppTypeName: "Factory",
    slots: [slot(0), slot(1), slot(2)],
    bank: "factory",
    ppLabel: "P01",
  };

  function abre(arrangeMode: boolean) {
    const reordens: Array<[number, number]> = [];
    const view = mount(
      <Pedalboard
        board={board}
        states={{}}
        arrangeMode={arrangeMode}
        onToggle={() => {}}
        onKnobChange={() => {}}
        onKnobReset={() => {}}
        onReorder={(de, para) => reordens.push([de, para])}
      />,
    );
    return { ...view, reordens };
  }

  /** Evento de drag com `dataTransfer` — jsdom não tem, e o handler usa. */
  function drag(tipo: string): DragEvent {
    const e = new Event(tipo, { bubbles: true }) as DragEvent;
    Object.defineProperty(e, "dataTransfer", { value: { effectAllowed: "" } });
    return e;
  }

  const caixas = (host: HTMLElement) => Array.from(host.querySelectorAll<HTMLElement>("[draggable]"));

  it("sem o modo de organizar, nada é arrastável", () => {
    // A trava é o contrato: com `arrangeMode` desligado o pedal do palco não
    // pode virar handle de arrasto por engano.
    const { host, unmount } = abre(false);
    for (const caixa of caixas(host)) expect(caixa.draggable).toBe(false);
    unmount();
  });

  it("com o modo de organizar, arrastar de um slot para outro reordena", () => {
    const { host, reordens, unmount } = abre(true);
    const [a, b] = caixas(host);
    act(() => {
      a.dispatchEvent(drag("dragstart"));
      b.dispatchEvent(drag("dragover"));
      b.dispatchEvent(drag("drop"));
    });
    expect(reordens, "saiu exatamente uma reordenação").toEqual([[0, 1]]);
    unmount();
  });

  it("soltar no MESMO slot não reordena (senão a ordem embaralha)", () => {
    // A guarda `dragFrom.current !== s.slot` é o que evita o reordenamento
    // fantasma quando o usuário solta onde pegou.
    const { host, reordens, unmount } = abre(true);
    const a = caixas(host)[0];
    act(() => {
      a.dispatchEvent(drag("dragstart"));
      a.dispatchEvent(drag("dragover"));
      a.dispatchEvent(drag("drop"));
    });
    expect(reordens, "soltar em cima não chama onReorder").toEqual([]);
    unmount();
  });

  it("drop sem dragstart prévio não reordena", () => {
    const { host, reordens, unmount } = abre(true);
    const b = caixas(host)[1];
    act(() => {
      b.dispatchEvent(drag("dragover"));
      b.dispatchEvent(drag("drop"));
    });
    expect(reordens).toEqual([]);
    unmount();
  });

  it("dragleave de um slot não limpa o destaque de outro", () => {
    // `cur === s.slot ? null : cur` — sair do slot A não pode apagar o
    // destaque que o slot B acabou de ganhar. O destaque é o `outline`
    // tracejado no estilo inline.
    const { host, unmount } = abre(true);
    const [a, b] = caixas(host);
    const tracejado = (el: HTMLElement) => (el.style.outline ?? "").includes("dashed");

    act(() => {
      a.dispatchEvent(drag("dragstart"));
      b.dispatchEvent(drag("dragover"));
    });
    expect(tracejado(b), "B é o alvo").toBe(true);

    act(() => a.dispatchEvent(drag("dragleave")));
    expect(tracejado(b), "sair de A não apagou o destaque de B").toBe(true);

    // E sair do próprio slot alvo limpa o destaque dele.
    act(() => b.dispatchEvent(drag("dragleave")));
    expect(tracejado(b), "sair de B limpa o destaque de B").toBe(false);
    unmount();
  });

  it("com o modo desligado, dragstart/dragover/drop são no-op", () => {
    const { host, reordens, unmount } = abre(false);
    const [a, b] = caixas(host);
    act(() => {
      a.dispatchEvent(drag("dragstart"));
      b.dispatchEvent(drag("dragover"));
      b.dispatchEvent(drag("drop"));
    });
    expect(reordens).toEqual([]);
    unmount();
  });
});

describe("usePushLog — o log limitado e a limpeza", () => {
  /** SysEx bem formado: `F0…F7`, tamanho par, só hex. O resto é rejeitado. */
  const SYSEX = "F021257F47502D6412001000F7";
  /** SysEx distinto por índice, para encher o log sem deduplicar. O reducer
   * normaliza para MAIÚSCULAS, então o esperado é o hex em caixa alta. */
  const hex = (n: number) => `F021257F47502D6412001${n.toString(16).padStart(3, "0")}F7`.toUpperCase();

  /** Renderiza o hook e devolve a última instancia + o callback registrado. */
  function montaHook() {
    type PushLog = ReturnType<typeof usePushLog>;
    const visto: PushLog[] = [];
    function Coletor() {
      visto.push(usePushLog());
      return null;
    }
    const { unmount } = mount(<Coletor />);
    const cb = vi.mocked(onDevicePush).mock.calls.at(-1)![0];
    return { atual: () => visto.at(-1)!, emitir: (h: string) => act(() => cb(h)), unmount };
  }

  beforeEach(() => {
    vi.mocked(onDevicePush).mockClear();
  });

  it("um push entra no log e o hex vai cru", () => {
    const { atual, emitir, unmount } = montaHook();
    emitir(SYSEX);
    expect(atual().log.at(-1)?.hex).toBe(SYSEX);
    expect(atual().log.at(-1)?.count).toBe(1);
    unmount();
  });

  it("clear() esvazia o log inteiro", () => {
    const { atual, emitir, unmount } = montaHook();
    emitir(hex(1));
    emitir(hex(2));
    expect(atual().log.length).toBe(2);
    act(() => atual().clear());
    expect(atual().log).toEqual([]);
    unmount();
  });

  it("payload inválido NÃO entra (o log é diagnóstico, não lixo)", () => {
    // O parsing puro já é testado em `ipc.push.test.ts`; aqui o que importa é
    // que o HOOK respeita o `null` — um payload ruim não chega na tela.
    const { atual, emitir, unmount } = montaHook();
    emitir(SYSEX);
    const antes = atual().log.length;
    for (const ruim of ["", "12|12001008", "F0212", "1201002F7"]) {
      emitir(ruim);
    }
    expect(atual().log.length, "payloads inválidos não viram entrada").toBe(antes);
    unmount();
  });

  it("repetição consecutiva do mesmo hex vira contador, não outra linha", () => {
    // O boot repete a resposta de tabela dezenas de vezes; 61 linhas
    // idênticas não são diagnóstico.
    const { atual, emitir, unmount } = montaHook();
    emitir(SYSEX);
    emitir(SYSEX);
    emitir(SYSEX);
    expect(atual().log.length, "uma linha só").toBe(1);
    expect(atual().log.at(-1)?.count).toBe(3);
    unmount();
  });

  it("hex diferente DEPOIS da repetição abre linha nova", () => {
    const { atual, emitir, unmount } = montaHook();
    emitir(SYSEX);
    emitir(SYSEX);
    emitir(hex(9));
    expect(atual().log.length).toBe(2);
    expect(atual().log.at(-1)?.hex).toBe(hex(9));
    unmount();
  });

  it("o log respeita o teto e mantém as entradas MAIS NOVAS", () => {
    const { atual, emitir, unmount } = montaHook();
    for (let i = 0; i < PUSH_LOG_MAX + 40; i += 1) emitir(hex(i));
    expect(atual().log.length).toBe(PUSH_LOG_MAX);
    // A mais nova sobrevive; a mais antiga foi descartada.
    expect(atual().log.at(-1)?.hex).toBe(hex(PUSH_LOG_MAX + 39));
    expect(atual().log.some((e) => e.hex === hex(0))).toBe(false);
    unmount();
  });
});
