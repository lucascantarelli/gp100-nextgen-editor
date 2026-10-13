/**
 * A política de escrita da TELA (#126 face (A); ADR-5).
 *
 * O que estes testes prendem é o que o fio não alcança provar: um build de
 * LEITURA precisa RECUSAR ANTES do clique, com o motivo escrito — a recusa
 * viraria um banner de erro depois de o dono ter girado o knob ou disparado
 * 143 blocos de SnapTone. Três frentes:
 *
 *  - **`escritaLiberada` é a única leitura do `writeVerified`** para decidir
 *    botão. Sem informação a resposta é `false`: uma trava que abre sozinha
 *    antes de saber quem é o aparelho não é trava.
 *  - **O gancho `gp100.debug.writeVerified`** é a forma de o teste (e o e2e)
 *    montar um build de leitura sem compilar a feature.
 *  - **Os três painéis que gravam** (knob, IR, SnapTone) desabilitam o
 *    botão E dizem o motivo — nos DOIS sentidos: travado não envia, destravado
 *    continua igual (um teste só do lado travado provaria um `disabled`
 *    permanente).
 *
 * Montagem no estilo do resto da suíte (`react-dom/client` + `act`), sem
 * testing-library.
 */
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it } from "vitest";
import { IrLabPanel } from "../src/components/IrLabPanel";
import { SnapTonePanel } from "../src/components/SnapTonePanel";
import { PedalModal } from "../src/components/PedalModal";
import { useIrs } from "../src/hooks/useIrs";
import { useTones } from "../src/hooks/useTones";
import { deviceInfo, escritaLiberada } from "../src/ipc/device";
import type { BoardSlot, DeviceInfo } from "../src/ipc/types";
import { resetaFallback as resetaIr } from "../src/ipc/ir";
import { resetaFallback as resetaTones } from "../src/ipc/tones";
import { MSG } from "../src/i18n/messages";

/** O mesmo nome do gancho em `src/ipc/device.ts` (espelhado aqui de propósito):
 *  um rename lá sem rename aqui derruba o teste, que é o efeito desejado. */
const WRITE_KEY = "gp100.debug.writeVerified";

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

beforeEach(() => {
  resetaIr();
  resetaTones();
  localStorage.removeItem(WRITE_KEY);
});

const info = (writeVerified: boolean): DeviceInfo => ({
  backend: "real",
  detail: "",
  presetCount: 99,
  currentPp: 0,
  currentName: "Teste",
  currentPpType: 0,
  irSlotsWithCrc: 20,
  irSlots: [],
  writeVerified,
});

describe("escritaLiberada — a política num lugar só", () => {
  it("sem aparelho (null/undefined) a resposta é TRAVADO", () => {
    expect(escritaLiberada(null)).toBe(false);
    expect(escritaLiberada(undefined)).toBe(false);
  });

  it("segue o writeVerified do aparelho nos dois sentidos", () => {
    expect(escritaLiberada(info(true))).toBe(true);
    expect(escritaLiberada(info(false))).toBe(false);
  });
});

describe("o fallback local simula o build de leitura pelo gancho", () => {
  it("gancho armado relata writeVerified false; sem gancho é true (ADR-5)", async () => {
    localStorage.setItem(WRITE_KEY, "false");
    await expect(deviceInfo()).resolves.toMatchObject({ writeVerified: false });

    localStorage.removeItem(WRITE_KEY);
    await expect(deviceInfo()).resolves.toMatchObject({ writeVerified: true });
  });
});

/* ── PedalModal: knob e caixa de valor ─────────────────────────────────── */

function mkSlot(): BoardSlot {
  return {
    slot: 0,
    family: "PRE",
    archetype: "BUFFER",
    name: "PRE-0",
    variant: "comp",
    code: 1,
    state: true,
    knobs: [
      { name: "Gain", pos: 0, kind: "knob", options: [], value: "50", default: "50", range: [0, 100] },
      { name: "Mode", pos: 1, kind: "switch", options: ["off", "on"], value: "off", default: "off" },
    ],
  };
}

function mountModal(podeGravar: boolean | undefined) {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root: Root = createRoot(host);
  act(() =>
    root.render(
      <PedalModal
        slot={mkSlot()}
        podeGravar={podeGravar}
        onToggle={() => {}}
        onKnobChange={() => {}}
        onKnobReset={() => {}}
        onChangeEffect={() => {}}
        onClose={() => {}}
      />,
    ),
  );
  return {
    dlg: () => host.querySelector<HTMLElement>('[role="dialog"]')!,
    done: () => {
      act(() => root.unmount());
      host.remove();
    },
  };
}

describe("PedalModal — build de leitura", () => {
  it("trava knob e caixa de valor COM O MOTIVO na tela", () => {
    const { dlg, done } = mountModal(false);
    const d = dlg();
    // o motivo é visível, não só no tooltip de um controle morto
    expect(d.textContent).toContain(MSG.writeLockedHint);
    // knobs viram display (role=img) — nenhum slider/botão acessível
    expect(d.querySelectorAll('svg[role="slider"], svg[role="button"]')).toHaveLength(0);
    expect(d.querySelectorAll('svg[role="img"]')).toHaveLength(2);
    // e a caixa de valor do modal não entra em modo edição
    const caixa = d.querySelector<HTMLInputElement>('input[aria-label="' + MSG.pedalValueAria + '"]');
    expect(caixa, "a caixa de valor existe").toBeTruthy();
    expect(caixa!.disabled).toBe(true);
    expect(caixa!.title).toBe(MSG.writeLockedHint);
    done();
  });

  it("sem a trava os controles seguem vivos (o default é o ADR-5)", () => {
    const { dlg, done } = mountModal(true);
    const d = dlg();
    expect(d.querySelectorAll('svg[role="slider"], svg[role="button"]')).toHaveLength(2);
    const caixa = d.querySelector<HTMLInputElement>('input[aria-label="' + MSG.pedalValueAria + '"]')!;
    expect(caixa.disabled).toBe(false);
    expect(caixa.title).toBe("");
    expect(d.textContent).not.toContain(MSG.writeLockedHint);
    done();
  });

  it("sem o prop (quem não decide) o comportamento é o de escrita liberada", () => {
    const { dlg, done } = mountModal(undefined);
    expect(dlg().querySelectorAll('svg[role="slider"], svg[role="button"]')).toHaveLength(2);
    done();
  });
});

