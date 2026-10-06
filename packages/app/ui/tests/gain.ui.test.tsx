/**
 * GainPanel — o assistente de gain staging na TELA (issue #115).
 *
 * O que este arquivo prova é o que o dono lê, e a DoD é explícita em dois
 * pontos que não podem virar teste de "o componente renderiza":
 *
 *  - **a limitação aparece NA UI**, não só no doc. O modo de falha da feature é
 *    o número parecer medição, então a tela tem de imprimir o método e o que o
 *    número NÃO é;
 *  - **a origem de cada número aparece** ao lado dele (nome do controle, valor
 *    cru e a faixa de onde a posição saiu);
 *  - **não há botão que escreve.** O assistente aponta; ajustar é do dono. Um
 *    botão "aplicar sugestão" seria o oposto do que a issue pede, e este teste
 *    falha no dia em que alguém adicionar um.
 *
 * O relatório é o REAL do fallback (o mesmo que o e2e do navegador exercita),
 * não um fixture inventado: um relatório à mão provaria só o JSX.
 */
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it } from "vitest";
import { GainPanel } from "../src/components/GainPanel";
import { useContentMenu } from "../src/hooks/useContentMenu";
import type { Gain } from "../src/hooks/useGain";
import { MSG } from "../src/i18n/messages";
import { deviceBoard } from "../src/ipc/device";
import { analisa } from "../src/ipc/gain";
import type { Relatorio } from "../src/ipc/gain";

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

beforeEach(() => localStorage.clear());

const relatorioDeFabrica = async (pp = 0): Promise<Relatorio> => analisa(await deviceBoard(pp));

async function flush(): Promise<void> {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
}

