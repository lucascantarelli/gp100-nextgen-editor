/**
 * abLevel — o NÍVEL das duas versões e o que a calibração mexeria (#116).
 *
 * **O problema que este módulo resolve.** Um A/B que muda o volume testa o
 * gain externo, não a cadeia: quem ouve prefere o lado mais alto sem nem
 * perceber que está comparando timbre. Então as duas versões precisam de um
 * número de nível — e ele precisa ser honesto sobre o que mede.
 *
 * **O número é POSIÇÃO, não dB.** O aparelho não expõe nível em dB por SysEx,
 * e inventar uma conversão seria a pior coisa que este arquivo poderia fazer
 * (é exatamente o que o ADR-10 existe para evitar: regra de parede com número
 * inventado). O índice é a média das posições dos controles de **saída** da
 * cadeia (os mesmos nomes que o assistente de gain classifica como papel
 * `saida`), 0..100. Ele diz "o botão está mais aberto", e a limitação vai
 * impressa na tela junto do número — mesma regra do `gain.ts`.
 *
 * **Por que reaproveita `analisa`.** A classificação de papel (o que é nível
 * de saída, o que é ganho, o que é mix) é uma regra com fonte única no
 * `ipc/gain.ts` (espelho de `gp100_core::gain`). Duas classificações deste
 * módulo divergiriam na primeira edição de uma lista de nomes, e o A/B
 * começaria a "medir" ganho como se fosse saída.
 *
 * Tudo aqui é puro: `BoardView` entra, número sai. Nenhum IPC, nenhum React —
 * é o que deixa o teste rápido e o relatório reproduzível.
 */
import { analisa } from "./ipc/gain";
import type { BoardView } from "./ipc/types";

/** Um controle de nível que os DOIS lados têm, com valores diferentes. */
export interface Calibracao {
  /** Slot da cadeia (0..8, ordem do sinal). */
  slot: number;
  /** Posição do knob dentro do slot (a identidade estável no fio, §13.11). */
  pos: number;
  /** Nome do controle como o dicionário o chama. */
  knob: string;
  /** Valor no lado A. */
  a: string;
  /** Valor no lado B. */
  b: string;
}

/** Controles de saída (papel `saida`) dos módulos LIGADOS, na ordem da cadeia. */
function saidas(board: BoardView): Array<{ slot: number; pos: number; knob: string; valor: string; posicao: number }> {
  return analisa(board)
    .modulos.filter((m) => m.ligado)
    .flatMap((m) =>
      m.leituras
        .filter((l) => l.papel === "saida")
        .map((l) => ({ slot: m.slot, pos: l.pos, knob: l.knob, valor: l.valor, posicao: l.posicao })),
    );
}

/**
 * Índice de nível de SAÍDA da cadeia: média das posições (0..100) dos
 * controles de saída dos módulos ligados.
 *
 * `null` = a cadeia não tem controle de nível nenhum — e aí o A/B **não tem
 * como calibrar**, que é o resultado honesto (um A/B sem controle de nível se
 * compara do jeito que der, com a limitação dita em voz alta).
 */
export function nivel(board: BoardView): number | null {
  const xs = saidas(board);
  if (xs.length === 0) return null;
  const media = xs.reduce((acc, x) => acc + x.posicao, 0) / xs.length;
  return Math.round(media * 1000) / 10;
}

/**
 * Diferença de nível A − B, em pontos do índice (0..100), com 1 casa.
 *
 * `null` quando QUALQUER dos dois lados não tem controle de nível — a
 * diferença de um lado só não existe.
 */
export function deltaNivel(a: BoardView, b: BoardView): number | null {
  const na = nivel(a);
  const nb = nivel(b);
  if (na == null || nb == null) return null;
  return Math.round((na - nb) * 10) / 10;
}

/**
 * Os controles de saída que os DOIS lados têm e que estão em valores
 * diferentes — o material do qual a calibração é feita.
 *
 * **Só os COMPARTILHADOS.** Um controle que existe só num lado não tem com o
 * que ser igualado (igualar a um botão que o outro lado nem tem é inventar
 * nível); esses sobram como "diferença residual" no relatório, e a tela diz.
 */
export function calibracao(a: BoardView, b: BoardView): Calibracao[] {
  const saidaA = new Map(saidas(a).map((x) => [`${x.slot}:${x.pos}`, x]));
  const saidaB = new Map(saidas(b).map((x) => [`${x.slot}:${x.pos}`, x]));
  const fora: Calibracao[] = [];
  for (const [chave, xa] of saidaA) {
    const xb = saidaB.get(chave);
    if (xb == null || xa.valor === xb.valor) continue;
    fora.push({ slot: xa.slot, pos: xa.pos, knob: xa.knob, a: xa.valor, b: xb.valor });
  }
  return fora.sort((x, y) => x.slot - y.slot || x.pos - y.pos);
}

/**
 * Slots cujo ALGORITMO trocou entre os dois lados.
 *
 * Estes a troca **não leva para o aparelho**: mandar o `set_param` de um slot
 * que no device está com outro algoritmo seria gravar num controle que não é
 * mais aquele. O canal para trocar algoritmo é o `0x47` (change-effect), que
 * ainda não tem formato capturado (BLOCKERS 10b) — e a tela precisa dizer
 * isso em vez de fingir que a troca foi completa.
 */
export function algoritmosTrocados(a: BoardView, b: BoardView): number[] {
  const codA = new Map(a.slots.map((s) => [s.slot, s.code]));
  const codB = new Map(b.slots.map((s) => [s.slot, s.code]));
  const fora = new Set<number>();
  for (const [slot, code] of codA) {
    const outro = codB.get(slot);
    if (outro != null && outro !== code) fora.add(slot);
  }
  return [...fora].sort((x, y) => x - y);
}

/**
 * A TROCA: o que precisa ir para o aparelho para o palco (e o device) saírem
 * de `de` para `para`.
 *
 * Devolve **só o que é gravável**: knobs (`kind === "knob"` — switch/combox
 * não têm canal no fio, é prévia local, ver `useStage.applyKnob`) dos slots
 * cujo algoritmo é o MESMO nos dois lados, e cujo valor difere.
 */
export function troca(
  de: BoardView,
  para: BoardView,
): Array<{ slot: number; code: number; pos: number; valor: string }> {
  const trocados = new Set(algoritmosTrocados(de, para));
  const mapa = new Map(para.slots.map((s) => [s.slot, s]));
  const saida: Array<{ slot: number; code: number; pos: number; valor: string }> = [];
  for (const antes of de.slots) {
    if (trocados.has(antes.slot)) continue;
    const alvo = mapa.get(antes.slot);
    if (alvo == null) continue;
    const valorA = new Map(antes.knobs.map((k) => [k.pos, k.value]));
    for (const k of alvo.knobs) {
      if (k.kind !== "knob") continue;
      if (valorA.get(k.pos) === k.value) continue;
      if (k.value == null) continue;
      saida.push({ slot: alvo.slot, code: alvo.code, pos: k.pos, valor: k.value });
    }
  }
  return saida;
}
