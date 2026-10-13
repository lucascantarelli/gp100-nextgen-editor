/**
 * ipc/history — a porta do HISTÓRICO de versões do patch (issue #113).
 *
 * **Por que este arquivo existe separado de `library.ts`.** A mesma razão que
 * tirou `diag.ts` de `device.ts`: `library.ts` já passou do teto de tamanho do
 * gate de módulo. E a divisão é natural, não só conveniente — o histórico é
 * *leitura e restauração* de algo imutável, enquanto `library.ts` é a lista e
 * o CRUD do registro corrente. Duas responsabilidades, dois arquivos.
 *
 * **A seta de dependência é de um lado só.** O store de memória (as versões do
 * fallback) mora em `library.ts`, e não aqui: a regra que garante a feature é
 * "toda gravação de patch de usuário versiona", e ela é do `librarySave`. Se o
 * store viesse para cá, o `librarySave` teria que importar deste arquivo e o
 * par viraria um ciclo. Mantendo a invariante num lugar só, este arquivo só
 * LÊ o histórico (`versoesDoPatch`/`versaoPorId`).
 *
 * **O diff é do crate.** Quem compara duas versões é
 * `gp100_library::diff::diff_cadeias` (Rust), e o e2e contra o shell prove
 * esse caminho. O `diffCadeias` abaixo é a cópia para o fallback em memória —
 * o e2e do Playwright roda o front no navegador, sem webview, e é contra o
 * fallback que o fluxo inteiro é exercitado. É o mesmo truque de `casaTexto`
 * em `library.ts`: o que o fallback não tem (SQLite) ele reimplementa, e o
 * preço de divergir é o app se comportar de um jeito dentro do shell e de outro
 * fora dele. A REGRA que amarra as duas cópias é a mesma — comparar o que está
 * gravado e dizer só o que mudou.
 */
import { IDEMPOTENTE, runCommand, semRetry } from "./device";
import {
  bancoFallback,
  dentroDoShell,
  invocar,
  librarySave,
  versaoPorId,
  versoesDoPatch,
} from "./library";

/**
 * Restaurar é uma ESCRITA em arquivo local, e repolar não conserta disco cheio
 * nem permissão — a falha se repete e o dono só espera mais para ver o mesmo
 * erro. O `reason` é obrigatório na assinatura (`semRetry`) e é o que fica no
 * log de debug quando alguém precisar saber por que aquilo não repetiu.
 */
const SEM_RETRY_ESCRITA = semRetry("escrita em arquivo local: repetir nao conserta disco/permissao");

/** Uma versão completa (com a cadeia) — o que `library_version` devolve. */
export interface LibraryVersion {
  /** Chave da linha (`preset_version.id`). */
  id: number;
  /** Patch dono. */
  presetId: string;
  /** Número da versão no patch, 1-based e monotônico. */
  seq: number;
  /** ISO-8601 da gravação. */
  savedAt: string;
  /** Nome do patch naquele instante (renomear também versiona). */
  name: string;
  /** Cadeia serializada (`BoardSlot[]`). */
  payload: string | null;
}

/** Uma linha do histórico, sem a cadeia (o painel só mostra número e data). */
export interface LibraryVersionRow {
  /** Chave da linha. */
  id: number;
  /** Patch dono. */
  presetId: string;
  /** Número da versão no patch. */
  seq: number;
  /** ISO-8601 da gravação. */
  savedAt: string;
  /** Nome do patch naquele instante. */
  name: string;
  /** Esta versão é a que o registro corrente espelha (o maior `seq`)? */
  current: boolean;
}

/**
 * Uma mudança de knob: `pos`, o nome, e o antes/depois.
 *
 * NÃO é export: ninguém nomeia este tipo fora daqui. Quem precisa falar de
 * "uma mudança de knob" fala do `ChainDiff` (exportado) ou do que a tela
 * desenha — e o `check_deadcode` cobra que todo `export` tenha consumidor.
 */
interface KnobDiff {
  /** Posição do knob dentro do slot — a identidade estável (o nome muda). */
  pos: number;
  /** Nome do knob como ele é na versão nova. */
  knob: string;
  /** Valor antigo (`null` = o knob não existia ou não tinha valor). */
  from: string | null;
  /** Valor novo. */
  to: string | null;
}

/** O que mudou num slot da cadeia. Não exportado pelo mesmo motivo de `KnobDiff`. */
interface SlotDiff {
  /** Número do slot (0..8, ordem do sinal). */
  slot: number;
  /** Nome do algoritmo antes. */
  nameBefore: string;
  /** Nome do algoritmo depois. */
  nameAfter: string;
  /** O slot estava ligado antes? */
  onBefore: boolean;
  /** O slot está ligado depois? */
  onAfter: boolean;
  /**
   * O ALGORITMO do slot trocou — evento DIFERENTE de "você girou N knobs".
   * Trocar o AMP troca todos os knobs dele, e sem esta bandeira o diff diria
   * que o dono mexeu em doze controles quando ele trocou um pedal.
   */
  algorithmChanged: boolean;
  /** Os knobs que mudaram, na ordem do `pos`. */
  knobs: KnobDiff[];
}