async function clique(el: Element): Promise<void> {
  await act(async () => {
    el.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
}

/** Clica no botão pelo TEXTO (é como o dono escolhe o destino da porta). */
async function cliqueTexto(host: HTMLElement, texto: string): Promise<void> {
  const alvo = Array.from(host.querySelectorAll("button")).find((b) => b.textContent === texto)!;
  await clique(alvo);
}

/** Um `Gain` de mentira para os cenários de erro/leitura (o resto é real). */
function gainCom(relatorio: Relatorio | null, err: Gain["err"] = null): Gain {
  return { relatorio, carregando: false, err, carrega: () => undefined };
}

/** Monta o painel com o relatório indicado. */
function monta(gain: Gain, pp = 0) {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root: Root = createRoot(host);
  let fechou = 0;
  act(() =>
    root.render(
      <GainPanel
        gain={gain}
        pp={pp}
        onClose={() => {
          fechou += 1;
        }}
      />,
    ),
  );
  return {
    host,
    dlg: () => host.querySelector<HTMLElement>('[role="dialog"]')!,
    fechou: () => fechou,
    fecha: () => {
      act(() => root.unmount());
      host.remove();
    },
  };
}

const pct = (v: number): string => `${Math.round(v * 100)}%`;

describe("GainPanel — o relatório na tela", () => {
  it("mostra o risco, a menor folga e a cadeia INTEIRA na ordem do sinal", async () => {
    const r = await relatorioDeFabrica(0);
    const c = monta(gainCom(r));
    const dlg = c.dlg();
    expect(dlg.getAttribute("aria-label")).toBe(MSG.gainTitle);
    expect(dlg.textContent).toContain(MSG.gainRiscoNivel[r.risco]);
    const fm = r.folgaMinima!;
    expect(dlg.textContent).toContain(MSG.gainFolgaDe(fm[0] + 1, pct(fm[1])));
    // o lugar é 1-based na tela, como no palco
    expect(dlg.textContent).toContain(MSG.gainModulo(1, r.modulos[0].familia, r.modulos[0].nome));
    expect(dlg.textContent).toContain(MSG.gainModulo(9, r.modulos[8].familia, r.modulos[8].nome));
    c.fecha();
  });

  it("cada número aparece com a ORIGEM (o valor cru e a faixa do dicionário)", async () => {
    const r = await relatorioDeFabrica(0);
    const c = monta(gainCom(r));
    const dlg = c.dlg();
    let mostradas = 0;
    for (const m of r.modulos) {
      for (const l of m.leituras) {
        expect(dlg.textContent).toContain(MSG.gainLeitura(l.knob, l.valor, l.faixa[0], l.faixa[1]));
        mostradas += 1;
      }
    }
    expect(mostradas).toBeGreaterThanOrEqual(4);
    c.fecha();
  });

  it("o MÉTODO e a LIMITAÇÃO aparecem na tela (não só no doc)", async () => {
    const r = await relatorioDeFabrica(0);
    const c = monta(gainCom(r));
    const dlg = c.dlg();
    expect(dlg.textContent).toContain(MSG.gainMetodoTitulo);
    expect(dlg.textContent).toContain(r.metodo.limitacao);
    expect(dlg.textContent).toContain(r.metodo.ordem);
    expect(dlg.textContent).toContain(r.metodo.nomesDeSaida.join(", "));
    expect(dlg.textContent).toContain(MSG.gainNaoEscreve);
    c.fecha();
  });

  it("NÃO existe botão que escreve: o único botão é o de fechar", async () => {
    const r = await relatorioDeFabrica(0);
    const c = monta(gainCom(r));
    const botoes = c.dlg().querySelectorAll("button");
    expect(botoes).toHaveLength(1);
    expect(botoes[0].getAttribute("aria-label")).toBe(MSG.gainClose);
    c.fecha();
  });

  it("o ✕ fecha a tela pelo dono", async () => {
    const c = monta(gainCom(await relatorioDeFabrica(0)));
    await clique(c.dlg().querySelector("button")!);
    expect(c.fechou()).toBe(1);
    c.fecha();
  });
});

describe("GainPanel — falha e leitura em andamento", () => {
  it("a falha vira banner COM ação, e o retry é do dono", async () => {
    let retries = 0;
    const c = monta(
      gainCom(null, {
        message: MSG.errGainReport,
        retry: () => {
          retries += 1;
        },
      }),
    );
    const alerta = c.host.querySelector('[role="alert"]')!;
    expect(alerta.textContent).toContain(MSG.errGainReport);
    await clique(alerta.querySelector("button")!);
    expect(retries).toBe(1);
    expect(c.fechou()).toBe(0);
    c.fecha();
  });

  it("enquanto lê, a tela diz que está lendo", () => {
    const c = monta({ relatorio: null, carregando: true, err: null, carrega: () => undefined });
    expect(c.host.querySelector('[role="status"]')!.textContent).toContain(MSG.gainCarregando);
    c.fecha();
  });
});

describe("useContentMenu — o assistente é destino da porta", () => {
  /** Monta o hook REAL (o caminho do App) com a porta e o botão de abrir. */
  function porta(pp = 0) {
    const host = document.createElement("div");
    document.body.appendChild(host);
    const root: Root = createRoot(host);
    function Casca() {
      const c = useContentMenu({
        tones: { abrir: () => undefined },
        irs: { abrir: () => undefined },
        preset: { pp, onImport: () => undefined },
        ab: { openUserId: null, rotulo: "U01", podeGravar: true },
      });
      return (
        <>
          <button onClick={c.abrir}>abrir</button>
          {c.node}
        </>
      );
    }
    act(() => root.render(<Casca />));
    return {
      host,
      feito: () => {
        act(() => root.unmount());
        host.remove();
      },
    };
  }

  it("escolher o gain LÊ o preset do palco e mostra o relatório", async () => {
    const r = await relatorioDeFabrica(0);
    const p = porta(0);
    await cliqueTexto(p.host, "abrir");
    await cliqueTexto(p.host, MSG.gainTitle);
    await flush();
    const dlg = p.host.querySelector<HTMLElement>('[role="dialog"]')!;
    expect(dlg.getAttribute("aria-label")).toBe(MSG.gainTitle);
    // o P01 não é acusado, e a cadeia inteira está lá
    expect(dlg.textContent).toContain(MSG.gainRiscoNivel.baixo);
    expect(dlg.textContent).toContain(MSG.gainModulo(9, r.modulos[8].familia, r.modulos[8].nome));
    // e a porta fechou: dois overlays empilhados não têm quem feche um
    expect(p.host.querySelectorAll('[role="dialog"]')).toHaveLength(1);
    p.feito();
  });
});
