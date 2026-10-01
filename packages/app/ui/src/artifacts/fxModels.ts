/**
 * Catálogo de MODELOS de pedal por variante (efeito real da pedaleira).
 *
 * Fonte dos efeitos: analysis/fx_map.json (gerado do parameters.json —
 * insumo do device, nunca hardcode). Cada variante mapeia para um `Model` com a FORMA do SVG (enclosure
 * compacto, caixão largo, treadle de wah, rack de eco…) e referência
 * visual real (o SVG não copia marcas, segue a LINGUAGEM da referência).
 * Sem entrada no catálogo → fallback pelo archetype (capacidade estável
 * para os 185 efeitos).
 */
import type { BoardSlot } from "../ipc/types";

export type ModelShape =
  | "mini" // enclosure pequeno (1–2 knobs)
  | "box" // caixa compacta 3 colunas (Boss/MXR vibe)
  | "widebox" // caixão largo 4–5 colunas (Timeline/BigSky vibe)
  | "treadle" // wah Cry Baby/Morley (rocker em cima)
  | "filter" // envelope filter/pedais de auto-wah (retângulo vertical)
  | "tscream" // linguagem Tube Screamer (verde, knobs em diagonal)
  | "fuzz" // Fuzz Face (círculo, 2–3 knobs na face)
  | "rotary" // Uni-Vibe/roto (painel em leque)
  | "amphead" // cabeçote de amp (tolex + painel dourado + válvulas)
  | "echo"; // rack de eco (janela de display + knobs em fileira)

export interface ModelSpec {
  shape: ModelShape;
  /** Largura base do SVG (a altura cresce com os knobs). */
  w: number;
  cols: number;
  /** Ajustes finos de cor da carcaça (default = paleta da família). */
  body?: string;
  face?: string;
  /** Referência visual (comentário/tooltip do modo engenheiro). */
  ref: string;
}

/** Catálogo: as variantes com identidade visual forte. As demais caem no
 * fallback por archetype (todas têm capacidade garantida). */
