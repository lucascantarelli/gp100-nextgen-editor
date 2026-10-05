/**
 * FieldDiagPanel — o diagnóstico de campo na tela.
 *
 * **O que estes testes atacam.** O painel é a ponte entre o operador de campo e
 * as capacidades que vieram do utilitário de terminal, e o modo como ele falha
 * é o que importa: um botão que trava SEM dizer por quê obriga o operador a
 * descobrir a política de escrita sozinho; um botão que grava no patch errado
 * estraga o patch de alguém. Por isso os testes ficam em três frentes — o que
 * a tela DIZ (badge, trava, razão), para ONDE ela mira (a semente do
 * formulário) e o que ela faz quando o aparelho recusa (erro visível).
 *
 * **Por que as portas são mockadas.** O painel não monta SysEx e não fala com
 * o fio: quem monta é o backend. Um teste que montasse o hex aqui passaria
 * mesmo com a prévia e o envio divergindo — que é exatamente o defeito que o
 * painel existe para não esconder. Por isso os frames vêm do mock como
 * `PreviewFrame[]`, e o teste não tem como "ajudar" o componente.
 */
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { FieldDiagPanel } from "../src/components/FieldDiagPanel";
import type { DeviceInfo, PreviewFrame } from "../src/ipc/types";
import { MSG } from "../src/i18n/messages";

const portas = vi.hoisted(() => ({
  // As implementações ignoram os argumentos de propósito: quem verifica se o
  // alvo chegou inteiro no backend são os `toHaveBeenCalledWith` abaixo.
  deviceSavePreset: vi.fn(async () => {}),
  deviceDumpPreset: vi.fn(async (pp: number) => ({
    pp,
    meta6: "AABBCC",
    pages: ["01", "02", "03", "04", "05", "06", "07", "08"],
  })),
  deviceLogSession: vi.fn(async () => true),
  deviceLogStop: vi.fn(async () => true),
  devicePreview: vi.fn(async (): Promise<PreviewFrame[]> => [
    { label: "13/1001F03", hex: "F021257F47502D64" },
    { label: "13/1001F04", hex: "F021257F47502D65" },
  ]),
}));

vi.mock("../src/ipc/diag", () => portas);

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

beforeEach(() => {
  localStorage.clear();
  for (const fn of Object.values(portas)) fn.mockClear();
});

const REAL: DeviceInfo = {
  backend: "real",
  presetCount: 199,
  currentPp: 24,
  currentName: "Mist",
  currentPpType: 3,
  irSlotsWithCrc: 0,
  irSlots: [],
  writeVerified: true,
};

const LEITURA: DeviceInfo = { ...REAL, writeVerified: false };

function mount(info: DeviceInfo | null) {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root: Root = createRoot(host);
  act(() => root.render(<FieldDiagPanel info={info} />));
  return {
    host,
    unmount: () => {
      act(() => root.unmount());
      host.remove();
    },
  };
}

/** Deixa as promessas do botão resolverem dentro de `act`. */
async function settle() {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
  });
}

function botao(host: HTMLElement, texto: string): HTMLButtonElement {
  const b = [...host.querySelectorAll("button")].find((x) => x.textContent?.trim() === texto);
  if (!b) throw new Error(`botão "${texto}" não está na tela`);
  return b;
}

