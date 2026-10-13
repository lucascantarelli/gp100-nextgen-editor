/**
 * ipc/tones — a porta do front para o gestor de SnapTone/NAM (issue #25).
 *
 * **Por que uma porta à parte de `library.ts`.** A biblioteca é o catálogo de
 * patches e a serialização deles; aqui o conteúdo são os **modelos convertidos**
 * (`.clo`, ~2,7 KB cada), que têm ciclo de vida próprio — importar, atribuir a
 * um dos 5 slots do aparelho, mandar para o device e ouvir em A/B. Misturar os
 * dois faria `LibraryPreset` carregar `Vec<u8>` e a tela de presets pagaria
 * bytes que não usa.
 *
 * **O `.clo` entra como bytes, não como caminho.** O webview do Tauri não tem
 * acesso ao disco (o projeto não usa plugin de `fs`) e o `invoke` trafega
 * JSON: quem lê o arquivo é o `<input type="file">` do componente, que entrega
 * os bytes como array de números — a mesma coisa que o serde produz de um
 * `Vec<u8>`. Passar caminho exigiria um plugin novo e uma permissão para ler o
 * disco inteiro do usuário, para escolher UM arquivo.
 *
 * **A conversão do `.nam` NÃO acontece aqui.** As strings do Valeton Suite
 * mostram o caminho (`Choose a nam file to open it` → `*.nam` → conversão →
 * `/name.clo`) e o motor NAM mora no exe da Valeton (`BLOCKERS.md`). O app
 * importa o `.clo` já convertido; um `.nam` importado aqui entraria como
 * bytes que o aparelho não entende, e a UI não deve fingir que converte o que
 * não converte.
 *
 * **Políticas de execução.** Ler é `IDEMPOTENTE`. Gravar em arquivo local é
 * `semRetry` (disco cheio e permissão não melhoram na segunda tentativa —
 * mesma política da #26). O ENVIO ao aparelho é a operação mais longa do app
 * (143 blocos × 250 ms de settle, §4/§5) e **não entra na política de retry
 * padrão**, porque o timeout por tentativa (8 s) é menor que o envio inteiro:
 * o `runCommand` abortaria a espera e o stream continuaria no actor. Por isso
 * ele declara o próprio timeout.
 */
import { IDEMPOTENTE, runCommand, semRetry } from "./device";
import type { CommandPolicy } from "./device";

/** Um tom na lista do gestor — mesmo shape do `ToneRow` do crate (#25). */
export interface Tone {
  /** Identificador derivado do conteúdo (`t<crc32 hex>`). */
  id: string;
  /** Nome que o dono deu. */
  name: string;
  /** Tamanho do `.clo` em bytes. */
  bytes: number;
  /** CRC-32 do `.clo` (integridade do conteúdo guardado). */
  crc32: number;
  /** Slot do device (1..=5) ou `null` se ainda não foi atribuído. */
  slot: number | null;
  /** ISO-8601 da importação. */
  savedAt: string;
}

/** O quadro do gestor: a lista e o estado dos slots, na MESMA leitura. */
export interface ToneBoard {
  /** Os tons (atribuídos primeiro pelo slot, depois soltos pelo nome). */
  tones: Tone[];
  /** Quantos slots o aparelho tem (`SnapTone1..5`, §5). */
  slots: number;
  /** Quantos slots têm tom. */
  usados: number;
}

/** Um tom COM o modelo — o que a tela de A/B toca e o envio manda. */
export interface ToneWithModel extends Tone {
  /** Os bytes do `.clo`. */
  model: number[];
  /**
   * O `nam_output_wav.wav` que o Suite renderiza ao lado do `.clo`, ou
   * `null` se o dono não o tiver.
   *
   * Não está em `Tone` de propósito: a lista são linhas de tela e o WAV são
   * alguns MB por tom — carregá-lo na busca faria a lista pagar áudio que
   * ninguém ouviu.
   */
  preview: number[] | null;
}

/** Relatório do envio de um tom ao aparelho (§5). */
export interface ToneSendReport {
  /** Slot de destino (1..=5). */
  slot: number;
  /** Blocos enviados. */
  blocks: number;
  /** ACKs de 16B validados. */
  acks: number;
  /** Bytes que foram para o fio. */
  bytes: number;
}