const VARIANTS: Record<string, ModelSpec> = {
  // ── PRE ──
  "c-wah": { shape: "treadle", w: 170, cols: 2, ref: "Cry Baby (treadle)" },
  "v-wah": { shape: "treadle", w: 170, cols: 2, ref: "Vox V847 (treadle)" },
  "t-wah": { shape: "treadle", w: 170, cols: 2, ref: "THC/Morley (treadle)" },
  "a-wah": { shape: "treadle", w: 170, cols: 2, ref: "Auto-wah em treadle" },
  "step-filter": { shape: "filter", w: 150, cols: 4, ref: "Auto Filter (sequência)" },
  comp: { shape: "mini", w: 118, cols: 2, ref: "Compactor (mini)" },
  comp4: { shape: "box", w: 150, cols: 3, ref: "Ross/CS-3 (compact)" },
  boost: { shape: "mini", w: 112, cols: 1, ref: "LPB-1 boost (mini)" },
  "14-boost": { shape: "mini", w: 112, cols: 1, ref: "Boost de linha" },
  octa: { shape: "box", w: 150, cols: 3, ref: "Octavia (compact)" },
  "ac-sim": { shape: "box", w: 150, cols: 3, ref: "AC booster (compact)" },
  saturate: { shape: "box", w: 150, cols: 3, ref: "Saturador compact" },
  hammy: { shape: "fuzz", w: 150, cols: 3, ref: "Fuzz face (círculo)" },
  "p-bend": { shape: "box", w: 156, cols: 3, ref: "Pitch shifter (compact)" },
  pitch: { shape: "box", w: 156, cols: 3, ref: "Pitch (compact)" },
  "ring-mod": { shape: "widebox", w: 176, cols: 4, ref: "Ring mod (experimental)" },

  // ── DST (as referências clássicas) ──
  "green-od": { shape: "tscream", w: 156, cols: 3, body: "#2e8b57", face: "#1f6b40", ref: "Tube Screamer (verde, diagonal)" },
  "scream-od": { shape: "tscream", w: 156, cols: 3, body: "#2e8b57", face: "#1f6b40", ref: "Tube Screamer (verde, diagonal)" },
  "yellow-od": { shape: "box", w: 150, cols: 3, body: "#e8b420", face: "#c2920f", ref: "SD-1 (amarelo)" },
  "blues-od": { shape: "box", w: 150, cols: 3, body: "#7ba7d7", face: "#5a86bd", ref: "Blues Driver (azul)" },
  "red-haze": { shape: "fuzz", w: 150, cols: 3, body: "#b03030", face: "#8a2020", ref: "Fuzz Face (círculo vermelho)" },
  "sm-dist": { shape: "box", w: 150, cols: 3, body: "#e07b2a", face: "#b85c18", ref: "Dist + (laranja)" },
  "bass-dist": { shape: "widebox", w: 170, cols: 4, ref: "Dist de baixo (wide)" },
  "flex-od": { shape: "widebox", w: 170, cols: 4, ref: "OD mult modo (wide)" },
  "tube-clipper": { shape: "box", w: 150, cols: 3, ref: "Clipper transparente" },
  "taichi-od": { shape: "box", w: 150, cols: 3, ref: "OD oriental (compact)" },
  "la-charger": { shape: "box", w: 150, cols: 3, body: "#c2452d", face: "#8f2d1c", ref: "Dist carregada" },
  lazaro: { shape: "box", w: 150, cols: 3, ref: "OD boutique" },
  darktale: { shape: "box", w: 150, cols: 3, body: "#3d3d4a", face: "#2a2a34", ref: "Dist dark" },
  chief: { shape: "box", w: 150, cols: 3, ref: "Dist chief" },
  penesas: { shape: "box", w: 150, cols: 3, ref: "Dist boutique" },
  plustortion: { shape: "widebox", w: 170, cols: 4, ref: "Dist mult (wide)" },
  "super-od": { shape: "box", w: 150, cols: 3, ref: "OD super" },

  // ── MOD ──
  "a-chorus": { shape: "box", w: 156, cols: 3, ref: "Chorus compact (CE-2 vibe)" },
  "b-chorus": { shape: "box", w: 156, cols: 3, ref: "Chorus bicamada" },
  "g-chorus": { shape: "widebox", w: 170, cols: 4, ref: "Chorus estéreo (wide)" },
  vibe: { shape: "rotary", w: 170, cols: 3, ref: "Uni-Vibe (leque)" },
  "opto-trem": { shape: "box", w: 150, cols: 3, ref: "Trem opto (Pulsar)" },
  "sine-trem": { shape: "box", w: 150, cols: 3, ref: "Trem senoidal" },
  "bias-trem": { shape: "box", w: 150, cols: 3, ref: "Trem de bias (Fender)" },
  "triangle-trem": { shape: "box", w: 150, cols: 3, ref: "Trem triangular" },
  phaser: { shape: "box", w: 156, cols: 4, ref: "Phase 90/100 (altura)" },
  flanger: { shape: "widebox", w: 170, cols: 4, ref: "BF-2/Electric Mistress" },
  vibrato: { shape: "box", w: 150, cols: 3, ref: "Vibrato compact" },
  detune: { shape: "mini", w: 124, cols: 2, ref: "Detune (mini)" },

  // ── DLY (maioria caixão largo com display) ──
  sweet: { shape: "echo", w: 196, cols: 4, ref: "Delay clean (display)" },
  "p-echo": { shape: "echo", w: 196, cols: 4, ref: "Delay ping-pong estéreo" },
  "ping-pong": { shape: "echo", w: 196, cols: 4, ref: "Delay ping-pong estéreo" },
  "999-echo": { shape: "echo", w: 196, cols: 4, ref: "Maestro Echoplex (rack)" },
  "m-echo": { shape: "echo", w: 196, cols: 4, ref: "Echo magnético" },
  "m-echo2": { shape: "echo", w: 196, cols: 4, ref: "Echo magnético 2" },
  "t-echo": { shape: "echo", w: 196, cols: 4, ref: "Tape echo (tape hiss)" },
  "vin-rack": { shape: "echo", w: 196, cols: 4, ref: "Rack vintage (DD-20)" },
  "rev-echo": { shape: "echo", w: 196, cols: 4, ref: "Reverse echo" },
  slapbk: { shape: "echo", w: 180, cols: 3, ref: "Slapback (compact)" },
  "dual-echo": { shape: "echo", w: 196, cols: 4, ref: "Dual delay (wide)" },
  "swp-echo": { shape: "echo", w: 196, cols: 4, ref: "Sweep echo (filtrado)" },

  // ── RVB (caixão largo, vibe Strymon) ──
  hall: { shape: "widebox", w: 196, cols: 4, ref: "Hall (big box)" },
  plate: { shape: "widebox", w: 196, cols: 4, ref: "Plate (big box)" },
  room: { shape: "widebox", w: 196, cols: 4, ref: "Room (big box)" },
  spring: { shape: "widebox", w: 196, cols: 4, body: "#b8892f", face: "#8a651c", ref: "Spring (tan, Fender vibe)" },
  church: { shape: "widebox", w: 196, cols: 4, ref: "Church (big box)" },
  "n-star": { shape: "widebox", w: 196, cols: 4, ref: "Reverb estrelar" },
  "mod-verb": { shape: "widebox", w: 196, cols: 4, ref: "Modulated verb" },
  "clear-sky": { shape: "widebox", w: 196, cols: 4, ref: "BlueSky vibe (azul claro)" },
  "deep-sea": { shape: "widebox", w: 196, cols: 4, body: "#1f4d6b", face: "#143a52", ref: "DeepSea vibe (azul profundo)" },
  shimmer: { shape: "widebox", w: 196, cols: 4, ref: "Shimmer (big box)" },

  // ── EQ/NR ──
  "eq-1": { shape: "widebox", w: 176, cols: 6, ref: "EQ gráfico (sliders)" },
  "eq-2": { shape: "widebox", w: 176, cols: 6, ref: "EQ gráfico (sliders)" },
  "mess-eq": { shape: "widebox", w: 176, cols: 6, ref: "EQ mesa (wide)" },
  "gate-1": { shape: "box", w: 124, cols: 2, ref: "Gate compact" },
  "gate-2": { shape: "box", w: 124, cols: 2, ref: "Gate compact 2" },
};

