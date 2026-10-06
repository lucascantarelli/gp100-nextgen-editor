/**
 * ipc/library — a porta do front para a biblioteca persistente (issue #26).
 *
 * **Por que esta porta existe separada de `ipc/device.ts`.** O device é
 * tráfego de fio: uma escrita que falhou pode ter ido a meio caminho, o USB
 * pisca, e repetir é o comportamento certo. A biblioteca é um arquivo local:
 * uma leitura que falhou é I/O, e uma escrita que falhou é disco cheio ou
 * arquivo corrompido — repetir não conserta nenhum dos dois, só demora mais
 * para mostrar o mesmo erro. Por isso a política aqui é `IDEMPOTENTE` para
 * ler e `semRetry(motivo)` para gravar, e o motivo está escrito no ponto da
 * chamada (é exatamente para isso que a política virou tipo no #92).
 *
 * **Fora do webview do Tauri** (vitest/jsdom, `pnpm dev` no navegador) não há
 * `window.__TAURI_INTERNALS__` e não há arquivo. O fallback é um banco EM
 * MEMÓRIA com o mesmo shape, semeado do mesmo artefato de fábrica que o
 * front já usava — a biblioteca continua utilizável fora do shell, e os
 * testes exercitam o caminho de verdade da UI sem SQLite.
 */
import type { BoardSlot } from "./types";
import type { LibraryVersion } from "./history";
import { FACTORY_PRESETS } from "../artifacts/presetData";
import { IDEMPOTENTE, runCommand, semRetry } from "./device";

/** Registro da biblioteca, no mesmo shape do `PresetRow` do crate (#26). */
export interface LibraryPreset {
  /** `f25` (fábrica, pp 25) ou `u<n>` (usuário). */
  id: string;
  /** `"factory"` ou `"user"`. */
  bank: "factory" | "user";
  /** Nº do preset de fábrica; `null` no patch de usuário. */
  pp: number | null;
  /** Nome visível. */
  name: string;
  /** Estilo/tipo numérico (4 = Rock, 6 = Pop). */
  ppType: number;
  /** Rótulo do estilo/tipo. */
  ppTypeName: string;
  /** ISO-8601 da gravação. */
  savedAt: string;
  /** A cadeia está guardada? */
  hasPayload: boolean;
}

/** Números da biblioteca (rodapé da UI). */
export interface LibraryStats {
  total: number;
  factory: number;
  user: number;
  /** `PRAGMA user_version` do arquivo. */
  schema: number;
  /** Versão do SQLite embutido. */
  sqlite: string;
}

/** Contabilidade de um import. */
export interface ImportReport {
  inserted: number;
  replaced: number;
  skipped: number;
}

/** Filtro da busca. Ausente = não filtra. */
export interface LibraryQuery {
  /** Substring do nome. */
  text?: string;
  /** Nº exato. */
  pp?: number;
  /** Estilo/tipo exato. */
  ppType?: number;
  /** Restringe a um banco. */
  bank?: "factory" | "user";
}

/** Registro completo, como o `library_save` recebe e o export carrega. */
export interface LibraryRecord extends LibraryPreset {
  /** Cadeia serializada (o snapshot que volta ao palco). */
  payload: string | null;
}

/**
 * Gravar no arquivo local não mejora com repetição: a falha é disco cheio,
 * arquivo corrompido ou permissão — nenhuma delas passa na segunda tentativa.
 * O `reason` é obrigatório na assinatura (`semRetry`), e ele é o que fica no
 * log de debug quando alguém precisar saber por que aquilo não repetiu.
 */
const SEM_RETRY_ESCRITA = semRetry("escrita em arquivo local: repetir nao conserta disco/permissao");

/** O import é tudo-ou-nada (transação); uma transação que falhou não gravou
 *  nada, então repetir é repetir o trabalho, não consertar a causa. */
const SEM_RETRY_IMPORT = semRetry("import e transacional: falhou = nada foi gravado");

