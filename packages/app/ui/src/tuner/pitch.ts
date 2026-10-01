/**
 * tuner/pitch — motor de afinação (PURO: sem DOM, sem áudio do device).
 *
 * O display do GP-100 (manual V1.8, p.5 "Using The TUNER") mostra a nota
 * NO CENTRO, flat à esquerda, sharp à direita, com a escala mudando
 * vermelho → amarelo → verde conforme a corda entra no ponto. Este módulo
 * produz exatamente essa leitura a partir de amostras:
 *
 *  - f0 por AUTOCORRELAÇÃO no domínio do tempo — para guitarra é mais
 *    confiável que o pico de FFT (que sofre com harmônicos e resolução
 *    de bin); pico refinado por interpolação parabólica (precisão
 *    sub-amostra, abordagem YIN simplificada) e escolha do PRIMEIRO pico
 *    forte (evita erro de oitava quando harmônicos dominam);
 *  - gate de RMS (silêncio = sem leitura) e janela fixa de análise;
 *  - conversão para nota/cents contra o REF PITCH (435–445 Hz, padrão
 *    440) — o MESMO range validado no firmware V2.1
 *    (`(Freq>=435) && (Freq<=445)` em audio.c);
 *  - banda de cor por |cents| (verde/âmbar/vermelho) — os limiares são
 *    constantes exportadas e calibradas nos testes.
 *
 * Hoje a UI não tem canal de áudio (prévia local honesta); quando existir,
 * o componente recebe a leitura daqui — nada de simulação falsa.
 */

export const NOTE_NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"] as const;

export type TunerBand = "ok" | "warn" | "error";

export interface TunerReading {
  frequency: number;
  /** número MIDI contínuo (69 = A4 @ refPitch) */
  midi: number;
  note: string;
  /** oitava científica (A4 = 440 Hz) */
  octave: number;
  /** desvio da nota mais próxima, −50..+50 */
  cents: number;
  band: TunerBand;
}

export const REF_PITCH_MIN = 435;
export const REF_PITCH_MAX = 445;
export const REF_PITCH_DEFAULT = 440;

/** |cents| ≤ 3 = no ponto (verde); ≤ 15 perto (âmbar); acima, vermelho. */
export const BAND_OK_CENTS = 3;
export const BAND_WARN_CENTS = 15;

export function bandForCents(cents: number): TunerBand {
  const a = Math.abs(cents);
  if (a <= BAND_OK_CENTS) return "ok";
  if (a <= BAND_WARN_CENTS) return "warn";
  return "error";
}

export function frequencyToReading(frequency: number, refPitch: number = REF_PITCH_DEFAULT): TunerReading {
  const midi = 69 + 12 * Math.log2(frequency / refPitch);
  const nearest = Math.round(midi);
  const cents = (midi - nearest) * 100;
  const pc = ((nearest % 12) + 12) % 12;
  return {
    frequency,
    midi,
    note: NOTE_NAMES[pc],
    octave: Math.floor(nearest / 12) - 1,
    cents,
    band: bandForCents(cents),
  };
}

/** Janela de análise: 2048 amostras cobre E2 (82.4 Hz) em 44.1 kHz com
 *  folga para a autocorrelação (lag máx = rate/40, limitado a N/2). */
const WINDOW = 2048;
/** RMS abaixo disso = silêncio/ruído → sem leitura (honesto). */
const RMS_GATE = 0.012;
/** 1º pico local com r ≥ 85% do global (evita travar em sub-harmônico). */
const PEAK_THRESHOLD = 0.85;
/** correlação mínima para considerar periodicidade real. */
const MIN_CORR = 0.3;

/** Detecta a frequência fundamental (Hz) ou null (silêncio/sem pitch). */
export function detectPitch(samples: ArrayLike<number>, sampleRate: number): number | null {
  const n = Math.min(samples.length, WINDOW);
  if (n < WINDOW) return null;

  let energy = 0;
  for (let i = 0; i < n; i += 1) energy += samples[i] * samples[i];
  if (Math.sqrt(energy / n) < RMS_GATE) return null;

  const minLag = Math.max(2, Math.floor(sampleRate / 1000)); // teto ~1 kHz
  const maxLag = Math.min(Math.floor(sampleRate / 40), n >> 1); // piso ~40 Hz
  if (maxLag <= minLag) return null;

  const r = new Float64Array(maxLag + 1);
  let bestR = 0;
  let bestLag = -1;
  for (let lag = minLag; lag <= maxLag; lag += 1) {
    let corr = 0;
    let e1 = 0;
    let e2 = 0;
    for (let i = 0; i < n - lag; i += 1) {
      const a = samples[i];
      const b = samples[i + lag];
      corr += a * b;
      e1 += a * a;
      e2 += b * b;
    }
    r[lag] = corr / Math.sqrt(e1 * e2 + 1e-12);
    if (r[lag] > bestR) {
      bestR = r[lag];
      bestLag = lag;
    }
  }
  if (bestLag < 0 || bestR < MIN_CORR) return null;

  // 1º pico LOCAL forte = período real (o global pode ser 2× o período)
  let lag = minLag;
  while (lag <= maxLag) {
    if (r[lag] >= PEAK_THRESHOLD * bestR && r[lag] >= (r[lag - 1] ?? 0) && r[lag] >= (r[lag + 1] ?? 0)) break;
    lag += 1;
  }
  if (lag > maxLag) lag = bestLag;

  // interpolação parabólica do pico (precisão sub-amostra)
  const y0 = r[lag - 1] ?? 0;
  const y1 = r[lag];
  const y2 = r[lag + 1] ?? 0;
  const denom = y0 - 2 * y1 + y2;
  const shift = denom !== 0 ? (0.5 * (y0 - y2)) / denom : 0;
  const refined = lag + Math.max(-1, Math.min(1, shift));
  return sampleRate / refined;
}

/**
 * Motor stateful do painel: acumula suavização do desvio entre janelas
 * (agulha estável, a transição de cor suave do manual) e zera no silêncio.
 */
export class TunerEngine {
  private smoothed: number | null = null;

  constructor(
    private refPitch: number = REF_PITCH_DEFAULT,
    private alpha = 0.35,
  ) {}

  setRefPitch(refPitch: number): void {
    this.refPitch = refPitch;
  }

  /** Processa uma janela de amostras; null = silêncio (sem leitura). */
  push(samples: ArrayLike<number>, sampleRate: number): TunerReading | null {
    const f = detectPitch(samples, sampleRate);
    if (f === null) {
      this.smoothed = null;
      return null;
    }
    this.smoothed = this.smoothed === null ? f : this.smoothed + this.alpha * (f - this.smoothed);
    return frequencyToReading(this.smoothed, this.refPitch);
  }
}
