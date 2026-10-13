/**
 * Biblioteca DENTRO do webview do Tauri (`tests/library.shell.test.tsx`).
 *
 * Os testes de `library.test.tsx` rodam em jsdom, onde não há
 * `__TAURI_INTERNALS__` e a porta cai no fallback em memória. Isso deixa duas
 * coisas sem cobertura: a **migração do `localStorage`** (que só roda no
 * shell) e o **nome/shape dos argumentos** de cada `invoke`.
 *
 * Aqui os dois são exercitados de verdade: o arquivo se declara webview e o
 * `@tauri-apps/api/core` é interceptado por um banco em memória que responde
 * pelo mesmo nome de command do shell (`library_search`, `library_save`, …).
 * Se o front mandar `{text, pp, ppType}` e o shell esperar outra coisa, este
 * arquivo quebra — que é o contrato que importa.
 */
import { act, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it } from "vitest";
import { useLibrary } from "../src/hooks/useLibrary";
import { LEGACY_KEY, librarySave } from "../src/ipc/library";
import type { LibraryPreset, LibraryQuery, LibraryRecord } from "../src/ipc/library";
import { FACTORY_PRESETS } from "../src/artifacts/presetData";
import { MSG } from "../src/i18n/messages";
import type { UserPatch } from "../src/userPatches";

/** Banco falso do shell: um Map, os mesmos comandos, nenhum Tauri. */
const banco = new Map<string, LibraryRecord>();
/** Chamadas recebidas, para conferir nome e argumentos. */
const chamadas: Array<{ cmd: string; args: Record<string, unknown> }> = [];

/** Resposta do `invoke` por nome de command — o contrato do shell. */
/** Um command que falha, a mensagem que a UI deve mostrar e como dispara-lo. */
interface CasoDeFalha {
  quebra: string;
  msg: string;
  roda: () => Promise<unknown>;
}

/** Command que a proxima invocacao deve recusar (simula o arquivo travado). */
let quebrado: string | null = null;

/**
 * Roda uma chamada do shell DENTRO de `act`: o `invoke` resolve depois da
 * leitura e o update de estado que vem dele, fora do `act`, vira o aviso
 * "not wrapped in act" do React — ruido que mascara corrida real (issue #142).
 */
async function emAct<T>(fn: () => Promise<T>): Promise<T> {
  let out!: T;
  await act(async () => {
    out = await fn();
  });
  return out;
}

async function invokeDoShell(cmd: string, args: Record<string, unknown> = {}): Promise<unknown> {
  chamadas.push({ cmd, args });
  if (quebrado === cmd) throw new Error(`falha simulada em ${cmd}`);
  switch (cmd) {
    case "library_search": {
      const q = args as unknown as Required<LibraryQuery>;
      const texto = (q.text ?? "").trim().toLowerCase();
      return [...banco.values()]
        .filter((r) => (q.bank == null ? true : r.bank === q.bank))
        .filter((r) => (q.pp == null ? true : r.pp === q.pp))
        .filter((r) => (q.ppType == null ? true : r.ppType === q.ppType))
        .filter((r) => (texto ? r.name.toLowerCase().includes(texto) : true))
        .sort((a, b) => (a.pp ?? 0) - (b.pp ?? 0))
        .map(
          (r): LibraryPreset => ({
            id: r.id,
            bank: r.bank,
            pp: r.pp,
            name: r.name,
            ppType: r.ppType,
            ppTypeName: r.ppTypeName,
            savedAt: r.savedAt,
            hasPayload: r.hasPayload,
          }),
        );
    }
    case "library_stats": {
      const total = banco.size;
      const factory = [...banco.values()].filter((r) => r.bank === "factory").length;
      return { total, factory, user: total - factory, schema: 2, sqlite: "3.46.0" };
    }
    case "library_save": {
      banco.set((args.preset as LibraryRecord).id, args.preset as LibraryRecord);
      return null;
    }
    case "library_get":
      return banco.get(args.id as string) ?? null;
    case "library_delete":
      return banco.delete(args.id as string);
    case "library_import": {
      const env = JSON.parse(args.json as string) as { presets: LibraryRecord[] };
      let inserted = 0;
      let replaced = 0;
      let skipped = 0;
      for (const p of env.presets) {
        if (banco.has(p.id)) {
          if (args.replace === true) {
            banco.set(p.id, p);
            replaced += 1;
          } else skipped += 1;
          continue;
        }
        banco.set(p.id, p);
        inserted += 1;
      }
      return { inserted, replaced, skipped };
    }
    case "library_export":
      return JSON.stringify({
        format: "gp100.library",
        version: 1,
        exportedAt: "2026-03-03T12:00:00Z",
        sqlite: "3.46.0",
        schema: 2,
        presets: [...banco.values()],
      });
    default:
      throw new Error(`command desconhecido: ${cmd}`);
  }
}

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
  // O webview de verdade: o `invoke` da biblioteca `@tauri-apps/api/core` nao
  // fala com o Rust, ele so delega para `window.__TAURI_INTERNALS__.invoke`.
  // Interceptar AQUI (e nao via `vi.mock`) exercita o caminho real — inclusive
  // o `await import()` dinamico da porta, que um mock de modulo nao pegava.
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
    invoke: invokeDoShell,
  };
});

