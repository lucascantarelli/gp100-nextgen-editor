/**
 * useGlobals — os registradores globais (§13.13) na UI: master VOL e DRUM.
 *
 * Duas direções do mesmo fio, dono único deste acoplamento:
 *
 *   - device → UI (seed no boot validado): `device_globals` traz o que o
 *     APARELHO diz (decode medido nibble-par no backend); o localStorage é
 *     só o fallback de quem não tem device (ADR-13: a fonte é o aparelho).
 *     PRIMEIRA ESCRITA VENCE (#211): o seed não apaga uma edição do
 *     operador que chegou antes de `deviceGlobals` resolver.
 *     Re-lê no ⟳ de propósito (o re-scan relê o device). Falha aqui NÃO vira
 *     banner: o palco já está no estado do device (#150) e este
 *     enriquecimento é opcional — o retry natural é o próprio ⟳.
 *
 *   - UI → device (write-through): prévia local SEMPRE + escrita no
 *     registrador quando o build permite (ADR-5: sem `write-verified` o
 *     transporte recusa antes do driver e o erro fica no console — mesma
 *     tratamento do knob). Só o que MUDOU dispara write (os botões ±/slider
 *     do DRUM mandam estado inteiro).
 *
 * Por ser um hook e não ficar no App: é o acoplamento `ipc/device` ×
 * `prefs` — coesão de dois vizinhos, não do casco (#82: App fino).
 */
import { useEffect, useRef } from "react";
import type { Dispatch, SetStateAction } from "react";
import { deviceGlobals, deviceSetGlobal } from "../ipc/device";
import type { DrumState } from "../components/DrumPanel";

/** O que o hook precisa dos dois lados: o `writeVerified` do device_info e
 *  os setters de persistência do `usePrefs`. */
interface GlobalsDeps {
  /** Boot validado — sem ele o seed não roda (o aparelho ainda não leu). */
  pronto: boolean;
  /** `device_info` corrente (null antes do primeiro response). */
  writeVerified: boolean;
  setMasterVol: (v: number) => void;
  drum: DrumState;
  setDrum: Dispatch<SetStateAction<DrumState>>;
}

interface GlobalsApi {
  /** Master VOL: prévia local + write do gVolume (quando liberado). */
  mudaMaster: (v: number) => void;
  /** DRUM: prévia local + write do que mudou (drumBpm/drumVolume). */
  mudaDrum: (d: DrumState) => void;
}

/** Clamp do valor medido na faixa do controle da tela (0..99 master,
 *  40..240 BPM, 0..100 volume DRUM — faixas da UI, não do protocolo). */
const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));

export function useGlobals({ pronto, writeVerified, setMasterVol, drum, setDrum }: GlobalsDeps): GlobalsApi {
  // PRIMEIRA ESCRITA VENCE (#211): o seed resolve quando `deviceGlobals`
  // responde — com device lento a edição do operador chega ANTES e não pode
  // ser apagada (flake CI: fill("55") em #drum-vol voltou para o 80 do
  // fallback). Flag por campo: mexer no DRUM não bloqueia o seed do master.
  // A flag nunca é rearmada: depois que o operador tocou no controle, a UI é
  // a fonte — inclusive se o boot re-semeia no ⟳.
  const mexeuMaster = useRef(false);
  const mexeuDrum = useRef(false);

  // Seed do aparelho no boot validado (device → UI).
  useEffect(() => {
    if (!pronto) return;
    void deviceGlobals()
      .then((g) => {
        if (g.volume != null && !mexeuMaster.current) setMasterVol(clamp(g.volume, 0, 99));
        if ((g.drumBpm != null || g.drumVolume != null) && !mexeuDrum.current) {
          setDrum((d) => ({
            ...d,
            ...(g.drumBpm != null ? { bpm: clamp(g.drumBpm, 40, 240) } : {}),
            ...(g.drumVolume != null ? { volume: clamp(g.drumVolume, 0, 100) } : {}),
          }));
        }
      })
      .catch((e: unknown) => {
        console.error("device_globals indisponível (localStorage segue):", e);
      });
  }, [pronto, setMasterVol, setDrum]);

  // UI → device: master VOL (gVolume).
  const mudaMaster = (v: number) => {
    mexeuMaster.current = true;
    setMasterVol(v);
    if (writeVerified) {
      void deviceSetGlobal("gVolume", v).catch((e: unknown) => {
        console.error("gVolume write falhou (prévia local):", e);
      });
    }
  };

  // UI → device: DRUM (drumBpm/drumVolume) — só o que MUDOU dispara write.
  const mudaDrum = (d: DrumState) => {
    const anterior = drum;
    mexeuDrum.current = true;
    setDrum(d);
    if (!writeVerified) return;
    if (d.bpm !== anterior.bpm) {
      void deviceSetGlobal("drumBpm", d.bpm).catch((e: unknown) => {
        console.error("drumBpm write falhou (prévia local):", e);
      });
    }
    if (d.volume !== anterior.volume) {
      void deviceSetGlobal("drumVolume", d.volume).catch((e: unknown) => {
        console.error("drumVolume write falhou (prévia local):", e);
      });
    }
  };

  return { mudaMaster, mudaDrum };
}
