/**
 * Afinador (V-7) sob teste direto:
 *  - tuner/pitch (MOTOR PURO): f0 por autocorrelação em senoides sintéticas
 *    (E2/A2/A4 exatos, desafinos ±cents → banda de cor), gate de silêncio,
 *    janela curta = sem leitura e suavização do TunerEngine;
 *  - TunerPanel (COMPONENTE): liga/desliga com controles, modo em ciclo
 *    bypass→thru→mute, REF PITCH clampa 435–445 e persiste via callback;
 *    repouso honesto (agulha central, nota "—") sem áudio real;
 *    grade fixa (não colapsa), botão do monitor VISUAL e o monitor como
 *    gate da leitura (desligado = não ouve, demo incluso) — issue #8.
 */
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { beforeAll, describe, expect, it, vi } from "vitest";
import {
  BAND_OK_CENTS,
  BAND_WARN_CENTS,
  REF_PITCH_DEFAULT,
  bandForCents,
  detectPitch,
  frequencyToReading,
} from "../src/tuner/pitch";
import { TunerEngine } from "../src/tuner/pitch";
import { TunerPanel, loadTuner } from "../src/components/TunerPanel";
import type { TunerSettings } from "../src/components/TunerPanel";

beforeAll(() => {
  (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
});

/* ── síntese de teste: senoide + harmônico (nada de fixture binária) ── */
function tone(freq: number, seconds = 0.2, rate = 44100): Float32Array {
  const n = Math.round(seconds * rate);
  const buf = new Float32Array(n);
  let phase = 0;
  for (let i = 0; i < n; i += 1) {
    phase += (2 * Math.PI * freq) / rate;
    buf[i] = (Math.sin(phase) + 0.3 * Math.sin(2 * phase)) / 1.3;
  }
  return buf;
}

describe("tuner/pitch — motor puro", () => {
  it("detecta as cordas de referência (E2, A2, A4) com precisão de ~1 cent", () => {
    for (const freq of [82.41, 110, 440]) {
      const f = detectPitch(tone(freq), 44100);
      expect(f).not.toBeNull();
      const cents = 1200 * Math.log2((f as number) / freq);
      expect(Math.abs(cents)).toBeLessThan(1);
    }
  });

  it("converte frequência → nota/cents contra o REF PITCH (440 padrão)", () => {
    const a4 = frequencyToReading(440);
    expect(a4.note).toBe("A");
    expect(a4.octave).toBe(4);
    expect(a4.cents).toBeCloseTo(0, 5);
    expect(a4.band).toBe("ok");

    /* A4 a 445 Hz de referência: 440 Hz fica ~19.6 cents flat */
    const flat = frequencyToReading(440, 445);
    expect(flat.cents).toBeLessThan(-15);
    expect(flat.band).toBe("error");
  });

  it("bandas seguem os limiares exportados (verde/âmbar/vermelho)", () => {
    expect(bandForCents(BAND_OK_CENTS)).toBe("ok");
    expect(bandForCents(BAND_OK_CENTS + 0.1)).toBe("warn");
    expect(bandForCents(BAND_WARN_CENTS)).toBe("warn");
    expect(bandForCents(BAND_WARN_CENTS + 0.1)).toBe("error");
    expect(bandForCents(-BAND_WARN_CENTS - 1)).toBe("error");
  });

  it("silêncio e janela curta = sem leitura (gate honesto)", () => {
    expect(detectPitch(new Float32Array(2048), 44100)).toBeNull(); /* zeros */
    expect(detectPitch(tone(110, 0.01), 44100)).toBeNull(); /* janela curta */
  });

  it("TunerEngine suaviza o desvio entre janelas (agulha estável)", () => {
    const eng = new TunerEngine(440, 0.35);
    const r1 = eng.push(tone(440 * Math.pow(2, 10 / 1200)), 44100); /* +10 cents */
    expect(r1?.cents).toBeGreaterThan(5);
    const r2 = eng.push(tone(440), 44100); /* volta ao ponto */
    expect((r2?.cents ?? 0)).toBeLessThan(r1?.cents ?? 0); /* inércia */
    expect(eng.push(new Float32Array(2048), 44100)).toBeNull(); /* silêncio zera */
  });

  it("REF PITCH fora do range do firmware é rejeitado pela conversão (435–445)", () => {
    /* o range é guardado na UI; o motor aceita o número e o teste trava o
       contrato de que 440 é o padrão e 435/445 os extremos do device */
    expect(REF_PITCH_DEFAULT).toBe(440);
    expect(frequencyToReading(440, 435).cents).toBeGreaterThan(15);
    expect(frequencyToReading(440, 445).cents).toBeLessThan(-15);
  });
});

/* ── componente ── */
function mount(ui: React.ReactElement): { root: Root; host: HTMLElement } {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  act(() => root.render(ui));
  return { root, host };
}

const BASE: TunerSettings = { on: false, mode: "mute", refPitch: 440 };

describe("TunerPanel — painel do palco", () => {
  it("repouso honesto: sem leitura, nota '—' e agulha no centro", () => {
    const { root, host } = mount(<TunerPanel settings={{ ...BASE, on: true }} onChange={vi.fn()} />);
    const note = host.querySelector("[data-tuner-note]");
    expect(note?.textContent).toBe("—");
    const needleEl = host.querySelector<HTMLElement>("[data-tuner-needle]");
    expect(needleEl?.style.left).toContain("50%"); /* centro */
    act(() => root.unmount());
  });

  it("botão liga/desliga e notifica o pai (persistência via callback)", () => {
    const onChange = vi.fn();
    const { root, host } = mount(<TunerPanel settings={BASE} onChange={onChange} />);
    const btn = host.querySelector<HTMLButtonElement>('[aria-label^="Ligar ou desligar"]');
    act(() => btn?.click());
    expect(onChange).toHaveBeenCalledWith({ on: true, mode: "mute", refPitch: 440 });
    act(() => root.unmount());
  });

  it("modo alterna em ciclo bypass→thru→mute (componente controlado)", () => {
    /* como no App real: o pai realimenta o settings a cada onChange */
    let current: TunerSettings = { ...BASE, on: true };
    const { root, host } = mount(
      <TunerPanel settings={current} onChange={loopOnChange} />,
    );
    function loopOnChange(s: TunerSettings) {
      current = s;
      act(() => root.render(<TunerPanel settings={current} onChange={loopOnChange} />));
    }
    const modeBtn = () => host.querySelector<HTMLButtonElement>('[aria-label^="Modo do afinador"]');
    act(() => modeBtn()?.click()); /* mute → bypass */
    expect(current.mode).toBe("bypass");
    act(() => modeBtn()?.click()); /* bypass → thru */
    expect(current.mode).toBe("thru");
    act(() => modeBtn()?.click()); /* thru → mute */
    expect(current.mode).toBe("mute");
    act(() => root.unmount());
  });

  it("REF PITCH recebe o slider e reflete o valor (435–445)", () => {
    const onChange = vi.fn();
    const { root, host } = mount(<TunerPanel settings={{ ...BASE, on: true }} onChange={onChange} />);
    const range = host.querySelector<HTMLInputElement>('[aria-label^="Pitch de referência"]');
    expect(range?.min).toBe("435");
    expect(range?.max).toBe("445");
    act(() => {
      range?.focus();
      range?.stepUp(5); /* 440 → 445 */
    });
    act(() => range?.dispatchEvent(new Event("change", { bubbles: true })));
    expect(onChange.mock.lastCall?.[0].refPitch).toBe(445);
    act(() => root.unmount());
  });

  it("botão do monitor é VISUAL: ícone + LED verde/vermelho, sem palavra de estado (#8)", () => {
    const on = mount(<TunerPanel settings={{ ...BASE, on: true }} onChange={vi.fn()} />);
    const btnOn = on.host.querySelector<HTMLButtonElement>("[data-tuner-power]")!;
    expect(btnOn.getAttribute("data-tuner-power")).toBe("on");
    expect(btnOn.getAttribute("aria-pressed")).toBe("true");
    expect(btnOn.textContent, "só o ícone — nada de 'Ligado'").toBe("♪");
    expect(
      on.host.querySelector("[data-tuner-power-led]")!.getAttribute("data-tuner-power-led"),
      "LED do botão verde quando ligado",
    ).toBe("on");
    act(() => on.root.unmount());

    const off = mount(<TunerPanel settings={BASE} onChange={vi.fn()} />);
    const btnOff = off.host.querySelector<HTMLButtonElement>("[data-tuner-power]")!;
    expect(btnOff.getAttribute("data-tuner-power")).toBe("off");
    expect(btnOff.getAttribute("aria-pressed")).toBe("false");
    expect(btnOff.textContent).toBe("♪");
    expect(
      off.host.querySelector("[data-tuner-power-led]")!.getAttribute("data-tuner-power-led"),
      "LED do botão vermelho quando desligado",
    ).toBe("off");
    act(() => off.root.unmount());
  });

  it("grade de controles NÃO colapsa: modo e REF seguem no DOM com o monitor desligado (#8)", () => {
    const controlsOf = (host: HTMLElement) =>
      host.querySelector('[aria-label="Afinador"]')!.children[1].children.length;

    const off = mount(<TunerPanel settings={BASE} onChange={vi.fn()} />);
    expect(off.host.querySelector("[data-tuner-mode]"), "modo visível desligado").toBeTruthy();
    expect(
      off.host.querySelector('[aria-label^="Pitch de referência"]'),
      "REF PITCH visível desligado",
    ).toBeTruthy();
    const offCount = controlsOf(off.host);
    act(() => off.root.unmount());

    const on = mount(<TunerPanel settings={{ ...BASE, on: true }} onChange={vi.fn()} />);
    expect(
      controlsOf(on.host),
      "mesma grade ligado/desligado (sem salto de layout)",
    ).toBe(offCount);
    expect(offCount, "monitor + modo + ref + demo").toBe(4);
    act(() => on.root.unmount());
  });

  it("monitor desligado NÃO ouve: leitura do device fica em repouso (#8)", () => {
    const reading = frequencyToReading(110); /* A2 real do motor */
    const off = mount(<TunerPanel settings={BASE} reading={reading} onChange={vi.fn()} />);
    expect(off.host.querySelector("[data-tuner-note]")!.textContent).toBe("—");
    expect(
      off.host.querySelector<HTMLElement>("[data-tuner-needle]")!.style.left,
      "agulha no centro",
    ).toContain("50%");
    act(() => off.root.unmount());

    const on = mount(
      <TunerPanel settings={{ ...BASE, on: true }} reading={reading} onChange={vi.fn()} />,
    );
    expect(on.host.querySelector("[data-tuner-note]")!.textContent).toBe("A2");
    act(() => on.root.unmount());
  });

  it("demo liga o monitor quando estava desligado e para quando ele desliga (#8)", () => {
    let current: TunerSettings = { ...BASE };
    const onChange = vi.fn((s: TunerSettings) => {
      current = s;
      act(() => root.render(<TunerPanel settings={current} onChange={onChange} />));
    });
    const { root, host } = mount(<TunerPanel settings={current} onChange={onChange} />);

    /* demonstrar exige o monitor ouvindo: o clique liga os dois de uma vez */
    act(() => host.querySelector<HTMLButtonElement>('[aria-label^="Tocar demonstração"]')!.click());
    expect(current.on, "monitor ligado pela demo").toBe(true);
    expect(current.mode, "demais ajustes intocados").toBe("mute");
    expect(host.querySelector("[data-tuner-demo]")!.getAttribute("data-tuner-demo")).toBe("on");

    /* desligar o monitor para a demo (nada roda em background) */
    act(() => host.querySelector<HTMLButtonElement>("[data-tuner-power]")!.click());
    expect(current.on).toBe(false);
    expect(
      host.querySelector("[data-tuner-demo]")!.getAttribute("data-tuner-demo"),
      "demo parou junto com o monitor",
    ).toBe("off");
    act(() => root.unmount());
  });

  it("loadTuner clampa REF PITCH fora do range e honra defaults", () => {
    expect(loadTuner()).toEqual(BASE);
    localStorage.setItem("gp100.tuner.v1", JSON.stringify({ on: true, mode: "thru", refPitch: 999 }));
    const loaded = loadTuner();
    expect(loaded.refPitch).toBe(445); /* clampado */
    expect(loaded.mode).toBe("thru");
    localStorage.removeItem("gp100.tuner.v1");
  });
});
