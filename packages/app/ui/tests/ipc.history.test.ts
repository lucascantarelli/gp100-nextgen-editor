/**
 * Histórico de versões — a porta e o fallback (issue #113).
 *
 * O que merece teste NÃO é "a lista aparece": é a REGRA que a issue chama de
 * única coisa que importa ali — *restaurar cria uma versão nova e nunca apaga a
 * história*. Tudo o que segue ataca essa regra pelos dois lados:
 *
 * - a gravação versiona (um `save` sem versão seria um buraco silencioso);
 * - o histórico não encolhe quando alguém restaura;
 * - a restauração pontual mexe em UM knob e não arrasta os outros junto;
 * - o diff diz "só o que mudou" e distingue trocar algoritmo de girar knob.
 *
 * Os testes rodam no fallback em memória (fora do webview do Tauri não há
 * arquivo), que é a MESMA porta que o shell usa — e é por isso que o e2e do
 * Playwright exercita este caminho.
 */
import { beforeEach, describe, expect, it } from "vitest";
import { librarySave, resetaFallback } from "../src/ipc/library";
import type { LibraryRecord } from "../src/ipc/library";
import {
  libraryDiff,
  libraryRestoreKnob,
  libraryRestoreVersion,
  libraryVersions,
} from "../src/ipc/history";

/** Um slot no formato que o palco grava (`BoardSlot[]`). */
function slot(slot: number, nome: string, ligado: boolean, knobs: [number, string, string | null][]): string {
  return JSON.stringify({
    slot,
    family: "AMP",
    archetype: "AMPLIFIER",
    name: nome,
    variant: nome.toLowerCase(),
    state: ligado,
    code: 1,
    knobs: knobs.map(([pos, n, value]) => ({
      pos,
      name: n,
      kind: "knob",
      ...(value == null ? {} : { value }),
    })),
  });
}

/** Cadeia de 2 slots: o PRE (Gain) e o AMP (Gain + Treble). */
function cadeia(gainPre: string, gainAmp: string, treble: string): string {
  return `[${slot(0, "Green OD", true, [[0, "Gain", gainPre]])}, ${slot(2, "Bog RedM", true, [
    [0, "Gain", gainAmp],
    [1, "Treble", treble],
  ])}]`;
}

function patch(id: string, payload: string, nome = "Meu patch"): LibraryRecord {
  return {
    id,
    bank: "user",
    pp: null,
    name: nome,
    ppType: 4,
    ppTypeName: "Rock",
    savedAt: "2026-10-05T12:00:00Z",
    hasPayload: true,
    payload,
  };
}

beforeEach(() => {
  resetaFallback();
});

describe("histórico — a gravação versiona", () => {
  it("o primeiro save cria a versão 1, e ela é a corrente", async () => {
    await librarySave(patch("u1", cadeia("40", "50", "60")));
    const v = await libraryVersions("u1");
    expect(v).toHaveLength(1);
    expect(v[0].seq).toBe(1);
    expect(v[0].current).toBe(true);
  });

  it("salvar três vezes cresce o histórico e só a última é a corrente", async () => {
    await librarySave(patch("u1", cadeia("40", "50", "60")));
    await librarySave(patch("u1", cadeia("55", "50", "60")));
    await librarySave(patch("u1", cadeia("70", "50", "60")));
    const v = await libraryVersions("u1");
    // do mais NOVO para o mais antigo
    expect(v.map((x) => x.seq)).toEqual([3, 2, 1]);
    expect(v.filter((x) => x.current)).toHaveLength(1);
    expect(v[0].current).toBe(true);
  });

  it("preset de fábrica não versiona — ele vem do all.prst e é imutável", async () => {
    await librarySave({
      id: "f3",
      bank: "factory",
      pp: 3,
      name: "Factory",
      ppType: 4,
      ppTypeName: "Rock",
      savedAt: "2026-10-05T12:00:00Z",
      hasPayload: false,
      payload: null,
    });
    expect(await libraryVersions("f3")).toEqual([]);
  });

  it("o histórico de um patch não se mistura com o de outro", async () => {
    await librarySave(patch("u1", cadeia("40", "50", "60")));
    await librarySave(patch("u2", cadeia("10", "20", "30")));
    await librarySave(patch("u1", cadeia("45", "50", "60")));
    expect(await libraryVersions("u1")).toHaveLength(2);
    expect(await libraryVersions("u2")).toHaveLength(1);
  });
});

