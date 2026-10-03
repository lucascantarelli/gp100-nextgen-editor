/**
 * App — casca do editor em VIEWPORT ÚNICA: navbar (logo/conexão/boot/
 * patch/DRUM/Master+kill) → meio com scroll próprio (looper, pushes,
 * biblioteca + palco) → rodapé da pedaleira (IN · marca · ⚙ · OUT).
 * A biblioteca de FÁBRICA é real (99 presets do all.prst; abrir = select
 * no device), o board mostra os 9 lugares (display LED + navbar com o
 * patch) e o modal Settings persiste local (badge "prévia local").
 * O boot vive INTEGRADO à navbar: progresso e erro aparecem só durante
 * o boot/falha (a seção de conexão permanente foi removida: duplicava
 * a navbar). A trava ⇄ mover pertence ao PALCO (EmptyBoard); o rodapé
 * é o chassi da pedaleira (IN · GP · OUT) — ações globais na navbar.
 *
 * O board é REAL (`device_board`): o palco desenha os 9 PEDAIS REAIS da
 * cadeia numa fileira (visão completa; o modelo de cada efeito vem do
 * catálogo fxModels). Knob numérico vira `device_set_param`;
 * toggle e switch/combox ainda são prévia local (sem comando no protocolo).
 *
 * Atalhos globais (doc na aba Help do Settings): Espaço = drum play/stop,
 * R = REC do looper, Esc = fecha o painel do topo (Settings → Drum → pushes).
 */
import { useCallback, useEffect, useRef, useState } from "react";
import type { BoardSlot, BoardView, DeviceInfo } from "./ipc/types";
import { deviceBoard, deviceInfo, deviceSelectPreset, deviceSetParam } from "./ipc/device";
import { useBoot } from "./hooks/useBoot";
import { BOOT_STAGE_LABEL } from "./hooks/useBoot";
import { usePushLog } from "./hooks/usePushLog";
import { useGlobalShortcuts } from "./hooks/useGlobalShortcuts";
import { TopBar } from "./components/TopBar";
import { LibraryPanel } from "./components/LibraryPanel";
import { Stage } from "./components/Stage";
import { SettingsModal, loadGeneral } from "./components/SettingsModal";
import { PedalModal } from "./components/PedalModal";
import type { GeneralSettings } from "./components/SettingsModal";
import { loadTuner, TUNER_KEY } from "./components/TunerPanel";
import type { TunerSettings } from "./components/TunerPanel";
import type { DrumState } from "./components/DrumPanel";
import { LooperPanel, loadLooper } from "./components/LooperPanel";
import type { LooperSettings } from "./components/LooperPanel";
import { PushLog } from "./components/PushLog";
import { MSG, setLanguage, useLanguage } from "./i18n/messages";
import { boardOfUserPatch, loadUserPatches, snapshotOf, storeUserPatches } from "./userPatches";
import type { UserPatch } from "./userPatches";
import { withAlgorithm } from "./effects";
import type { FxAlgorithm } from "./artifacts/fxData";

const DRUM_KEY = "gp100.drum.v2";
const MASTER_KEY = "gp100.master.v1";

/** Ação de recuperação do banner de erro (alvo ≥32px — a11y). */
const retryBtn: React.CSSProperties = {
  minHeight: 32,
  padding: "var(--space-4) var(--space-12)",
  borderRadius: "var(--space-4)",
  border: "1px solid currentColor",
  background: "transparent",
  color: "inherit",
  cursor: "pointer",
  font: "inherit",
  whiteSpace: "nowrap",
};

function loadDrum(): DrumState {
  try {
    const raw = localStorage.getItem(DRUM_KEY);
    // o drum NUNCA volta tocando: restaura os parâmetros (gênero, estilo,
    // BPM, compasso, volume, speed) mas play/stop inicia DESLIGADO — o
    // device não parte em execução nem o navegador herda reprodução
    if (raw) return { ...(JSON.parse(raw) as DrumState), on: false };
  } catch {
    /* teste: sem localStorage */
  }
  return { on: false, genre: "Rock", style: "Rock 1", bpm: 120, beat: "4/4", volume: 80, speed: 50 };
}