/** O diff inteiro entre duas versões. */
export interface ChainDiff {
  /** Slots com alguma mudança, na ordem do número do slot. */
  slots: SlotDiff[];
  /** Slots que existem só na versão nova. */
  slotsAdded: number[];
  /** Slots que existem só na versão antiga. */
  slotsRemoved: number[];
  /**
   * Nada mudou. A tela diz "idêntico" em vez de mostrar uma lista vazia — e
   * lista vazia é ambígua entre "são iguais" e "não consegui ler".
   */
  identical: boolean;
}

/** Slot reduzido ao que o diff compara. */
interface SlotLeido {
  slot: number;
  name: string;
  on: boolean;
  knobs: { pos: number; name: string; value: string | null }[];
}

/**
 * Lê a cadeia serializada que o palco grava (`BoardSlot[]`).
 *
 * Campo ausente tem um default DELIBERADO e não é erro: uma cadeia vinda de um
 * app anterior sem `state` não pode impedir o dono de ver o que mudou nos
 * knobs. O que não é lista de slots vira lista vazia — e comparar duas cadeias
 * ilegíveis como "idêntico" é o melhor que se pode dizer sem inventar dado.
 */
function leSlots(payload: string): SlotLeido[] {
  let cru: unknown;
  try {
    cru = JSON.parse(payload);
  } catch {
    return [];
  }
  if (!Array.isArray(cru)) return [];
  return cru.map((item, idx) => {
    const s = (item ?? {}) as Record<string, unknown>;
    const knobs = Array.isArray(s.knobs) ? s.knobs : [];
    return {
      slot: typeof s.slot === "number" ? s.slot : idx,
      name: typeof s.name === "string" ? s.name : "",
      // Ausente = ligado: o palco só grava `state: false` para um slot
      // desligado, e uma cadeia sem o campo não pode virar "o dono desligou
      // todos os pedais".
      on: typeof s.state === "boolean" ? s.state : true,
      knobs: knobs.map((k, kidx) => {
        const knob = (k ?? {}) as Record<string, unknown>;
        return {
          pos: typeof knob.pos === "number" ? knob.pos : kidx,
          name: typeof knob.name === "string" ? knob.name : "",
          // `value` ausente NÃO é `""`: tratá-los como a mesma coisa faria o
          // diff mentir sobre o que a restauração vai desfazer.
          value: typeof knob.value === "string" ? knob.value : null,
        };
      }),
    };
  });
}

/** O MESMO algoritmo de `gp100_library::diff::diff_cadeias`. */
function diffCadeias(antes: string, depois: string): ChainDiff {
  const a = leSlots(antes);
  const b = leSlots(depois);
  const slots: SlotDiff[] = [];
  for (const antigo of a) {
    const novo = b.find((s) => s.slot === antigo.slot);
    if (!novo) continue;
    const knobs: KnobDiff[] = [];
    for (const k of antigo.knobs) {
      const alvo = novo.knobs.find((d) => d.pos === k.pos);
      if (!alvo) knobs.push({ pos: k.pos, knob: k.name, from: k.value, to: null });
      else if (alvo.value !== k.value) knobs.push({ pos: k.pos, knob: alvo.name, from: k.value, to: alvo.value });
    }
    for (const d of novo.knobs) {
      if (!antigo.knobs.some((k) => k.pos === d.pos)) knobs.push({ pos: d.pos, knob: d.name, from: null, to: d.value });
    }
    knobs.sort((x, y) => x.pos - y.pos);
    const algorithmChanged = antigo.name !== novo.name;
    if (algorithmChanged || antigo.on !== novo.on || knobs.length > 0) {
      slots.push({
        slot: antigo.slot,
        nameBefore: antigo.name,
        nameAfter: novo.name,
        onBefore: antigo.on,
        onAfter: novo.on,
        algorithmChanged,
        knobs,
      });
    }
  }
  slots.sort((x, y) => x.slot - y.slot);
  const slotsAdded = b.filter((s) => !a.some((x) => x.slot === s.slot)).map((s) => s.slot).sort((x, y) => x - y);
  const slotsRemoved = a.filter((s) => !b.some((x) => x.slot === s.slot)).map((s) => s.slot).sort((x, y) => x - y);
  return {
    slots,
    slotsAdded,
    slotsRemoved,
    identical: slots.length === 0 && slotsAdded.length === 0 && slotsRemoved.length === 0,
  };
}

