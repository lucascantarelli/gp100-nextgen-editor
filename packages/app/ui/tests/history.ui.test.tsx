/**
 * HistoryPanel — o histórico do patch na tela (issue #113).
 *
 * **Por que estes testes usam a porta de VERDADE.** A porta tem dois lados: o
 * `invoke` para o shell e o fallback em memória. Aqui o que se quer provar é
 * que a TELHA conta a verdade — a lista cresce a cada gravação, o diff mostra
 * só o que mudou, e a restauração **acrescenta** uma versão em vez de trocar o
 * registro por baixo dos panos. Se a porta fosse mockada, o teste passaria
 * mesmo que a tela mentisse sobre o que a restauração fez, que é exatamente o
 * defeito que a issue existe para impedir.
 *
 * O jsdom não tem webview do Tauri, então isto exercita o fallback — o mesmo
 * caminho que o e2e do Playwright exercita no navegador.
 */
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it } from "vitest";
import { HistoryPanel } from "../src/components/HistoryPanel";
import { useHistory } from "../src/hooks/useHistory";
import { librarySave, resetaFallback } from "../src/ipc/library";
import type { LibraryRecord } from "../src/ipc/library";
import { MSG } from "../src/i18n/messages";

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

beforeEach(() => {
  resetaFallback();
});

function slot(slot: number, nome: string, knobs: [number, string, string][]): string {
  return JSON.stringify({
    slot,
    family: "AMP",
    archetype: "AMPLIFIER",
    name: nome,
    variant: nome.toLowerCase(),
    state: true,
    code: 1,
    knobs: knobs.map(([pos, n, value]) => ({ pos, name: n, kind: "knob", value })),
  });
}

const CADEIA_A = `[${slot(0, "Green OD", [[0, "Gain", "40"]])}]`;
const CADEIA_B = `[${slot(0, "Green OD", [[0, "Gain", "40"]])}, ${slot(2, "Bog RedM", [
  [0, "Gain", "50"],
  [1, "Treble", "60"],
])}]`;
const CADEIA_C = `[${slot(0, "Green OD", [[0, "Gain", "40"]])}, ${slot(2, "Bog RedM", [
  [0, "Gain", "50"],
  [1, "Treble", "95"],
])}]`;

function patch(id: string, payload: string): LibraryRecord {
  return {
    id,
    bank: "user",
    pp: null,
    name: "Meu patch",
    ppType: 4,
    ppTypeName: "Rock",
    savedAt: "2026-10-05T12:00:00Z",
    hasPayload: true,
    payload,
  };
}

/** Hospeda o hook e o painel juntos — é assim que a tela é montada de verdade. */
function Harness() {
  const hist = useHistory();
  return (
    <>
      <button onClick={() => hist.abre("u1")}>abrir histórico</button>
      {hist.patchId != null && <HistoryPanel hist={hist} nome="Meu patch" onClose={hist.fecha} />}
    </>
  );
}

function mount() {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root: Root = createRoot(host);
  act(() => root.render(<Harness />));
  return {
    host,
    unmount: () => {
      act(() => root.unmount());
      host.remove();
    },
  };
}

async function settle() {
  await act(async () => {
    for (let i = 0; i < 8; i++) await Promise.resolve();
  });
}

function botao(host: HTMLElement, texto: string): HTMLButtonElement {
  const b = [...host.querySelectorAll("button")].find((x) => x.textContent?.trim() === texto);
  if (!b) throw new Error(`botão "${texto}" não está na tela`);
  return b;
}

function abre(host: HTMLElement) {
  act(() => botao(host, "abrir histórico").click());
}

describe("histórico na tela", () => {
  it("sem versões, diz que o patch ainda não tem histórico (e não mostra diff vazio)", async () => {
    const { host, unmount } = mount();
    abre(host);
    await settle();
    expect(host.textContent).toContain(MSG.histEmpty);
    unmount();
  });

  it("mostra as versões da mais nova para a mais antiga, marcando a corrente", async () => {
    await librarySave(patch("u1", CADEIA_A));
    await librarySave(patch("u1", CADEIA_B));
    const { host, unmount } = mount();
    abre(host);
    await settle();

    // o par padrão é a última contra a penúltima
    expect(host.textContent).toContain(MSG.histBefore);
    expect(host.textContent).toContain(MSG.histAfter);
    const selects = [...host.querySelectorAll("select")];
    expect(selects).toHaveLength(2);
    expect(selects[1].value).toBe(String(2)); // v2 — a mais nova
    expect(selects[0].value).toBe(String(1)); // v1
    unmount();
  });

  it("o diff padrão mostra só o knob que mudou, no formato antigo → novo", async () => {
    await librarySave(patch("u1", CADEIA_B));
    await librarySave(patch("u1", CADEIA_C));
    const { host, unmount } = mount();
    abre(host);
    await settle();

    expect(host.textContent).toContain(MSG.histArrow("60", "95"));
    // o Gain do AMP não mudou e não pode aparecer
    expect(host.textContent).not.toContain(MSG.histArrow("50", "50"));
    unmount();
  });

  it("restaurar um knob cria a versão nova, a lista cresce e o relato diz qual", async () => {
    await librarySave(patch("u1", CADEIA_B));
    await librarySave(patch("u1", CADEIA_C));
    const { host, unmount } = mount();
    abre(host);
    await settle();

    const knob = [...host.querySelectorAll("button")].find((b) =>
      b.getAttribute("aria-label") === MSG.histRestoreKnob("Treble", "60", "95"),
    );
    if (!knob) throw new Error("não há botão de restaurar o Treble na tela");
    await act(async () => {
      (knob as HTMLButtonElement).click();
      await Promise.resolve();
    });
    await settle();

    expect(host.textContent).toContain(MSG.histRestored(3));
    // três versões agora — e a 1 e a 2 continuam lá. O primeiro <option> é o
    // "escolha uma versão", que existe para desfazer a seleção; as versões são
    // as três seguintes.
    const selects = [...host.querySelectorAll("select")];
    expect(selects[0].options).toHaveLength(4);
    expect(selects[0].options[1].textContent).toContain("v3");
    expect(selects[0].options[3].textContent).toContain("v1");
    unmount();
  });

  it("restaurar a versão inteira também cria uma versão nova", async () => {
    await librarySave(patch("u1", CADEIA_A));
    await librarySave(patch("u1", CADEIA_C));
    const { host, unmount } = mount();
    abre(host);
    await settle();

    const restaura = [...host.querySelectorAll("button")].find(
      (b) => b.getAttribute("aria-label") === MSG.histRestoreAll(1),
    );
    if (!restaura) throw new Error("não há botão de restaurar a versão 1");
    await act(async () => {
      (restaura as HTMLButtonElement).click();
      await Promise.resolve();
    });
    await settle();

    expect(host.textContent).toContain(MSG.histRestored(3));
    const selects = [...host.querySelectorAll("select")];
    // placeholder + as três versões
    expect(selects[0].options).toHaveLength(4);
    unmount();
  });

  it("o painel fecha e o palco volta a ser o dono da tela", async () => {
    await librarySave(patch("u1", CADEIA_A));
    const { host, unmount } = mount();
    abre(host);
    await settle();
    expect(host.querySelector('[role="dialog"]')).not.toBeNull();

    await act(async () => {
      botao(host, "✕").click();
      await Promise.resolve();
    });
    await settle();
    expect(host.querySelector('[role="dialog"]')).toBeNull();
    unmount();
  });
});
