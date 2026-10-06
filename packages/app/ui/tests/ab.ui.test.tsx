/**
 * AbPanel + useAb — o A/B e o BLIND na tela (issue #116).
 *
 * **Por que estes testes usam a porta de verdade.** O par A/B vem do
 * histórico versionado (#113): gravar duas vezes o MESMO patch é o que cria
 * as duas versões. Mockar a porta provaria que o mock tem duas versões — não
 * que a tela monta o par certo e não vaza a resposta.
 *
 * **O teste que a DoD pede é o do BLIND.** Armado, a tela esconde rótulo,
 * versão, nível e relatório; o que ele NÃO pode fazer é vazar a resposta por
 * um canto que ninguém lembrou de esconder (o relatório da troca, o número do
 * nível, o "soando agora"). Cada um desses cantos vira uma asserção de
 * AUSÊNCIA aqui — e a presença depois da resposta prova que a ausência era
 * esconder, não sumir.
 *
 * Montagem no estilo do resto da suíte (`react-dom/client` + `act`), sem
 * testing-library.
 */
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it } from "vitest";
import { AbPanel } from "../src/components/AbPanel";
import { useAb } from "../src/hooks/useAb";
import { librarySave, resetaFallback } from "../src/ipc/library";
import type { LibraryRecord } from "../src/ipc/library";
import type { BoardView } from "../src/ipc/types";
import { MSG } from "../src/i18n/messages";

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

beforeEach(() => {
  resetaFallback();
});

/** Slot serializado com knobs que `analisa` classifica (range presente). */
function slot(slot: number, nome: string, knobs: [number, string, string][]): string {
  return JSON.stringify({
    slot,
    family: "AMP",
    archetype: "AMPLIFIER",
    name: nome,
    variant: nome.toLowerCase(),
    state: true,
    code: 1,
    knobs: knobs.map(([pos, n, value]) => ({
      pos,
      name: n,
      kind: "knob",
      range: [0, 100],
      options: [],
      value,
    })),
  });
}

/** v1: Volume 40 (nível 40). */
const CADEIA_V1 = `[${slot(2, "Green OD", [[0, "Volume", "40"], [1, "Gain", "60"]])}]`;
/** v2: Volume 80 → nível 80 e um knob de ganho que também muda. */
const CADEIA_V2 = `[${slot(2, "Green OD", [[0, "Volume", "80"], [1, "Gain", "60"]])}]`;
/** v3: mesmo slot com OUTRO algoritmo (code 99) — troca que não vai ao fio. */
const CADEIA_V3 = JSON.stringify([
  {
    slot: 2,
    family: "AMP",
    archetype: "AMPLIFIER",
    name: "Bog RedM",
    variant: "bog-redm",
    state: true,
    code: 99,
    knobs: [
      { pos: 0, name: "Volume", kind: "knob", range: [0, 100], options: [], value: "40" },
      { pos: 1, name: "Gain", kind: "knob", range: [0, 100], options: [], value: "60" },
    ],
  },
]);

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

/** Monta o hook + painel como a porta monta, com um `aplica` observável. */
function mount(podeGravar: boolean) {
  const aplicados: BoardView[] = [];
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root: Root = createRoot(host);
  function Casca() {
    const ab = useAb({
      openUserId: "u1",
      rotulo: "U01",
      podeGravar,
      aplica: (b) => {
        aplicados.push(b);
      },
    });
    return (
      <>
        <button
          onClick={() => {
            void ab.carrega();
          }}
        >
          abrir A/B
        </button>
        <AbPanel ab={ab} podeGravar={podeGravar} onClose={() => undefined} />
      </>
    );
  }
  act(() => root.render(<Casca />));
  return {
    host,
    aplicados,
    unmount: () => {
      act(() => root.unmount());
      host.remove();
    },
  };
}

async function settle() {
  await act(async () => {
    for (let i = 0; i < 12; i++) await Promise.resolve();
  });
}

function botao(host: HTMLElement, texto: string): HTMLButtonElement {
  const b = [...host.querySelectorAll("button")].find((x) => x.textContent?.trim() === texto);
  if (!b) throw new Error(`botão "${texto}" não está na tela`);
  return b;
}

function clica(host: HTMLElement, texto: string) {
  act(() => botao(host, texto).click());
}

function checkbox(host: HTMLElement): HTMLInputElement {
  const el = host.querySelector<HTMLInputElement>('input[type="checkbox"]');
  if (!el) throw new Error("o checkbox do blind não está na tela");
  return el;
}

/** O texto da tela, para as asserções de AUSÊNCIA do blind. */
function texto(host: HTMLElement): string {
  return host.textContent ?? "";
}

