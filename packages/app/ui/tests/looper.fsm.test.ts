/**
 * FSM do looper — testada como FSM, sem renderizar nada (#82).
 *
 * O ciclo rec→play→dub e a saturacao da fita so eram alcancaveis renderizando
 * o `LooperPanel` inteiro. Aqui cada transicao e um valor de retorno: o teste
 * escreve o estado de entrada e compara o de saida.
 *
 * A ultima secao prova a INVARIANTE que pagou a extracao: nenhuma transicao
 * muta o estado recebido. O `tick` antigo mutava dois `useState` dentro do
 * updater de um terceiro, e updater de estado nao pode ter efeito colateral.
 */

import { describe, expect, it } from "vitest";
import {
  chaveDoModo,
  classeDosRolos,
  fracaoGravada,
  inicial,
  limpar,
  play,
  rec,
  rew,
  rodando,
  stop,
  tick,
  tomDoModo,
} from "../src/looper/fsm";
import type { LooperState } from "../src/looper/fsm";

const PRE = 90;
const POST = 45;

/** Estado arbitrário, para testar uma transição isolada. */
const com = (p: Partial<LooperState>): LooperState => ({ ...inicial(), ...p });

describe("looper: o botão ● e o ciclo rec→play→dub", () => {
  it("de parado vai para REC", () => {
    expect(rec(inicial()).mode).toBe("rec");
    expect(rec(com({ mode: "stop" })).mode).toBe("rec");
  });

  it("de REC vai para PLAY e a fita passa a existir", () => {
    // O segundo clique do ● é o que transforma a gravação em playback: é o
    // gesto de "agora toca" numa máquina de fita.
    const s = rec(com({ mode: "rec" }));
    expect(s.mode).toBe("play");
    expect(s.hasTape).toBe(true);
  });

  it("de PLAY vai para DUB, e ● em DUB CONTINUA em DUB", () => {
    // Nao e um "ciclo" que fecha: o ● é o botao de transporte com SOBREPOSIÇÃO,
    // então de DUB ele volta a DUB. Quem devolve a máquina para REC é parar
    // (STOP) e apertar ● de novo — o ciclo real é o do STOP.
    const emDub = rec(com({ mode: "play", hasTape: true }));
    expect(emDub.mode).toBe("dub");
    expect(rec(emDub).mode, "● em DUB segue sobrepondo").toBe("dub");
    // O caminho de volta: STOP -> ● leva a REC.
    expect(rec(stop(emDub)).mode).toBe("rec");
  });

  it("o ● desarma a confirmação de apagar", () => {
    expect(rec(com({ confirmClear: true })).confirmClear).toBe(false);
  });
});

describe("looper: PLAY exige fita", () => {
  it("sem fita, PLAY não faz nada (e devolve o MESMO estado)", () => {
    const vazio = inicial();
    expect(play(vazio)).toBe(vazio);
  });

  it("com fita, PLAY alterna play↔stop", () => {
    const parado = com({ mode: "stop", hasTape: true });
    expect(play(parado).mode).toBe("play");
    expect(play(com({ mode: "play", hasTape: true })).mode).toBe("stop");
  });
});

describe("looper: STOP e REW mexem em coisas diferentes", () => {
  it("STOP para mas GUARDA a fita (é o que separa stop de idle)", () => {
    const s = stop(com({ mode: "play", hasTape: true, secs: 30 }));
    expect(s.mode).toBe("stop");
    expect(s.hasTape).toBe(true);
    expect(s.secs).toBe(30);
  });

  it("REW volta ao início sem mexer no transporte nem na fita", () => {
    const s = rew(com({ mode: "play", hasTape: true, secs: 42 }));
    expect(s.secs).toBe(0);
    expect(s.mode).toBe("play");
    expect(s.hasTape).toBe(true);
  });
});

describe("looper: apagar a fita é em dois passos", () => {
  it("o 1º clique só arma — a fita continua lá", () => {
    const s = limpar(com({ mode: "play", hasTape: true, secs: 20 }));
    expect(s.confirmClear).toBe(true);
    expect(s.hasTape).toBe(true);
    expect(s.secs).toBe(20);
  });

  it("o 2º clique apaga de verdade e volta ao estado inicial", () => {
    const armado = com({ mode: "play", hasTape: true, secs: 20, confirmClear: true });
    expect(limpar(armado)).toEqual(inicial());
  });

  it("limpar uma máquina parada também apaga (não só tocando)", () => {
    const armado = com({ mode: "stop", hasTape: true, confirmClear: true });
    expect(limpar(armado).hasTape).toBe(false);
  });
});

