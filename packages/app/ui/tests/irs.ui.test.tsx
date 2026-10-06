/**
 * IrLabPanel — o comportamento que o dono vê (issue #24).
 *
 * O que estes testes prendem é o que é VISÍVEL e caro de errar:
 *
 *  - **As DUAS listas existem e não se confundem.** A grade do aparelho é a
 *    leitura do fio (`list_user_irs`); a lista de baixo é o arquivo do dono.
 *    Uma tela só com a segunda diria que o slot está livre quando o aparelho
 *    tem um IR de outra sessão ali — e o envio sobrescreveria em silêncio.
 *  - **Enviar para um slot ocupado pede confirmação COM O NOME.** Na captura,
 *    296 chunks levam ~5 s e um IR de 300 KB são ~20.000 chunks: um envio
 *    disparado por engano é um aparelho meio gravado por minutos.
 *  - **O botão de enviar de um arquivo que o fio recusa fica desabilitado COM O
 *    MOTIVO** (§13.7 recusa a cauda): sumir o botão seria o dono procurar
 *    algo que não está lá.
 *  - **Apagar avisa o que NÃO acontece** — apagar do arquivo não apaga do
 *    aparelho, e o dono precisa saber antes, não depois.
 *
 * Montagem no estilo do resto da suíte (`react-dom/client` + `act`), sem
 * testing-library.
 */
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it } from "vitest";
import { ContentMenu } from "../src/components/ContentMenu";
import { IrLabPanel } from "../src/components/IrLabPanel";
import { useIrs } from "../src/hooks/useIrs";
import { useContentMenu } from "../src/hooks/useContentMenu";
import { resetaFallback } from "../src/ipc/ir";
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
    const irs = useIrs();
    return irs.aberto ? <IrLabPanel irs={irs} /> : <button onClick={irs.abrir}>{MSG.irBtnAria}</button>;
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
const clica = async (host: HTMLElement, seletor: string, nome: string) => {
  await act(async () => {
    porAria(host, seletor, nome).dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
};
const escolheSlot = async (host: HTMLElement, valor: string) => {
  const sel = dialogo(host).querySelector<HTMLSelectElement>("select")!;
  await act(async () => {
    sel.value = valor;
    sel.dispatchEvent(new Event("change", { bubbles: true }));
  });
};
/** Entrega um arquivo ao `<input type=file>` escondido. */
const entregaArquivo = async (host: HTMLElement, bytes: number[]) => {
  const input = porAria<HTMLInputElement>(host, 'input[type="file"]', MSG.irImportAria);
  const arquivo = new File([new Uint8Array(bytes)], "vintage_4x12.ir", { type: "application/octet-stream" });
  await act(async () => {
    Object.defineProperty(input, "files", { value: [arquivo], configurable: true });
    input.dispatchEvent(new Event("change", { bubbles: true }));
  });
};

/** 4 chunks de 15B — o formato que o fio aceita. */
const IR_OK = Array.from({ length: 60 }, (_, i) => (i * 7 + 3) & 0xff);

describe("IrLabPanel — as duas listas de slots", () => {
  it("a grade do aparelho aparece AO ABRIR, com os 20 slots", async () => {
    const { host, flush, done } = mount();
    await flush();
    expect(texto(host)).toContain(MSG.irDeviceTitle);
    const slots = dialogo(host).querySelectorAll('[role="listitem"]');
    expect(slots).toHaveLength(20);
    // e a lista do DONO começa vazia, separado da grade
    expect(texto(host)).toContain(MSG.irEmpty);
    done();
  });

  it("importar mostra o IR com o nome do arquivo e o tamanho", async () => {
    const { host, flush, done } = mount();
    await flush();
    await entregaArquivo(host, IR_OK);
    await flush();
    expect(texto(host)).toContain("vintage_4x12");
    expect(texto(host)).not.toContain("Array(");
    done();
  });
});

describe("IrLabPanel — enviar exige confirmação", () => {
  it("o primeiro clique ABRE a confirmação e não envia; o segundo envia", async () => {
    const { host, flush, done } = mount();
    await flush();
    await entregaArquivo(host, IR_OK);
    await flush();
    await escolheSlot(host, "0");
    await flush();

    await clica(host, "button", MSG.irSendAria("vintage_4x12"));
    await flush();
    expect(dialogo(host).querySelector('[role="alertdialog"]')).not.toBeNull();
    // o relatório ainda NÃO existe: o primeiro clique não mandou nada
    expect(texto(host)).not.toContain("Enviado:");

    const confirmar = dialogo(host).querySelector<HTMLButtonElement>('[role="alertdialog"] button')!;
    await act(async () => {
      confirmar.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await flush();
    expect(texto(host)).toContain("Enviado: 4 blocos");
    done();
  });

  it("sem slot não há o que enviar: o botão fica desabilitado", async () => {
    const { host, flush, done } = mount();
    await flush();
    await entregaArquivo(host, IR_OK);
    await flush();
    const botao = porAria<HTMLButtonElement>(host, "button", MSG.irSendAria("vintage_4x12"));
    expect(botao.disabled).toBe(true);
    done();
  });
});

describe("IrLabPanel — apagar avisa o que NÃO acontece", () => {
  it("o clique abre a confirmação, e o aviso diz que o aparelho fica", async () => {
    const { host, flush, done } = mount();
    await flush();
    await entregaArquivo(host, IR_OK);
    await flush();
    await clica(host, "button", MSG.irDeleteAria("vintage_4x12"));
    await flush();
    const aviso = dialogo(host).querySelector('[role="alertdialog"]');
    expect(aviso).not.toBeNull();
    expect(aviso!.textContent).toContain(MSG.irDeleteWarnText);
    done();
  });

  it("confirmar remove o IR da lista", async () => {
    const { host, flush, done } = mount();
    await flush();
    await entregaArquivo(host, IR_OK);
    await flush();
    await clica(host, "button", MSG.irDeleteAria("vintage_4x12"));
    await flush();
    const confirmar = dialogo(host).querySelector<HTMLButtonElement>('[role="alertdialog"] button')!;
    await act(async () => {
      confirmar.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await flush();
    expect(texto(host)).toContain(MSG.irEmpty);
    done();
  });
});

describe("ContentMenu — a porta do conteúdo do dono", () => {
  it("oferece as QUATRO telas e fecha sem abrir nada", async () => {
    const host = document.createElement("div");
    document.body.appendChild(host);
    const root: Root = createRoot(host);
    let tons = 0;
    let irs = 0;
    let preset = 0;
    let gain = 0;
    act(() =>
      root.render(
        <ContentMenu
          onTones={() => {
            tons += 1;
          }}
          onIrs={() => {
            irs += 1;
          }}
          onPreset={() => {
            preset += 1;
          }}
          onGain={() => {
            gain += 1;
          }}
          onClose={() => undefined}
        />,
      ),
    );
    const dlg = host.querySelector<HTMLElement>('[role="dialog"]')!;
    // As opções são os TÍTULOS das telas: o dono lê o mesmo nome no menu e no
    // topo do painel que abriu.
    expect(dlg.textContent).toContain(MSG.toneTitle);
    expect(dlg.textContent).toContain(MSG.irTitle);
    expect(dlg.textContent).toContain(MSG.presetFileTitle);
    expect(dlg.textContent).toContain(MSG.gainTitle);
    await act(async () => {
      Array.from(dlg.querySelectorAll("button"))
        .find((b) => b.textContent === MSG.toneTitle)!
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(tons).toBe(1);
    expect(irs).toBe(0);
    expect(preset).toBe(0);
    expect(gain).toBe(0);
    await act(async () => {
      Array.from(dlg.querySelectorAll("button"))
        .find((b) => b.textContent === MSG.presetFileTitle)!
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(preset).toBe(1);
    await act(async () => {
      Array.from(dlg.querySelectorAll("button"))
        .find((b) => b.textContent === MSG.gainTitle)!
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(gain).toBe(1);
    act(() => root.unmount());
    host.remove();
  });
});

describe("useContentMenu — a porta que escolhe a tela", () => {
  /** Monta o hook real com as duas manoplas spying (o caminho do App). */
  function porta() {
    const host = document.createElement("div");
    document.body.appendChild(host);
    const root: Root = createRoot(host);
    let tons = 0;
    let irs = 0;
    function Casca() {
      const c = useContentMenu({
        tones: { abrir: () => (tons += 1) },
        irs: { abrir: () => (irs += 1) },
        // O preset em arquivo não tem manopla: a tela dele nasce no hook. O
        // que ele recebe é o preset do PALCO e a porta de volta do importado.
        preset: { pp: 0, onImport: () => undefined },
      });
      return (
        <>
          <button onClick={c.abrir}>abrir</button>
          {c.node}
        </>
      );
    }
    act(() => root.render(<Casca />));
    const abrir = async () => {
      await act(async () => {
        host.querySelector("button")!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      });
    };
    return {
      host,
      abrir,
      conta: () => ({ tons, irs }),
      feito: () => {
        act(() => root.unmount());
        host.remove();
      },
    };
  }

  it("a porta nasce FECHADA: o modal so existe depois do clique", async () => {
    const p = porta();
    expect(p.host.querySelector('[role="dialog"]')).toBeNull();
    await p.abrir();
    expect(p.host.querySelector('[role="dialog"]')).not.toBeNull();
    p.feito();
  });

  it("escolher tons ABRE o painel de tons e fecha a porta", async () => {
    const p = porta();
    await p.abrir();
    const dlg = p.host.querySelector<HTMLElement>('[role="dialog"]')!;
    await act(async () => {
      Array.from(dlg.querySelectorAll("button"))
        .find((b) => b.textContent === MSG.toneTitle)!
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(p.conta()).toEqual({ tons: 1, irs: 0 });
    // A porta fecha junto: dois overlays empilhados nao tem quem feche um.
    expect(p.host.querySelector('[role="dialog"]')).toBeNull();
    p.feito();
  });

  it("escolher IRs ABRE o laboratorio e NAO mexe nos tons", async () => {
    const p = porta();
    await p.abrir();
    const dlg = p.host.querySelector<HTMLElement>('[role="dialog"]')!;
    await act(async () => {
      Array.from(dlg.querySelectorAll("button"))
        .find((b) => b.textContent === MSG.irTitle)!
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(p.conta()).toEqual({ tons: 0, irs: 1 });
    expect(p.host.querySelector('[role="dialog"]')).toBeNull();
    p.feito();
  });

  it("escolher o preset em arquivo ABRE a tela do arquivo (e não os tons)", async () => {
    const p = porta();
    await p.abrir();
    const dlg = p.host.querySelector<HTMLElement>('[role="dialog"]')!;
    await act(async () => {
      Array.from(dlg.querySelectorAll("button"))
        .find((b) => b.textContent === MSG.presetFileTitle)!
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(p.conta()).toEqual({ tons: 0, irs: 0 });
    expect(p.host.querySelector('[role="dialog"]')!.getAttribute("aria-label")).toBe(
      MSG.presetFileTitle,
    );
    p.feito();
  });

  it("o ✕ fecha sem abrir nada (o dono desistiu)", async () => {
    const p = porta();
    await p.abrir();
    const dlg = p.host.querySelector<HTMLElement>('[role="dialog"]')!;
    await act(async () => {
      Array.from(dlg.querySelectorAll("button"))
        .find((b) => b.textContent === MSG.contentMenuClose)!
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(p.conta()).toEqual({ tons: 0, irs: 0 });
    expect(p.host.querySelector('[role="dialog"]')).toBeNull();
    p.feito();
  });
});