export default function App() {
  const boot = useBoot();
  const { log, clear } = usePushLog();
  const [info, setInfo] = useState<DeviceInfo | null>(null);
  const [pp, setPp] = useState(0);
  const [presetName, setPresetName] = useState("…");
  /** Board REAL do preset (device_board): slots/knobs do dicionário. */
  const [board, setBoard] = useState<BoardView | null>(null);
  const [masterVol, setMasterVol] = useState(() => {
    try {
      const raw = localStorage.getItem(MASTER_KEY);
      return raw ? (Number(raw) || 99) : 99;
    } catch {
      return 99;
    }
  });
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
  const [looper, setLooper] = useState<LooperSettings>(loadLooper);
  const [arrangeMode, setArrangeMode] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  /** Slot da cadeia em edição ampliada (PedalModal; null = fechado). */
  const [editing, setEditing] = useState<number | null>(null);
  const [celebrate, setCelebrate] = useState(false);
  /** Erro amigável + AÇÃO de recuperação (issue #20: nunca spinner eterno). */
  const [err, setErr] = useState<{ message: string; retry: () => void } | null>(null);
  const [drumOpen, setDrumOpen] = useState(false); // modal do drum (subiu do TopBar p/ precedência do Esc)
  const [recRequest, setRecRequest] = useState(0); // pulso do atalho R (looper)
  const [tuner, setTuner] = useState<TunerSettings>(loadTuner);
  const [, setLooperPlaying] = useState(false); /* looper reporta tocar/gravar */
  const pushRef = useRef<HTMLDetailsElement>(null); // drawer "pushes do device" (Esc fecha)
  /** Patches de USUÁRIO (#11): a escrita no device não tem canal USB ainda,
   *  então vivem no localStorage como PRÉVIA — abrir um deles repõe no palco
   *  exatamente a cadeia salva (mesmo shape de BoardView do device). */
  const [userPatches, setUserPatches] = useState<UserPatch[]>(loadUserPatches);
  /** patch de usuário aberto no palco (null = banco de fábrica em uso) */
  const [openUserId, setOpenUserId] = useState<string | null>(null);

  // info + biblioteca no mount
  useEffect(() => {
    void deviceInfo()
      .then(setInfo)
      .catch(() => setInfo(null));
    // Biblioteca de fábrica = artefato gerado do all.prst (presetData.ts).
    // `device_preset_library` volta a alimentar esta lista na integração real.
  }, []);

  // Boot FALHOU = o device não respondeu: a UI não pode continuar exibindo
  // "on" (LED mentiroso). Boot OK → re-lê o device: a recuperação pelo ⟳
  // devolve a conexão ao estado real (idem após um disconnect mid-boot).
  useEffect(() => {
    if (boot.state.kind === "error") {
      setInfo(null);
      return;
    }
    if (boot.state.kind === "ready") {
      void deviceInfo()
        .then(setInfo)
        .catch(() => setInfo(null));
    }
  }, [boot.state.kind]);

  // abre um preset: select no device + leitura do board. O estado `pp`/nome
  // só muda DEPOIS do device confirmar — estado otimista nunca contamina a
  // UI (select falho deixaria navbar/LED/biblioteca mostrando um preset que
  // o device não aceitou). Falha vira banner com AÇÃO de retry: recuperação
  // visível, nunca spinner eterno (issue #20).
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
      setEditing(null); // trocar de preset fecha a edição ampliada
      setErr(null);
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
    setEditing(null);
    setErr(null);
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
  // não valem para ele). O que o dono salva depois (patch de usuário) já leva
  // o efeito novo junto.
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

  // LED display: flip só no boot MANUAL (re-escanear) — o auto-boot do
  // mount é silencioso (a navbar já mostra "on"; flip automático deixaria
  // o display dependente de timing no primeiro segundo da página)
  const booting = boot.state.kind === "loading";
  useEffect(() => {
    if (boot.state.kind === "ready" && boot.origin === "manual") {
      setCelebrate(true);
      const t = setTimeout(() => setCelebrate(false), 1000);
      return () => clearTimeout(t);
    }
  }, [boot.state.kind, boot.origin]);

  // persistência local (prévia) — drum/master/settings
  useEffect(() => {
    try {
      localStorage.setItem(DRUM_KEY, JSON.stringify(drum));
    } catch {
      /* noop */
    }
  }, [drum]);
  useEffect(() => {
    try {
      localStorage.setItem(MASTER_KEY, String(masterVol));
    } catch {
      /* noop */
    }
  }, [masterVol]);
  // patches de usuário persistem local (prévia) — o storage é a única fonte
  useEffect(() => {
    storeUserPatches(userPatches);
  }, [userPatches]);
  const onChangeGeneral = useCallback((s: GeneralSettings) => {
    setGeneral(s);
    try {
      localStorage.setItem("gp100.settings.general.v1", JSON.stringify(s));
    } catch {
      /* noop */
    }
  }, []);

  // Atalhos globais (doc: aba Help do Settings) — Espaço = drum, R = REC do
  // looper, Esc fecha o painel do topo. Transporte fica inerte com o modal
  // aberto (Esc fecha o modal antes).
  useGlobalShortcuts({
    onDrumToggle: () => {
      if (!settingsOpen && editing == null) setDrum((d) => ({ ...d, on: !d.on }));
    },
    onLooperRec: () => {
      if (!settingsOpen && editing == null) setRecRequest((n) => n + 1);
    },
    onEscape: () => {
      // precedência do painel do TOPO: edição do pedal → settings → drum → pushes
      if (editing != null) setEditing(null);
      else if (settingsOpen) setSettingsOpen(false);
      else if (drumOpen) setDrumOpen(false);
      else pushRef.current?.removeAttribute("open");
    },
  });

  const connected = info != null;


  // faixa de progresso só durante o boot (não ocupa layout permanente);
  // stage pode vir de um beat pendente após o fim — congelar em 100%/fim
  const stageNow = booting ? boot.stage : null;

  // página FLUIDA: navbar → conteúdo → rodapé num só fluxo; a rolagem é a
  // natural da JANELA (nada de scroll interno no meio da página). O
  // min-height + a linha 1fr mantêm o rodapé no fundo quando sobra espaço.
  return (
    <main
      style={{
        minHeight: "100vh",
        background: "var(--bg)",
        color: "var(--text)",
        padding: "var(--space-8) var(--space-20) var(--space-12)",
        display: "grid",
        gridTemplateRows: "auto 1fr auto",
        gap: "var(--space-12)",
      }}
    >
      {/* topo da mesa: navbar + faixas transitórias de boot/erro num bloco
          só — a grade da página tem 3 filhos exatos (topo · meio · rodapé) */}
      <div style={{ display: "grid", gap: "var(--space-12)" }}>
        <TopBar
          connected={connected}
          mock={info?.backend === "mock"}
          presetLabel={`${board?.ppLabel ?? MSG.libPp(pp)} ${presetName}`}
          masterVol={masterVol}
          drum={drum}
          drumOpen={drumOpen}
          booting={booting}
          onBoot={() => boot.startBoot("manual")}
          settingsOpen={settingsOpen}
          onOpenSettings={() => setSettingsOpen(true)}
          onMasterVol={setMasterVol}
          onDrum={setDrum}
          onDrumOpenChange={setDrumOpen}
          onPrevPatch={() => stepPreset(-1)}
          onNextPatch={() => stepPreset(1)}
        />

        {/* boot/erros: faixas transitórias coladas no topo (sempre visíveis) */}

        {booting && boot.progress !== null && (
          <div role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={boot.progress} aria-label={MSG.bootProgressAria} style={{ position: "relative", height: 18, borderRadius: 9, overflow: "hidden", background: "color-mix(in srgb, var(--text-muted) 18%, transparent)" }}>
            <div style={{ position: "absolute", inset: 0, width: `${boot.progress}%`, background: "linear-gradient(90deg, color-mix(in srgb, var(--accent) 65%, transparent), var(--accent))", transition: "width var(--motion-fast) var(--ease-out)" }} />
            <span style={{ position: "absolute", inset: 0, display: "flex", alignItems: "center", justifyContent: "center", fontSize: "var(--text-xs)", color: "var(--text)", textShadow: "0 1px 2px rgba(0,0,0,.6)" }}>
              {stageNow ? BOOT_STAGE_LABEL[stageNow] : MSG.bootStarting} · {boot.progress}%
            </span>
          </div>
        )}
        {boot.state.kind === "error" && (
          <div role="alert" style={{ background: "color-mix(in srgb, var(--error) 12%, transparent)", border: "1px solid color-mix(in srgb, var(--error) 45%, transparent)", color: "var(--error)", borderRadius: "var(--space-8)", padding: "var(--space-8) var(--space-12)", fontSize: "var(--text-sm)" }}>
            {MSG.connBootError}
          </div>
        )}
        {err != null && (
          <div
            role="alert"
            style={{
              background: "#2a1414",
              border: "1px solid #5b2626",
              color: "#ffb3b3",
              padding: "8px 12px",
              borderRadius: 8,
              fontSize: 13,
              display: "flex",
              alignItems: "center",
              gap: "var(--space-12)",
            }}
          >
            <span>{err.message}</span>
            <button type="button" onClick={err.retry} aria-label={MSG.errRetryAria} style={retryBtn}>
              {MSG.errRetry}
            </button>
          </div>
        )}
      </div>

      {/* MEIO da mesa: looper no topo, pushes, biblioteca (248px, com a
          LISTA rolável por dentro) à esquerda e palco à direita — o meio
          nunca tem scroll próprio: quem rola é a página inteira */}
      <div className="shell-content">
        <LooperPanel
          settings={looper}
          recRequest={recRequest}
          onChange={(s) => {
            setLooper(s);
            try {
              localStorage.setItem("gp100.looper.v1", JSON.stringify(s));
            } catch {
              /* noop */
            }
          }}
          onPlayingChange={setLooperPlaying}
        />

        <details ref={pushRef} className="gp-surface gp-surface--flat" style={{ overflow: "hidden" }}>
          <summary style={{ cursor: "pointer", padding: "8px 12px", fontSize: 12, color: "var(--text-muted)", fontFamily: "var(--font-mono)" }}>
            {`${MSG.pushSummary} (${log.length})`}
          </summary>
          <PushLog log={log} onClear={clear} />
        </details>

        {/* linha biblioteca ↔ pedalboard: alturas iguais (stretch), sem lacunas */}
        <div className="shell-main">
          <LibraryPanel
            currentPp={pp}
            bank={board?.bank ?? "factory"}
            userPatches={userPatches}
            currentUserId={openUserId}
            onSelect={(target) => void openPreset(target)}
            onOpenUser={openUserPatch}
            onSave={saveUserPatch}
            onDelete={deleteUserPatch}
          />
          <Stage
            board={board}
            celebrate={celebrate}
            engineer={general.engineerMode}
            arrangeMode={arrangeMode}
            onToggleArrange={() => setArrangeMode((v) => !v)}
            tuner={tuner}
            onTunerChange={(s) => {
              setTuner(s);
              try {
                localStorage.setItem(TUNER_KEY, JSON.stringify(s));
              } catch {
                /* noop */
              }
            }}
            onToggle={onToggle}
            onKnobChange={applyKnob}
            onKnobReset={onKnobReset}
            onEdit={(s) => setEditing(s.slot)}
          />
        </div>
      </div>

      {/* rodapé da pedaleira: ENTRADA · GP · SAÍDA (o chassi fecha a página;
          ⚙ voltou para a navbar — o drawer do drum cobre o rodapé) */}
      <footer className="page-footer" role="contentinfo">
        <span className="pg-jack" aria-hidden="true">{MSG.stageIn}</span>
        <span className="pg-mark" aria-hidden="true">
          {MSG.brand}
          <span className="pg-tagline">{MSG.tagline}</span>
        </span>
        <span className="pg-jack" aria-hidden="true">{MSG.stageOut}</span>
      </footer>

      <SettingsModal open={settingsOpen} general={general} onChangeGeneral={onChangeGeneral} onClose={() => setSettingsOpen(false)} />
      <PedalModal
        slot={editing == null ? null : (board?.slots.find((s) => s.slot === editing) ?? null)}
        engineer={general.engineerMode}
        onToggle={onToggle}
        onKnobChange={applyKnob}
        onKnobReset={onKnobReset}
        onChangeEffect={onChangeEffect}
        onClose={() => setEditing(null)}
      />
    </main>
  );
}