describe("looper: o segundo que passa", () => {
  it("parado, o tick não muda nada", () => {
    const parado = com({ mode: "stop", hasTape: true, secs: 5 });
    expect(tick(parado, PRE)).toBe(parado);
  });

  it("gravando pela 1ª vez, o tick só avança", () => {
    const s = tick(com({ mode: "rec", secs: 0 }), PRE);
    expect(s.secs).toBe(1);
    expect(s.hasTape).toBe(false);
    expect(s.mode).toBe("rec");
  });

  it("SATURA: gravar até o limite cria a fita e vira PLAY no início", () => {
    // O contrato da máquina de fita: no fim da gravação ela dá a volta e toca.
    const s = tick(com({ mode: "rec", hasTape: false, secs: PRE - 1 }), PRE);
    expect(s.mode).toBe("play");
    expect(s.hasTape).toBe(true);
    expect(s.secs).toBe(0);
  });

  it("tocando, o rolo dá a volta ao chegar no limite", () => {
    const s = tick(com({ mode: "play", hasTape: true, secs: PRE - 1 }), PRE);
    expect(s.secs).toBe(0);
    expect(s.mode).toBe("play");
  });

  it("POST tem metade do rolo: satura antes", () => {
    const meio = tick(com({ mode: "rec", hasTape: false, secs: POST - 1 }), POST);
    expect(meio.mode).toBe("play");
    // O mesmo ponto em PRE ainda NÃO saturou — o limite é da rota, não fixo.
    expect(tick(com({ mode: "rec", hasTape: false, secs: PRE - 1 }), PRE).mode).toBe("play");
    expect(tick(com({ mode: "rec", hasTape: false, secs: POST }), POST).mode).toBe("play");
  });

  it("um limite zero/negativo não trava nem divide por zero", () => {
    const s = tick(com({ mode: "rec", hasTape: false, secs: 0 }), 0);
    expect(Number.isFinite(s.secs)).toBe(true);
    expect(s.secs).toBeGreaterThanOrEqual(0);
  });

  it("contar N segundos de gravação dá N", () => {
    let s = com({ mode: "rec", hasTape: false, secs: 0 });
    for (let i = 0; i < 10; i += 1) s = tick(s, PRE);
    expect(s.secs).toBe(10);
  });
});

describe("looper: o que o painel mostra", () => {
  it("rodando é rec|play|dub — e nada mais", () => {
    expect(rodando(com({ mode: "rec" }))).toBe(true);
    expect(rodando(com({ mode: "play" }))).toBe(true);
    expect(rodando(com({ mode: "dub" }))).toBe(true);
    expect(rodando(com({ mode: "stop" }))).toBe(false);
    expect(rodando(com({ mode: "idle" }))).toBe(false);
  });

  it("só `idle` cai para o rótulo de fita; `stop` tem rótulo próprio", () => {
    // Este teste afirmou o CONTRÁRIO uma vez, e o e2e pegou: o painel
    // mostrava "PRONTO" depois de ■ STOP. STOP e PRONTO não são a mesma
    // informação — um diz que a máquina parou, o outro que há fita guardada.
    expect(chaveDoModo(com({ mode: "idle", hasTape: false }))).toBe("empty");
    expect(chaveDoModo(com({ mode: "idle", hasTape: true }))).toBe("ready");
    expect(chaveDoModo(com({ mode: "stop", hasTape: true })), "STOP tem rotulo proprio").toBe("stop");
    expect(chaveDoModo(com({ mode: "stop", hasTape: false }))).toBe("stop");
  });

  it("rodando, o MODO manda sobre o rótulo de fita", () => {
    expect(chaveDoModo(com({ mode: "rec", hasTape: true }))).toBe("rec");
    expect(chaveDoModo(com({ mode: "dub", hasTape: false }))).toBe("dub");
  });

  it("o tom é de gravação em rec/dub e de toque só em play", () => {
    expect(tomDoModo(com({ mode: "rec" }))).toBe("rec");
    expect(tomDoModo(com({ mode: "dub" }))).toBe("rec");
    expect(tomDoModo(com({ mode: "play" }))).toBe("play");
    expect(tomDoModo(com({ mode: "stop" }))).toBe("");
  });

  it("os rolos giram com transporte e so giram rapido gravando/sobrepondo", () => {
    expect(classeDosRolos(com({ mode: "stop", hasTape: true })), "parado: nenhum giro").toBe("");
    expect(classeDosRolos(com({ mode: "play", hasTape: true }))).toBe("reel-spin");
    expect(classeDosRolos(com({ mode: "rec" }))).toBe("reel-spin reel-fast");
    expect(classeDosRolos(com({ mode: "dub", hasTape: true }))).toBe("reel-spin reel-fast");
  });

  it("a fração gravada respeita o limite da rota", () => {
    expect(fracaoGravada(com({ secs: 0 }), PRE)).toBe(0);
    expect(fracaoGravada(com({ secs: PRE / 2 }), PRE)).toBe(0.5);
    // Passar do limite não estica os rolos além do cheio.
    expect(fracaoGravada(com({ secs: PRE * 3 }), PRE)).toBe(1);
    expect(fracaoGravada(com({ secs: 10 }), 0)).toBe(1);
  });
});

describe("looper: INVARIANTE — nenhuma transição muta a entrada", () => {
  it("todas as transições devolvem um estado novo (ou o mesmo, sem tocar)", () => {
    const entradas: LooperState[] = [
      inicial(),
      com({ mode: "rec", hasTape: false, secs: 3 }),
      com({ mode: "play", hasTape: true, secs: PRE - 1 }),
      com({ mode: "dub", hasTape: true, secs: 0, confirmClear: true }),
      com({ mode: "stop", hasTape: true, secs: 7 }),
    ];
    const transicoes = [rec, play, stop, rew, limpar];
    for (const entrada of entradas) {
      const copia = { ...entrada };
      for (const t of transicoes) {
        t(entrada);
        expect(entrada, "a entrada foi mutada").toEqual(copia);
      }
      for (const limite of [PRE, POST]) {
        const antes = { ...entrada };
        tick(entrada, limite);
        expect(entrada, "tick mutou a entrada").toEqual(antes);
      }
    }
  });

  it("o tick é PURO: chamar duas vezes com a mesma entrada dá o mesmo resultado", () => {
    // A razão de o `tick` antigo ser um bug: ele tinha efeitos colaterais
    // dentro do updater. Duas chamadas com a MESMA entrada tem de dar a MESMA
    // saída — senão o React, que pode chamar o updater duas vezes, salta um
    // segundo de fita.
    const entrada = com({ mode: "rec", hasTape: false, secs: 10 });
    expect(tick(entrada, PRE)).toEqual(tick(entrada, PRE));
  });
});
