/**
 * ipc/gain — a porta do ASSISTENTE DE GAIN STAGING (issue #115).
 *
 * **Uma porta de LEITURA, e nada mais.** O assistente lê o preset de fábrica e
 * devolve um relatório: não há `set_param`, não há gravação, não há sequer um
 * caminho de escrita nesta coorte. É a mesma família de `ipc/preset.ts` (as
 * projeções puras sobre o `all.prst`), e a política é `IDEMPOTENTE` — repetir
 * uma leitura que falhou pode curar; não há o que "não repetir" aqui.
 *
 * **O que é estimado, e o que é fato.** A posição de um controle na faixa do
 * dicionário é ARITMÉTICA sobre o insumo (o valor do preset e o `min`/`max` do
 * dicionário); o que é estimativa declarada é o que se conclui dela — que um
 * controle no teto é candidato a clipar e onde isso se acumula na cadeia. O
 * método e a limitação vêm no próprio relatório (`metodo.ordem`,
 * `metodo.limitacao`) para a tela mostrar, não para ficarem só no código.
 *
 * **Fora do webview do Tauri** (vitest/jsdom, `pnpm dev` no navegador) não há
 * core Rust, e o e2e do Playwright roda aí — então o fallback REPRODUZ A REGRA,
 * igual ao `diffCadeias` de `history.ts` e ao `casaTexto` de `library.ts`. As
 * duas cópias são a MESMA regra escrita duas vezes, e o preço de divergirem é o
 * relatório dizer uma coisa dentro do app e outra fora dele. A verdade do
 * cálculo é do core (`packages/core/src/gain.rs`), testado nos 3 SOs da matriz;
 * aqui a cópia existe para a tela poder ser exercitada no navegador.
 */
import type { BoardView, BoardSlot } from "./types";
import { IDEMPOTENTE, deviceBoard, runCommand } from "./device";
import { dentroDoShell, invocar } from "./library";

/** Papel de um controle na conta de nível. */
export type Papel = "ganho" | "saida" | "mix";

/** Risco declarado da cadeia inteira. */
export type Risco = "baixo" | "medio" | "alto";

/** Um controle que entrou na conta, com a ORIGEM do número. */
export interface Leitura {
  knob: string;
  pos: number;
  valor: string;
  papel: Papel;
  /** Faixa do dicionário `[lo, hi]` de onde a posição saiu. */
  faixa: [number, number];
  posicao: number;
  folga: number;
}

/** Um módulo da cadeia (os 9, ligados ou não). */
export interface Modulo {
  slot: number;
  familia: string;
  nome: string;
  ligado: boolean;
  leituras: Leitura[];
  ignorados: string[];
  /** A menor folga dos controles de nível — `null` = sem controle de nível. */
  folga: number | null;
  noTeto: boolean;
}

/** Um passo sugerido de ajuste. */
export interface Sugestao {
  slot: number;
  familia: string;
  knob: string;
  pos: number;
  valor: string;
  papel: Papel;
}

/** O método DECLARADO, para a tela mostrar junto do relatório. */
export interface Metodo {
  nomesDeGanho: string[];
  nomesDeSaida: string[];
  nomesDeMix: string[];
  limiarTeto: number;
  /** A ordem do ajuste, escrita. */
  ordem: string;
  /** A limitação, escrita — o que o número NÃO é. */
  limitacao: string;
}

/** O relatório do assistente. */
export interface Relatorio {
  pp: number;
  nome: string;
  modulos: Modulo[];
  foraDoDicionario: number[];
  risco: Risco;
  /** `[slot, folga]` — onde falta headroom. */
  folgaMinima: [number, number] | null;
  ajuste: Sugestao[];
  metodo: Metodo;
}

// ───────────────────────── a tabela declarada (espelho do core) ──────────────
//
// MESMAS listas de `packages/core/src/gain.rs`. Elas saíram do dicionário
// (`analysis/parameters.json`, 105 nomes distintos): `VOL`/`Volume` são as duas
// grafias do nível de saída, `H-Vol`/`L-Vol` são as vozes de um pedal de
// harmonia, e `Wet`/`Dry` são as metades de uma mistura (mistura não soma
// nível). `Sustain`, `Fuzz`/`Bias` e os ganhos de banda do EQ ficam fora:
// mexem no timbre, não no nível de entrada/saída do estágio.
const NOMES_DE_GANHO = ["Gain"];
const NOMES_DE_SAIDA = ["VOL", "VOL 1", "VOL 2", "Volume", "Master", "Output", "Level", "H-Vol", "L-Vol"];
const NOMES_DE_MIX = ["Mix", "Mix A", "Mix B", "Blend", "Wet", "Dry"];

/** As duas strings do método: as mesmas do core (`gain.rs`). */
const ORDEM =
  "Primeiro os NÍVEIS de saída que estão no teto, do fim da cadeia para o começo (tirar nível depois do ponto que clipa não mexe no timbre do drive); depois os GANHOS que estão no teto, do começo para o fim (o ganho de entrada é o que os estágios seguintes amplificam).";
const LIMITACAO =
  "A estimativa é a POSIÇÃO de cada controle na faixa do dicionário — não é nível de sinal medido. O assistente não conhece o nível da guitarra, a resposta em dB de cada pedal, nem a IR no slot: ele aponta onde o preset está no teto e onde isso se acumula na cadeia. Controles de banda (os ganhos do EQ) também ficam fora: eles mexem no timbre, não no nível de entrada/saída do estágio.";

/** O limite de teto declarado (mesmo valor do core). */
const LIMIAR_TETO = 0.95;

