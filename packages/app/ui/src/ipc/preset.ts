/**
 * ipc/preset — a porta do PRESET COMO ARQUIVO (issue #114).
 *
 * **Por que esta porta existe separada de `library.ts`.** A biblioteca é o
 * banco de patches (99 de fábrica + os do dono, num arquivo SQLite); isto aqui
 * é UM preset levado para fora, em dois formatos que um humano lê: o JSON
 * versionado e a folha de timbre em PDF. Um é o arquivo do banco, o outro é a
 * exportação do preset corrente — sem nome parecido que os confunda.
 *
 * **A política é `IDEMPOTENTE` nas três.** Nada aqui escreve: exportar lê o
 * `all.prst` embutido e projeta, importar lê o arquivo do dono. Repetir uma
 * leitura que falhou pode curar (antivírus segurando o arquivo no Windows), e
 * o `runCommand` já cuida disso. O que NÃO existe aqui é escrita no aparelho:
 * o importado vai para o PALCO, e gravar no GP-100 continua sendo um ato
 * separado, com a trava do build de campo (ADR-4/ADR-5).
 *
 * **Fora do webview do Tauri** (vitest/jsdom, `pnpm dev` no navegador) não há
 * `window.__TAURI_INTERNALS__` e não há core Rust. O fallback reproduz o
 * CONTRATO — exportar, reimportar e ver a cadeia preservada — com o único
 * documento que ele tem: a cadeia que o palco já desenha (`deviceBoard`).
 * É por isso que o formato do fallback NÃO é o `gp100.preset` do core e sim um
 * envelope próprio, versionado:
 *
 *   - o `gp100.preset` é o `.prst` INTEIRO (a árvore de texto com o layout de
 *     quebra de linha), e o front não tem o `.prst` — só a cadeia derivada;
 *   - usar o MESMO `format` para as duas formas seria pior do que usar outro:
 *     os dois parsers recusariam o arquivo um do outro com a mesma mensagem
 *     ambígua, e um arquivo exportado no navegador pareceria um preset
 *     corrompido dentro do app. Com formato próprio, cada lado diz exatamente
 *     o que recebeu.
 *
 * A verdade sobre o round-trip byte-idêntico do `.prst` não está aqui: está
 * nos testes Rust (`packages/core/tests/preset_json_roundtrip.rs`), que é onde
 * o arquivo existe.
 */
import type { BoardSlot, BoardView } from "./types";
import { IDEMPOTENTE, deviceBoard, runCommand } from "./device";
import { dentroDoShell, invocar } from "./library";

/**
 * Formato e versão do envelope do FALLBACK (fora do app).
 *
 * Sem `export`: ninguém fora deste arquivo nomeia estas constantes, e o gate
 * `check_deadcode` (#78) cobra o export que não tem consumidor. Quem precisa
 * do valor lê o JSON — é o contrato que interessa.
 */
const FORMATO_CADEIA = "gp100.preset.chain";
const VERSAO_CADEIA = 1;

/** O documento que o fallback exporta e sabe reler. */
interface CadeiaEnvelope {
  format: string;
  version: number;
  preset: {
    pp: number;
    name: string;
    ppType: number;
    ppTypeName: string;
    slots: BoardSlot[];
  };
}

/**
 * A folha de timbre é gerada pelo motor de PDF do core (`tone_sheet.rs`),
 * embutido no app: fora dele não há quem monte o documento, e reimplementar o
 * writer em JS seria um SEGUNDO desenho da mesma folha — que é a forma de os
 * dois divergirem sem ninguém perceber.
 *
 * A tela pergunta isto para DESABILITAR o botão com explicação, em vez de
 * oferecer um botão que sempre falha. É o mesmo tratamento do `writeVerified`
 * do build de campo: a capacidade ausente se anuncia antes do clique.
 */
export function folhaDeTimbreDisponivel(): boolean {
  return dentroDoShell();
}

/**
 * `preset_export_json` — o preset de fábrica `pp` em JSON versionado.
 *
 * Dentro do app o JSON é o do core (layout do `.prst` incluído, o que faz ele
 * voltar a ser um `.prst` byte a byte). Fora dele é o envelope de cadeia
 * descrito no cabeçalho.
 *
 * # Erros
 * No app, a mensagem do core quando o `pp` não existe no arquivo de fábrica.
 */