describe("histórico — o diff diz só o que mudou", () => {
  it("cadeias iguais não têm diff", async () => {
    await librarySave(patch("u1", cadeia("40", "50", "60")));
    await librarySave(patch("u1", cadeia("40", "50", "60")));
    const [a, b] = await libraryVersions("u1");
    const d = await libraryDiff(b.id, a.id);
    expect(d.identical).toBe(true);
    expect(d.slots).toEqual([]);
  });

  it("lista o knob que mudou e não lista os iguais", async () => {
    await librarySave(patch("u1", cadeia("40", "50", "60")));
    await librarySave(patch("u1", cadeia("40", "50", "75")));
    const [nova, antiga] = await libraryVersions("u1");
    const d = await libraryDiff(antiga.id, nova.id);
    expect(d.identical).toBe(false);
    expect(d.slots).toHaveLength(1);
    expect(d.slots[0].slot).toBe(2);
    expect(d.slots[0].algorithmChanged).toBe(false);
    expect(d.slots[0].knobs).toHaveLength(1);
    expect(d.slots[0].knobs[0]).toEqual({ pos: 1, knob: "Treble", from: "60", to: "75" });
  });

  it("trocar o algoritmo é evento separado de girar um knob", async () => {
    await librarySave(patch("u1", cadeia("40", "50", "60")));
    const troca = `[${slot(0, "Green OD", true, [[0, "Gain", "40"]])}, ${slot(2, "Plexi", true, [
      [0, "Gain", "10"],
      [1, "Treble", "90"],
    ])}]`;
    await librarySave(patch("u1", troca));
    const [nova, antiga] = await libraryVersions("u1");
    const d = await libraryDiff(antiga.id, nova.id);
    expect(d.slots[0].algorithmChanged).toBe(true);
    expect(d.slots[0].nameBefore).toBe("Bog RedM");
    expect(d.slots[0].nameAfter).toBe("Plexi");
  });

  it("knob sem valor não é o mesmo que knob com valor vazio", async () => {
    await librarySave(patch("u1", `[${slot(0, "A", true, [[0, "Gain", null]])}]`));
    await librarySave(patch("u1", `[${slot(0, "A", true, [[0, "Gain", ""]])}]`));
    const [nova, antiga] = await libraryVersions("u1");
    const d = await libraryDiff(antiga.id, nova.id);
    expect(d.slots[0].knobs[0].from).toBeNull();
    expect(d.slots[0].knobs[0].to).toBe("");
  });

  it("desligar um slot aparece no diff, e não como knob mexido", async () => {
    await librarySave(patch("u1", `[${slot(2, "Bog RedM", true, [[0, "Gain", "50"]])}]`));
    await librarySave(patch("u1", `[${slot(2, "Bog RedM", false, [[0, "Gain", "50"]])}]`));
    const [nova, antiga] = await libraryVersions("u1");
    const d = await libraryDiff(antiga.id, nova.id);
    expect(d.slots[0].onBefore).toBe(true);
    expect(d.slots[0].onAfter).toBe(false);
    expect(d.slots[0].knobs).toEqual([]);
  });
});

describe("histórico — restaurar cria versão nova", () => {
  it("restaurar a versão 1 quando a 3 é a corrente cria a 4 e não apaga nada", async () => {
    await librarySave(patch("u1", cadeia("40", "50", "60")));
    const v1 = (await libraryVersions("u1"))[0].id;
    await librarySave(patch("u1", cadeia("55", "50", "60")));
    await librarySave(patch("u1", cadeia("70", "50", "60")));

    const criada = await libraryRestoreVersion(v1);
    expect(criada.seq).toBe(4);

    const depois = await libraryVersions("u1");
    // as três primeiras continuam lá — o presente não some
    expect(depois.map((x) => x.seq)).toEqual([4, 3, 2, 1]);
    expect(depois[0].current).toBe(true);
    // e o patch corrente voltou a ser a cadeia da versão 1
    const registro = (await import("../src/ipc/library")).libraryGet;
    const atual = await registro("u1");
    expect(atual?.payload).toContain('"40"');
  });

  it("restaurar um knob mexe só nele e não arrasta os outros", async () => {
    await librarySave(patch("u1", cadeia("40", "50", "85")));
    const v1 = (await libraryVersions("u1"))[0].id;
    await librarySave(patch("u1", cadeia("95", "10", "20")));

    const criada = await libraryRestoreKnob(v1, 2, 0);
    expect(criada.seq).toBe(3);

    const { libraryGet } = await import("../src/ipc/library");
    const atual = await libraryGet("u1");
    const doc = JSON.parse(atual!.payload!) as { slot: number; knobs: { pos: number; value?: string }[] }[];
    const amp = doc.find((s) => s.slot === 2)!;
    // o Gain do AMP voltou para 50…
    expect(amp.knobs.find((k) => k.pos === 0)!.value).toBe("50");
    // …e o Treble NÃO voltou junto: ele está no 20, não nos 85 da v1
    expect(amp.knobs.find((k) => k.pos === 1)!.value).toBe("20");
    // o outro slot também não mexeu
    const pre = doc.find((s) => s.slot === 0)!;
    expect(pre.knobs.find((k) => k.pos === 0)!.value).toBe("95");
  });

  it("restaurar um knob que não existe naquela versão é erro, não apagão", async () => {
    await librarySave(patch("u1", cadeia("40", "50", "60")));
    const v1 = (await libraryVersions("u1"))[0].id;
    await expect(libraryRestoreKnob(v1, 7, 0)).rejects.toThrow(/slot 7/);
    // e a recusa não criou versão nenhuma
    expect(await libraryVersions("u1")).toHaveLength(1);
  });

  it("restaurar uma versão que não existe é erro", async () => {
    await librarySave(patch("u1", cadeia("40", "50", "60")));
    await expect(libraryRestoreVersion(9999)).rejects.toThrow(/9999/);
  });
});