/** React só enxerga `value` de input controlled pelo setter NATIVO do DOM. */
function digita(el: HTMLInputElement, valor: string) {
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  act(() => {
    setter.call(el, valor);
    el.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

describe("diagnóstico de campo — o que a tela diz antes de qualquer clique", () => {
  it("sem aparelho: diz que não há aparelho (não chama de simulação)", () => {
    const { host, unmount } = mount(null);
    // "Simulação" aqui seria mentira: simulação é quando existe aparelho de
    // mentira, e a diferença é o que o operador precisa saber.
    expect(host.textContent).toContain(MSG.diagBackendNone);
    expect(host.textContent).not.toContain(MSG.diagBackendMock);
    // e o que acontece com a edição é dito explicitamente, não fica a cargo
    // de o operador deduzir que "sem aparelho" quer dizer "salvei em algum lugar"
    expect(host.textContent).toContain(MSG.diagNoDeviceHint);
    unmount();
  });

  it("com o aparelho: o badge diz que está ligado", () => {
    const { host, unmount } = mount(REAL);
    expect(host.textContent).toContain(MSG.diagBackendReal);
    expect(botao(host, MSG.diagSave).disabled).toBe(false);
    unmount();
  });

  it("build de leitura: gravar fica travado E com a razão escrita", () => {
    const { host, unmount } = mount(LEITURA);
    expect(botao(host, MSG.diagSave).disabled).toBe(true);
    // a razão fica visível: um botão morto sem explicação obriga o operador a
    // descobrir sozinho que aquela instalação não escreve.
    expect(host.textContent).toContain(MSG.diagWriteLocked);
    unmount();
  });

  it("ler e descrever continuam disponíveis no build de leitura", () => {
    // Ler e descrever não WRITAM nada — travá-los junto com a gravação deixaria
    // o build de leitura incapaz de dizer o que o aparelho tem.
    const { host, unmount } = mount(LEITURA);
    expect(botao(host, MSG.diagDump).disabled).toBe(false);
    expect(botao(host, MSG.diagPreviewBtn).disabled).toBe(false);
    unmount();
  });
});

describe("diagnóstico de campo — para ONDE a tela mira", () => {
  it("os três campos nasce no patch que a sessão já tem aberto", () => {
    // Se o formulário nascesse em 0/"", gravar mandaria o patch 0 sem querer.
    const { host, unmount } = mount(REAL);
    const campos = [...host.querySelectorAll("input")];
    expect(campos[0].value).toBe("24");
    expect(campos[1].value).toBe("3");
    expect(campos[2].value).toBe("Mist");
    unmount();
  });

  it("gravar manda exatamente o que está nos campos (edição é respeitada)", async () => {
    const { host, unmount } = mount(REAL);
    const campos = [...host.querySelectorAll("input")];
    digita(campos[0], "7");
    digita(campos[2], "Campo 1");
    await act(async () => void botao(host, MSG.diagSave).click());
    await settle();
    expect(portas.deviceSavePreset).toHaveBeenCalledWith(7, 3, "Campo 1");
    unmount();
  });

  it("descrever pede a prévia da MESMA gravação que o botão gravar faria", async () => {
    // Se os dois usassem alvos diferentes, a prévia mentiria — que é o defeito
    // mais caro desta tela: ela existe para mostrar o que SAIRIA.
    const { host, unmount } = mount(REAL);
    digita([...host.querySelectorAll("input")][0], "7");
    await act(async () => void botao(host, MSG.diagPreviewBtn).click());
    await settle();
    expect(portas.devicePreview).toHaveBeenCalledWith({ op: "save", pp: 7, ppType: 3, name: "Mist" });
    unmount();
  });
});

describe("diagnóstico de campo — o que a tela mostra quando volta", () => {
  it("ler mostra a identificação e as páginas do aparelho", async () => {
    const { host, unmount } = mount(REAL);
    await act(async () => void botao(host, MSG.diagDump).click());
    await settle();
    expect(portas.deviceDumpPreset).toHaveBeenCalledWith(24);
    expect(host.textContent).toContain("AABBCC");
    // as 8 páginas: o hex é o que o operador compara com o golden
    expect(host.querySelectorAll("code")).toHaveLength(9);
    unmount();
  });

  it("a prévia mostra os frames que o BACKEND devolveu (o front não monta hex)", async () => {
    const { host, unmount } = mount(REAL);
    await act(async () => void botao(host, MSG.diagPreviewBtn).click());
    await settle();
    expect(host.textContent).toContain("13/1001F03");
    expect(host.textContent).toContain("F021257F47502D64");
    expect(host.textContent).toContain(MSG.diagPreviewCount(2));
    unmount();
  });

  it("o botão de cópia diz o que faz, e só depois confirma que copiou", async () => {
    const { host, unmount } = mount(REAL);
    await act(async () => void botao(host, MSG.diagPreviewBtn).click());
    await settle();
    // em repouso é o verbo da AÇÃO — "copiado" em repouso faria a lista
    // parecer já copiada
    expect(botao(host, MSG.diagCopyBtn)).toBeTruthy();
    await act(async () => void botao(host, MSG.diagCopyBtn).click());
    await settle();
    expect(botao(host, MSG.diagCopied)).toBeTruthy();
    unmount();
  });

  it("gravar confirma na tela (uma vez) e o log liga/desliga no caminho digitado", async () => {
    const { host, unmount } = mount(REAL);
    await act(async () => void botao(host, MSG.diagSave).click());
    await settle();
    expect(host.textContent).toContain(MSG.diagSaveDone);

    // o log grava no arquivo que o operador escolheu — não num caminho fixo
    digita([...host.querySelectorAll("input")][3], "minha-sessao.jsonl");
    await act(async () => void botao(host, MSG.diagLogStart).click());
    await settle();
    expect(portas.deviceLogSession).toHaveBeenCalledWith("minha-sessao.jsonl");
    expect(host.textContent).toContain(MSG.diagLogOn("minha-sessao.jsonl"));

    await act(async () => void botao(host, MSG.diagLogStop).click());
    await settle();
    expect(portas.deviceLogStop).toHaveBeenCalled();
    unmount();
  });

  it("recusa do aparelho aparece como erro visível, não some", async () => {
    portas.deviceSavePreset.mockRejectedValueOnce(new Error("gp100: gravacao recusada"));
    const { host, unmount } = mount(REAL);
    await act(async () => void botao(host, MSG.diagSave).click());
    await settle();
    expect(host.querySelector('[role="alert"]')?.textContent).toContain("gp100: gravacao recusada");
    // e NÃO há "gravado" — a tela não pode dizer que gravou o que não gravou
    expect(host.textContent).not.toContain(MSG.diagSaveDone);
    unmount();
  });
});