export async function presetExportJson(pp: number): Promise<string> {
  if (!dentroDoShell()) {
    // A cadeia vem do MESMO caminho que o palco usa para desenhar o preset —
    // o fallback não inventa uma segunda leitura do artefato.
    const board = await deviceBoard(pp);
    const envelope: CadeiaEnvelope = {
      format: FORMATO_CADEIA,
      version: VERSAO_CADEIA,
      preset: {
        pp: board.pp,
        name: board.name,
        ppType: board.ppType,
        ppTypeName: board.ppTypeName,
        slots: board.slots,
      },
    };
    return JSON.stringify(envelope, null, 2);
  }
  return runCommand("preset_export_json", IDEMPOTENTE, () =>
    invocar<string>("preset_export_json", { pp }),
  );
}

/**
 * `preset_export_tone_sheet` — a folha de timbre do preset `pp` em PDF.
 *
 * Devolve os BYTES (a tela monta o download). O arquivo é pequeno: ~5,6 KB por
 * preset de fábrica.
 *
 * # Erros
 * Fora do app não há motor de folha — a tela nem oferece o botão, e este
 * caminho é a defesa em profundidade para quem chamar a porta direto.
 */
export async function presetExportToneSheet(pp: number): Promise<Uint8Array> {
  if (!dentroDoShell()) {
    throw new Error(
      "a folha de timbre e gerada pelo motor de PDF do app: fora dele nao existe",
    );
  }
  return runCommand("preset_export_tone_sheet", IDEMPOTENTE, () =>
    invocar<number[]>("preset_export_tone_sheet", { pp }).then(
      (bytes) => Uint8Array.from(bytes),
    ),
  );
}

/**
 * `preset_import_json` — a cadeia do arquivo, pronta para o palco.
 *
 * Devolve um `BoardView`, o MESMO tipo do `device_board`: a tela aplica pelo
 * caminho que já existe (nada de um segundo formato de cadeia na UI).
 *
 * # Erros
 * Envelope de outra versão, de outro formato, ou sem a cadeia. A mensagem diz
 * o que veio e o que este build lê — a mesma promessa do core, para que o dono
 * entenda que o arquivo é de um app mais novo em vez de achar que corrompeu.
 */
export async function presetImportJson(json: string): Promise<BoardView> {
  if (!dentroDoShell()) return importaCadeia(json);
  return runCommand("preset_import_json", IDEMPOTENTE, () =>
    invocar<BoardView>("preset_import_json", { json }),
  );
}

/** O cabeçalho do envelope, com o rigor do `from_json` do core. */
function importaCadeia(json: string): BoardView {
  let cru: unknown;
  try {
    cru = JSON.parse(json);
  } catch (e) {
    throw new Error(`o arquivo nao e um JSON: ${e instanceof Error ? e.message : String(e)}`, {
      cause: e,
    });
  }
  if (cru === null || typeof cru !== "object" || Array.isArray(cru)) {
    throw new Error(`envelope com format = "${FORMATO_CADEIA}" (veio um JSON sem objeto)`);
  }
  const env = cru as Partial<CadeiaEnvelope>;
  if (env.format !== FORMATO_CADEIA) {
    throw new Error(
      `formato alheio: ${JSON.stringify(env.format ?? null)} (esperado "${FORMATO_CADEIA}")`,
    );
  }
  if (env.version !== VERSAO_CADEIA) {
    throw new Error(
      `versao do arquivo: ${String(env.version ?? "ausente")} (este build le a ${VERSAO_CADEIA})`,
    );
  }
  const p = env.preset;
  if (p == null || typeof p !== "object" || !Array.isArray(p.slots)) {
    throw new Error(`envelope incompleto: falta o preset com a cadeia (preset.slots)`);
  }
  return {
    pp: typeof p.pp === "number" ? p.pp : 0,
    name: typeof p.name === "string" ? p.name : "",
    ppType: typeof p.ppType === "number" ? p.ppType : 0,
    ppTypeName: typeof p.ppTypeName === "string" ? p.ppTypeName : "",
    slots: p.slots,
    // Um arquivo exportado é sempre de fábrica: o `.prst` vem do `all.prst`
    // embutido, e o patch do dono mora no arquivo da biblioteca.
    bank: "factory",
    ppLabel: `P${String((typeof p.pp === "number" ? p.pp : 0) + 1).padStart(2, "0")}`,
  };
}
