/**
 * Interações do App — os handlers que a casca não exercita:
 *   - navegação ◀/▶ (ciclo P01→P99) e persistência do master;
 *   - faixa de progresso do boot (aparece durante, some no fim), faixa de
 *     ERRO do boot (failDevice=boot) com recuperação pelo ⟳ e erro amigável
 *     do openPreset (failDevice=board);
 *   - looper: FSM completa por cliques (REC→PLAY→DUB→STOP);
 *   - biblioteca: busca por nome/número e estado vazio;
 *   - Settings: as 6 abas trocam de conteúdo;
 *   - push log: push injetado aparece, clear limpa, cap de 100 entradas;
 *   - sliders do shell (master e drum) com persistência local.
 */
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../src/App";
import { onDevicePush, deviceBoard, deviceInfo, deviceSelectPreset, deviceSetParam, onBootProgress, onDevicePush as onPush } from "../src/ipc/device";
import { Knob } from "../src/components/Knob";
import type { BoardKnob } from "../src/ipc/types";

// Mock PARCIAL dos módulos Tauri: a BRANCH de webview do ipc/device (o
// caminho `inTauri()` que o jsdom nunca pega) fica exercitável simulando
// o runtime — invoke/listen com captura de argumentos.
const tauri = vi.hoisted(() => ({ cbs: [] as Array<(e: { payload: unknown }) => void> }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(async (cmd: string) => ({ cmd })) }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_e: string, cb: (e: { payload: unknown }) => void) => {
    tauri.cbs.push(cb);
    return () => {
      tauri.cbs = tauri.cbs.filter((c) => c !== cb);
    };
  }),
}));
import { FACTORY_PRESETS } from "../src/artifacts/presetData";
import { PRESET_CHAINS } from "../src/artifacts/presetChains";

// Mock PARCIAL: onDevicePush continua registrando no Set real do device.ts
// (o App funciona normalmente), mas o vi.fn captura o callback que o App
// passou — permite INJETAR pushes direto no consumidor da UI. deviceSetParam
// idem: o vi.fn captura os argumentos REAIS que o palco manda ao device.
vi.mock("../src/ipc/device", async (importOriginal) => {
  const mod = await importOriginal<typeof import("../src/ipc/device")>();
  return { ...mod, onDevicePush: vi.fn(mod.onDevicePush), deviceSetParam: vi.fn(mod.deviceSetParam) };
});

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});  beforeEach(() => {
    localStorage.clear();
    vi.mocked(onDevicePush).mockClear();
    vi.mocked(deviceSetParam).mockClear();
  });

