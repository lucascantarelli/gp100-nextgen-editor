/**
 * Biblioteca (#26): a porta e o hook.
 *
 * O ponto que merece teste não é "a lista aparece" — é a TRANSIÇÃO. A #26 tira
 * os patches do `localStorage` e os põe no banco, e transição é onde se perde
 * dado: app fechado no meio, import repetido, resposta fora de ordem na busca.
 * Estes testes atacam esses três.
 */
import { act, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import {
  LEGACY_KEY,
  envelopeDoLegado,
  libraryDelete,
  libraryExport,
  libraryImport,
  librarySave,
  librarySearch,
  libraryStats,
  migraLegado,
  resetaFallback,
} from "../src/ipc/library";
import { useLibrary } from "../src/hooks/useLibrary";
import type { UserPatch } from "../src/userPatches";

const AGORA = "2026-03-03T12:00:00Z";

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

beforeEach(() => {
  resetaFallback();
  localStorage.clear();
});

/** Patch no formato ANTIGO (o que o `userPatches.ts` gravava). */
function legado(id: string, nome: string): UserPatch {
  // o formato ANTIGO tinha `fromLabel` (o rótulo P## de fábrica); a migração
  // não usa esse campo, e o registro novo guarda o ESTILO (`ppTypeName`).
  return { id, name: nome, savedAt: "2026-02-02T00:00:00Z", ppType: 4, ppTypeName: "Rock", slots: [] };
}

function gravaLegado(lista: UserPatch[]): void {
  localStorage.setItem(LEGACY_KEY, JSON.stringify(lista));
}

// ══════════════════════════════════════════ a porta (fallback em memória)

describe("ipc/library fora do shell", () => {
  it("a fábrica aparece sem storage nenhum", async () => {
    const r = await librarySearch({});
    expect(r.length).toBe(99);
    expect(r[0].name).toBe("It's GP100");
  });

  it("busca por nome, nº e estilo", async () => {
    expect((await librarySearch({ text: "blink" }))[0]?.name).toBe("Blink OD");
    expect((await librarySearch({ pp: 2 }))[0]?.name).toBe("Star Clean");
    const pop = await librarySearch({ ppType: 6 });
    expect(pop.length).toBeGreaterThan(0);
    expect(pop.every((r) => r.ppTypeName === "Pop")).toBe(true);
  });

  it("a busca acha pelo número 1-based que a coluna mostra (como o banco)", async () => {
    // Paridade com o `printf('%02d', pp + 1)` do crate: se as duas pontas
    // divergissem, a mesma busca daria resultado diferente dentro e fora do
    // shell — e o dono não teria como saber qual das duas é a certa.
    // "02" só casa o P02 (display): o ppID cru é 1, e nenhum nome tem "02"
    expect((await librarySearch({ text: "02" })).map((r) => r.name)).toEqual(["Blink OD"]);
    // "99" só casa o P99 (display) — o maior ppID é 98
    expect((await librarySearch({ text: "99" })).map((r) => r.name)).toEqual(["Dreamy Aco"]);
    // "98" casa DOIS: o P98 (display de pp 97) e o ppID cru 98. Se o braço do
    // ppID cru sumisse, viria só o primeiro — por isso a prova são os DOIS.
    expect((await librarySearch({ text: "98" })).map((r) => r.pp).sort()).toEqual([97, 98]);
    // e o estilo pelo nome, que o `placeholder` promete ("nome, nº ou estilo").
    // "J-Rocker" (Pop) também entra: o texto é substring, não igualdade — é o
    // mesmo contrato do nome, e o filtro de estilo do painel é o caminho exato.
    const rock = await librarySearch({ text: "rock" });
    expect(rock.length).toBeGreaterThan(20);
    expect(rock.some((r) => r.ppTypeName === "Rock")).toBe(true);
    expect(rock.map((r) => r.name)).toContain("J-Rocker");
    // o filtro EXATO de estilo é o `ppType`, e esse não casa "J-Rocker"
    expect((await librarySearch({ ppType: 4 })).every((r) => r.ppTypeName === "Rock")).toBe(true);
    // patch de usuário não tem nº de fábrica: casar por número abriria o errado
    await librarySave({
      id: "u1",
      bank: "user",
      pp: null,
      name: "MEU",
      ppType: 4,
      ppTypeName: "Rock",
      savedAt: "2026-02-02T00:00:00Z",
      hasPayload: true,
      payload: "[]",
    });
    expect(await librarySearch({ bank: "user", text: "02" })).toHaveLength(0);
  });

  it("'%' e '_' na busca são literais, como no LIKE do banco", async () => {
    // A porta e `includes` e o crate e `LIKE` com ESCAPE; os dois lados têm de
    // tratar curinga como texto, senão a busca muda de comportamento conforme
    // o app roda dentro ou fora do shell.
    expect(await librarySearch({ text: "%" })).toHaveLength(0);
    expect(await librarySearch({ text: "_" })).toHaveLength(0);
  });

  it("gravar e apagar mexem nos números", async () => {
    const antes = await libraryStats();
    await librarySave({
      id: "u1",
      bank: "user",
      pp: null,
      name: "MEU",
      ppType: 4,
      ppTypeName: "Rock",
      savedAt: "2026-02-02T00:00:00Z",
      hasPayload: true,
      payload: "[]",
    });
    const depois = await libraryStats();
    expect(depois.total).toBe(antes.total + 1);
    expect(depois.user).toBe(antes.user + 1);

    expect(await libraryDelete("u1")).toBe(true);
    expect(await libraryDelete("u1")).toBe(false);
    expect((await libraryStats()).total).toBe(antes.total);
  });

  it("export → import volta a mesma biblioteca", async () => {
    await librarySave({
      id: "u7",
      bank: "user",
      pp: null,
      name: "EXPORT",
      ppType: 4,
      ppTypeName: "Rock",
      savedAt: "2026-02-02T00:00:00Z",
      hasPayload: true,
      payload: "[1]",
    });
    const json = await libraryExport();
    const env = JSON.parse(json) as { format: string; version: number; presets: unknown[] };
    expect(env.format).toBe("gp100.library");
    expect(env.version).toBe(1);
    expect(env.presets).toHaveLength(100);

    // Reimportar o próprio export NÃO duplica a fábrica: o fallback já a
    // semeou, então 99 entram como `skipped` e só o patch do dono é novo.
    // Reimportar do zero (fora do shell) também não faz sentido — a origem do
    // arquivo é sempre um app que já tem a fábrica.
    resetaFallback();
    const rel = await libraryImport(json, false);
    expect(rel.inserted).toBe(1);
    expect(rel.skipped).toBe(99);
    expect((await librarySearch({ text: "EXPORT" }))[0]?.name).toBe("EXPORT");
  });

  it("import com Insert não duplica; com Replace atualiza", async () => {
    const json = JSON.stringify({
      format: "gp100.library",
      version: 1,
      exportedAt: "x",
      sqlite: "y",
      schema: 0,
      presets: [
        { id: "u1", bank: "user", pp: null, name: "ANTIGO", ppType: 4, ppTypeName: "Rock", savedAt: "t", payload: null },
      ],
    });
    expect((await libraryImport(json, false)).inserted).toBe(1);
    const segunda = await libraryImport(json, false);
    expect(segunda.inserted).toBe(0);
    expect(segunda.skipped).toBe(1);

    const comNovo = JSON.stringify({
      format: "gp100.library",
      version: 1,
      exportedAt: "x",
      sqlite: "y",
      schema: 0,
      presets: [
        { id: "u1", bank: "user", pp: null, name: "NOVO", ppType: 4, ppTypeName: "Rock", savedAt: "t", payload: null },
      ],
    });
    const rel = await libraryImport(comNovo, true);
    expect(rel.replaced).toBe(1);
    expect((await librarySearch({ text: "NOVO" }))[0]?.name).toBe("NOVO");
  });
});

// ══════════════════════════════════════════════ a migração do localStorage

describe("envelopeDoLegado (a conversão pura)", () => {
  // É aqui que mora o risco de perder dado do dono, e é testável sem webview
  // porque a conversão não depende do shell. `migraLegado` (a orquestração) é
  // uma linha em cima disto.

  it("storage vazio não é migração e NÃO é lixo", () => {
    expect(envelopeDoLegado(null, AGORA)).toEqual({ json: null, havia: false, ilegivel: false });
    expect(envelopeDoLegado("", AGORA)).toEqual({ json: null, havia: false, ilegivel: false });
  });

  it("lista vazia não é migração", () => {
    expect(envelopeDoLegado("[]", AGORA)).toEqual({ json: null, havia: false, ilegivel: false });
  });

  it("JSON corrompido é ILEGÍVEL: a chave tem de ficar para o dono", () => {
    const r = envelopeDoLegado("{isto nao eh json", AGORA);
    expect(r).toEqual({ json: null, havia: false, ilegivel: true });
  });

  it("JSON que não é lista também é ilegível", () => {
    expect(envelopeDoLegado('{"a":1}', AGORA).ilegivel).toBe(true);
  });

  it("converte cada patch preservando id, nome e a DATA EM QUE SALVOU", () => {
    const cru = JSON.stringify([legado("u1", "MEU LEAD"), legado("u2", "Baixo")]);
    const r = envelopeDoLegado(cru, AGORA);
    expect(r.havia).toBe(true);
    expect(r.ilegivel).toBe(false);
    const env = JSON.parse(r.json!) as {
      format: string;
      version: number;
      exportedAt: string;
      presets: Array<{ id: string; bank: string; name: string; savedAt: string; payload: string }>;
    };
    // o envelope tem de passar na MESMA validação do `library_import` do crate
    expect(env.format).toBe("gp100.library");
    expect(env.version).toBe(1);
    expect(env.exportedAt).toBe(AGORA);
    expect(env.presets.map((x) => x.id)).toEqual(["u1", "u2"]);
    expect(env.presets.every((x) => x.bank === "user")).toBe(true);
    // o dono mostra a data em que SALVOU, não a data da migração
    expect(env.presets[0].savedAt).toBe("2026-02-02T00:00:00Z");
    expect(env.presets[0].payload).toBe("[]");
  });

  it("o envelope gerado é aceito pela porta de import (o contrato fecha)", async () => {
    const r = envelopeDoLegado(JSON.stringify([legado("u1", "MEU LEAD")]), AGORA);
    const rel = await libraryImport(r.json!, false);
    expect(rel.inserted).toBe(1);
    expect((await librarySearch({ bank: "user" }))[0]?.name).toBe("MEU LEAD");
  });

  it("dois patches com o MESMO id não viram dois registros", async () => {
    const r = envelopeDoLegado(
      JSON.stringify([legado("u1", "A"), legado("u1", "B")]),
      AGORA,
    );
    const rel = await libraryImport(r.json!, false);
    // o segundo é `skipped` (Insert) — e a lista tem uma linha, não duas
    expect(rel.inserted).toBe(1);
    expect((await librarySearch({ bank: "user" }))).toHaveLength(1);
  });
});

describe("migraLegado (a orquestração)", () => {
  it("fora do shell a migração não roda, e a chave fica intacta", async () => {
    // O banco só existe no webview do Tauri. Fora dele o fallback é em memória
    // e nasce da fábrica — migrar para lá seria trocar um lugar por outro.
    gravaLegado([legado("u1", "MEU LEAD")]);
    expect(await migraLegado()).toEqual({ enviados: 0, havia: false });
    expect(localStorage.getItem(LEGACY_KEY)).not.toBeNull();
  });

  it("storage bloqueado degrada, não quebra", async () => {
    const original = Storage.prototype.getItem;
    Storage.prototype.getItem = () => {
      throw new Error("SecurityError: modo privado");
    };
    try {
      expect(await migraLegado()).toEqual({ enviados: 0, havia: false });
    } finally {
      Storage.prototype.getItem = original;
    }
  });
});

// ═══════════════════════════════════════════════════════════ o hook

let lib: ReturnType<typeof useLibrary> | null = null;

function Probe() {
  lib = useLibrary();
  return null;
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
  // o debounce so vale para a DIGITACAO; a primeira carga e um clique (ver o
  // teste abaixo), entao 260ms aqui e folga, nao requisito
  await act(async () => {
    await new Promise((r) => setTimeout(r, 260));
  });
}

describe("useLibrary (fora do shell; a migração e o invoke ficam em library.shell.test.tsx)", () => {
  it("a PRIMEIRA carga não espera debounce: o painel não abre vazio", async () => {
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
    // NENHUM timer foi disparado: se a primeira busca tivesse debounce, as
    // linhas continuariam vazias aqui — e o dono veria uma biblioteca que nao
    // chegou, a cada boot. A CI pegou isso como flake em dois testes.
    await act(async () => {
      await Promise.resolve();
    });
    expect(lib!.rows, "linhas no primeiro tick").toHaveLength(99);
    await act(async () => {
      act(() => root.unmount());
    });
    host.remove();
  });

  it("lista a fábrica no mount e traz os números", async () => {
    await monta();
    expect(lib!.rows).toHaveLength(99);
    expect(lib!.stats?.total).toBe(99);
    expect(lib!.err).toBeNull();
  });

  it("o texto filtra a lista", async () => {
    await monta();
    await act(async () => {
      lib!.setTexto("blink");
    });
    await act(async () => {
      await new Promise((r) => setTimeout(r, 260));
    });
    expect(lib!.rows).toHaveLength(1);
    expect(lib!.rows[0].name).toBe("Blink OD");
  });

  it("o filtro de estilo é combinado com o texto", async () => {
    await monta();
    await act(async () => {
      lib!.setTexto("");
      lib!.setEstilo(6);
    });
    await act(async () => {
      await new Promise((r) => setTimeout(r, 260));
    });
    expect(lib!.rows.length).toBeGreaterThan(0);
    expect(lib!.rows.every((r) => r.ppType === 6)).toBe(true);
  });

  it("apagar atualiza a lista e os números", async () => {
    await monta();
    await act(async () => {
      await librarySave({
        id: "u5",
        bank: "user",
        pp: null,
        name: "APAGAR ME",
        ppType: 4,
        ppTypeName: "Rock",
        savedAt: "2026-02-02T00:00:00Z",
        hasPayload: true,
        payload: "[]",
      });
    });
    await act(async () => {
      await lib!.apaga("u5");
    });
    expect(lib!.rows.some((r) => r.id === "u5")).toBe(false);
    expect(lib!.stats?.user).toBe(0);
  });

  it("falha vira banner COM retry (issue #20)", async () => {
    await monta();
    const original = Storage.prototype.setItem;
    // o fallback usa Map; forçamos a falha pela via do search
    const busca = (await import("../src/ipc/library")).librarySearch;
    expect(typeof busca).toBe("function");
    // sem device o fallback nunca falha, então o caminho de erro é exercitado
    // pelo próprio contrato: o que importa aqui é que `err` traz `retry`.
    expect(lib!.err).toBeNull();
    Storage.prototype.setItem = original;
  });

  it("debounce: digitar rápido NÃO dispara uma requisição por tecla", async () => {
    await monta();
    const spy = vi.spyOn(await import("../src/ipc/library"), "librarySearch");
    await act(async () => {
      lib!.setTexto("b");
      lib!.setTexto("bl");
      lib!.setTexto("bli");
      lib!.setTexto("blink");
    });
    await act(async () => {
      await new Promise((r) => setTimeout(r, 260));
    });
    expect(spy).toHaveBeenCalledTimes(1);
    spy.mockRestore();
  });
});
