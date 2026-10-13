/**
 * ipc/ir — a porta do front para o laboratório de IRs (issue #24).
 *
 * **Por que uma porta à parte de `library.ts` e de `tones.ts`.** A biblioteca
 * é o catálogo de patches; o tones.ts é o conteúdo convertido do SnapTone. O IR
 * é o response de cabinets do dono — arquivo que vai direto para o aparelho,
 * byte a byte, sem conversão. Misturar os três faria `LibraryPreset` carregar
 * `Vec<u8>` e a lista de presets pagaria centenas de KB por linha.
 *
 * **O `.ir` entra como bytes, não como caminho.** O webview do Tauri não tem
 * acesso ao disco (o projeto não usa plugin de `fs`) e o `invoke` trafega
 * JSON: quem lê o arquivo é o `<input type="file">` do componente, que entrega
 * os bytes como array de números — a mesma coisa que o serde produz de um
 * `Vec<u8>`.
 *
 * **Os 20 slots e o chunk de 15B vêm do crate, não de literais aqui.** São as
 * mesmas regras do fio (§13.7 e §13.12): o que a tela valida antes de offering
 * um botão precisa ser o mesmo número que o `upload_ir` valida, ou o dono vê
 * um botão habilitado que o aparelho recusa.
 *
 * **Políticas de execução.** Ler é `IDEMPOTENTE`. Gravar em arquivo local é
 * `semRetry` (disco cheio e permissão não melhoram na segunda tentativa). O
 * ENVIO ao aparelho é a operação mais LONGA do app — na captura, 296 chunks
 * levaram ~5 s, o que dá ~17 ms por chunk; um IR de 300 KB são ~20.000 chunks
 * e quase 6 minutos — e **não entra na política de retry padrão**: repetir um
 * stream de 20.000 chunks deixaria o aparelho no meio de duas transferências.
 */
import { IDEMPOTENTE, runCommand, semRetry } from "./device";
import type { CommandPolicy } from "./device";

/** Os 20 slots de User IR (`<ppIRInfo0..19>`, §13.12). */
export const IR_SLOTS = 20;

/** Bytes de payload por chunk (§13.7): 30 nibbles = 15 bytes reais. */
export const IR_CHUNK_BYTES = 15;

/**
 * Timeout do envio: ~17 ms por chunk na captura (296 chunks em ~5 s) e folga
 * de 2× sobre um IR de 300 KB (~6 min de envio, 12 min de espera).
 *
 * O padrão do `runCommand` (8 s) desistiria no primeiro minuto de um envio que
 * estava dando certo, e a tela mostraria erro para um aparelho meio gravado.
 */
const ENVIO_TIMEOUT_MS = 720_000;

/** Um IR na lista do laboratório — mesmo shape do `IrRow` do crate (#24). */
export interface Ir {
  /** Identificador derivado do conteúdo (`i<crc32 hex>`). */
  id: string;
  /** Nome que o dono deu (é o que a tela de User IR do aparelho mostra). */
  name: string;
  /** Tamanho do `.ir` em bytes. */
  bytes: number;
  /** CRC-32 do `.ir` (integridade do conteúdo guardado). */
  crc32: number;
  /** Slot do device (0..=19) ou `null` se ainda não foi atribuído. */
  slot: number | null;
  /** ISO-8601 da importação. */
  savedAt: string;
}

/** O quadro do laboratório: a lista e o estado dos slots, na MESMA leitura. */
export interface IrBoard {
  /** Os IRs (atribuídos primeiro pelo slot, depois soltos pelo nome). */
  irs: Ir[];
  /** Quantos slots o aparelho tem. */
  slots: number;
  /** Quantos slots têm IR. */
  usados: number;
}

/**
 * Um IR COM o conteúdo.
 *
 * **Não é exportado.** O laboratorio nunca lê o conteúdo no front: o envio
 * busca os bytes pelo id, do lado do shell (`ir_send`), que é quem tem o
 * banco. Uma função de leitura aqui seria superfície sem quem use — e o gate
 * de deadcode existe para obrigar essa pergunta antes do `export`.
 */
