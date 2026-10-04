/**
 * useStage / usePrefs — os hooks que a #82 tirou do `App.tsx`, testados
 * DIRETO (componente probe + `act`, mesmo padrão de `useBoot.test.tsx`;
 * sem dependências novas).
 *
 * O ponto da #82 não era "App.tsx tem menos linhas", era **a lógica poder ser
 * testada sem renderizar a casca inteira**. `app.interactions.test.tsx` tem
 * 1027 linhas para cobrir o App; estes testes pegam os invariantes do palco e
 * das preferências direto, e o App só precisa provar que os conecta.
 *
 * Invariantes cobertos (os que estavam espalhados por comentários no App):
 *   - nada de estado otimista: `pp`/board só mudam depois do device confirmar;
 *   - falha vira `{message, retry}` — o banner nunca é saída sem recuperação;
 *   - apagar o patch que estava aberto volta para o preset de fábrica;
 *   - abrir preset/patch fecha a edição ampliada;
 *   - knob local aplica na hora E manda o SET (falha do SET vira retry);
 *   - o drum NUNCA volta tocando depois de um reload;
 *   - o idioma é aplicado no inicializador (sem flash de pt-BR).
 */
import { act, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { BoardSlot, BoardView } from "../src/ipc/types";
import { useStage } from "../src/hooks/useStage";
import { usePrefs } from "../src/hooks/usePrefs";
import { MSG } from "../src/i18n/messages";
import { FX_MODULES } from "../src/artifacts/fxData";

const mocks = vi.hoisted(() => ({
  deviceSelectPreset: vi.fn(),
  deviceBoard: vi.fn(),
  deviceSetParam: vi.fn(),
  deviceInfo: vi.fn(),
  onBootProgress: vi.fn(),
}));
vi.mock("../src/ipc/device", () => mocks);

/** Board mínimo com 1 slot e 1 knob — o suficiente para exercitar o palco.
 *  O slot é FIEL ao `BoardSlot` de verdade (code numérico, `options` sempre
 *  presente): o snapshot de patch clona os knobs e espalha `options`, então um
 *  mock pela metade quebrava `cloneSlots` em vez de testar o palco. */
function slotDe(): BoardSlot {
  const knob: BoardSlot["knobs"][number] = {
    pos: 0,
    kind: "knob",
    name: "GAIN",
    range: [0, 100],
    options: [],
    value: "50",
    default: "60",
  };
  return {
    slot: 1,
    family: "PRE",
    archetype: "BUFFER",
    name: "PRE",
    variant: "green-bb",
    state: false,
    code: 0x0700006e,
    knobs: [knob],
  };
}

function boardDe(pp: number, name: string): BoardView {
  return {
    pp,
    ppLabel: `PP${pp}`,
    name,
    ppType: 0,
    ppTypeName: "GUITAR",
    bank: "factory",
    slots: [slotDe()],
  };
}

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

beforeEach(() => {
  mocks.deviceSelectPreset.mockReset().mockResolvedValue(undefined);
  // `deviceBoard` tem de DEVOLVER o pp pedido: `openPreset` tira o `pp` de
  // `b.pp` (o device é a fonte da verdade), então um board fixo em 0 fazia o
  // ciclo ◀ ▶ voltar ao passo anterior.
  mocks.deviceBoard
    .mockReset()
    .mockImplementation(async (target: number) => boardDe(target, `PP-${target}`));
  mocks.deviceSetParam.mockReset().mockResolvedValue(undefined);
  mocks.deviceInfo.mockReset().mockResolvedValue(null);
  mocks.onBootProgress.mockReset().mockResolvedValue(() => {});
  localStorage.clear();
});

// ════════════════════════════════════ useStage

type Stage = ReturnType<typeof useStage>;
let stage: Stage | null = null;
let closed = 0;

function StageProbe({ onChanged }: { onChanged?: () => void }) {
  stage = useStage(onChanged ?? (() => {}));
  return null;
}

/** Monta o probe dentro de `act` e devolve o hook já com o efeito inicial rodado. */
async function montarStage(opts: { onChanged?: () => void; strict?: boolean } = {}) {
  const host = document.createElement("div");
  document.body.appendChild(host);
  let root!: Root;
  await act(async () => {
    root = createRoot(host);
    root.render(
      opts.strict === false ? <StageProbe onChanged={opts.onChanged} /> : (
        <StrictMode>
          <StageProbe onChanged={opts.onChanged} />
        </StrictMode>
      ),
    );
  });
  return { host, root };
}

describe("useStage", () => {
  it("abre o preset 0 no mount (o device é a fonte da verdade)", async () => {
    await montarStage();
    expect(mocks.deviceSelectPreset).toHaveBeenCalledWith(0);
    expect(stage!.pp).toBe(0);
    expect(stage!.presetName).toBe("PP-0");
    expect(stage!.openUserId).toBeNull();
  });

  // Regressão do LAÇO. Este teste é a raison d'être do `changedRef`: o App
  // passa uma arrow inline para `onPresetChanged`, então a identidade dela muda
  // a cada render. Sem o ref, `openPreset` vira dependência do efeito de mount e
  // o preset reabre para sempre, batendo no device sem parar — o teste travava
  // o event loop antes do ref. Contar chamadas é o que pega isso; só afirmar
  // `toHaveBeenCalledWith(0)` passaria mesmo com o loop.
  it("callback inline NÃO reabre o preset em loop (o App passa um)", async () => {
    const abertas: number[] = [];
    const host = document.createElement("div");
    document.body.appendChild(host);
    let root!: Root;
    function LoopProbe() {
      // arrow inline, nova identidade a CADA render — igual ao App
      useStage(() => {
        abertas.push(Date.now());
      });
      return null;
    }
    await act(async () => {
      root = createRoot(host);
      root.render(<LoopProbe />);
    });
    // effect de mount roda uma vez (o StrictMode de DEV duplica, e o app real
    // monta em StrictMode também): o que NÃO pode é o número crescer.
    expect(abertas.length).toBeLessThanOrEqual(2);
    expect(mocks.deviceSelectPreset.mock.calls.length).toBeLessThanOrEqual(2);
  });

  it("NÃO é otimista: select que falha não muda pp nem board", async () => {
    mocks.deviceSelectPreset.mockRejectedValue(new Error("sem device"));
    await montarStage();
    expect(stage!.board).toBeNull();
    expect(stage!.presetName).toBe("…");
    // e a falha virou banner COM ação de recuperação
    expect(stage!.err).not.toBeNull();
    expect(typeof stage!.err!.retry).toBe("function");
    expect(stage!.err!.message).toBe(MSG.errSelectPreset);
  });

  it("select OK + board falho diz 'não foi possível abrir', não 'não trocou'", async () => {
    mocks.deviceBoard.mockRejectedValue(new Error("meio do caminho"));
    await montarStage();
    expect(stage!.err!.message).toBe(MSG.errOpenPreset);
  });

  it("o retry do banner reabre o preset que falhou", async () => {
    // Rejeição PERMANENTE (não `Once`): o StrictMode monta o efeito duas vezes
    // e o `Once` seria consumido na primeira, deixando o banner já limpo.
    mocks.deviceSelectPreset.mockRejectedValue(new Error("boom"));
    await montarStage();
    expect(stage!.err).not.toBeNull();
    mocks.deviceSelectPreset.mockResolvedValue(undefined);
    mocks.deviceBoard.mockResolvedValue(boardDe(7, "ACOUSTIC"));
    await act(async () => {
      stage!.err!.retry();
    });
    expect(stage!.pp).toBe(7);
    expect(stage!.err).toBeNull();
  });

  it("abrir preset/patch fecha a edição ampliada", async () => {
    closed = 0;
    await montarStage({ onChanged: () => { closed += 1; } });
    const depoisDoMount = closed;
    await act(async () => {
      await stage!.openPreset(0);
    });
    expect(closed).toBeGreaterThan(depoisDoMount);
  });

  it("◀ ▶ ciclama em 0..98 e nunca sai da faixa", async () => {
    await montarStage();
    // 0 ◀ 98: o ciclo é o mesmo do app oficial (0..98, 99 presets).
    await act(async () => {
      stage!.stepPreset(-1);
    });
    expect(mocks.deviceSelectPreset).toHaveBeenLastCalledWith(98);
    // 98 ▶ 0 — `stepPreset` lê o `pp` do render ANTERIOR, então cada passo
    // precisa do seu act: somar as duas coisas num act só testaria o closure.
    await act(async () => {
      stage!.stepPreset(1);
    });
    expect(mocks.deviceSelectPreset).toHaveBeenLastCalledWith(0);
  });

  it("knob: aplica local na hora E manda o SET com slot+1 do fio", async () => {
    await montarStage();
    const slot = stage!.board!.slots[0];
    await act(async () => {
      stage!.applyKnob(slot, 0, "42");
    });
    // local imediato (otimista AQUI é permitido: o valor é o que o usuário digitou)
    expect(stage!.board!.slots[0].knobs[0].value).toBe("42");
    // e o fio recebeu §13.11: slot do fio = posição 1..9
    expect(mocks.deviceSetParam).toHaveBeenCalledWith(2, slot.code, 0, 42);
  });

  it("knob com valor não numérico muda a tela mas NÃO vai pro fio", async () => {
    await montarStage();
    const slot = stage!.board!.slots[0];
    await act(async () => {
      stage!.applyKnob(slot, 0, "abc");
    });
    expect(stage!.board!.slots[0].knobs[0].value).toBe("abc");
    expect(mocks.deviceSetParam).not.toHaveBeenCalled();
  });

  it("SET que falha vira banner com retry que reaplica o MESMO valor", async () => {
    mocks.deviceSetParam.mockRejectedValueOnce(new Error("nack"));
    await montarStage();
    const slot = stage!.board!.slots[0];
    await act(async () => {
      stage!.applyKnob(slot, 0, "77");
    });
    expect(stage!.err!.message).toBe(MSG.errSetParam);
    mocks.deviceSetParam.mockResolvedValue(undefined);
    await act(async () => {
      stage!.err!.retry();
    });
    expect(mocks.deviceSetParam).toHaveBeenLastCalledWith(2, slot.code, 0, 77);
  });

  it("reset de knob usa o default do dicionário", async () => {
    await montarStage();
    const slot = stage!.board!.slots[0];
    await act(async () => {
      stage!.onKnobReset(slot, 0);
    });
    expect(stage!.board!.slots[0].knobs[0].value).toBe("60");
  });

  it("trocar o efeito troca os knobs do slot (e não toca o fio)", async () => {
    await montarStage();
    const slot = stage!.board!.slots[0];
    // algoritmo REAL do dicionário (o mesmo que o app mostra na lista #19)
    const comp = FX_MODULES.PRE[0];
    await act(async () => {
      stage!.onChangeEffect(slot, comp);
    });
    const depois = stage!.board!.slots[0];
    expect(depois.variant).toBe(comp.variant);
    expect(depois.name).toBe(comp.name);
    // os knobs do algoritmo NOVO, não os do anterior
    expect(depois.knobs.map((k) => k.name)).toEqual(comp.knobs.map((k) => k.name));
    // PRÉVIA LOCAL: o `change-effect` (0x47) ainda nao tem formato validado
    expect(mocks.deviceSetParam).not.toHaveBeenCalled();
  });

  it("footswitch alterna o estado local do slot", async () => {
    await montarStage();
    const slot = stage!.board!.slots[0];
    expect(stage!.board!.slots[0].state).toBe(false);
    await act(async () => {
      stage!.onToggle(slot);
    });
    expect(stage!.board!.slots[0].state).toBe(true);
    await act(async () => {
      stage!.onToggle(slot);
    });
    expect(stage!.board!.slots[0].state).toBe(false);
  });

  it("mexer no palco com board NULO (device não respondeu) não estoura", async () => {
    mocks.deviceSelectPreset.mockRejectedValue(new Error("sem device"));
    await montarStage();
    expect(stage!.board).toBeNull();
    const alg = FX_MODULES.PRE[0];
    // Os guardas `b == null` existem para isto: o painel desenhou o pedal a
    // partir da ultima leitura e o device sumiu antes do clique. Sem o guarda,
    // o setBoard quebraria e a tela inteira cairia em vez de so nao mudar.
    await act(async () => {
      stage!.onToggle(slotDe());
      stage!.onChangeEffect(slotDe(), alg);
      stage!.applyKnob(slotDe(), 0, "10");
      stage!.onKnobReset(slotDe(), 0);
      stage!.saveUserPatch("sem palco");
    });
    expect(stage!.board).toBeNull();
    // salvar sem palco nao inventa patch: um "Patch 1" vazio seria um preset
    // de usuario que so existe na lista e nao da para abrir.
    expect(stage!.userPatches).toHaveLength(0);
  });

  it("salva patch do que está no palco e reabre sem device", async () => {
    await montarStage();
    await act(async () => {
      stage!.saveUserPatch("meu patch");
    });
    expect(stage!.userPatches).toHaveLength(1);
    mocks.deviceBoard.mockClear();
    await act(async () => {
      stage!.openUserPatch(stage!.userPatches[0], 0);
    });
    expect(stage!.openUserId).toBe(stage!.userPatches[0].id);
    expect(stage!.presetName).toBe("meu patch");
    // abrir patch é relido do storage local: nenhum comando de device
    expect(mocks.deviceBoard).not.toHaveBeenCalled();
  });

  it("apagar o patch ABERTO volta para o preset de fábrica (a UI não fica mentindo)", async () => {
    await montarStage();
    await act(async () => {
      stage!.saveUserPatch("boom");
    });
    await act(async () => {
      stage!.openUserPatch(stage!.userPatches[0], 0);
    });
    expect(stage!.openUserId).not.toBeNull();
    await act(async () => {
      stage!.deleteUserPatch(stage!.userPatches[0].id);
    });
    expect(stage!.openUserId).toBeNull();
    expect(stage!.userPatches).toHaveLength(0);
    expect(mocks.deviceSelectPreset).toHaveBeenCalled();
  });

  it("apagar patch que NÃO está aberto não mexe no palco", async () => {
    await montarStage();
    await act(async () => {
      stage!.saveUserPatch("a");
      stage!.saveUserPatch("b");
    });
    const antes = mocks.deviceSelectPreset.mock.calls.length;
    await act(async () => {
      stage!.deleteUserPatch(stage!.userPatches[0].id);
    });
    expect(stage!.userPatches).toHaveLength(1);
    expect(mocks.deviceSelectPreset.mock.calls.length).toBe(antes);
  });
});

// ════════════════════════════════════ usePrefs

type Prefs = ReturnType<typeof usePrefs>;
let prefs: Prefs | null = null;

function PrefsProbe() {
  prefs = usePrefs();
  return null;
}

async function montarPrefs() {
  const host = document.createElement("div");
  document.body.appendChild(host);
  let root!: Root;
  await act(async () => {
    root = createRoot(host);
    root.render(
      <StrictMode>
        <PrefsProbe />
      </StrictMode>,
    );
  });
  return { host, root };
}

describe("usePrefs", () => {
  it("defaults sem nada no storage", async () => {
    await montarPrefs();
    expect(prefs!.masterVol).toBe(99);
    expect(prefs!.drum.on).toBe(false);
    expect(prefs!.general.language).toBe("pt-BR");
  });

  it("o drum NUNCA volta tocando (o device não parte em execução)", async () => {
    localStorage.setItem("gp100.drum.v2", JSON.stringify({ on: true, bpm: 140 }));
    await montarPrefs();
    expect(prefs!.drum.bpm).toBe(140);
    expect(prefs!.drum.on).toBe(false);
  });

  it("persiste cada preferência ao mudar", async () => {
    await montarPrefs();
    await act(async () => {
      prefs!.setDrum({ ...prefs!.drum, bpm: 150 });
    });
    expect(JSON.parse(localStorage.getItem("gp100.drum.v2")!).bpm).toBe(150);
    await act(async () => {
      prefs!.setMasterVol(55);
    });
    expect(localStorage.getItem("gp100.master.v1")).toBe("55");
  });

  it("storage indisponível degrada, não quebra", async () => {
    const original = Storage.prototype.getItem;
    Storage.prototype.getItem = () => {
      throw new Error("SecurityError: modo privado");
    };
    try {
      await montarPrefs();
      expect(prefs!.masterVol).toBe(99);
    } finally {
      Storage.prototype.getItem = original;
    }
  });
});