describe("A/B — o par de versões", () => {
  it("sem duas versões, diz que não há o que comparar (e não inventa um lado)", async () => {
    await librarySave(patch("u1", CADEIA_V1));
    const { host, unmount } = mount(true);
    clica(host, "abrir A/B");
    await settle();
    expect(texto(host)).toContain(MSG.abEmpty);
    expect(texto(host)).not.toContain(MSG.abLado("A"));
    unmount();
  });

  it("com duas versões, A é a corrente, B é a anterior — e A entra no palco", async () => {
    await librarySave(patch("u1", CADEIA_V1));
    await librarySave(patch("u1", CADEIA_V2));
    const { host, aplicados, unmount } = mount(true);
    clica(host, "abrir A/B");
    await settle();

    expect(texto(host)).toContain(MSG.abLado("A"));
    expect(texto(host)).toContain(MSG.abLado("B"));
    expect(texto(host)).toContain(MSG.abSoando);
    // o par padrão é o mesmo do painel de histórico: corrente × anterior
    expect(texto(host)).toContain("v2");
    expect(texto(host)).toContain("v1");
    // nível medido e publicado com método e limitação junto
    expect(texto(host)).toContain(MSG.abNivel("A", "80.0"));
    expect(texto(host)).toContain(MSG.abNivel("B", "40.0"));
    expect(texto(host)).toContain(MSG.abDelta("+40.0"));
    expect(texto(host)).toContain(MSG.abNivelMetodo);
    // A (a corrente) é a que entrou em cena
    expect(aplicados).toHaveLength(1);
    expect(aplicados[0].slots[0].knobs[0].value).toBe("80");
    unmount();
  });

  it("trocar de lado põe a outra cadeia no palco e relata quantos knobs saíram", async () => {
    await librarySave(patch("u1", CADEIA_V1));
    await librarySave(patch("u1", CADEIA_V2));
    const { host, aplicados, unmount } = mount(true);
    clica(host, "abrir A/B");
    await settle();

    clica(host, MSG.abOuvir("B"));
    await settle();
    expect(aplicados[1].slots[0].knobs[0].value).toBe("40");
    // liberado: os knobs que mudaram vão para o aparelho (mock resolve)
    expect(texto(host)).toContain(MSG.abRelatoEnviado(1));
    unmount();
  });

  it("build de leitura: a troca acontece na TELA e o relato diz que o aparelho não recebe", async () => {
    await librarySave(patch("u1", CADEIA_V1));
    await librarySave(patch("u1", CADEIA_V2));
    const { host, aplicados, unmount } = mount(false);
    clica(host, "abrir A/B");
    await settle();

    clica(host, MSG.abOuvir("B"));
    await settle();
    // o palco mudou (desenhar não tem fio)…
    expect(aplicados[1].slots[0].knobs[0].value).toBe("40");
    // …e a tela não mente sobre o fio
    expect(texto(host)).toContain(MSG.abRelatoLocal(1));
    expect(texto(host)).not.toContain(MSG.abRelatoEnviado(1));
    unmount();
  });

  it("algoritmo trocado: a troca vale na tela e o relato DIZ que o fio não leva", async () => {
    await librarySave(patch("u1", CADEIA_V1));
    await librarySave(patch("u1", CADEIA_V3)); // mesmo slot, outro code
    const { host, aplicados, unmount } = mount(true);
    clica(host, "abrir A/B");
    await settle();

    // A (corrente) está com o code 99; ir para B (code 1) não tem formato no fio
    clica(host, MSG.abOuvir("B"));
    await settle();
    expect(texto(host)).toContain(MSG.abAlgoritmos(1));
    expect(texto(host)).not.toContain(MSG.abRelatoEnviado(1));
    // o PALCO recebeu a cadeia (desenhar não tem fio)
    expect(aplicados[1].slots[0].code).toBe(1);
    unmount();
  });
});

describe("A/B — a calibração de nível", () => {
  it("igualando: o lado ALTO desce para o BAIXO e o delta vira 0", async () => {
    await librarySave(patch("u1", CADEIA_V1));
    await librarySave(patch("u1", CADEIA_V2));
    const { host, unmount } = mount(true);
    clica(host, "abrir A/B");
    await settle();
    expect(texto(host)).toContain(MSG.abDelta("+40.0"));

    clica(host, MSG.abCalibrar);
    await settle();
    expect(texto(host)).toContain(MSG.abRelatoCalibra(1, "40.0", "0.0"));
    expect(texto(host)).toContain(MSG.abDelta("0.0"));
    // delta 0 → não há mais o que calibrar e o botão some da ativação
    expect(botao(host, MSG.abCalibrar).disabled).toBe(true);
    unmount();
  });

  it("sem escrita, calibrar nasce travado com o motivo (mesmo botão do knob/IR)", async () => {
    await librarySave(patch("u1", CADEIA_V1));
    await librarySave(patch("u1", CADEIA_V2));
    const { host, unmount } = mount(false);
    clica(host, "abrir A/B");
    await settle();

    const b = botao(host, MSG.abCalibrar);
    expect(b.disabled).toBe(true);
    expect(b.title).toBe(MSG.writeLockedHint);
    // e a limitação do método continua impressa junto do número
    expect(texto(host)).toContain(MSG.abNivelMetodo);
    unmount();
  });
});