interface IrWithBlob extends Ir {
  /** Os bytes do `.ir`. */
  blob: number[];
}

/** Relatório do envio de um IR ao aparelho (§13.7). */
export interface IrSendReport {
  /** Slot de destino (0..=19). */
  slot: number;
  /** Chunks ÚNICOS enviados (o último vai 2×: o marcador de fim é a duplicação). */
  chunks: number;
  /** ACKs validados (um por chunk + o do marcador). */
  acks: number;
  /** Bytes que foram para o fio. */
  bytes: number;
}

/** Um slot como o APARELHO o relata (`list_user_irs`, §13.12). */
export interface DeviceIrSlot {
  /** Slot 0..=19. */
  slot: number;
  /** Nome que o aparelho tem gravado (`""` = slot vazio, 0xFF*32). */
  name: string;
}

/**
 * Gravar em arquivo local não melhora com repetição: a falha é disco cheio,
 * arquivo corrompido ou permissão — nenhuma delas passa na segunda vez.
 */
const SEM_RETRY_ESCRITA = semRetry("escrita em arquivo local: repetir nao conserta disco/permissao");

/** Importar é tudo-ou-nada (transação); uma transação que falhou não gravou nada. */
const SEM_RETRY_IMPORT = semRetry("import e transacional: falhou = nada foi gravado");

/**
 * O envio é LONGO e NÃO é idempotente como os outros: repetir um stream de
 * 20.000 chunks porque o front não ouviu a resposta deixaria o aparelho no meio
 * de duas transferências — e o último chunk duplicado (§13.7) fecharia a
 * segunda no meio da primeira.
 */
const SEM_RETRY_ENVIO = semRetry(
  "envio de IR leva minutos (20.000 chunks em um IR de 300KB) e repetir = dois streams no device",
);

/**
 * Envia com timeout PRÓPRIO, porque `runCommand` fixa o global (8 s).
 * Sem isso o front desiste no primeiro minuto de um envio que estava dando
 * certo, e mostra erro para um aparelho meio gravado.
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

let fallback: Map<string, IrWithBlob> | null = null;
let fallbackSlot: Map<number, string> | null = null;

/** Zera o fallback (usado pelos testes; fora deles é caso de bug). */
export function resetaFallback(): void {
  fallback = null;
  fallbackSlot = null;
}

function banco(): Map<string, IrWithBlob> {
  if (fallback === null) fallback = new Map();
  return fallback;
}

function slots(): Map<number, string> {
  if (fallbackSlot === null) fallbackSlot = new Map();
  return fallbackSlot;
}

/** CRC-32 IEEE, o mesmo do crate — o fallback tem que produzir o MESMO id. */
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
 * **Id ocupado NÃO é o mesmo que colisão.** Se o id já existe E os bytes são
 * os mesmos, é o mesmo arquivo sendo reimportado — e o registro é atualizado
 * (o id volta, o nome muda). Só quando os bytes DIFEREM é colisão de CRC, e aí
 * o sufixo `-2` impede que um arquivo sobrescreva o outro. Sem a comparação
 * dos bytes, reimportar o mesmo `.ir` criava um `-2` a cada vez.
 */
function idFallback(blob: number[]): string {
  const base = `i${crc32(blob).toString(16).padStart(8, "0")}`;
  const guardado = banco().get(base);
  if (guardado === undefined) return base;
  if (iguais(guardado.blob, blob)) return base;
  let n = 2;
  while (banco().has(`${base}-${n}`)) n += 1;
  return `${base}-${n}`;
}

/** Dois arrays de bytes são iguais? */
function iguais(a: number[], b: number[]): boolean {
  return a.length === b.length && a.every((v, i) => v === b[i]);
}