function semeiaFabrica(): void {
  banco.clear();
  for (const p of FACTORY_PRESETS) {
    banco.set(`f${p.pp}`, {
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
}

beforeEach(() => {
  semeiaFabrica();
  chamadas.length = 0;
  quebrado = null;
  localStorage.clear();
});

function legado(id: string, nome: string): UserPatch {
  // o formato ANTIGO tinha `fromLabel` (o rótulo P## de fábrica); a migração
  // não usa esse campo, e o registro novo guarda o ESTILO (`ppTypeName`).
  return { id, name: nome, savedAt: "2026-02-02T00:00:00Z", ppType: 4, ppTypeName: "Rock", slots: [] };
}

let lib: ReturnType<typeof useLibrary> | null = null;

function Probe() {
  lib = useLibrary();
  return null;
}

/**
 * Espera uma condição. A leitura da biblioteca tem POLÍTICA de retry com
 * backoff (`IDEMPOTENTE`), então uma espera fixa de 260ms pode voltar ANTES da
 * rejeição — o teste então leria o banner antes de ele existir.
 */
async function espera(pred: () => boolean, what: string, ms = 6000): Promise<void> {
  const fim = Date.now() + ms;
  while (!pred() && Date.now() < fim) {
    await act(async () => {
      await new Promise((r) => setTimeout(r, 50));
    });
  }
  expect(pred(), `esperado: ${what}`).toBe(true);
}

async function monta(): Promise<void> {
  const host = document.createElement("div");
  document.body.appendChild(host);
  let root!: Root;
  await act(async () => {
    root = createRoot(host);
    root.render(
      <StrictMode>
        <Probe />
      </StrictMode>,
    );
  });
  await act(async () => {
    await new Promise((r) => setTimeout(r, 260));
  });
}

describe("biblioteca no webview", () => {
  it("no SHELL a aba Patches NÃO consulta o banco (a lista é do aparelho, #150)", async () => {
    await monta();
    // **(#150)** a lista de slots vem do device (`device_preset_library`) —
    // a busca SQLite de fábrica é da porta da aba do DONO; no shell ela nem
    // dispara (sem consulta a dado que não é do aparelho).
    expect(chamadas.filter((c) => c.cmd === "library_search")).toHaveLength(0);
    // os números do ARQUIVO do dono continuam vindo do shell (esquema 2)
    expect(chamadas.map((c) => c.cmd)).toContain("library_stats");
    expect(lib!.stats?.schema).toBe(2);
    expect(lib!.stats?.sqlite).toBe("3.46.0");
  });

  it("o nome e o shape dos argumentos são os que o shell espera (aba do dono)", async () => {
    await monta();
    await act(async () => {
      lib!.setBanco("user");
    });
    await act(async () => {
      lib!.setTexto("blink");
    });
    await act(async () => {
      await new Promise((r) => setTimeout(r, 260));
    });
    const busca = [...chamadas].reverse().find((c) => c.cmd === "library_search");
    expect(busca).toBeDefined();
    // `library_search` no Rust declara {text, pp, ppType, bank} — snake no
    // DTO, camel no invoke. Qualquer desalinhamento aqui é o bug do #26.
    expect(Object.keys(busca!.args).sort()).toEqual(["bank", "pp", "ppType", "text"]);
    expect(busca!.args.text).toBe("blink");
    expect(busca!.args.pp).toBeNull();
    expect(busca!.args.bank).toBe("user");
  });

  it("a aba do dono busca no banco do dono (o `bank` desce no invoke)", async () => {
    // Sem `bank` no comando, a busca da aba do dono rodava sobre os 99 de
    // fábrica e a lista do dono vinha vazia — sem erro, sem mensagem.
    await monta();
    await librarySave({
      id: "u1",
      bank: "user",
      pp: null,
      name: "MEU LEAD",
      ppType: 4,
      ppTypeName: "Rock",
      savedAt: "2026-02-02T00:00:00Z",
      hasPayload: true,
      payload: "[]",
    });
    await act(async () => {
      lib!.setBanco("user");
    });
    await act(async () => {
      await new Promise((r) => setTimeout(r, 260));
    });
    expect(lib!.rows.map((r) => r.name)).toEqual(["MEU LEAD"]);
    const busca = [...chamadas].reverse().find((c) => c.cmd === "library_search");
    expect(busca!.args.bank).toBe("user");
  });

  it("abrir patch: `library_get` traz a cadeia, e um id sumido devolve null", async () => {
    await monta();
    await act(async () => {
      await librarySave({
        id: "u1",
        bank: "user",
        pp: null,
        name: "MEU LEAD",
        ppType: 4,
        ppTypeName: "Rock",
        savedAt: "2026-02-02T00:00:00Z",
        hasPayload: true,
        payload: "[{\"slot\":1}]",
      });
    });
    let rec: LibraryRecord | null = null;
    await act(async () => {
      rec = await lib!.carrega("u1");
    });
    expect(rec!.name).toBe("MEU LEAD");
    expect(rec!.payload).toBe("[{\"slot\":1}]");
    // id apagado em outro lugar: `null`, e a UI volta para a fábrica
    await act(async () => {
      rec = await lib!.carrega("nao-existe");
    });
    expect(rec).toBeNull();
  });

  it("cada escrita que falha vira banner COM retry (o #20 no arquivo local)", async () => {
    // O banco é um arquivo: disco cheio, permissão e antivírus não passam na
    // segunda tentativa, mas o dono precisa do caminho de volta na tela.
    const registro = {
      id: "u1",
      bank: "user" as const,
      pp: null,
      name: "MEU",
      ppType: 4,
      ppTypeName: "Rock",
      savedAt: "2026-02-02T00:00:00Z",
      hasPayload: true,
      payload: "[]",
    };
    // o tipo vem por nome: em `.tsx`, `Array<{` abre JSX e o arquivo nem
    // compila (armadilha de arquivo, não do teste)
    const casos: CasoDeFalha[] = [
      { quebra: "library_save", msg: MSG.errLibrarySave, roda: () => lib!.salva(registro) },
      { quebra: "library_delete", msg: MSG.errLibraryDelete, roda: () => lib!.apaga("u1") },
      { quebra: "library_export", msg: MSG.errLibraryExport, roda: () => lib!.exportar() },
      { quebra: "library_get", msg: MSG.errLibrarySearch, roda: () => lib!.carrega("u1") },
      {
        quebra: "library_import",
        msg: MSG.errLibraryImport,
        roda: () => lib!.importar(JSON.stringify({ format: "gp100.library", version: 1, presets: [] }), false),
      },
    ];
    for (const c of casos) {
      await monta();
      quebrado = c.quebra;
      await act(async () => {
        await c.roda();
      });
      expect(lib!.err?.message, `falha de ${c.quebra} precisa de banner`).toBe(c.msg);
      expect(typeof lib!.err?.retry, "o banner tem ACAO, nao so texto").toBe("function");
      // retry que falha DE NOVO tem que manter o banner: limpar aqui deixaria a
      // tela "limpa" com o arquivo ainda travado, e o dono sem nenhuma pista
      await act(async () => {
        lib!.err?.retry();
      });
      await act(async () => {
        await new Promise((r) => setTimeout(r, 50));
      });
      // o banner persiste enquanto o arquivo estiver quebrado
      expect(lib!.err, `retry que falhou de novo (${c.quebra})`).not.toBeNull();
      // o retry é real: destravado o arquivo, ele volta a ler
      quebrado = null;
      await act(async () => {
        lib!.err?.retry();
      });
      await act(async () => {
        await new Promise((r) => setTimeout(r, 50));
      });
      expect(lib!.err, `retry de ${c.quebra}`).toBeNull();
      // e o `clearErr` também é caminho de saída
      quebrado = c.quebra;
      await act(async () => {
        await c.roda();
      });
      expect(lib!.err).not.toBeNull();
      act(() => lib!.clearErr());
      expect(lib!.err).toBeNull();
    }
  });

  it("a migração do localStorage ACONTECE no shell e some da lista", async () => {
    localStorage.setItem(LEGACY_KEY, JSON.stringify([legado("u1", "MEU LEAD"), legado("u2", "Baixo")]));
    await monta();

    expect(lib!.migrouLegado).toBe(true);
    expect(localStorage.getItem(LEGACY_KEY)).toBeNull();
    expect(lib!.stats?.user).toBe(2);
    // a lista é do banco VISÍVEL: os migrados aparecem quando a aba do dono
    // está aberta (é o que o painel faz ao clicar na aba)
    expect(lib!.rows.every((r) => r.bank === "factory")).toBe(true);
    await act(async () => {
      lib!.setBanco("user");
    });
    await act(async () => {
      await new Promise((r) => setTimeout(r, 260));
    });
    expect(lib!.rows.map((r) => r.name).sort()).toEqual(["Baixo", "MEU LEAD"]);
  });

  it("retry que falha DE NOVO repõe o banner (a tela limpa não pode ficar limpa)", async () => {
    // A busca é a operação cujo retry passa por `refeito`. Se ele limpasse o
    // banner depois de uma repetição que falhou, a tela ficaria "limpa" com o
    // arquivo ainda travado — o dono não teria como saber que a segunda
    // tentativa também não deu.
    //
    // O teste parte de um banner LIMPO de propósito: com o banner já no lugar,
    // o efeito do retry seria invisível (o valor antes e depois é o mesmo).
    quebrado = "library_search";
    await monta();
    // a busca que dispara é a da ABA DO DONO (#150: a de fábrica nem roda
    // no shell — a lista de slots é do aparelho)
    await act(async () => {
      lib!.setBanco("user");
    });
    await espera(() => lib!.err != null, "o banner da busca quebrada");

    const repetir = lib!.err!.retry;
    act(() => lib!.clearErr());
    expect(lib!.err).toBeNull();

    await act(async () => {
      repetir();
    });
    await espera(() => lib!.err != null, "o banner da repeticao que falhou");
    expect(lib!.err?.message).toBe(MSG.errLibrarySearch);

    // destravado, a MESMA repetição limpa a tela (na aba do dono, sem patch
    // semeado, a lista volta VAZIA — o importante é o banner sair)
    quebrado = null;
    await act(async () => {
      repetir();
    });
    await espera(() => lib!.err == null && lib!.rows.length === 0, "a lista voltou (vazia, sem erro)");
  });

  it("migração que falha NAO apaga a chave e oferece retry", async () => {
    // O pior caso do dono: os patches estão no storage, o banco recusou o
    // import. Se a chave saísse aqui, o patch sumiria — apagado pelo app.
    localStorage.setItem(LEGACY_KEY, JSON.stringify([legado("u1", "MEU LEAD")]));
    quebrado = "library_import";
    await monta();
    await act(async () => {
      await new Promise((r) => setTimeout(r, 300));
    });
    console.log("DEBUG err:", JSON.stringify(lib!.err), "migrou:", lib!.migrouLegado,
      "imports:", chamadas.filter((c) => c.cmd === "library_import").length);
    expect(lib!.migrouLegado).toBe(true);
    expect(lib!.err?.message).toBe(MSG.errLibrarySearch);
    // o dado do dono continua no storage: a chave é a rede de segurança
    expect(localStorage.getItem(LEGACY_KEY)).not.toBeNull();
    // destravado, o retry completa a migração e some com a chave
    quebrado = null;
    await act(async () => {
      lib!.err?.retry();
      await new Promise((r) => setTimeout(r, 50));
    });
    await act(async () => {
      await new Promise((r) => setTimeout(r, 260));
    });
    expect(localStorage.getItem(LEGACY_KEY)).toBeNull();
    await act(async () => {
      lib!.setBanco("user");
    });
    await act(async () => {
      await new Promise((r) => setTimeout(r, 260));
    });
    expect(lib!.rows.map((r) => r.name)).toEqual(["MEU LEAD"]);
  });

  it("sem nada no storage, `migrouLegado` fica falso e não há import", async () => {
    await monta();
    expect(lib!.migrouLegado).toBe(false);
    expect(chamadas.filter((c) => c.cmd === "library_import")).toHaveLength(0);
  });

  it("migração é idempotente: rodar duas vezes não duplica", async () => {
    localStorage.setItem(LEGACY_KEY, JSON.stringify([legado("u1", "MEU LEAD")]));
    // primeiro mount migra e apaga a chave
    await monta();
    expect(banco.size).toBe(100);
    // o storage volta a ter a chave (outro dispositivo/profil) e o app reabre
    localStorage.setItem(LEGACY_KEY, JSON.stringify([legado("u1", "MEU LEAD")]));
    await monta();
    expect(banco.size).toBe(100);
    expect([...banco.values()].filter((r) => r.id === "u1")).toHaveLength(1);
  });

  it("a data que o dono salvou sobrevive à migração", async () => {
    localStorage.setItem(LEGACY_KEY, JSON.stringify([legado("u9", "COM DATA")]));
    await monta();
    const r = banco.get("u9")!;
    expect(r.savedAt).toBe("2026-02-02T00:00:00Z");
  });

  it("exportar e reimportar não duplica a fábrica", async () => {
    await act(async () => {
      await lib!.salva({
        id: "u20",
        bank: "user",
        pp: null,
        name: "EXPORTADO",
        ppType: 4,
        ppTypeName: "Rock",
        savedAt: "2026-02-02T00:00:00Z",
        hasPayload: true,
        payload: "[]",
      });
    });
    const json = await emAct(() => lib!.exportar());
    expect(json).not.toBeNull();
    const rel = await emAct(() => lib!.importar(json!, false));
    expect(rel!.inserted).toBe(0);
    expect(rel!.skipped).toBe(100);
    expect(banco.size).toBe(100);
  });

  it("import com replace atualiza o que existe", async () => {
    const envelope = JSON.stringify({
      format: "gp100.library",
      version: 1,
      exportedAt: "x",
      sqlite: "y",
      schema: 2,
      presets: [
        { id: "u30", bank: "user", pp: null, name: "NOVO", ppType: 4, ppTypeName: "Rock", savedAt: "t", payload: null },
      ],
    });
    const rel = await emAct(() => lib!.importar(envelope, true));
    expect(rel!.inserted).toBe(1);
    expect(banco.get("u30")?.name).toBe("NOVO");
  });
});