/* ── Painéis: IR e SnapTone ────────────────────────────────────────────── */

/** Monta o hook REAL com o painel aberto e o `podeGravar` escolhido. */
function mountPainel(qual: "ir" | "tom", podeGravar: boolean) {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root: Root = createRoot(host);
  function Casca() {
    const irs = useIrs();
    const tones = useTones();
    if (qual === "ir") return irs.aberto ? <IrLabPanel irs={irs} podeGravar={podeGravar} /> : <button onClick={irs.abrir}>ir</button>;
    return tones.aberto ? <SnapTonePanel tones={tones} podeGravar={podeGravar} /> : <button onClick={tones.abrir}>tom</button>;
  }
  act(() => root.render(<Casca />));
  act(() => {
    host.querySelector("button")!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
  return {
    host,
    flush: async () => {
      await act(async () => {
        await Promise.resolve();
      });
    },
    done: () => {
      act(() => root.unmount());
      host.remove();
    },
  };
}

const dialogo = (host: HTMLElement) => host.querySelector<HTMLElement>('[role="dialog"]')!;
const porAria = <T extends HTMLElement>(host: HTMLElement, seletor: string, nome: string): T =>
  Array.from(dialogo(host).querySelectorAll<T>(seletor)).find((e) => e.getAttribute("aria-label") === nome)!;
const texto = (host: HTMLElement) => dialogo(host).textContent ?? "";
const escolheSlot = async (host: HTMLElement, linha: number, valor: string) => {
  const sel = dialogo(host).querySelectorAll<HTMLSelectElement>("select")[linha];
  await act(async () => {
    sel.value = valor;
    sel.dispatchEvent(new Event("change", { bubbles: true }));
  });
};
const entregaArquivo = async (host: HTMLElement, aria: string, nome: string, bytes: number[]) => {
  const input = porAria<HTMLInputElement>(host, 'input[type="file"]', aria);
  const arquivo = new File([new Uint8Array(bytes)], nome, { type: "application/octet-stream" });
  await act(async () => {
    Object.defineProperty(input, "files", { value: [arquivo], configurable: true });
    input.dispatchEvent(new Event("change", { bubbles: true }));
  });
};

const IR_OK = Array.from({ length: 60 }, (_, i) => (i * 7 + 3) & 0xff);
const CLO = Array.from({ length: 40 }, (_, i) => (i * 7 + 3) & 0xff);

describe("IrLabPanel — build de leitura", () => {
  it("o envio fica desabilitado COM O MOTIVO, mesmo com slot escolhido", async () => {
    const { host, flush, done } = mountPainel("ir", false);
    await flush();
    // a tela avisa ANTES de qualquer clique
    expect(texto(host)).toContain(MSG.writeLockedHint);
    await entregaArquivo(host, MSG.irImportAria, "vintage_4x12.ir", IR_OK);
    await flush();
    await escolheSlot(host, 0, "0");
    await flush();

    const botao = porAria<HTMLButtonElement>(host, "button", MSG.irSendAria("vintage_4x12"));
    expect(botao.disabled, "travado não envia").toBe(true);
    expect(botao.title).toBe(MSG.writeLockedHint);
    done();
  });

  it("com escrita liberada o mesmo fluxo envia normalmente (controle)", async () => {
    const { host, flush, done } = mountPainel("ir", true);
    await flush();
    expect(texto(host)).not.toContain(MSG.writeLockedHint);
    await entregaArquivo(host, MSG.irImportAria, "vintage_4x12.ir", IR_OK);
    await flush();
    await escolheSlot(host, 0, "0");
    await flush();

    const botao = porAria<HTMLButtonElement>(host, "button", MSG.irSendAria("vintage_4x12"));
    expect(botao.disabled).toBe(false);
    done();
  });
});

describe("SnapTonePanel — build de leitura", () => {
  it("o envio fica desabilitado COM O MOTIVO, mesmo com slot escolhido", async () => {
    const { host, flush, done } = mountPainel("tom", false);
    await flush();
    expect(texto(host)).toContain(MSG.writeLockedHint);
    await entregaArquivo(host, MSG.toneImportCloAria, "modelo.clo", CLO);
    await flush();
    await escolheSlot(host, 0, "1");
    await flush();

    const botao = porAria<HTMLButtonElement>(host, "button", MSG.toneSendAria("modelo"));
    expect(botao.disabled, "travado não envia").toBe(true);
    expect(botao.title).toBe(MSG.writeLockedHint);
    done();
  });

  it("com escrita liberada o mesmo fluxo envia normalmente (controle)", async () => {
    const { host, flush, done } = mountPainel("tom", true);
    await flush();
    expect(texto(host)).not.toContain(MSG.writeLockedHint);
    await entregaArquivo(host, MSG.toneImportCloAria, "modelo.clo", CLO);
    await flush();
    await escolheSlot(host, 0, "1");
    await flush();

    const botao = porAria<HTMLButtonElement>(host, "button", MSG.toneSendAria("modelo"));
    expect(botao.disabled).toBe(false);
    done();
  });
});