/** A linha da lista: o MESMO shape do crate, sem o `blob` (a lista é leve). */
function linhaDe(ir: IrWithBlob): Ir {
  return {
    id: ir.id,
    name: ir.name,
    bytes: ir.bytes,
    crc32: ir.crc32,
    slot: ir.slot,
    savedAt: ir.savedAt,
  };
}

/** O quadro do fallback: mesma ordem e mesmos contadores do crate. */
function quadroFallback(): IrBoard {
  const irs = [...banco().values()]
    .map(linhaDe)
    .sort((a, b) => {
      if (a.slot == null && b.slot != null) return 1;
      if (a.slot != null && b.slot == null) return -1;
      if (a.slot !== b.slot) return (a.slot ?? 0) - (b.slot ?? 0);
      return a.name.localeCompare(b.name);
    });
  return { irs, slots: IR_SLOTS, usados: irs.filter((i) => i.slot != null).length };
}

/**
 * O arquivo é ENVIÁVEL? A mesma regra do §13.7 que o `upload_ir` e a
 * importação aplicam — e a tela usa isto para desabilitar o botão COM o
 * motivo, em vez de descobrir a recusa com o aparelho na mão.
 */
export function irEnviavel(ir: Pick<Ir, "bytes">): boolean {
  return ir.bytes > 0 && ir.bytes % IR_CHUNK_BYTES === 0;
}

/** O quadro completo (lista + slots). */
export async function irBoard(): Promise<IrBoard> {
  if (!dentroDoShell()) return runCommand("ir_board", IDEMPOTENTE, async () => quadroFallback());
  return runCommand("ir_board", IDEMPOTENTE, () => invocar<IrBoard>("ir_board"));
}

/** Números do laboratório (rodapé). */
export async function irStats(): Promise<IrBoard> {
  return irBoard();
}

/**
 * A tabela de User IRs **do aparelho** (`list_user_irs`, §13.12): 20 slots com
 * o nome que o device tem gravado.
 *
 * **Esta leitura é do DEVICE, não do banco.** São coisas diferentes e
 * confundi-las seria a tela afirmar que o slot 3 tem o "Vintage" porque o
 * arquivo está aqui, quando o aparelho tem outra coisa nesse slot. Por isso
 * ela é uma função separada, com o nome do comando no lugar.
 */
export async function deviceIrTable(): Promise<DeviceIrSlot[]> {
  if (!dentroDoShell()) {
    // Fora do shell não há device: 20 slots vazios é a resposta honesta
    // (0xFF*32 = slot nunca usado, §13.12), e a UI sabe que não device.
    return runCommand("list_user_irs", IDEMPOTENTE, async () =>
      Array.from({ length: IR_SLOTS }, (_, slot) => ({ slot, name: "" })),
    );
  }
  return runCommand("list_user_irs", IDEMPOTENTE, async () => {
    const tabela = await invocar<{ slots: DeviceIrSlot[] }>("list_user_irs");
    return tabela.slots;
  });
}

/**
 * Importa um `.ir` (bytes escolhidos pelo dono).
 *
 * O id é o MESMO do crate (CRC do conteúdo), para que a lista não "suma do
 * nada" ao trocar do fallback para o shell — dois lugares com polinômios
 * diferentes dariam dois ids para o mesmo arquivo.
 *
 * **Regrava NÃO perde o slot.** O mesmo `.ir` reimportado com um nome novo
 * continua no slot que tinha: quem dependia do slot 3 não perde a atribuição
 * por causa de uma reimportação.
 */
export async function irImport(name: string, blob: number[]): Promise<Ir> {
  if (!dentroDoShell()) {
    const linhas = banco();
    const id = idFallback(blob);
    const existente = linhas.get(id);
    const linha: IrWithBlob = {
      id,
      name: name.trim(),
      bytes: blob.length,
      crc32: crc32(blob),
      slot: existente?.slot ?? null,
      savedAt: new Date().toISOString(),
      blob: [...blob],
    };
    linhas.set(id, linha);
    return linhaDe(linha);
  }
  return runCommand("ir_import", SEM_RETRY_IMPORT, () =>
    invocar<Ir>("ir_import", { name, blob }),
  );
}