export function dentroDoShell(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

// ─────────────────────────── fallback fora do webview ───────────────────────
//
// Um `Map` por módulo. Ele vive enquanto a página viver — que é a semântica
// certa fora do shell, onde não há disco para onde ir.

type RegistroCompleto = LibraryRecord;

function semeiaFabrica(): Map<string, RegistroCompleto> {
  const m = new Map<string, RegistroCompleto>();
  for (const p of FACTORY_PRESETS) {
    m.set(`f${p.pp}`, {
      id: `f${p.pp}`,
      bank: "factory",
      pp: p.pp,
      name: p.name,
      ppType: p.ppType,
      ppTypeName: p.ppTypeName,
      savedAt: "1970-01-01T00:00:00Z",
      hasPayload: false,
      payload: null,
    });
  }
  return m;
}

let fallback: Map<string, RegistroCompleto> | null = null;

/**
 * Historico de versao em memoria (issue #113): os snapshots que o crate
 * guardaria em `preset_version`.
 *
 * **O store mora AQUI e nao em `history.ts`, e e deliberado.** A regra que
 * garante a feature e "toda gravacao de patch de usuario versiona" — e ela e
 * do `librarySave`, que e o unico caminho de escrita do banco. Se o store
 * vivesse no arquivo do historico, o `librarySave` teria que IMPORTAR dali e o
 * par de arquivos viraria ciclo; mais honesto e manter a invariante num lugar
 * so e deixar `history.ts` ler daqui (a seta de dependencia fica de um lado
 * so: `history -> library`).
 *
 * Vive no MESCO modulo (e nao numa struct) para o reset dos testes apagar banco
 * e historico no mesmo gesto — historico de um patch que o teste acabou de
 * apagar faria o teste seguinte passar errado.
 */
let versoesFallback: LibraryVersion[] = [];
/** Contador de `id`, como o AUTOINCREMENT do SQLite. */
let proximoId = 1;

/**
 * Acrescenta o snapshot de um patch de usuario (espelha `append_versao` do
 * crate). Patch de fabrica nao versiona: ele vem do `all.prst` embutido e e
 * imutavel por construcao.
 */
function registraVersao(p: LibraryRecord): LibraryVersion | null {
  if (p.bank !== "user") return null;
  const doPatch = versoesFallback.filter((v) => v.presetId === p.id);
  const seq = (doPatch.at(-1)?.seq ?? 0) + 1;
  const v: LibraryVersion = {
    id: proximoId++,
    presetId: p.id,
    seq,
    savedAt: p.savedAt,
    name: p.name,
    payload: p.payload,
  };
  versoesFallback = [...versoesFallback, v];
  return v;
}

/** As versoes de um patch, em ordem de gravacao (1..N). Leitura do historico. */
export function versoesDoPatch(presetId: string): LibraryVersion[] {
  return versoesFallback.filter((v) => v.presetId === presetId).sort((a, b) => a.seq - b.seq);
}

/** Uma versao pelo id, ou `undefined`. Leitura do historico. */
export function versaoPorId(id: number): LibraryVersion | undefined {
  return versoesFallback.find((v) => v.id === id);
}

export function bancoFallback(): Map<string, RegistroCompleto> {
  if (fallback === null) fallback = semeiaFabrica();
  return fallback;
}

/** Zera o fallback (usado pelos testes; fora deles é caso de bug). */
export function resetaFallback(): void {
  fallback = null;
  // O historico tambem e estado do fallback: deixar ele sobreviver ao reset
  // faria um teste seguinte ver versoes de um patch que ele ja apagou.
  versoesFallback = [];
  proximoId = 1;
}

/**
 * Busca no fallback. Mesma ordem da busca real: banco (`user` primeiro), depois
 * número. Sem escape de curinga aqui porque é `includes`, não `LIKE` — os dois
 * jeitos são literais e o filtro do SQLite trata os dois como literais também.
 *
 * O texto casa NOME **ou o nº que a tela mostra** (`P01..P99`, 1-based) —
 * mesma regra do `printf('%02d', pp + 1)` do crate. Se as duas pontas
 * divergissem, a mesma busca devolveria coisa diferente dentro e fora do shell,
 * e o dono não teria como saber qual das duas é a certa.
 */
function semPayload(r: RegistroCompleto): LibraryPreset {
  return {
    id: r.id,
    bank: r.bank,
    pp: r.pp,
    name: r.name,
    ppType: r.ppType,
    ppTypeName: r.ppTypeName,
    savedAt: r.savedAt,
    hasPayload: r.hasPayload,
  };
}

/**
 * O texto casa em QUATRO colunas, como o SQL do crate: nome, rótulo do estilo,
 * o número que a coluna mostra (1-based, `P01..P99`) e o `ppID` cru do
 * `all.prst`. Patch de usuário não tem número (`pp` nulo) e por isso não casa
 * por número — casar abriria o patch errado sem erro visível.
 *
 * Esta função e o `WHERE` do crate são o MESMO filtro escrito duas vezes. É
 * duplicação de propósito (o fallback não tem SQL): o preço de divergirem é a
 * busca dar resultado diferente dentro e fora do shell sem ninguém perceber.
 */
function casaTexto(r: RegistroCompleto, texto: string): boolean {
  if (r.name.toLowerCase().includes(texto)) return true;
  if (r.ppTypeName.toLowerCase().includes(texto)) return true;
  if (r.pp == null) return false;
  return String(r.pp + 1).padStart(2, "0").includes(texto) || String(r.pp).includes(texto);
}

function buscaFallback(q: LibraryQuery): LibraryPreset[] {
  const texto = q.text?.trim().toLowerCase();
  return [...bancoFallback().values()]
    .filter((r) => (q.bank ? r.bank === q.bank : true))
    .filter((r) => (q.pp != null ? r.pp === q.pp : true))
    .filter((r) => (q.ppType != null ? r.ppType === q.ppType : true))
    .filter((r) =>
      texto ? casaTexto(r, texto) : true,
    )
    .sort((a, b) => {
      if (a.bank !== b.bank) return a.bank === "user" ? -1 : 1;
      const pa = a.pp ?? Number.MAX_SAFE_INTEGER;
      const pb = b.pp ?? Number.MAX_SAFE_INTEGER;
      return pa !== pb ? pa - pb : a.name.localeCompare(b.name);
    })
    .map(semPayload);
}

// ────────────────────────────── a porta (invoke) ────────────────────────────

export async function invocar<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

/** Busca a biblioteca. `text` vazio/ausente devolve a lista toda. */
export async function librarySearch(q: LibraryQuery = {}): Promise<LibraryPreset[]> {
  if (!dentroDoShell()) return buscaFallback(q);
  // Ler é idempotente e o timeout ajuda: o arquivo pode estar momentaneamente
  // travado por um antivírus no Windows, e essa falha não se cura sozinha.
  return runCommand("library_search", IDEMPOTENTE, () =>
    invocar<LibraryPreset[]>("library_search", {
      text: q.text ?? null,
      pp: q.pp ?? null,
      ppType: q.ppType ?? null,
      // o banco desce JUNTO: e ele que separa a aba de fabrica da aba do dono
      bank: q.bank ?? null,
    }),
  );
}

/** Números da biblioteca. */
export async function libraryStats(): Promise<LibraryStats> {
  if (!dentroDoShell()) {
    const todas = [...bancoFallback().values()];
    const factory = todas.filter((r) => r.bank === "factory").length;
    return {
      total: todas.length,
      factory,
      user: todas.length - factory,
      schema: 0,
      sqlite: "memoria",
    };
  }
  return runCommand("library_stats", IDEMPOTENTE, () => invocar<LibraryStats>("library_stats"));
}

/**
 * Grava um patch do dono.
 *
 * `semRetry`: gravar de novo no banco não conserta disco cheio nem arquivo
 * corrompido. A falha aqui é do arquivo, e repetir só atrasa o erro na tela.
 */
export async function librarySave(p: LibraryRecord): Promise<void> {
  if (!dentroDoShell()) {
    bancoFallback().set(p.id, p);
    // Toda gravacao de patch de usuario versiona — e por aqui, e nao numa
    // chamada a parte, porque e a MESMA porta que o shell usa (`library_save`
    // ja versiona no crate). Um caminho de gravacao que nao versiona seria a
    // historia com um buraco, e nao haveria como o dono saber onde.
    registraVersao(p);
    return;
  }
  await runCommand("library_save", SEM_RETRY_ESCRITA, () => invocar<void>("library_save", { preset: p }));
}

/**
 * Lê UM registro, com a cadeia. `null` = não existe mais.
 *
 * A lista (`librarySearch`) não traz o `payload` de propósito: são 99 linhas
 * e o palco é quem precisa da cadeia. Recarregar a biblioteca inteira para
 * abrir um patch seria trocar uma leitura de 1 por uma de 99 — e o dono
 * sentiria isso em cada clique.
 */
export async function libraryGet(id: string): Promise<LibraryRecord | null> {
  if (!dentroDoShell()) return bancoFallback().get(id) ?? null;
  return runCommand("library_get", IDEMPOTENTE, () => invocar<LibraryRecord | null>("library_get", { id }));
}

/** Apaga pelo id. `false` = não existia. */
export async function libraryDelete(id: string): Promise<boolean> {
  if (!dentroDoShell()) return bancoFallback().delete(id);
  return runCommand("library_delete", SEM_RETRY_ESCRITA, () => invocar<boolean>("library_delete", { id }));
}

/**
 * Importa um envelope JSON. `replace = false` não pisa no que já existe.
 *
 * `semRetry` pelo mesmo motivo de `librarySave`: o import é tudo-ou-nada
 * (uma transação), e uma transação que falhou já não gravou nada — repetir é
 * repetir o trabalho, não consertar.
 */
export async function libraryImport(json: string, replace: boolean): Promise<ImportReport> {
  if (!dentroDoShell()) return importaFallback(json, replace);
  return runCommand("library_import", SEM_RETRY_IMPORT, () =>
    invocar<ImportReport>("library_import", { json, replace }),
  );
}

/** Exporta a biblioteca em JSON versionado. */
export async function libraryExport(): Promise<string> {
  if (!dentroDoShell()) {
    const presets = [...bancoFallback().values()];
    return JSON.stringify(
      {
        format: "gp100.library",
        version: 1,
        exportedAt: new Date().toISOString(),
        sqlite: "memoria",
        schema: 0,
        presets,
      },
      null,
      2,
    );
  }
  return runCommand("library_export", IDEMPOTENTE, () => invocar<string>("library_export"));
}

/** Importa no fallback, com a mesma política de conflito do crate. */
function importaFallback(json: string, replace: boolean): ImportReport {
  let envelope: { presets?: LibraryRecord[] };
  try {
    envelope = JSON.parse(json) as { presets?: LibraryRecord[] };
  } catch {
    return { inserted: 0, replaced: 0, skipped: 0 };
  }
  const rel = { inserted: 0, replaced: 0, skipped: 0 };
  for (const p of envelope.presets ?? []) {
    const banco = bancoFallback();
    if (banco.has(p.id) && !replace) {
      rel.skipped += 1;
      continue;
    }
    if (banco.has(p.id)) rel.replaced += 1;
    else rel.inserted += 1;
    banco.set(p.id, p);
  }
  return rel;
}

// ─────────────────────────── migração do localStorage ───────────────────────

/** Chave antiga dos patches de usuário (pré-#26). */
export const LEGACY_KEY = "gp100.userpatch.v1";

/** Formato do patch antigo, como o `userPatches.ts` gravava. */
interface PatchLegado {
  id: string;
  name: string;
  fromLabel: string;
  savedAt: string;
  slots: BoardSlot[];
}

/** Relatório da migração do `localStorage` para o banco. */
interface MigracaoRelato {
  /** Quantos patches foram enviados ao banco. */
  enviados: number;
  /** Havia algo para migrar? */
  havia: boolean;
}

/**
 * Lê o conteúdo cru do `localStorage` antigo — **síncrono e sem efeito
 * colateral**.
 *
 * Existe separado porque o hook precisa saber "há o que migrar?" ANTES de
 * qualquer `await`: em StrictMode o efeito de mount roda duas vezes, e uma
 * decisão tomada depois do await depende de qual das duas terminou por
 * último. Lendo de cara, a resposta é a mesma nas duas.
 *
 * Devolve `null` quando não há chave, quando está vazia, ou quando o storage
 * está bloqueado (modo privado) — nos três casos a resposta é a mesma para o
 * chamador: não há o que migrar agora.
 */
export function lerLegadoCru(): string | null {
  if (!dentroDoShell()) return null;
  try {
    const cru = localStorage.getItem(LEGACY_KEY);
    return cru && cru.length > 0 ? cru : null;
  } catch {
    return null; // storage bloqueado
  }
}

/**
 * O que a conversão do legado produz, antes de falar com o banco.
 */
interface EnvelopeLegado {
  /** O envelope pronto para `library_import`, ou `null` se não há o que migrar. */
  json: string | null;
  /** Havia patch legado para converter? */
  havia: boolean;
  /** O texto era lixo e NÃO deve ser apagado? */
  ilegivel: boolean;
}

/**
 * Converte o conteúdo cru do `localStorage` antigo em envelope de biblioteca.
 *
 * **Função PURA, e é por isso que ela é separada.** `migraLegado` só roda
 * dentro do webview do Tauri (o banco só existe lá), então testar a migração
 * inteira em vitest/jsdom seria impossível — e a migração é exatamente a parte
 * que pode perder dado do dono. Separando a conversão, o que é arriscado fica
 * testável em qualquer lugar e o que depende do shell fica com uma linha.
 *
 * `ilegivel` é separado de `havia` de propósito: JSON corrompido é o caso em
 * que a chave tem de **ficar** no storage (o dado pode ser recuperável à mão),
 * enquanto storage vazio é o caso em que ela pode sair.
 *
 * @param cru o conteúdo de `localStorage[gp100.userpatch.v1]`
 * @param agora ISO-8601 do instante (o envelope exige um)
 */
export function envelopeDoLegado(cru: string | null, agora: string): EnvelopeLegado {
  if (!cru) return { json: null, havia: false, ilegivel: false };
  let lido: unknown;
  try {
    lido = JSON.parse(cru);
  } catch {
    return { json: null, havia: false, ilegivel: true };
  }
  if (!Array.isArray(lido)) return { json: null, havia: false, ilegivel: true };
  const velhos = lido as PatchLegado[];
  if (velhos.length === 0) return { json: null, havia: false, ilegivel: false };

  const envelope = {
    format: "gp100.library",
    version: 1,
    exportedAt: agora,
    sqlite: "migracao-localStorage",
    schema: 0,
    presets: velhos.map((v) => ({
      id: v.id,
      bank: "user" as const,
      pp: null,
      name: v.name,
      ppType: 4,
      ppTypeName: "Rock",
      savedAt: v.savedAt,
      hasPayload: true,
      payload: JSON.stringify(v.slots),
    })),
  };
  return { json: JSON.stringify(envelope), havia: true, ilegivel: false };
}

/**
 * Empurra os patches que ainda estão no `localStorage` para o banco, uma vez.
 *
 * **Idempotente por construção:** o import vai em `Insert`, então rodar de novo
 * (outro dispositivo, outro perfil, janela aberta duas vezes) conta como
 * `skipped` e não duplica nada. E a chave é removida só depois do import dar
 * certo — um app fechado no meio não perde patch.
 *
 * **Só no shell.** Fora dele o fallback nasce da fábrica e o `localStorage` é
 * justamente de onde os patches vêm para a lista em memória; migrar para a
 * memória seria trocar um lugar por outro sem ganho.
 */
export async function migraLegado(): Promise<MigracaoRelato> {
  if (!dentroDoShell()) return { enviados: 0, havia: false };

  const cru = lerLegadoCru();
  if (cru === null) return { enviados: 0, havia: false };

  const env = envelopeDoLegado(cru, new Date().toISOString());
  if (env.json === null) return { enviados: 0, havia: env.havia };

  const rel = await libraryImport(env.json, false);
  // Só agora a chave pode sair: o import é transacional no crate. E só se o
  // conteúdo era legível — um JSON corrompido fica para o dono recuperar.
  if (!env.ilegivel) {
    try {
      localStorage.removeItem(LEGACY_KEY);
    } catch {
      /* segue: o import ja foi, e a chave orfa e inofensiva */
    }
  }
  return { enviados: rel.inserted, havia: env.havia };
}