/** Fallbacks por archetype (garantem capacidade p/ as variantes sem entry). */
export const ARCHETYPE_FALLBACK: Record<BoardSlot["archetype"], ModelSpec> = {
  BUFFER: { shape: "mini", w: 112, cols: 1, ref: "Buffer/switcher (mini)" },
  DISTORTION: { shape: "box", w: 156, cols: 3, ref: "Dist compact" },
  // AMP é ITEM DA CADEIA: cabeçote com os knobs reais do algoritmo
  // (Bog RedM etc. têm 6–7 knobs) — drag-and-drop como qualquer pedal.
  AMPLIFIER: { shape: "amphead", w: 216, cols: 3, ref: "Head de amp (na cadeia)" },
  NOISEGATE: { shape: "box", w: 124, cols: 2, ref: "Gate compact" },
  CABINET: { shape: "box", w: 170, cols: 3, ref: "Cabinet IR (slot)" },
  EQ: { shape: "widebox", w: 176, cols: 6, ref: "EQ gráfico" },
  MODULATION: { shape: "box", w: 156, cols: 3, ref: "Mod compact" },
  DELAY: { shape: "echo", w: 196, cols: 4, ref: "Delay (display)" },
  REVERB: { shape: "widebox", w: 196, cols: 4, ref: "Reverb (big box)" },
};

/** Resolve o modelo do slot: variante primeiro, archetype como fallback. */
export function modelFor(slot: BoardSlot): ModelSpec {
  return VARIANTS[slot.variant] ?? ARCHETYPE_FALLBACK[slot.archetype];
}
