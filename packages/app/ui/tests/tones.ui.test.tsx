/**
 * SnapTonePanel — o comportamento que o dono vê (issue #25).
 *
 * O que estes testes prendem é o que é VISÍVEL e irreversível:
 *
 *  - **O aviso do CAB aparece antes de qualquer envio.** As notas de release do
 *    aparelho (V2.1) dizem que ligar o SnapTone desliga o módulo CAB. O dono
 *    que usa o CAB perde o som se o app mandar o tom calado, e um aviso que só
 *    existe no console é um aviso que ninguém lê.
 *  - **Enviar pede confirmação.** Um clique é um gesto acidental; dois cliques
 *    são dois streams de 143 blocos cada (§5).
 *  - **O A/B é por SLOT**, e o botão de ouvir existe com o motivo escrito
 *    quando o slot está vazio — sumir seria o dono procurar um botão que não
 *    está lá.
 *
 * Montagem no estilo do resto da suíte (`react-dom/client` + `act`), sem
 * testing-library: os outros arquivos do projeto não têm a dependência e
 * cruzar com o mesmo caminho de render evita que o teste passe aqui e falhe no
 * CI por um ambiente diferente.
 */
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it } from "vitest";
import { SnapTonePanel } from "../src/components/SnapTonePanel";
import { useTones } from "../src/hooks/useTones";
import { resetaFallback } from "../src/ipc/tones";
import { MSG } from "../src/i18n/messages";

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

beforeEach(() => resetaFallback());

