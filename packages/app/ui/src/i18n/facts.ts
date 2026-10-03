/**
 * i18n/facts — os NÚMEROS que aparecem nos textos, derivados dos artefatos
 * gerados (nunca digitados à mão num dicionário).
 *
 * Por que este arquivo existe: a base pt-BR já derivava `PRESET_COUNT`,
 * `DRUM_COUNT`, `FX_COUNT`… do catálogo. Traduzir esses textos exigindo o
 * mesmo cálculo em 4 idiomas convida a divergência — alguém traduz "99
 * presets" e o catálogo passa a ter 100 sem nenhum teste reclamar. Com os
 * fatos num módulo só, os 4 dicionários leem a MESMA conta e a reta só muda
 * num lugar.
 */
import { FACTORY_PRESETS } from "../artifacts/presetData";
import { DRUM_BEATS, DRUM_GENRES } from "../artifacts/drumData";
import { FX_MODULES } from "../artifacts/fxData";
import { version as REACT_VERSION } from "react";

export const PRESET_COUNT = FACTORY_PRESETS.length;
export const DRUM_GENRE_COUNT = DRUM_GENRES.length;
export const DRUM_COUNT = DRUM_GENRES.reduce((n, g) => n + g.styles.length, 0);
export const DRUM_BEAT_RANGE = `${DRUM_BEATS[0]}…${DRUM_BEATS[DRUM_BEATS.length - 1]}`;
export const FX_COUNT = Object.values(FX_MODULES).reduce((n, a) => n + a.length, 0);
export const CTRL_COUNT = Object.values(FX_MODULES).reduce(
  (n, a) => n + a.reduce((m, alg) => m + alg.knobs.length + alg.switches.length + alg.comboxes.length, 0),
  0,
);
export const REACT_VER = REACT_VERSION;
/** user agent do ambiente — `-` quando não há browser (testes/node) */
export const UA = typeof navigator !== "undefined" ? navigator.userAgent : "-";