/** Renomeia. `false` = não existia. */
export async function irRename(id: string, name: string): Promise<boolean> {
  if (!dentroDoShell()) {
    return runCommand("ir_rename", SEM_RETRY_ESCRITA, async () => {
      const ir = banco().get(id);
      if (!ir) return false;
      ir.name = name.trim();
      return true;
    });
  }
  return runCommand("ir_rename", SEM_RETRY_ESCRITA, () => invocar<boolean>("ir_rename", { id, name }));
}

/**
 * Atribui (ou desliga, com `slot = null`) o slot de um IR.
 *
 * **Um slot, um IR.** O fallback recusa a segunda atribuição com o mesmo erro
 * do crate: aceitar dois IRs no slot 3 deixaria a tela dizendo que o aparelho
 * tem uma coisa que ele não tem.
 */
export async function irAssignSlot(id: string, slot: number | null): Promise<void> {
  if (!dentroDoShell()) {
    return runCommand("ir_assign_slot", SEM_RETRY_ESCRITA, async () => {
      if (slot != null && (slot < 0 || slot >= IR_SLOTS)) {
        throw new Error(`slot de User IR invalido: ${slot} (esperado 0..=${IR_SLOTS - 1})`);
      }
      const dono = slots().get(slot ?? -1);
      if (dono != null && dono !== id) {
        throw new Error(`o slot ${slot} ja e do IR '${banco().get(dono)?.name ?? dono}'`);
      }
      const ir = banco().get(id);
      if (!ir) throw new Error(`IR ${id} inexistente`);
      for (const [s, quem] of slots()) if (quem === id) slots().delete(s);
      ir.slot = slot;
      if (slot != null) slots().set(slot, id);
    });
  }
  return runCommand("ir_assign_slot", SEM_RETRY_ESCRITA, () =>
    invocar<void>("ir_assign_slot", { id, slot }),
  );
}

/** Apaga da BIBLIOTECA. `false` = não existia. */
export async function irDelete(id: string): Promise<boolean> {
  if (!dentroDoShell()) {
    return runCommand("ir_delete", SEM_RETRY_ESCRITA, async () => {
      for (const [s, quem] of slots()) if (quem === id) slots().delete(s);
      return banco().delete(id);
    });
  }
  return runCommand("ir_delete", SEM_RETRY_ESCRITA, () => invocar<boolean>("ir_delete", { id }));
}

/**
 * Envia o IR ao aparelho (§13.7).
 *
 * **O comando bloqueia por MINUTOS** — na captura, 296 chunks levaram ~5 s, e
 * um IR de 300 KB são ~20.000 chunks. A tela mostra "enviando…" e espera o
 * `invoke` resolver; o botão fica travado porque repetir aqui seriam dois
 * streams no aparelho.
 */
export async function irSend(id: string): Promise<IrSendReport> {
  if (!dentroDoShell()) {
    return envio("ir_send", SEM_RETRY_ENVIO, async () => {
      const ir = banco().get(id);
      if (!ir) throw new Error(`IR ${id} nao existe`);
      if (ir.slot == null) throw new Error("o IR nao tem slot atribuido");
      // O stream de §13.7: 15B por chunk, com ACK por chunk e o último
      // duplicado como marcador. O fallback não tem ACK nem device, então o
      // que ele reproduz é a CONTA — que é o que a tela mostra no relatório.
      const chunks = Math.ceil(ir.blob.length / IR_CHUNK_BYTES);
      return { slot: ir.slot, chunks, acks: chunks + 1, bytes: ir.blob.length };
    });
  }
  return envio("ir_send", SEM_RETRY_ENVIO, () => invocar<IrSendReport>("ir_send", { id }));
}
