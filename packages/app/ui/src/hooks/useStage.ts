/**
 * useStage — o que está NO PALCO, e as operações sobre isso.
 *
 * Extraído do `App.tsx` pela #82 (o App era um monólito de 514 linhas com um
 * arquivo de teste do dobro do tamanho). O critério de corte não foi "tamanho
 * de arquivo", foi **coesão**: um preset de fábrica e um patch de usuário são
 * a mesma coisa vista de dois lugares (um `BoardView`), então abrir, salvar,
 * excluir e mexer no palco são TODAS operações sobre um único estado. Eles
 * ficam juntos; o que não é palco (preferências do menu, drum, looper,
 * tuner) foi para o `usePrefs`.
 *
 * Invariantes que este hook centraliza (e que antes estavam espalhadas pelo
 * App, cada uma com seu próprio parágrafo de comentário):
 *
 * - **Nada de estado otimista.** `pp`/`presetName`/`board` só mudam DEPOIS
 *   que o device confirma. Se o select falhar, a navbar continua mostrando o
 *   preset que o device realmente tem.
 * - **Falha tem ação.** Todo erro vira `{message, retry}` (issue #20): o
 *   banner nunca é um spinner eterno sem saída.
 * - **Patch apagado não fica no palco.** Excluir o patch que estava aberto
 *   volta para o preset de fábrica confirmado — deixar a UI desenhando algo
 *   apagado é a UI mentindo sobre o que existe.
 * - **Abrir preset fecha a edição ampliada** e solta o patch de usuário, para
 *   navbar, palco e modal nunca discordarem sobre o que está aberto.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import type { BoardSlot, BoardView } from "../ipc/types";
import { deviceBoard, deviceSelectPreset, deviceSetParam } from "../ipc/device";
import { MSG } from "../i18n/messages";
import { boardOfUserPatch, loadUserPatches, snapshotOf, storeUserPatches } from "../userPatches";
import type { UserPatch } from "../userPatches";
import { withAlgorithm } from "../effects";
import type { FxAlgorithm } from "../artifacts/fxData";

/** Erro amigável + AÇÃO de recuperação (issue #20: nunca spinner eterno).
 *  Sem `export`: o contrato público do hook é a interface `Stage`, e ninguém
 *  precisa nomear esse tipo para ler `err` — o gate `check_deadcode` (#78) cobra
 *  o export que não tem consumidor. */
interface StageError {
  message: string;
  retry: () => void;
}

export interface Stage {
  pp: number;
  presetName: string;
  board: BoardView | null;
  /** Patch de usuário aberto no palco (null = banco de fábrica em uso). */
  openUserId: string | null;
  userPatches: UserPatch[];
  err: StageError | null;
  clearErr: () => void;
  openPreset: (target: number) => Promise<void>;
  stepPreset: (delta: 1 | -1) => void;
  openUserPatch: (patch: UserPatch, index: number) => void;
  saveUserPatch: (name: string) => void;
  deleteUserPatch: (id: string) => void;
  applyKnob: (slot: BoardSlot, pos: number, value: string) => void;
  onKnobReset: (slot: BoardSlot, pos: number) => void;
  onChangeEffect: (slot: BoardSlot, alg: FxAlgorithm) => void;
  onToggle: (slot: BoardSlot) => void;
}