/** O histórico de um patch, do mais NOVO ao mais antigo, sem a cadeia. */
export async function libraryVersions(presetId: string): Promise<LibraryVersionRow[]> {
  if (!dentroDoShell()) {
    const doPatch = versoesDoPatch(presetId).reverse();
    return doPatch.map((v) => ({
      id: v.id,
      presetId: v.presetId,
      seq: v.seq,
      savedAt: v.savedAt,
      name: v.name,
      current: v.seq === doPatch[0]?.seq,
    }));
  }
  return runCommand("library_versions", IDEMPOTENTE, () =>
    invocar<LibraryVersionRow[]>("library_versions", { presetId }),
  );
}

/** Uma versão completa (com a cadeia). `null` = não existe mais. */
export async function libraryVersion(id: number): Promise<LibraryVersion | null> {
  if (!dentroDoShell()) return versaoPorId(id) ?? null;
  return runCommand("library_version", IDEMPOTENTE, () => invocar<LibraryVersion | null>("library_version", { id }));
}

/** O diff no nível do knob entre duas versões. */
export async function libraryDiff(before: number, after: number): Promise<ChainDiff> {
  if (!dentroDoShell()) {
    const a = versaoPorId(before);
    const b = versaoPorId(after);
    if (!a || !b) throw new Error(`a versao ${!a ? before : after} nao existe`);
    return diffCadeias(a.payload ?? "[]", b.payload ?? "[]");
  }
  return runCommand("library_diff", IDEMPOTENTE, () => invocar<ChainDiff>("library_diff", { before, after }));
}

/**
 * Restauração TOTAL: o patch volta a ser o que a versão `id` era.
 *
 * **Cria uma versão nova** — nasce por `librarySave`, que é a MESMA função que
 * o resto da app usa para salvar (e que versiona no crate). O presente não
 * some: a versão restaurada continua na lista, e o patch ganha a `N+1`.
 */
export async function libraryRestoreVersion(id: number): Promise<LibraryVersion> {
  if (!dentroDoShell()) {
    const antiga = versaoPorId(id);
    if (!antiga) throw new Error(`a versao ${id} nao existe`);
    const atual = bancoFallback().get(antiga.presetId);
    if (!atual) throw new Error("o patch nao existe mais");
    await librarySave({ ...atual, name: antiga.name, savedAt: new Date().toISOString(), payload: antiga.payload });
    const criada = versoesDoPatch(antiga.presetId).at(-1);
    if (!criada) throw new Error("a restauracao nao criou versao");
    return criada;
  }
  return runCommand("library_restore_version", SEM_RETRY_ESCRITA, () =>
    invocar<LibraryVersion>("library_restore_version", { id }),
  );
}

/**
 * Restauração PONTUAL: só o knob `(slot, pos)` volta ao valor da versão `id`.
 *
 * O valor vem da versão ANTIGA e é escrito na cadeia CORRENTE — se viesse da
 * antiga, restaurar um knob traria junto todas as outras mudanças e viraria
 * uma restauração total disfarcada.
 */
export async function libraryRestoreKnob(id: number, slot: number, pos: number): Promise<LibraryVersion> {
  if (!dentroDoShell()) {
    const antiga = versaoPorId(id);
    if (!antiga) throw new Error(`a versao ${id} nao existe`);
    const atual = bancoFallback().get(antiga.presetId);
    if (!atual?.payload) throw new Error("o patch corrente nao tem cadeia gravada");
    const alvo = leSlots(antiga.payload ?? "[]").find((s) => s.slot === slot);
    const knob = alvo?.knobs.find((k) => k.pos === pos);
    if (!alvo || !knob) throw new Error(`o slot ${slot} nao tem knob na posicao ${pos}`);
    // Só o valor muda: o resto do objeto do knob (range, options, kind) vem do
    // documento inteiro, para que um campo novo do palco não seja apagado.
    const doc = JSON.parse(atual.payload) as Record<string, unknown>[];
    const alvoSlot = doc.find((x) => x.slot === slot);
    const knobs = (alvoSlot?.knobs ?? []) as Record<string, unknown>[];
    const alvoKnob = knobs.find((x) => x.pos === pos);
    if (!alvoKnob) throw new Error(`o slot ${slot} nao tem knob na posicao ${pos}`);
    alvoKnob.value = knob.value;
    await librarySave({ ...atual, savedAt: new Date().toISOString(), payload: JSON.stringify(doc) });
    const criada = versoesDoPatch(antiga.presetId).at(-1);
    if (!criada) throw new Error("a restauracao nao criou versao");
    return criada;
  }
  return runCommand("library_restore_knob", SEM_RETRY_ESCRITA, () =>
    invocar<LibraryVersion>("library_restore_knob", { id, slot, pos }),
  );
}