/** O papel declarado de um controle pelo nome (case-insensitive). */
function papelDe(nome: string): Papel | null {
  const n = nome.trim();
  const igual = (lista: string[]) => lista.some((g) => g.toLowerCase() === n.toLowerCase());
  if (igual(NOMES_DE_GANHO)) return "ganho";
  if (igual(NOMES_DE_SAIDA)) return "saida";
  if (igual(NOMES_DE_MIX)) return "mix";
  return null;
}

/**
 * O relatório do assistente para um board — a REGRA, em TS.
 *
 * Espelho de `gp100_core::gain::relatorio`: mesmas listas, mesmos limiares,
 * mesma ordem de ajuste. Serve ao fallback (navegador/testes); dentro do app o
 * número vem do core.
 */
export function analisa(board: BoardView): Relatorio {
  const modulos: Modulo[] = board.slots.map((s) => {
    const { leituras, ignorados } = leControles(s);
    const deNivel = leituras.filter((l) => l.papel !== "mix");
    const folga =
      s.state && deNivel.length > 0 ? Math.min(...deNivel.map((l) => l.folga)) : null;
    return {
      slot: s.slot,
      familia: s.family,
      nome: s.name,
      ligado: s.state,
      leituras,
      ignorados,
      folga,
      noTeto: folga != null && 1 - folga >= LIMIAR_TETO,
    };
  });
  modulos.sort((a, b) => a.slot - b.slot);

  const foraDoDicionario = board.slots.filter((s) => s.knobs.length === 0).map((s) => s.slot);

  const comFolga = modulos.filter((m) => m.folga != null).map((m) => [m.slot, m.folga!] as [number, number]);
  const folgaMinima =
    comFolga.length === 0
      ? null
      : comFolga.reduce((melhor, atual) => (atual[1] < melhor[1] ? atual : melhor));

  const noTeto = modulos
    .filter((m) => m.noTeto)
    .flatMap((m) => m.leituras.filter((l) => l.papel !== "mix" && 1 - l.folga >= LIMIAR_TETO));
  const ganhosNoTeto = noTeto.filter((l) => l.papel === "ganho").length;

  // O caso da issue: o AMP no teto com um CAB ligado atrás (gain alto + IR).
  const ampNoTeto = modulos.some(
    (m) => m.noTeto && m.familia === "AMP" && m.leituras.some((l) => l.papel === "ganho"),
  );
  const cabLigado = board.slots.some((s) => s.state && s.family === "CAB");

  const risco: Risco =
    ganhosNoTeto >= 2 || (ampNoTeto && cabLigado) ? "alto" : noTeto.length > 0 ? "medio" : "baixo";

  return {
    pp: board.pp,
    nome: board.name,
    modulos,
    foraDoDicionario,
    risco,
    folgaMinima,
    ajuste: ordemDeAjuste(modulos),
    metodo: {
      nomesDeGanho: [...NOMES_DE_GANHO],
      nomesDeSaida: [...NOMES_DE_SAIDA],
      nomesDeMix: [...NOMES_DE_MIX],
      limiarTeto: LIMIAR_TETO,
      ordem: ORDEM,
      limitacao: LIMITACAO,
    },
  };
}

/** As leituras de nível de um módulo + o que ficou de fora, declarado. */
function leControles(s: BoardSlot): { leituras: Leitura[]; ignorados: string[] } {
  const leituras: Leitura[] = [];
  const ignorados: string[] = [];
  for (const k of s.knobs) {
    const papel = papelDe(k.name);
    if (papel == null) continue; // não é controle de nível
    if (k.kind !== "knob" || k.range == null || k.value == null) {
      ignorados.push(k.name);
      continue;
    }
    const v = Number(k.value);
    if (!Number.isFinite(v)) {
      ignorados.push(k.name);
      continue;
    }
    const [lo, hi] = k.range;
    if (!(hi > lo)) {
      ignorados.push(k.name);
      continue;
    }
    const posicao = Math.min(1, Math.max(0, (v - lo) / (hi - lo)));
    leituras.push({
      knob: k.name,
      pos: k.pos,
      valor: k.value,
      papel,
      faixa: [lo, hi],
      posicao,
      folga: 1 - posicao,
    });
  }
  leituras.sort((a, b) => a.pos - b.pos);
  return { leituras, ignorados };
}

/** Saída primeiro (fim da cadeia), depois ganho (começo) — ver `ORDEM`. */
function ordemDeAjuste(modulos: Modulo[]): Sugestao[] {
  const saida: Sugestao[] = [];
  const ganho: Sugestao[] = [];
  for (const m of modulos.filter((m) => m.noTeto)) {
    for (const l of m.leituras.filter((l) => 1 - l.folga >= LIMIAR_TETO)) {
      const s: Sugestao = {
        slot: m.slot,
        familia: m.familia,
        knob: l.knob,
        pos: l.pos,
        valor: l.valor,
        papel: l.papel,
      };
      if (l.papel === "saida") saida.push(s);
      else if (l.papel === "ganho") ganho.push(s);
    }
  }
  saida.sort((a, b) => b.slot - a.slot);
  ganho.sort((a, b) => a.slot - b.slot);
  return [...saida, ...ganho];
}

/**
 * `preset_gain_report` — o relatório de gain staging do preset `pp`.
 *
 * # Erros
 * No app, a mensagem do core quando o `pp` não existe no arquivo de fábrica.
 */
export async function presetGainReport(pp: number): Promise<Relatorio> {
  if (!dentroDoShell()) return analisa(await deviceBoard(pp));
  return runCommand("preset_gain_report", IDEMPOTENTE, () =>
    invocar<Relatorio>("preset_gain_report", { pp }),
  );
}