describe("A/B — o BLIND não vaza antes da resposta (DoD)", () => {
  async function abreComBlind(podeGravar = true) {
    await librarySave(patch("u1", CADEIA_V1));
    await librarySave(patch("u1", CADEIA_V2));
    const m = mount(podeGravar);
    clica(m.host, "abrir A/B");
    await settle();
    // arma o blind pelo MESMO input que o dono usa
    const cb = checkbox(m.host);
    act(() => {
      cb.click();
    });
    await settle();
    return m;
  }

  it("armado, esconde lado, versão, nível e relatório — a pergunta e os palpites ficam", async () => {
    const { host, unmount } = await abreComBlind();

    // o que FICA: a pergunta e os dois palpites (não nomeiam o lado ativo)
    expect(texto(host)).toContain(MSG.abPergunta);
    expect(texto(host)).toContain(MSG.abTrocar);
    expect(texto(host)).toContain(MSG.abPalpite("A"));
    expect(texto(host)).toContain(MSG.abPalpite("B"));

    // o que SOME: identidade dos lados, versões, nível, relatório
    expect(texto(host)).not.toContain(MSG.abLado("A"));
    expect(texto(host)).not.toContain(MSG.abLado("B"));
    expect(texto(host)).not.toContain(MSG.abSoando);
    expect(texto(host)).not.toContain("v1");
    expect(texto(host)).not.toContain("v2");
    expect(texto(host)).not.toContain(MSG.abNivel("A", "80.0"));
    expect(texto(host)).not.toContain(MSG.abNivel("B", "40.0"));
    expect(texto(host)).not.toContain(MSG.abDelta("+40.0"));
    expect(texto(host)).not.toContain(MSG.abNivelMetodo);
    expect(texto(host)).not.toContain(MSG.abCalibrar);
    expect(texto(host)).not.toContain(MSG.abRelatoNada);
    expect(texto(host)).not.toContain(MSG.abRelatoLocal(1));
    expect(texto(host)).not.toContain(MSG.abRelatoEnviado(1));
    // os botões que ouvem um lado NOMEADO também some (só "trocar" resta)
    expect(texto(host)).not.toContain(MSG.abOuvir("A"));
    expect(texto(host)).not.toContain(MSG.abOuvir("B"));
    unmount();
  });

  it("trocar de lado no escuro muda o palco mas o relatório continua escondido", async () => {
    const { host, aplicados, unmount } = await abreComBlind();
    const antes = aplicados.length;

    clica(host, MSG.abTrocar);
    await settle();

    // a TROCA aconteceu (o ouvido é quem julga, a tela só esconde o nome)
    expect(aplicados.length).toBe(antes + 1);
    expect(texto(host)).toContain(MSG.abPergunta);
    // …mas nem o relatório nem o lado ativo vazam
    expect(texto(host)).not.toContain(MSG.abRelatoEnviado(1));
    expect(texto(host)).not.toContain(MSG.abRelatoLocal(1));
    expect(texto(host)).not.toContain(MSG.abSoando);
    expect(texto(host)).not.toContain("v1");
    expect(texto(host)).not.toContain("v2");
    unmount();
  });

  it("depois da resposta, tudo revela de uma vez — inclusive o que ficou escondido", async () => {
    const { host, unmount } = await abreComBlind();

    // 1ª troca no escuro: muda o lado ativo E deixa um relatório escondido
    clica(host, MSG.abTrocar);
    await settle();
    expect(texto(host)).not.toContain(MSG.abRelatoEnviado(1));

    // palpite errado de propósito (o lado ativo agora é o B): a tela revela
    clica(host, MSG.abPalpite("A"));
    await settle();

    expect(texto(host)).toContain(MSG.abResposta("B", false));
    expect(texto(host)).toContain(MSG.abLado("B"));
    expect(texto(host)).toContain(MSG.abSoando);
    expect(texto(host)).toContain("v1");
    expect(texto(host)).toContain("v2");
    expect(texto(host)).toContain(MSG.abNivel("A", "80.0"));
    expect(texto(host)).toContain(MSG.abNivelMetodo);
    // e o relatório da troca que existia mas estava escondido
    expect(texto(host)).toContain(MSG.abRelatoEnviado(1));
    unmount();
  });

  it("desarmar o blind volta a mostrar tudo sem responder", async () => {
    const { host, unmount } = await abreComBlind();

    const cb = checkbox(host);
    act(() => {
      cb.click();
    });
    await settle();

    expect(texto(host)).toContain(MSG.abSoando);
    expect(texto(host)).toContain("v2");
    unmount();
  });
});
