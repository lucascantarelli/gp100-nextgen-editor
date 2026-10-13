/**
 * usePrefs — as preferências que sobrevivem ao fechar a janela.
 *
 * Extraído do `App.tsx` pela #82. Tudo aqui tem a mesma natureza: **lê do
 * `localStorage` na montagem e escreve de volta a cada mudança**. Nenhum item
 * vai para o device (a escrita real no aparelho é o gate H2, ADR-5), então a
 * UI as rotula como "prévia local" — e o código precisa deixar isso explícito
 * em vez de deixar parecer que o pedal foi mexido.
 *
 * Dois detalhes que o App repetia e que valem mais aqui:
 *
 * - **O idioma é aplicado no INICIALIZADOR.** `general.language` é a fonte da
 *   verdade e `setLanguage` roda antes do primeiro paint; esperar um efeito
 *   mostraria português para quem pediu inglês por um frame.
 * - **O drum NUNCA volta tocando.** Ao restaurar, `play/stop` é forçado a
 *   desligado: o device não parte em execução e o navegador não deve herdar
 *   reprodução de uma sessão anterior.
 */
import { useCallback, useEffect, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import { setLanguage, useLanguage } from "../i18n/messages";
import { loadGeneral, KEY as GENERAL_KEY } from "../components/SettingsModal";
import type { GeneralSettings } from "../components/SettingsModal";
import { loadTuner, TUNER_KEY } from "../components/TunerPanel";
import type { TunerSettings } from "../components/TunerPanel";
import { loadLooper, KEY as LOOPER_KEY } from "../components/LooperPanel";
import type { LooperSettings } from "../components/LooperPanel";
import type { DrumState } from "../components/DrumPanel";

const DRUM_KEY = "gp100.drum.v2";
const MASTER_KEY = "gp100.master.v1";

const DEFAULT_DRUM: DrumState = {
  on: false,
  genre: "Rock",
  style: "Rock 1",
  bpm: 120,
  beat: "4/4",
  volume: 80,
  speed: 50,
};

function loadDrum(): DrumState {
  try {
    const raw = localStorage.getItem(DRUM_KEY);
    if (raw) return { ...(JSON.parse(raw) as DrumState), on: false };
  } catch {
    /* teste: sem localStorage */
  }
  return DEFAULT_DRUM;
}

function loadMaster(): number {
  try {
    const raw = localStorage.getItem(MASTER_KEY);
    return raw ? (Number(raw) || 99) : 99;
  } catch {
    return 99;
  }
}

/** `localStorage` pode lançar (modo privado, quota). Ler é degradar, não quebrar. */
function persist(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* noop */
  }
}

export interface Prefs {
  masterVol: number;
  setMasterVol: (v: number) => void;
  drum: DrumState;
  /** `Dispatch<SetStateAction>` e não `(d: DrumState) => void`: os atalhos
   *  globais (Espaço = play/stop) usam a forma de UPDATER, e um tipo estreito
   *  aqui obrigaria o App a reimplementar o estado que o hook já tem. */
  setDrum: Dispatch<SetStateAction<DrumState>>;
  tuner: TunerSettings;
  setTuner: (t: TunerSettings) => void;
  looper: LooperSettings;
  setLooper: (l: LooperSettings) => void;
  general: GeneralSettings;
  onChangeGeneral: (s: GeneralSettings) => void;
}

export function usePrefs(): Prefs {
  const [masterVol, setMasterVol] = useState<number>(loadMaster);
  const [drum, setDrum] = useState<DrumState>(loadDrum);

  /* Idioma (#30): `general.language` é a FONTE da verdade (persistida em
   * localStorage); o catálogo vive em `i18n/messages`. Aplicar já no
   * inicializador evita o flash de português antes do primeiro paint, e o
   * efeito abaixo cobre a troca pelo seletor.
   *
   * `useLanguage()` reassina a troca PARA O APP INTEIRO re-renderizar — é o
   * único ponto do código que precisa saber que existe idioma; os ~15
   * componentes abaixo só leem `MSG.*` como sempre leram. */
  const [general, setGeneral] = useState<GeneralSettings>(() => {
    const g = loadGeneral();
    setLanguage(g.language);
    return g;
  });
  useLanguage();
  useEffect(() => {
    setLanguage(general.language);
  }, [general.language]);

  const [tuner, setTuner] = useState<TunerSettings>(loadTuner);
  const [looper, setLooper] = useState<LooperSettings>(loadLooper);

  useEffect(() => {
    persist(DRUM_KEY, JSON.stringify(drum));
  }, [drum]);
  useEffect(() => {
    persist(MASTER_KEY, String(masterVol));
  }, [masterVol]);
  useEffect(() => {
    persist(TUNER_KEY, JSON.stringify(tuner));
  }, [tuner]);
  useEffect(() => {
    persist(LOOPER_KEY, JSON.stringify(looper));
  }, [looper]);

  const onChangeGeneral = useCallback((s: GeneralSettings) => {
    setGeneral(s);
    persist(GENERAL_KEY, JSON.stringify(s));
  }, []);

  return {
    masterVol,
    setMasterVol,
    drum,
    setDrum,
    tuner,
    setTuner,
    looper,
    setLooper,
    general,
    onChangeGeneral,
  };
}