/** Timeout do envio ao aparelho: 143 blocos × 250 ms + folga. */
const ENVIO_TIMEOUT_MS = 60_000;

/**
 * Gravar em arquivo local não melhora com repetição: a falha é disco cheio,
 * arquivo corrompido ou permissão — nenhuma delas passa na segunda vez.
 */
const SEM_RETRY_ESCRITA = semRetry("escrita em arquivo local: repetir nao conserta disco/permissao");

/** Importar é tudo-ou-nada (transação); uma transação que falhou não gravou nada. */
const SEM_RETRY_IMPORT = semRetry("import e transacional: falhou = nada foi gravado");

/**
 * O envio é LONGO e não é idempotente como os outros comandos: repetir um
 * stream de 143 blocos porque o front não ouviu a resposta poderia deixar o
 * aparelho no meio de duas transferências. O timeout declarado é maior que o
 * envio inteiro — com o padrão (8 s) o `runCommand` desistiria na primeira
 * tentativa e mostraria erro para um envio que estava dando certo.
 */
const SEM_RETRY_ENVIO = semRetry(
  "envio de SnapTone leva ~36s (143 blocos x 250ms do §4) e repetir = dois streams no device",
);

/**
 * O envio precisa de um timeout PRÓPRIO, porque `runCommand` fixa o global
 * (8 s) que é curto para um stream de 143 blocos — o front desistiria da
 * espera e mostraria erro para um envio que estava dando certo. Sem retry:
 * repetir um stream deixa o aparelho no meio de duas transferências.
 */
async function envio<T>(op: string, policy: CommandPolicy, run: () => Promise<T>): Promise<T> {
  const tentativas = policy.kind === "retry" ? Math.max(1, policy.attempts) : 1;
  let ultimo: unknown;
  for (let tentativa = 1; tentativa <= tentativas; tentativa += 1) {
    try {
      return await Promise.race([
        run(),
        new Promise<never>((_, recusa) => {
          setTimeout(() => recusa(new Error(`${op} excedeu ${ENVIO_TIMEOUT_MS} ms`)), ENVIO_TIMEOUT_MS);
        }),
      ]);
    } catch (e) {
      ultimo = e;
    }
  }
  console.error(`${op} falhou:`, ultimo);
  throw ultimo;
}