function mount(): { root: Root; host: HTMLElement } {
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

/** Espera MACROTASKS reais: o boot simulado corre em lotes via
 * setTimeout(0) — microtasks (settle) não bastam para o terminar. */
async function waitFor(predicate: () => boolean, what: string, timeoutMs = 15_000) {
  const start = Date.now();
  while (!predicate() && Date.now() - start < timeoutMs) {
    await act(async () => {
      await new Promise((r) => setTimeout(r, 50));
    });
  }
  expect(predicate(), `esperado: ${what}`).toBe(true);
}

/** Limpa e re-render: cada teste monta o App do zero. */
function teardown(root: Root, host: HTMLElement) {
  act(() => root.unmount());
  host.remove();
}

function setInput(el: HTMLInputElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(
    window.HTMLInputElement.prototype,
    "value",
  )!.set!;
  setter.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
  el.dispatchEvent(new Event("change", { bubbles: true }));
}

const byAria = (host: HTMLElement, aria: string) =>
  Array.from(host.querySelectorAll("button")).find(
    (b) => b.getAttribute("aria-label") === aria,
  );

describe("Navegação de patch e master", () => {
  it("▶ avança P01→P02 e ◀ volta com ciclo P01→P99 (manual do device)", async () => {
    const { root, host } = mount();
    await settle();

    const label = () =>
      Array.from(host.querySelectorAll("strong")).find((s) => /^P\d{2}/.test(s.textContent ?? ""))
        ?.textContent ?? "";

    expect(label()).toBe(`P01 ${FACTORY_PRESETS[0].name}`);
    act(() => byAria(host, "Próximo patch")!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    // nome VEM DO ARTEFATO (all.prst) — o fallback do board nunca inventa nome
    expect(label()).toBe(`P02 ${FACTORY_PRESETS[1].name}`);

    act(() => byAria(host, "Patch anterior")!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    act(() => byAria(host, "Patch anterior")!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    expect(label()).toBe(`P99 ${FACTORY_PRESETS[98].name}`); // ciclo: P01 - 1 = P99
    teardown(root, host);
  });

  it("slider do master aplica o valor (handler do estado + persistência)", async () => {
    const { root, host } = mount();
    await settle();

    // 1º range do DOM = master VOL (TopBar vem primeiro)
    const slider = host.querySelector<HTMLInputElement>('input[type="range"]');
    expect(slider).toBeTruthy();
    setInput(slider!, "33");
    await settle();
    expect(slider!.value).toBe("33"); // handler ligado: estado re-renderizou
    teardown(root, host);
  });
});

describe("Faixas de boot e erro", () => {
  it("progressbar do boot: aparece durante (0–100%), some quando pronto", async () => {
    const { root, host } = mount();

    const bar = host.querySelector('[role="progressbar"]');
    expect(bar, "faixa de progresso durante o boot").toBeTruthy();
    const now = Number(bar!.getAttribute("aria-valuenow"));
    expect(now).toBeGreaterThanOrEqual(0);
    expect(now).toBeLessThanOrEqual(100);
    expect(bar!.textContent).toMatch(/· \d+%$/);

    await settle();
    await waitFor(
      () => host.querySelector('[role="progressbar"]') === null,
      "faixa de progresso some no fim do boot",
    );
    teardown(root, host);
  });

  it("falha do boot: alerta amigável aparece e o ⟳ recupera (re-scan)", async () => {
    localStorage.setItem("gp100.debug.failDevice", "boot");
    const { root, host } = mount();
    await settle();

    const alert = host.querySelector('[role="alert"]');
    expect(alert, "faixa de erro do boot").toBeTruthy();
    expect(alert!.textContent!.length).toBeGreaterThan(0);

    // recuperação: sem falha, o ícone ⟳ re-escaneia e o alerta some
    localStorage.removeItem("gp100.debug.failDevice");
    act(() => byAria(host, "Reescanear device")!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    expect(host.querySelector('[role="alert"]')).toBeNull();
    expect(host.querySelector('[role="progressbar"]')).toBeTruthy(); // novo boot em curso
    teardown(root, host);
  });

  it("falha do board ao abrir preset: erro amigável (detalhe técnico só no console)", async () => {
    const { root, host } = mount();
    await settle();

    localStorage.setItem("gp100.debug.failDevice", "board");
    const mist = Array.from(host.querySelectorAll('[role="option"]')).find((o) =>
      o.textContent?.includes("Mist"),
    );
    act(() => mist!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    // o command falha 3× com backoff real (~360 ms) antes de virar banner
    await waitFor(() => host.querySelector('[role="alert"]') !== null, "banner do board");

    const alert = host.querySelector('[role="alert"]');
    expect(alert?.textContent).toContain("Não foi possível abrir o preset");
    // recuperação VISÍVEL: o banner carrega a ação (issue #20)
    expect(byAria(host, "Tentar novamente a operação que falhou")).toBeTruthy();
    teardown(root, host);
  });

  it("falha de SELECT: a UI fica no preset REAL e o retry do banner aplica a intenção", async () => {
    const { root, host } = mount();
    await settle();
    const label = () =>
      Array.from(host.querySelectorAll("strong")).find((s) => /^P\d{2}/.test(s.textContent ?? ""))
        ?.textContent ?? "";
    expect(label()).toBe(`P01 ${FACTORY_PRESETS[0].name}`);

    localStorage.setItem("gp100.debug.failDevice", "select");
    act(() => byAria(host, "Próximo patch")!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    // retry/backoff real (~360 ms até desistir): espera o banner
    await waitFor(() => host.querySelector('[role="alert"]') !== null, "banner da falha de select");
    expect(host.querySelector('[role="alert"]')!.textContent).toContain(
      "O device não aceitou a troca de preset",
    );
    // a UI NÃO mente: segue no preset confirmado pelo device (P01), sem P02 fantasma
    expect(label()).toBe(`P01 ${FACTORY_PRESETS[0].name}`);

    // recuperação: gancho fora + ação do banner → a INTENÇÃO original (P02) aplica
    localStorage.removeItem("gp100.debug.failDevice");
    act(() =>
      byAria(host, "Tentar novamente a operação que falhou")!.dispatchEvent(
        new MouseEvent("click", { bubbles: true }),
      ),
    );
    await waitFor(() => label() === `P02 ${FACTORY_PRESETS[1].name}`, "P02 após o retry");
    expect(host.querySelector('[role="alert"]')).toBeNull();
    teardown(root, host);
  });
});

describe("Looper — FSM completa por cliques", () => {
  it("REC grava → PLAY toca → DUB sobrepõe → STOP para (estados e fita)", async () => {
    const { root, host } = mount();
    await settle();

    const rec = () => byAria(host, "Gravar loop (REC)") ?? byAria(host, "Sobrepor (overdub)") ?? byAria(host, "Parar gravação e tocar");
    const play = () => byAria(host, "Tocar loop (PLAY)");

    expect(play()!.hasAttribute("disabled"), "PLAY travado sem fita").toBe(true);
    const status = () =>
      host.querySelector('[aria-label="Looper (máquina de fita)"] [role="status"]');
    expect(status()?.textContent).toContain("vazia");

    act(() => rec()!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    expect(rec()!.getAttribute("aria-pressed")).toBe("true"); // gravando
    // PLAY continua travado ENQUANTO grava: a fita só existe no fim (sem fita = sem PLAY)
    expect(play()!.hasAttribute("disabled"), "PLAY só habilita com fita pronta").toBe(true);

    act(() => rec()!.dispatchEvent(new MouseEvent("click", { bubbles: true }))); // REC→PLAY
    await settle();
    expect(rec()!.getAttribute("aria-label")).toBe("Sobrepor (overdub)"); // em PLAY, ● vira DUB
    expect(status()?.textContent).toContain("90"); // PRE · 90s

    act(() => rec()!.dispatchEvent(new MouseEvent("click", { bubbles: true }))); // PLAY→DUB
    await settle();
    expect(rec()!.getAttribute("aria-pressed")).toBe("true");

    act(() => byAria(host, "Parar (STOP)")!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    expect(rec()!.getAttribute("aria-label")).toBe("Gravar loop (REC)"); // volta ao repouso
    teardown(root, host);
  });

  it("REW zera o tempo e CLEAR pede confirmação antes de apagar a fita", async () => {
    const { root, host } = mount();
    await settle();

    const status = () =>
      host.querySelector('[aria-label="Looper (máquina de fita)"] [role="status"]');
    const rec = () =>
      byAria(host, "Gravar loop (REC)") ?? byAria(host, "Parar gravação e tocar");
    act(() => rec()!.dispatchEvent(new MouseEvent("click", { bubbles: true }))); // grava
    await settle();
    act(() => rec()!.dispatchEvent(new MouseEvent("click", { bubbles: true }))); // → PLAY
    await settle();
    const timer = host.querySelector('[role="timer"]');
    expect(timer?.textContent).toMatch(/^\d{2}:\d{2}$/);

    act(() => byAria(host, "Retroceder ao início do loop (REW)")!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    expect(host.querySelector('[role="timer"]')?.textContent).toBe("00:00");

    const clear = () =>
      Array.from(host.querySelectorAll("button")).find((b) => {
        const a = b.getAttribute("aria-label") ?? "";
        return a.startsWith("Limpar fita") || a === "Confirmar limpar fita";
      })!;
    act(() => clear().dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    expect(clear().getAttribute("aria-label")).toBe("Confirmar limpar fita"); // 1º clique = pedido

    act(() => clear().dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    expect(status()?.textContent).toContain("vazia"); // fita apagada
    teardown(root, host);
  });
});

describe("Biblioteca — busca", () => {
  it("filtra por nome, número display (1-based) e mostra estado vazio", async () => {
    const { root, host } = mount();
    await settle();

    const search = host.querySelector<HTMLInputElement>("aside input");
    expect(search).toBeTruthy();
    const options = () => host.querySelectorAll('[role="option"]').length;

    setInput(search!, "mist");
    await settle();
    expect(options()).toBe(1); // P25 Mist

    setInput(search!, "99");
    await settle();
    expect(options()).toBe(1); // display 1-based: P99
    expect(host.textContent).toContain("Dreamy Aco");

    setInput(search!, "zzz-nada");
    await settle();
    expect(options()).toBe(0);

    setInput(search!, "");
    await settle();
    expect(options()).toBe(99);
    teardown(root, host);
  });
});

/* ── #11: a biblioteca COMANDA o pedalboard ── */
/* ── #11/#19: passos que atravessam biblioteca ↔ palco (compartilhados) ── */
const clickTab = async (host: HTMLElement, nome: string) => {
  act(() =>
    Array.from(host.querySelectorAll('[role="tab"]'))
      .find((t) => t.textContent?.includes(nome))!
      .dispatchEvent(new MouseEvent("click", { bubbles: true })),
  );
  await settle();
};
const userTab = (host: HTMLElement) => clickTab(host, "User Patch");

/** clica no 1º patch de usuário da lista (U01) */
const openUserPatch = async (host: HTMLElement) => {
  await userTab(host);
  act(() =>
    host.querySelectorAll('[role="listitem"]')[0].querySelector("button")!
      .dispatchEvent(new MouseEvent("click", { bubbles: true })),
  );
  await settle();
};

/** nomeia e salva o patch CORRENTE como patch de usuário */
const saveAsUserPatch = async (host: HTMLElement, name: string) => {
  await userTab(host);
  setInput(
    host.querySelector<HTMLInputElement>('input[aria-label="Nome do patch de usuário a salvar"]')!,
    name,
  );
  act(() =>
    Array.from(host.querySelectorAll("button"))
      .find((b) => b.textContent === "Salvar")!
      .dispatchEvent(new MouseEvent("click", { bubbles: true })),
  );
  await settle();
};

/** abre um preset pelo número EXIBIDO (P06 = ppID 5, como no app oficial) */
const openPatch = async (host: HTMLElement, no: string) => {
  // volta para a aba de fábrica (a aba segue o banco aberto no palco)
  act(() =>
    Array.from(host.querySelectorAll('[role="tab"]'))
      .find((t) => t.textContent?.includes("Factory"))!
      .dispatchEvent(new MouseEvent("click", { bubbles: true })),
  );
  await settle();
  const search = host.querySelector<HTMLInputElement>(
    'input[aria-label="Buscar preset por nome, número ou estilo"]',
  )!;
  setInput(search, no);
  await settle();
  act(() =>
    host.querySelectorAll('[role="option"]')[0].dispatchEvent(new MouseEvent("click", { bubbles: true })),
  );
  await settle();
};

describe("Biblioteca — o patch aberto é o que o pedalboard mostra", () => {
  /** nomes dos 9 pedais desenhados, na ordem do palco */
  const chainOnStage = (host: HTMLElement) =>
    Array.from(host.querySelectorAll('svg[role="group"]')).map((g) => g.getAttribute("aria-label") ?? "");

  it("trocar de patch troca a CADEIA dos 9 pedais (inclusive a de ordem trocada)", async () => {
    const { root, host } = mount();
    await settle();

    // P01: PRE/C-Wah, cadeia canônica
    const p01 = [...PRESET_CHAINS[0].slots].sort((a, b) => a.slot - b.slot);
    expect(chainOnStage(host)[0]).toContain(p01[0].name);
    expect(host.querySelector('[aria-label="Slot 1: PRE"]')).toBeTruthy();

    // P06 (ppID 5): no all.prst a cadeia é TROCA — DST antes do PRE
    const p06 = [...PRESET_CHAINS[5].slots].sort((a, b) => a.slot - b.slot);
    expect(p06[0].family).toBe("DST");
    await openPatch(host, "06");

    // o palco mostra a CADEIA, não só o nome: o slot 1 virou DST
    expect(chainOnStage(host)[0]).toContain(p06[0].name);
    expect(host.querySelector('[aria-label="Slot 1: DST"]')).toBeTruthy();
    expect(host.querySelector('[aria-label="Slot 2: PRE"]')).toBeTruthy();
    expect(
      Array.from(host.querySelectorAll("strong")).find((s) => /^P\d{2}/.test(s.textContent ?? ""))?.textContent,
    ).toBe(`P06 ${FACTORY_PRESETS[5].name}`);
    teardown(root, host);
  });

  it("User Patch: snapshot da cadeia salva volta ao palco e excluir devolve a fábrica", async () => {
    const { root, host } = mount();
    await settle();

    // 1. abre o P06 (cadeia trocada: DST na frente) e salva essa cadeia
    await openPatch(host, "06");
    await userTab(host);
    expect(host.textContent).toContain("Nenhum patch salvo ainda");
    await saveAsUserPatch(host, "DST na frente");
    expect(host.textContent).toContain("DST na frente");

    // 2. volta para o P01 (cadeia canônica) e ABRE o patch salvo: quem manda
    //    no palco agora é o snapshot (U01), não o preset de fábrica
    await openPatch(host, "01");
    expect(host.querySelector('[aria-label="Slot 1: PRE"]')).toBeTruthy();
    await openUserPatch(host);
    const p06 = [...PRESET_CHAINS[5].slots].sort((a, b) => a.slot - b.slot);
    expect(host.querySelector('[aria-label="Slot 1: DST"]'), "a cadeia SALVA voltou").toBeTruthy();
    expect(chainOnStage(host)[0]).toContain(p06[0].name);
    // rótulo de usuário (U01) no LED e na navbar — a UI diz de onde vem
    expect(host.querySelector('[aria-label^="Pedalboard"] [role="status"]')?.textContent).toContain("U01");
    expect(
      Array.from(host.querySelectorAll("strong")).find((s) => /^U\d{2}/.test(s.textContent ?? ""))?.textContent,
    ).toBe("U01 DST na frente");

    // 3. excluir o patch aberto devolve o palco ao preset de fábrica real
    act(() =>
      host.querySelector('[aria-label^="Excluir o patch"]')!.dispatchEvent(new MouseEvent("click", { bubbles: true })),
    );
    await settle();
    expect(host.textContent).not.toContain("DST na frente");
    expect(host.querySelector('[aria-label="Slot 1: PRE"]')).toBeTruthy();
    expect(
      Array.from(host.querySelectorAll("strong")).find((s) => /^P\d{2}/.test(s.textContent ?? ""))?.textContent,
    ).toBe(`P01 ${FACTORY_PRESETS[0].name}`);
    teardown(root, host);
  });

  it("o patch salvo é um RETRATO: mexer no pedal depois não altera o guardado", async () => {
    const { root, host } = mount();
    await settle();

    const stageValue = () =>
      host.querySelector('[aria-label="Slot 1: PRE"] [data-value]')!.getAttribute("data-value");

    // 1. salva a cadeia como está (PRE/C-Wah Range = 50)
    const saved = stageValue();
    await saveAsUserPatch(host, "Retrato");

    // 2. mexe no pedal DEPOIS de salvar
    act(() =>
      host.querySelector('[aria-label="Slot 1: PRE"] svg[role="group"]')!
        .dispatchEvent(new MouseEvent("click", { bubbles: true })),
    );
    await settle();
    act(() =>
      host.querySelector('[role="dialog"] svg[role="slider"][aria-label="Range"]')!
        .dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true })),
    );
    await settle();
    act(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })));
    await settle();
    expect(stageValue(), "o palco aceitou o ajuste").not.toBe(saved);

    // 3. outro preset e volta ao patch salvo: o retrato devolve o valor antigo
    await openPatch(host, "06");
    await openUserPatch(host);
    expect(stageValue(), "o retrato devolve o valor do momento do save").toBe(saved);
    teardown(root, host);
  });
});

describe("Settings — as 6 abas", () => {
  it("General: persiste local (input level e language aplicam pelo handler)", async () => {
    const { root, host } = mount();
    await settle();

    act(() => byAria(host, "Abrir configurações")!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();

    const dialog = host.querySelector("[role='dialog']") as HTMLElement;
    // 1º range do General = input level — aplicado E persistido localmente
    const range = dialog.querySelector<HTMLInputElement>('input[type="range"]')!;
    setInput(range, "70");
    await settle();
    expect(range.value).toBe("70");
    expect(JSON.parse(localStorage.getItem("gp100.settings.general.v1")!)).toMatchObject({ inputLevel: 70 });

    // Footswitch ainda sem canal no device vem desabilitado; Idioma NÃO
    // (#30) — é software local puro, funcional e coberto em tests/i18n.
    const disabled = Array.from(dialog.querySelectorAll("select")).filter((s) => s.disabled);
    expect(disabled.length).toBeGreaterThanOrEqual(1);
    teardown(root, host);
  });

  it("modo engenheiro: switch na aba General liga o tooltip addr/code/ctrl e persiste", async () => {
    const { root, host } = mount();
    await settle();

    act(() => byAria(host, "Abrir configurações")!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    const sw = host.querySelector<HTMLElement>(
      '[role="switch"][aria-label^="Modo engenheiro"]',
    );
    expect(sw, "switch do modo engenheiro na aba General").toBeTruthy();
    expect(sw!.getAttribute("aria-checked"), "default desligado").toBe("false");

    act(() => sw!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    expect(sw!.getAttribute("aria-checked")).toBe("true");
    expect(JSON.parse(localStorage.getItem("gp100.settings.general.v1")!)).toMatchObject({
      engineerMode: true,
    });

    // fecha o modal: o tooltip do knob do COMP passa a mostrar o endereço do SET
    act(() =>
      host
        .querySelector("[role='dialog']")!
        .dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })),
    );
    await settle();
    const title = host.querySelector('[aria-label="Slot 1: PRE"] svg[role="img"] title');
    expect(title?.textContent).toContain("addr 10 01 00 02");
    expect(title?.textContent).toContain("ctrl 0");
    teardown(root, host);
  });

  it("cada aba ativa e renderiza seu conteúdo (tabela onde aplicável)", async () => {
    const { root, host } = mount();
    await settle();

    act(() => byAria(host, "Abrir configurações")!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();

    for (const name of ["General", "Global EQ", "About", "Info Frame", "Help", "Release Note"]) {
      const tab = Array.from(host.querySelectorAll('[role="tab"]')).find((t) => t.textContent === name);
      expect(tab, `aba ${name} existe`).toBeTruthy();
      act(() => tab!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
      await settle();
      expect(tab!.getAttribute("aria-selected")).toBe("true");
      if (["About", "Info Frame", "Help"].includes(name)) {
        expect(host.querySelectorAll('[role="dialog"] table').length, name).toBeGreaterThan(0);
      }
    }
    teardown(root, host);
  });
});

describe("Push log — pushes do device", () => {
  it("push injetado aparece no summary, cap segura em 100 e clear limpa", async () => {
    const { root, host } = mount();
    await settle();

    // callback registrado pelo App no mount (mock captura)
    const appCalls = vi.mocked(onDevicePush).mock.calls;
    expect(appCalls.length).toBe(1);
    const appPush = appCalls[0][0];

    const summary = host.querySelector("details summary")!;
    expect(summary.textContent).toContain("(0)");

    // 120 pushes VÁLIDOS e distintos → cap de PUSH_LOG_MAX segura o log em 100
    await act(async () => {
      for (let i = 0; i < 120; i += 1) {
        appPush(`F021257F47502D6412001000${i.toString(16).padStart(2, "0")}F7`);
      }
    });
    await settle();

    expect(summary.textContent).toContain("(100)");
    expect(host.querySelectorAll('[aria-label="Log de pushes"] li').length).toBe(100);

    // clear do PushLog esvazia (summary de volta a 0, estado vazio)
    act(() =>
      Array.from(host.querySelectorAll("button"))
        .find((b) => b.textContent === "Limpar")!
        .dispatchEvent(new MouseEvent("click", { bubbles: true })),
    );
    await settle();
    expect(summary.textContent).toContain("(0)");
    teardown(root, host);
  });

  it("push INVÁLIDO é ignorado e repetição consecutiva vira contador (×N)", async () => {
    const { root, host } = mount();
    await settle();

    const appPush = vi.mocked(onDevicePush).mock.calls[0][0];
    const summary = host.querySelector("details summary")!;
    const items = () => host.querySelectorAll('[aria-label="Log de pushes"] li');

    // lixo do fio (não-SysEx, ímpar, char não-hex, vazio) NÃO entra no log
    await act(async () => {
      appPush("12|12001008");
      appPush("F02125");
      appPush("F021257F47502D64 00F7");
      appPush("");
      appPush(undefined as unknown as string);
    });
    await settle();
    expect(summary.textContent).toContain("(0)");
    expect(items().length).toBe(0);

    // o boot REPETE a mesma resposta: 3× o mesmo hex = 1 linha com ×3
    const repetido = "F021257F47502D6412001000F7";
    await act(async () => {
      appPush(repetido);
      appPush(repetido);
      appPush(repetido);
    });
    await settle();
    expect(summary.textContent).toContain("(1)");
    expect(items().length).toBe(1);
    expect(items()[0].textContent).toContain("×3");

    // hex DIFERENTE depois da repetição abre linha nova (dedupe é CONSECUTIVO)
    await act(async () => {
      appPush("F021257F47502D6412001001F7");
    });
    await settle();
    expect(summary.textContent).toContain("(2)");
    teardown(root, host);
  });
});

describe("Sliders do shell — drum (persistência local)", () => {
  it("volume e speed do drum aplicam pelo handler do estado", async () => {
    const { root, host } = mount();
    await settle();

    // abre o modal de gestão do drum (chip na navbar)
    const chip = Array.from(host.querySelectorAll("button")).find((b) =>
      (b.getAttribute("aria-label") ?? "").includes("abrir gestão de ritmos"),
    );
    act(() => chip!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();

    const vol = host.querySelector<HTMLInputElement>("#drum-vol");
    const speed = host.querySelector<HTMLInputElement>("#drum-speed");
    expect(vol).toBeTruthy();
    setInput(vol!, "40");
    await settle();
    expect(vol!.value).toBe("40");
    setInput(speed!, "60");
    await settle();
    expect(speed!.value).toBe("60");
    teardown(root, host);
  });

  it("on/off e compasso do drum aplicam pelo handler (toggle da navbar)", async () => {
    const { root, host } = mount();
    await settle();

    // play/stop vive na navbar (sem abrir o modal — UX da issue #10)
    const onOff = () =>
      host.querySelector<HTMLButtonElement>(
        'button[aria-label="Tocar ou parar o ritmo da bateria"]',
      )!;
    expect(onOff().getAttribute("aria-pressed")).toBe("false");
    act(() => onOff().dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    expect(onOff().getAttribute("aria-pressed")).toBe("true"); // handler ligou o drum
    expect(onOff().textContent).toContain("⏹");
    act(() => onOff().dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    expect(onOff().getAttribute("aria-pressed")).toBe("false"); // e desligou
    expect(onOff().textContent).toContain("⏵");

    // o modal de gestão continua abrindo pelo chip (compassos reais)
    const chip = Array.from(host.querySelectorAll("button")).find((b) =>
      (b.getAttribute("aria-label") ?? "").includes("abrir gestão de ritmos"),
    );
    act(() => chip!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();

    // compasso: opções reais do firmware, mudança aplicada
    const beat = host.querySelector<HTMLSelectElement>("#drum-beat")!;
    const setter = Object.getOwnPropertyDescriptor(
      window.HTMLSelectElement.prototype,
      "value",
    )!.set!;
    act(() => {
      setter.call(beat, "6/8");
      beat.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await settle();
    expect(beat.value).toBe("6/8");
    teardown(root, host);
  });
});

describe("Palco — afinador e drag-and-drop dos slots (Stage)", () => {
  it("afinador no cabeçalho está sempre visível e o drag de slots só com a trava destrancada", async () => {
    const { root, host } = mount();
    await settle();

    // o afinador ocupa o lugar do antigo VU: display sempre visível,
    // nota em repouso, botões de monitor e demo alcançáveis
    const tunerAria = Array.from(host.querySelectorAll("button")).find(
      (b) => (b.getAttribute("aria-label") ?? "").includes("Ligar ou desligar a monitoração"),
    );
    expect(tunerAria, "botão de monitor do afinador presente").toBeTruthy();
    expect(tunerAria!.getAttribute("aria-pressed")).toBe("false"); // monitor desligado por padrão

    // trava ⇄ mover vira cadeado 🔒/🔓 logo abaixo do display do patch
    const toggle = Array.from(host.querySelectorAll("button")).find((b) =>
      (b.getAttribute("aria-label") ?? "").startsWith("Trava de mover"),
    )!;
    act(() => toggle.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    expect(toggle.getAttribute("aria-pressed")).toBe("true");

    const slot = (n: number, fam: string) =>
      host.querySelector(`[aria-label="Slot ${n}: ${fam}"]`);
    // o PRE agora é o pedal REAL (COMP): o arrasto sai DELE
    const pedal = slot(1, "PRE")!;
    expect(pedal).toBeTruthy();
    expect(pedal.querySelector('svg[role="group"]'), "COMP no slot 1").toBeTruthy();
    act(() => {
      pedal.dispatchEvent(new Event("dragstart", { bubbles: true }));
      slot(2, "DST")!.dispatchEvent(new Event("dragover", { bubbles: true }));
    });
    await settle();
    act(() => slot(2, "DST")!.dispatchEvent(new Event("drop", { bubbles: true })));
    await settle();
    // prévia LOCAL: o pedal vai para a posição 2 e o lugar 1 volta a ser o DST
    expect(slot(2, "PRE")!.querySelector('svg[role="group"]')).toBeTruthy();
    expect(slot(1, "DST")).toBeTruthy();
    expect(host.querySelectorAll('[aria-label^="Slot "]').length).toBe(9);
    teardown(root, host);
  });
});

describe("Palco — o pedal REAL do PRE (U-3: pedais reais na cadeia inteira)", () => {
  /* O PRE do P01 no all.prst é C-Wah (o mock antigo mentia com COMP). Os
     asserts de nome/code saem do artefato GERADO — se a cadeia mudar, o
     teste avisa em vez de acompanhar a mentira. */
  const P01_PRE = PRESET_CHAINS[0].slots.find((s) => s.slot === 0)!;

  it("palco: knobs travados com o valor em texto; o knob do MODAL manda device_set_param (slot do fio 1..9)", async () => {
    const { root, host } = mount();
    await settle();

    // o board do mock traz o PRE REAL do P01: C-Wah, Range/Q/VOL = 50/50/50
    const pedal = host.querySelector('[aria-label="Slot 1: PRE"] svg[role="group"]');
    expect(pedal, `${P01_PRE.name} no slot PRE`).toBeTruthy();
    expect(pedal!.getAttribute("aria-label")).toContain(P01_PRE.name);
    // a cadeia inteira desenha pedal real (visor completo das 9 posições)
    expect(host.querySelectorAll('[aria-label^="Slot "]').length).toBe(9);
    expect(host.querySelectorAll('svg[role="group"]').length).toBe(9);

    // palco: valor REAL do dicionário em texto e nenhum knob ajustável
    const stageValues = () =>
      Array.from(
        host.querySelectorAll('[aria-label="Slot 1: PRE"] [data-value]'),
      ).map((t) => t.getAttribute("data-value"));
    expect(stageValues(), "params reais do P01 no palco").toEqual(["50", "50", "50"]);
    expect(
      host.querySelector('[aria-label="Slot 1: PRE"] svg[role="slider"]'),
      "knob do palco não é ajustável",
    ).toBeNull();

    // clique no pedal abre a edição; o knob ampliado faz 50 → 51 com SET no
    // device (code do C-Wah = effectCode do .prst, ctrl = pos 0, slot = 1)
    act(() => pedal!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    const dlg = host.querySelector('[role="dialog"]')!;
    const range = dlg.querySelector('svg[role="slider"][aria-label="Range"]')!;
    act(() => range.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true })));
    await settle();
    expect(vi.mocked(deviceSetParam)).toHaveBeenLastCalledWith(1, P01_PRE.code, 0, 51);
    expect(stageValues()[0], "estado único: o palco já mostra 51").toBe("51");

    // ValueBox do modal: digitar 42 + Enter aplica ao device e volta ao palco
    const box = dlg.querySelector<HTMLInputElement>('input[aria-label="Valor (Enter para editar)"]')!;
    act(() => box.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    const editing = host.querySelector<HTMLInputElement>('input[aria-label^="Valor do knob"]')!;
    act(() => {
      const setter = Object.getOwnPropertyDescriptor(
        window.HTMLInputElement.prototype,
        "value",
      )!.set!;
      setter.call(editing, "42");
      editing.dispatchEvent(new Event("input", { bubbles: true }));
      editing.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    });
    await settle();
    expect(vi.mocked(deviceSetParam)).toHaveBeenLastCalledWith(1, P01_PRE.code, 0, 42);
    expect(stageValues()[0], "o palco reflete o valor ajustado no modal").toBe("42");
    teardown(root, host);
  });

  it("clique no PRE abre a edição ampliada; knob de lá ajusta o MESMO estado e Esc fecha", async () => {
    const { root, host } = mount();
    await settle();

    const pedal = host.querySelector('[aria-label="Slot 1: PRE"] svg[role="group"]')!;
    act(() => pedal.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    const dlg = host.querySelector('[role="dialog"]')!;
    expect(dlg, "modal de edição aberto").toBeTruthy();
    expect(dlg.getAttribute("aria-label")).toContain(P01_PRE.name);

    // knob AMPLIADO: mesmo handler → SET no device e valor volta ao palco
    const knob = dlg.querySelector('svg[role="slider"][aria-label="Range"]')!;
    expect(knob.getAttribute("aria-valuenow")).toBe("50");
    act(() => knob.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true })));
    await settle();
    expect(vi.mocked(deviceSetParam)).toHaveBeenLastCalledWith(1, P01_PRE.code, 0, 51);
    const stageValue = host.querySelector('[aria-label="Slot 1: PRE"] [data-value]');
    expect(stageValue!.getAttribute("data-value"), "estado único: o palco já mostra 51").toBe(
      "51",
    );

    // Esc (global do App, precedência do painel do topo) fecha
    act(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })));
    await settle();
    expect(host.querySelector('[role="dialog"]')).toBeNull();
    teardown(root, host);
  });

  it("Effects List (#19): trocar o efeito no modal troca o pedal NO PALCO", async () => {
    const { root, host } = mount();
    await settle();

    // abre o modal pelo pedal do palco (o PRE do P01 = C-Wah)
    const pedal = host.querySelector('[aria-label="Slot 1: PRE"] svg[role="group"]')!;
    act(() => pedal.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    const dlg = host.querySelector('[role="dialog"]')!;

    // a lista é do MÓDULO: só PRE, com o efeito atual marcado
    const list = dlg.querySelector('[role="listbox"]')!;
    expect(list.getAttribute("aria-label")).toBe("Lista de efeitos do módulo PRE");
    const opts = Array.from(list.querySelectorAll('[role="option"]'));
    const marked = opts.filter((o) => o.getAttribute("aria-selected") === "true");
    expect(marked.map((o) => o.textContent), "só o efeito atual vem marcado").toEqual(["✓C-Wah"]);
    expect(opts.map((o) => o.textContent)).toContain("COMP");
    // nada de AMP na lista de PRE
    expect(opts.map((o) => o.textContent)).not.toContain("Bog RedM");

    // troca: C-Wah → COMP. O palco tem que mostrar o novo efeito E os knobs
    // novos, já nos defaults do COMP (não herda o 50 do C-Wah)
    const comp = opts.find((o) => o.textContent === "COMP")!;
    act(() => comp.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    const stage = host.querySelector('[aria-label="Slot 1: PRE"] svg[role="group"]')!;
    expect(stage.getAttribute("aria-label"), "o palco trocou de efeito").toContain("COMP");
    expect(
      Array.from(host.querySelectorAll('[aria-label="Slot 1: PRE"] [data-value]')).map((t) =>
        t.getAttribute("data-value"),
      ),
    ).toEqual(["20.0", "50.0"]);
    // e o modal já reflete a troca (estado único): o título e a marca seguem
    expect(host.querySelector('[role="dialog"]')!.getAttribute("aria-label")).toContain("COMP");

    // a busca filtra a lista do módulo
    const search = dlg.querySelector<HTMLInputElement>('input[aria-label="Buscar efeito do módulo por nome"]')!;
    setInput(search, "wah");
    await settle();
    const filtered = Array.from(
      dlg.querySelectorAll('[role="listbox"] [role="option"]'),
    ).map((o) => o.textContent);
    expect(filtered.length).toBeGreaterThan(0);
    expect(filtered.every((t) => t?.toLowerCase().includes("wah"))).toBe(true);
    setInput(search, "zzz-nada");
    await settle();
    expect(dlg.querySelectorAll('[role="listbox"] [role="option"]').length).toBe(0);
    expect(dlg.textContent).toContain("Nenhum efeito para");

    act(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })));
    await settle();
    teardown(root, host);
  });

  it("o efeito trocado entra no patch de usuário (o retrato do palco, não do preset)", async () => {
    const { root, host } = mount();
    await settle();

    // troca o PRE para COMP e SALVA como patch de usuário
    act(() =>
      host.querySelector('[aria-label="Slot 1: PRE"] svg[role="group"]')!
        .dispatchEvent(new MouseEvent("click", { bubbles: true })),
    );
    await settle();
    const dlg = host.querySelector('[role="dialog"]')!;
    act(() =>
      Array.from(dlg.querySelectorAll('[role="option"]'))
        .find((o) => o.textContent === "COMP")!
        .dispatchEvent(new MouseEvent("click", { bubbles: true })),
    );
    await settle();
    act(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })));
    await settle();
    await saveAsUserPatch(host, "Meu COMP");

    // volta para a fábrica (o P01 é C-Wah) e abre o patch salvo: o COMP foi
    // junto, porque o patch é o retrato do PALCO no momento do save
    await openPatch(host, "02");
    await openUserPatch(host);
    expect(host.querySelector('[aria-label="Slot 1: PRE"] svg[role="group"]')!.getAttribute("aria-label")).toContain(
      "COMP",
    );
    teardown(root, host);
  });

  it("footswitch alterna LOCAL (LED verde → vermelho), sem comando de toggle no protocolo", async () => {
    const { root, host } = mount();
    await settle();

    // o PRE do P01 vem DESLIGADO no all.prst (`state` do .prst, não palpite)
    const slot = host.querySelector('[aria-label="Slot 1: PRE"]')!;
    expect(slot.querySelector('[data-led="off"]'), "PRE desligado de fábrica").toBeTruthy();
    const foot = Array.from(slot.querySelectorAll('[role="button"]')).find((b) =>
      (b.getAttribute("aria-label") ?? "").startsWith("Ligar efeito"),
    );
    expect(foot, "footswitch presente").toBeTruthy();
    act(() => foot!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    expect(slot.querySelector('[data-led="on"]'), "LED verde após o clique").toBeTruthy();
    teardown(root, host);
  });
});

describe("Looper — rota PRE/POST e ▶", () => {
  it("alterna PRE⇄POST e ▶ para o transporte", async () => {
    const { root, host } = mount();
    await settle();

    const rec = () =>
      byAria(host, "Gravar loop (REC)") ?? byAria(host, "Parar gravação e tocar");
    act(() => rec()!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    act(() => rec()!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle(); // em PLAY

    // ▶ em PLAY = stop
    act(() => byAria(host, "Tocar loop (PLAY)")!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    expect((byAria(host, "Gravar loop (REC)") ?? byAria(host, "Parar gravação e tocar"))!.getAttribute("aria-label")).toBe("Gravar loop (REC)");

    // rota: POST muda o tempo da fita (45s) e volta para PRE (90s)
    act(() => byAria(host, "Looper em POST (45 segundos, com efeitos)")!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    act(() => rec()!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle(); // gravando em POST
    expect(
      host.querySelector('[aria-label="Looper (máquina de fita)"] [role="status"]')?.textContent,
    ).toContain("45");
    act(() => byAria(host, "Looper em PRE (90 segundos, sem efeitos gravados)")!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    teardown(root, host);
  });
});

describe("Knob — arrasto vertical (pointer events)", () => {
  it("arrasto para cima incrementa proporcional (60px ≈ 23%) e soltar encerra", () => {
    const onChange = vi.fn();
    const onReset = vi.fn();
    const k: BoardKnob = { name: "Gain", pos: 0, kind: "knob", options: [], value: "50", default: "50", range: [0, 100] };
    const host2 = document.createElement("div");
    document.body.appendChild(host2);
    const root2 = createRoot(host2);
    act(() => root2.render(<Knob knob={k} onChange={onChange} onReset={onReset} />));
    const svg = host2.querySelector('svg[role="slider"]')!;

    act(() => svg.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true, clientY: 300 })));
    act(() => svg.dispatchEvent(new MouseEvent("pointermove", { bubbles: true, clientY: 240 }))); // +60px
    expect(onChange).toHaveBeenLastCalledWith(0, "73"); // 50 + 60/260*100
    act(() => svg.dispatchEvent(new MouseEvent("pointerup", { bubbles: true })));
    const after = onChange.mock.calls.length;
    act(() => svg.dispatchEvent(new MouseEvent("pointermove", { bubbles: true, clientY: 100 })));
    expect(onChange.mock.calls.length).toBe(after); // solto: move não altera
    act(() => root2.unmount());
    host2.remove();
  });
});

describe("ipc/device — branch de WEBVIEW Tauri (invoke/listen reais)", () => {
  it("commands e eventos passam pelo invoke/listen do Tauri", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: {}, configurable: true });
    try {
      // info: resolve com o payload do backend
      const { invoke } = await import("@tauri-apps/api/core");
      vi.mocked(invoke).mockResolvedValueOnce({ backend: "real", presetCount: 99 });
      await expect(deviceInfo()).resolves.toMatchObject({ backend: "real" });
      expect(vi.mocked(invoke)).toHaveBeenLastCalledWith("device_info");

      // board: pp vira argumento nomeado do command
      vi.mocked(invoke).mockResolvedValueOnce({ pp: 5, name: "x", ppType: 0, ppTypeName: "y", slots: [] });
      await deviceBoard(5);
      expect(vi.mocked(invoke)).toHaveBeenLastCalledWith("device_board", { pp: 5 });

      // select/set_param: argumentos ida (void)
      vi.mocked(invoke).mockResolvedValueOnce(null);
      await deviceSelectPreset(7);
      expect(vi.mocked(invoke)).toHaveBeenLastCalledWith("device_select_preset", { pp: 7 });
      vi.mocked(invoke).mockResolvedValueOnce(null);
      await deviceSetParam(3, 0x0700006e, 2, 55);
      expect(vi.mocked(invoke)).toHaveBeenLastCalledWith("device_set_param", { slot: 3, code: 0x0700006e, ctrl: 2, value: 55 });

      // eventos: listen devolve unlisten funcional
      const un1 = await onBootProgress(() => {});
      const un2 = await onPush(() => {});
      expect(tauri.cbs.length).toBe(2);
      un1();
      expect(tauri.cbs.length).toBe(1);
      un2();
      expect(tauri.cbs.length).toBe(0);
    } finally {
      delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    }
  });
});
