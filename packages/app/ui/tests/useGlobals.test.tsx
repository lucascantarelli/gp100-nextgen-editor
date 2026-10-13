/**
 * useGlobals — contrato do seed device→UI × escrita do usuário (#211).
 *
 * O seed roda quando `pronto` vira true e resolve DEPOIS (device lento):
 * uma edição do operador que chega antes não pode ser apagada — primeira
 * escrita vence (flake CI run 38441218878: fill("55") em #drum-vol voltou
 * para o 80 do fallback). Contratos aqui:
 *   - seed aplica deviceGlobals quando o usuário NÃO mexeu (master + drum);
 *   - edição ANTES da resolução do seed vence (master VOL e DRUM bpm/volume);
 *   - write-through (deviceSetGlobal) segue o writeVerified do build.
 */
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { useGlobals } from "../src/hooks/useGlobals";
import type { DrumState } from "../src/components/DrumPanel";
import type { DeviceGlobals } from "../src/ipc/types";

const mocks = vi.hoisted(() => ({
  deviceGlobals: vi.fn(),
  deviceSetGlobal: vi.fn(),
}));
vi.mock("../src/ipc/device", () => mocks);

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

beforeEach(() => {
  mocks.deviceGlobals.mockReset();
  mocks.deviceSetGlobal.mockReset();
  mocks.deviceSetGlobal.mockResolvedValue(undefined);
});

/** Estado dos setters reais do hook, espelhado num objeto de teste. */
interface Espelho {
  masterVol: number;
  drum: DrumState;
  mudaMaster: (v: number) => void;
  mudaDrum: (d: DrumState) => void;
}

function monta(espelho: { current: Espelho | null }, pronto: boolean, writeVerified: boolean) {
  function Sonda() {
    const [masterVol, setMasterVol] = useState(40);
    const [drum, setDrum] = useState<DrumState>(DRUM_INICIAL);
    const api = useGlobals({ pronto, writeVerified, setMasterVol, drum, setDrum });
    espelho.current = {
      masterVol,
      drum,
      mudaMaster: api.mudaMaster,
      mudaDrum: api.mudaDrum,
    };
    return null;
  }
  const div = document.createElement("div");
  const raiz = createRoot(div);
  act(() => {
    raiz.render(<Sonda />);
  });
  return () => {
    act(() => raiz.unmount());
  };
}

// Estado inicial do DRUM no teste (só bpm/volume interessam ao contrato).
const DRUM_INICIAL: DrumState = {
  on: false,
  genre: "ROCK",
  style: "A",
  bpm: 120,
  beat: "4/4",
  volume: 80,
  speed: 1,
};

/** DeviceGlobals completo (os campos que o teste não exercita ficam null). */
function globals(parcial: Partial<DeviceGlobals>): DeviceGlobals {
  return { volume: null, drumBpm: null, drumVolume: null, inputLevel: null, raw: {}, ...parcial };
}

/** deviceGlobals adiado: o teste decide quando o seed resolve. */
function globalsAdiado() {
  let resolve: (g: DeviceGlobals) => void = () => undefined;
  const promessa = new Promise<DeviceGlobals>((r) => {
    resolve = r;
  });
  mocks.deviceGlobals.mockReturnValue(promessa);
  return resolve;
}

describe("useGlobals — seed × edição do usuário (#211)", () => {
  it("seed aplica master e drum quando o usuário não mexeu", async () => {
    const espelho: { current: Espelho | null } = { current: null };
    const resolveSeed = globalsAdiado();
    const desmonta = monta(espelho, true, false);

    await act(async () => {
      resolveSeed(globals({ volume: 77, drumBpm: 99, drumVolume: 55 }));
    });

    expect(espelho.current?.masterVol).toBe(77);
    expect(espelho.current?.drum.bpm).toBe(99);
    expect(espelho.current?.drum.volume).toBe(55);
    desmonta();
  });

  it("edição de DRUM antes da resolução do seed vence (o 80 do fallback não volta)", async () => {
    const espelho: { current: Espelho | null } = { current: null };
    const resolveSeed = globalsAdiado();
    const desmonta = monta(espelho, true, false);

    act(() => {
      espelho.current?.mudaDrum({ ...DRUM_INICIAL, bpm: 130, volume: 55 });
    });
    await act(async () => {
      resolveSeed(globals({ volume: 77, drumBpm: 99, drumVolume: 80 }));
    });

    expect(espelho.current?.drum.bpm).toBe(130);
    expect(espelho.current?.drum.volume).toBe(55);
    desmonta();
  });

  it("edição de master VOL antes da resolução do seed vence", async () => {
    const espelho: { current: Espelho | null } = { current: null };
    const resolveSeed = globalsAdiado();
    const desmonta = monta(espelho, true, false);

    act(() => {
      espelho.current?.mudaMaster(30);
    });
    await act(async () => {
      resolveSeed(globals({ volume: 77, drumBpm: 99, drumVolume: 80 }));
    });

    expect(espelho.current?.masterVol).toBe(30);
    desmonta();
  });

  it("write-through: o que o usuário manda vai para o device quando writeVerified", async () => {
    const espelho: { current: Espelho | null } = { current: null };
    mocks.deviceGlobals.mockResolvedValue(globals({}));
    const desmonta = monta(espelho, true, true);

    await act(async () => {
      espelho.current?.mudaMaster(61);
      espelho.current?.mudaDrum({ ...DRUM_INICIAL, bpm: 121, volume: 62 });
    });

    expect(mocks.deviceSetGlobal).toHaveBeenCalledWith("gVolume", 61);
    expect(mocks.deviceSetGlobal).toHaveBeenCalledWith("drumBpm", 121);
    expect(mocks.deviceSetGlobal).toHaveBeenCalledWith("drumVolume", 62);
    desmonta();
  });
});