function dentroDoShell(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function invocar<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

// ─────────────────────────── fallback fora do webview ───────────────────────
//
// Um `Map` por módulo. Fora do shell não há disco nem device: a tela continua
// utilizável e os testes exercitam o caminho real da UI.

let fallback: Map<string, ToneWithModel> | null = null;
let fallbackSlot: Map<number, string> | null = null;

/** Zera o fallback (usado pelos testes; fora deles é caso de bug). */
export function resetaFallback(): void {
  fallback = null;
  fallbackSlot = null;
}

function banco(): Map<string, ToneWithModel> {
  if (fallback === null) fallback = new Map();
  return fallback;
}

function slots(): Map<number, string> {
  if (fallbackSlot === null) fallbackSlot = new Map();
  return fallbackSlot;
}

/** CRC-32 IEEE, o mesmo do crate — o fallback tem que producir o MESMO id. */
function crc32(dados: number[]): number {
  let crc = 0xffffffff;
  for (const b of dados) {
    crc = (crc ^ (b & 0xff)) >>> 0;
    for (let k = 0; k < 8; k += 1) {
      crc = crc & 1 ? (crc >>> 1) ^ 0xedb88320 : crc >>> 1;
      crc = crc >>> 0;
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

/**
 * O id do fallback é derivado do conteúdo, igual ao do crate.
 *
 * **Id occupied is NOT the same as collision.** Se o id já existe E os bytes são
 * os mesmos, é o mesmo arquivo sendo reimportado — e o registro é atualizado
 * (o id volta, o nome muda). Só quando os bytes DIFEREM é colisão de CRC, e aí
 * o sufixo `-2` impede que um arquivo sobrescreva o outro. Sem a comparação dos
 * bytes, reimportar o mesmo `.clo` criava um `-2` a cada vez — o dono
 * reexportava do Suite, reimportava, e via duplicados com o mesmo nome.
 */
function idFallback(modelo: number[]): string {
  const base = `t${crc32(modelo).toString(16).padStart(8, "0")}`;
  const guardado = banco().get(base);
  if (guardado === undefined) return base;
  if (iguais(guardado.model, modelo)) return base;
  let n = 2;
  while (banco().has(`${base}-${n}`)) n += 1;
  return `${base}-${n}`;
}

/** Dois arrays de bytes são iguais? */
function iguais(a: number[], b: number[]): boolean {
  return a.length === b.length && a.every((v, i) => v === b[i]);
}

/** A linha da lista: o MESMO shape do crate, sem o `model` (a lista é leve). */
function linhaDe(t: ToneWithModel): Tone {
  return {
    id: t.id,
    name: t.name,
    bytes: t.bytes,
    crc32: t.crc32,
    slot: t.slot,
    savedAt: t.savedAt,
  };
}

/** O quadro do fallback: mesma ordem e mesmos contadores do crate. */
function quadroFallback(): ToneBoard {
  const tons = [...banco().values()]
    .map(linhaDe)
    .sort((a, b) => {
      if (a.slot == null && b.slot != null) return 1;
      if (a.slot != null && b.slot == null) return -1;
      if (a.slot !== b.slot) return (a.slot ?? 0) - (b.slot ?? 0);
      return a.name.localeCompare(b.name);
    });
  return { tones: tons, slots: 5, usados: tons.filter((t) => t.slot != null).length };
}

/** O quadro completo (lista + slots). */
export async function toneBoard(): Promise<ToneBoard> {
  if (!dentroDoShell()) return runCommand("tone_board", IDEMPOTENTE, async () => quadroFallback());
  return runCommand("tone_board", IDEMPOTENTE, () => invocar<ToneBoard>("tone_board"));
}

/** Números do gestor (rodapé). */
export async function toneStats(): Promise<ToneBoard> {
  return toneBoard();
}

/**
 * Importa um `.clo` (bytes já convertidos pelo Suite) e, opcionalmente, o
 * `nam_output_wav.wav` que o Suite renderiza ao lado dele.
 *
 * `model` são os bytes do `.clo` e `preview` os do WAV (`null` = o dono não
 * tem áudio, que é o caso normal). O fallback calcula o MESMO id por CRC que o
 * crate — dois lugares que discordassem dariam um id diferente dentro e fora do
 * shell, e a lista "sumiria" do nada ao abrir o app de verdade.
 *
 * **Regrava NÃO perde o preview nem o slot.** O mesmo `.clo` reimportado com
 * um nome novo continua com o áudio e com o slot que tinha: quem atribuiu o
 * slot 3 não perdeu a atribuição por causa de uma reimportação, e o A/B não
 * perdeu o áudio que o dono tinha exportado.
 */
export async function toneImport(name: string, model: number[], preview?: number[]): Promise<Tone> {
  if (!dentroDoShell()) {
    const linhas = banco();
    const id = idFallback(model);
    const existente = linhas.get(id);
    const linha: ToneWithModel = {
      id,
      name: name.trim(),
      bytes: model.length,
      crc32: crc32(model),
      slot: existente?.slot ?? null,
      savedAt: new Date().toISOString(),
      model: [...model],
      preview: preview != null ? [...preview] : (existente?.preview ?? null),
    };
    linhas.set(id, linha);
    return linhaDe(linha);
  }
  // O command devolve a linha gravada; devolvê-la direto é melhor do que
  // reler o quadro e caçar o id — uma releitura seria uma segunda fonte de
  // verdade para a tela pintar.
  return runCommand("tone_import", SEM_RETRY_IMPORT, () =>
    invocar<Tone>("tone_import", { name, model, preview: preview ?? null }),
  );
}

/** Grava (ou apaga, com `null`) só o áudio de preview de um tom. */
export async function toneSetPreview(id: string, preview: number[] | null): Promise<void> {
  if (!dentroDoShell()) {
    return runCommand("tone_set_preview", SEM_RETRY_ESCRITA, async () => {
      const t = banco().get(id);
      if (!t) throw new Error(`tom ${id} inexistente`);
      t.preview = preview != null ? [...preview] : null;
    });
  }
  return runCommand("tone_set_preview", SEM_RETRY_ESCRITA, () =>
    invocar<void>("tone_set_preview", { id, preview }),
  );
}

/** Lê UM tom com o modelo. `null` = não existe. */
export async function toneGet(id: string): Promise<ToneWithModel | null> {
  if (!dentroDoShell()) return runCommand("tone_get", IDEMPOTENTE, async () => banco().get(id) ?? null);
  return runCommand("tone_get", IDEMPOTENTE, () => invocar<ToneWithModel | null>("tone_get", { id }));
}

/** O tom de um slot, com o modelo. `null` = slot vazio. */
export async function toneDoSlot(slot: number): Promise<ToneWithModel | null> {
  if (!dentroDoShell()) {
    return runCommand("tone_do_slot", IDEMPOTENTE, async () => {
      const id = slots().get(slot);
      return id ? (banco().get(id) ?? null) : null;
    });
  }
  return runCommand("tone_do_slot", IDEMPOTENTE, () => invocar<ToneWithModel | null>("tone_do_slot", { slot }));
}

/** Renomeia. `false` = não existia. */
export async function toneRename(id: string, name: string): Promise<boolean> {
  if (!dentroDoShell()) {
    return runCommand("tone_rename", SEM_RETRY_ESCRITA, async () => {
      const t = banco().get(id);
      if (!t) return false;
      t.name = name.trim();
      return true;
    });
  }
  return runCommand("tone_rename", SEM_RETRY_ESCRITA, () => invocar<boolean>("tone_rename", { id, name }));
}

/**
 * Atribui (ou desliga, com `slot = null`) o slot de um tom.
 *
 * **Um slot, um tom.** O fallback recusa a segunda atribuição com o mesmo erro
 * do crate: aceitar dois tons no slot 3 deixaria a tela dizendo que o
 * aparelho tem uma coisa que ele não tem.
 */
export async function toneAssignSlot(id: string, slot: number | null): Promise<void> {
  if (!dentroDoShell()) {
    return runCommand("tone_assign_slot", SEM_RETRY_ESCRITA, async () => {
      if (slot != null && (slot < 1 || slot > 5)) {
        throw new Error(`slot de SnapTone invalido: ${slot} (esperado 1..=5)`);
      }
      const dono = slots().get(slot ?? -1);
      if (dono != null && dono !== id) {
        throw new Error(`o slot ${slot} ja e do tom '${banco().get(dono)?.name ?? dono}'`);
      }
      const t = banco().get(id);
      if (!t) throw new Error(`tom ${id} inexistente`);
      for (const [s, quem] of slots()) if (quem === id) slots().delete(s);
      t.slot = slot;
      if (slot != null) slots().set(slot, id);
    });
  }
  return runCommand("tone_assign_slot", SEM_RETRY_ESCRITA, () =>
    invocar<void>("tone_assign_slot", { id, slot }),
  );
}

/** Apaga. `false` = não existia. */
export async function toneDelete(id: string): Promise<boolean> {
  if (!dentroDoShell()) {
    return runCommand("tone_delete", SEM_RETRY_ESCRITA, async () => {
      for (const [s, quem] of slots()) if (quem === id) slots().delete(s);
      return banco().delete(id);
    });
  }
  return runCommand("tone_delete", SEM_RETRY_ESCRITA, () => invocar<boolean>("tone_delete", { id }));
}

/**
 * Envia o tom ao aparelho (§5).
 *
 * **O comando bloqueia por dezenas de segundos** — 143 blocos com o settle de
 * 250 ms entre eles. A tela mostra "enviando…" e espera o `invoke` resolver; o
 * botão fica travado porque repetir aqui seria dois streams no aparelho.
 */
export async function toneSend(id: string): Promise<ToneSendReport> {
  if (!dentroDoShell()) {
    return envio("tone_send", SEM_RETRY_ENVIO, async () => {
      const t = banco().get(id);
      if (!t) throw new Error(`tom ${id} nao existe`);
      if (t.slot == null) throw new Error("o tom nao tem slot atribuido");
      // O stream de §5: 19B por bloco, o último curto. O fallback não tem ACK
      // nem settle de 250 ms (não há device), então o que ele reproduz é a
      // CONTA — que é o que a tela mostra no relatório.
      const blocos = Math.ceil(t.model.length / 19);
      return { slot: t.slot, blocks: blocos, acks: blocos, bytes: t.model.length };
    });
  }
  return envio("tone_send", SEM_RETRY_ENVIO, () => invocar<ToneSendReport>("tone_send", { id }));
}