export function useStage(onPresetChanged: () => void): Stage {
  // O callback do dono do modal (fechar a edição ampliada) vai para um REF,
  // não para as dependências do `openPreset`.
  //
  // Sem isso o hook tem uma armadilha mortal: se quem chama passar uma
  // arrow-function inline, a identidade dela muda a cada render, o
  // `openPreset` é recriado, o efeito de abertura inicial roda DE NOVO — e a
  // app abre o preset em loop, batendo no device sem parar. Não é hipótese:
  // o teste de hook com `() => {}` inline travou o event loop antes deste
  // ref. Um hook de biblioteca não pode exigir que o chamador lembre de
  // memoizar.
  const changedRef = useRef(onPresetChanged);
  changedRef.current = onPresetChanged;

  const [pp, setPp] = useState(0);
  const [presetName, setPresetName] = useState("…");
  /** Board REAL do preset (device_board): slots/knobs do dicionário. */
  const [board, setBoard] = useState<BoardView | null>(null);
  const [err, setErr] = useState<StageError | null>(null);
  /** Patches de USUÁRIO (#11): a escrita no device não tem canal USB ainda,
   *  então vivem no localStorage como PRÉVIA — abrir um deles repõe no palco
   *  exatamente a cadeia salva (mesmo shape de BoardView do device). */
  const [userPatches, setUserPatches] = useState<UserPatch[]>(loadUserPatches);
  const [openUserId, setOpenUserId] = useState<string | null>(null);

  // Abre um preset: select no device + leitura do board. O estado `pp`/nome
  // só muda DEPOIS do device confirmar. Falha vira banner com AÇÃO de retry.
  const openPreset = useCallback(async (target: number) => {
    let selected = false;
    try {
      await deviceSelectPreset(target);
      selected = true;
      const b = await deviceBoard(target);
      setPp(b.pp);
      setPresetName(b.name);
      setBoard(b);
      setOpenUserId(null); // banco de fábrica: nenhum patch de usuário aberto
      setErr(null);
      changedRef.current(); // o dono do modal fecha a edição ampliada
    } catch (e) {
      // O usuário vê a mensagem amigável; o detalhe técnico fica no console.
      // Select OK + board falho: o curso certo é reler (retry) — a mensagem
      // diz "não foi possível abrir", não "o preset não trocou".
      console.error("openPreset falhou:", e);
      setErr({
        message: selected ? MSG.errOpenPreset : MSG.errSelectPreset,
        retry: () => void openPreset(target),
      });
    }
  }, []);

  // Abre um patch de USUÁRIO: o palco passa a desenhar o snapshot salvo
  // (mesmo formato do device — o Stage não sabe a diferença). Sem erro de
  // device possível aqui: nada é escrito, é relido do armazenamento local.
  const openUserPatch = useCallback((patch: UserPatch, index: number) => {
    setBoard(boardOfUserPatch(patch, index));
    setPresetName(patch.name);
    setOpenUserId(patch.id);
    setErr(null);
    changedRef.current();
  }, []);

  // Salva o patch CORRENTE: a cadeia que está no palco agora (fábrica ou
  // usuário), com o nome digitado — sem nome, numeração do dono.
  const saveUserPatch = useCallback(
    (name: string) => {
      if (board == null) return;
      const label = name.trim() || MSG.userPatchDefaultName(userPatches.length);
      setUserPatches((list) => [...list, snapshotOf(board, label, list.length)]);
    },
    [board, userPatches.length],
  );

  // Exclui um patch de usuário. Se era o que estava no palco, volta para o
  // preset de fábrica confirmado: deixar um patch apagado no palco seria a UI
  // mentindo sobre o que existe.
  const deleteUserPatch = useCallback(
    (id: string) => {
      setUserPatches((list) => list.filter((p) => p.id !== id));
      if (openUserId === id) {
        setOpenUserId(null);
        void openPreset(pp);
      }
    },
    [openUserId, openPreset, pp],
  );

  // ◀ ▶ reproduzem a coluna do patch do app oficial: 0..98 em ciclo.
  const stepPreset = useCallback(
    (delta: 1 | -1) => {
      void openPreset((pp + delta + 99) % 99);
    },
    [pp, openPreset],
  );

  // Knob do pedal (Fase 2 — U-3): aplica LOCAL (o valor aparece na hora) e
  // manda o SET ao device (§13.11: `slot` do fio = posição 1..9, `ctrl` =
  // pos do dicionário, value f32). Switch/combox ainda não têm canal (o SET
  // é f32) — prévia local. Falha permanente vira banner com retry, como no
  // #20: nada de estado otimista silencioso.
  const applyKnob = useCallback((slot: BoardSlot, pos: number, value: string) => {
    setBoard((b) =>
      b == null
        ? b
        : {
            ...b,
            slots: b.slots.map((s) =>
              s.slot === slot.slot
                ? { ...s, knobs: s.knobs.map((k) => (k.pos === pos ? { ...k, value } : k)) }
                : s,
            ),
          },
    );
    const knob = slot.knobs.find((k) => k.pos === pos);
    if (knob?.kind !== "knob") return;
    const num = Number(value);
    if (!Number.isFinite(num)) return;
    void deviceSetParam(slot.slot + 1, slot.code, pos, num).catch((e: unknown) => {
      console.error("device_set_param falhou:", e);
      setErr({ message: MSG.errSetParam, retry: () => applyKnob(slot, pos, value) });
    });
  }, []);

  const onKnobReset = useCallback(
    (slot: BoardSlot, pos: number) => {
      const knob = slot.knobs.find((k) => k.pos === pos);
      if (knob?.default != null) applyKnob(slot, pos, knob.default);
    },
    [applyKnob],
  );

  // Troca de EFEITO dentro do slot (issue #19, "Effects List"): é PRÉVIA LOCAL
  // — o `change-effect` (`0x47` da família 0x4X) ainda não tem formato
  // validado no fio (BLOCKERS 10b; roteiro de captura em CAPTURE_PLAN
  // CAPTURA 5). O estado é o mesmo do palco, então o pedal troca na hora e os
  // controles voltam aos defaults do algoritmo novo (os valores do anterior
  // não valem para ele).
  const onChangeEffect = useCallback((slot: BoardSlot, alg: FxAlgorithm) => {
    setBoard((b) =>
      b == null
        ? b
        : { ...b, slots: b.slots.map((s) => (s.slot === slot.slot ? withAlgorithm(s, alg) : s)) },
    );
  }, []);

  // Footswitch: sem comando de toggle capturado no protocolo — alterna LOCAL
  // (LED verde/vermelho) até o fluxo do device existir.
  const onToggle = useCallback((slot: BoardSlot) => {
    setBoard((b) =>
      b == null
        ? b
        : {
            ...b,
            slots: b.slots.map((s) => (s.slot === slot.slot ? { ...s, state: !s.state } : s)),
          },
    );
  }, []);

  // abertura INICIAL: o device é a fonte da verdade do preset corrente
  useEffect(() => {
    void openPreset(0);
  }, [openPreset]);

  // patches de usuário persistem local (prévia) — o storage é a única fonte
  useEffect(() => {
    storeUserPatches(userPatches);
  }, [userPatches]);

  const clearErr = useCallback(() => setErr(null), []);

  return {
    pp,
    presetName,
    board,
    openUserId,
    userPatches,
    err,
    clearErr,
    openPreset,
    stepPreset,
    openUserPatch,
    saveUserPatch,
    deleteUserPatch,
    applyKnob,
    onKnobReset,
    onChangeEffect,
    onToggle,
  };
}