/** Monta o hook REAL (o mesmo caminho do App) com o painel aberto. */
function mount() {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root: Root = createRoot(host);
  function Casca() {
    const tones = useTones();
    return tones.aberto ? <SnapTonePanel tones={tones} /> : <button onClick={tones.abrir}>{MSG.toneBtn}</button>;
  }
  act(() => root.render(<Casca />));
  // O painel começa FECHADO (o hook é quem decide, como no App): o primeiro
  // passo de cada teste é o mesmo que o dono dá — clicar no botão da navbar.
  act(() => {
    host.querySelector("button")!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
  return {
    host,
    /** Re-renderiza depois de um await (o quadro chega do IPC). */
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
const clica = async (host: HTMLElement, seletor: string, nome: string) => {
  await act(async () => {
    porAria(host, seletor, nome).dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
};
/** Troca o valor de um `<select>` (o React escuta `change`, não `click`). */
const escolheSlot = async (host: HTMLElement, linha: number, valor: string) => {
  const selects = dialogo(host).querySelectorAll<HTMLSelectElement>("select");
  await act(async () => {
    const sel = selects[linha];
    sel.value = valor;
    sel.dispatchEvent(new Event("change", { bubbles: true }));
  });
};
/** Entrega um arquivo ao `<input type=file>` escondido. */
const entregaArquivo = async (host: HTMLElement, nome: string, bytes: number[]) => {
  const input = porAria<HTMLInputElement>(host, 'input[type="file"]', nome);
  const arquivo = new File([new Uint8Array(bytes)], "modelo.clo", { type: "application/octet-stream" });
  await act(async () => {
    Object.defineProperty(input, "files", { value: [arquivo], configurable: true });
    input.dispatchEvent(new Event("change", { bubbles: true }));
  });
};

/**
 * Escreve num input CONTROLADO do React.
 *
 * Atribuir `campo.value = x` direto não chega ao React: o input controlado
 * guarda o valor no DOM, o React lê o mesmo campo e conclui que nada mudou,
 * porque o setter nativo foi substituído por um que memoriza o valor. O
 * `defineProperty` com o setter do protótipo é o caminho que o próprio React
 * usa internamente nos testes dele.
 */
const escreve = async (campo: HTMLInputElement, valor: string) => {
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
  setter.call(campo, valor);
  campo.dispatchEvent(new Event("input", { bubbles: true }));
  await act(async () => {
    await Promise.resolve();
  });
};

const CLO = Array.from({ length: 40 }, (_, i) => (i * 7 + 3) & 0xff);

describe("SnapTonePanel — o aviso do CAB", () => {
  it("o aviso aparece AO ABRIR, antes de qualquer tom ou envio", async () => {
    const { host, flush, done } = mount();
    await flush();
    // O texto do catálogo está na tela com a lista ainda vazia: o dono precisa
    // ler a consequência ANTES de ter feito a pergunta.
    expect(texto(host)).toContain(MSG.toneCabWarning);
    expect(texto(host)).toContain(MSG.toneCabTitle);
    done();
  });
});

describe("SnapTonePanel — importar", () => {
  it("importar um .clo mostra o tom com o nome do arquivo", async () => {
    const { host, flush, done } = mount();
    await flush();
    expect(texto(host)).toContain(MSG.toneEmpty);
    await entregaArquivo(host, MSG.toneImportCloAria, CLO);
    await flush();
    expect(texto(host)).toContain("modelo");
    // A lista é leve: a linha NÃO carrega o modelo inteiro (2,7 KB por tom).
    expect(texto(host)).not.toContain("Array(");
    done();
  });

  it("o nome digitado tem preferência sobre o nome do arquivo", async () => {
    const { host, flush, done } = mount();
    await flush();
    await act(async () => {
      const campo = porAria<HTMLInputElement>(host, "input", MSG.toneNameAria);
      await escreve(campo, "Marshall 4x12");
    });
    await entregaArquivo(host, MSG.toneImportCloAria, CLO);
    await flush();
    expect(texto(host)).toContain("Marshall 4x12");
    done();
  });
});

describe("SnapTonePanel — enviar exige confirmação", () => {
  it("o primeiro clique ABRE a confirmação e não envia; o segundo envia", async () => {
    const { host, flush, done } = mount();
    await flush();
    await entregaArquivo(host, MSG.toneImportCloAria, CLO);
    await flush();
    await escolheSlot(host, 0, "1");
    await flush();

    await clica(host, "button", MSG.toneSendAria("modelo"));
    await flush();
    // a confirmação aparece e o aviso do CAB vai junto
    expect(dialogo(host).querySelector('[role="alertdialog"]')).not.toBeNull();
    // e o relatório ainda NÃO existe: o primeiro clique não mandou nada
    expect(texto(host)).not.toContain("Enviado:");

    // o botão "Enviar" dentro da confirmação
    const confirmar = dialogo(host).querySelector<HTMLButtonElement>('[role="alertdialog"] button')!;
    await act(async () => {
      confirmar.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await flush();
    expect(texto(host)).toContain("Enviado: 3 blocos");
    done();
  });

  it("sem slot não há o que enviar: o botão fica desabilitado", async () => {
    const { host, flush, done } = mount();
    await flush();
    await entregaArquivo(host, MSG.toneImportCloAria, CLO);
    await flush();
    const botao = porAria<HTMLButtonElement>(host, "button", MSG.toneSendAria("modelo"));
    expect(botao.disabled).toBe(true);
    done();
  });
});

describe("SnapTonePanel — A/B e apagar", () => {
  it("o A/B mostra os dois lados com o motivo quando o slot está vazio", async () => {
    const { host, flush, done } = mount();
    await flush();
    expect(texto(host)).toContain(MSG.toneAbTitle);
    expect(texto(host)).toContain(MSG.toneAbEmpty);
    expect(texto(host)).toContain(MSG.toneNoPreview);
    done();
  });

  it("apagar remove o tom da lista", async () => {
    const { host, flush, done } = mount();
    await flush();
    await entregaArquivo(host, MSG.toneImportCloAria, CLO);
    await flush();
    await clica(host, "button", MSG.toneDeleteAria("modelo"));
    await flush();
    expect(texto(host)).toContain(MSG.toneEmpty);
    done();
  });
});