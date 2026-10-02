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

    // selects ainda sem canal no device vêm desabilitados (Footswitch, Idioma)
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
    const title = host.querySelector('[aria-label="Slot 1: PRE"] svg[role="slider"] title');
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

    // abre o drawer do drum (chip na navbar)
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

  it("on/off e compasso do drum aplicam pelo handler (estado do chip)", async () => {
    const { root, host } = mount();
    await settle();

    const chip = Array.from(host.querySelectorAll("button")).find((b) =>
      (b.getAttribute("aria-label") ?? "").includes("abrir gestão de ritmos"),
    );
    act(() => chip!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();

    const onOff = () =>
      Array.from(host.querySelectorAll("button")).find(
        (b) => b.textContent === "⏵ tocar" || b.textContent === "⏹ parar",
      )!;
    expect(onOff().textContent).toBe("⏵ tocar");
    act(() => onOff().dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    expect(onOff().textContent).toBe("⏹ parar"); // handler ligou o drum
    act(() => onOff().dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    expect(onOff().textContent).toBe("⏵ tocar"); // e desligou

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

describe("Palco — COMP (U-3: 1 efeito por vez)", () => {
  it("knob do dicionário aplica LOCAL e manda device_set_param (slot do fio 1..9)", async () => {
    const { root, host } = mount();
    await settle();

    // o board do mock traz PRE/COMP (fxData: Sustain 20.0, Output 50.0)
    const pedal = host.querySelector('[aria-label="Slot 1: PRE"] svg[role="group"]');
    expect(pedal, "COMP no slot PRE").toBeTruthy();
    expect(pedal!.getAttribute("aria-label")).toContain("COMP");
    // os outros 8 lugares seguem placeholders
    expect(host.querySelectorAll('[aria-label^="Slot "]').length).toBe(9);

    // knob Sustain: ArrowUp 20 → 21 e SET no device (code do COMP = 0,
    // ctrl = pos 0, slot do fio = 1)
    const sustain = host.querySelector('svg[role="slider"][aria-label="Sustain"]')!;
    act(() => sustain.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true })));
    await settle();
    expect(vi.mocked(deviceSetParam)).toHaveBeenLastCalledWith(1, 0, 0, 21);
    const boxes = host.querySelectorAll<HTMLInputElement>('input[aria-label="Valor (Enter para editar)"]');
    expect(boxes[0].value, "valor local do Sustain").toBe("21");

    // ValueBox: digitar 42 + Enter aplica ao device e o valor fica na caixa
    act(() => boxes[0].dispatchEvent(new MouseEvent("click", { bubbles: true })));
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
    expect(vi.mocked(deviceSetParam)).toHaveBeenLastCalledWith(1, 0, 0, 42);
    expect(
      host.querySelectorAll<HTMLInputElement>('input[aria-label="Valor (Enter para editar)"]')[0].value,
    ).toBe("42");
    teardown(root, host);
  });

  it("clique no COMP abre a edição ampliada; knob de lá ajusta o MESMO estado e Esc fecha", async () => {
    const { root, host } = mount();
    await settle();

    const pedal = host.querySelector('[aria-label="Slot 1: PRE"] svg[role="group"]')!;
    act(() => pedal.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    const dlg = host.querySelector('[role="dialog"]')!;
    expect(dlg, "modal de edição aberto").toBeTruthy();
    expect(dlg.getAttribute("aria-label")).toContain("COMP");

    // knob AMPLIADO: mesmo handler → SET no device e valor volta ao palco
    const knob = dlg.querySelector('svg[role="slider"][aria-label="Sustain"]')!;
    expect(knob.getAttribute("aria-valuenow")).toBe("20");
    act(() => knob.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true })));
    await settle();
    expect(vi.mocked(deviceSetParam)).toHaveBeenLastCalledWith(1, 0, 0, 21);
    const stageBox = host.querySelector<HTMLInputElement>(
      '[aria-label="Slot 1: PRE"] input[aria-label="Valor (Enter para editar)"]',
    );
    expect(stageBox!.value, "estado único: o palco já mostra 21").toBe("21");

    // Esc (global do App, precedência do painel do topo) fecha
    act(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })));
    await settle();
    expect(host.querySelector('[role="dialog"]')).toBeNull();
    teardown(root, host);
  });

  it("footswitch alterna LOCAL (LED verde → vermelho), sem comando de toggle no protocolo", async () => {
    const { root, host } = mount();
    await settle();

    expect(host.querySelector('[data-led="off"]'), "COMP ligado de fábrica").toBeNull();
    const foot = Array.from(host.querySelectorAll('[role="button"]')).find((b) =>
      (b.getAttribute("aria-label") ?? "").startsWith("Desligar efeito"),
    );
    expect(foot, "footswitch presente").toBeTruthy();
    act(() => foot!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle();
    expect(host.querySelector('[data-led="off"]'), "LED vermelho após o clique").toBeTruthy();
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
