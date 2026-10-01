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
 * O board ainda não renderiza pedais: knobs/toggle/set_param entram um
 * efeito por vez — até lá o palco mostra só os lugares da cadeia.
 *
 * Atalhos globais (doc na aba Help do Settings): Espaço = drum play/stop,
 * R = REC do looper, Esc = fecha o painel do topo (Settings → Drum → pushes).
 */
import { useCallback, useEffect, useRef, useState } from "react";
import type { DeviceInfo } from "./ipc/types";
import { deviceBoard, deviceInfo, deviceSelectPreset } from "./ipc/device";
import { useBoot } from "./hooks/useBoot";
import { BOOT_STAGE_LABEL } from "./hooks/useBoot";
import { usePushLog } from "./hooks/usePushLog";
import { useGlobalShortcuts } from "./hooks/useGlobalShortcuts";
import { TopBar } from "./components/TopBar";
import { LibraryPanel } from "./components/LibraryPanel";
import { EmptyBoard } from "./components/EmptyBoard";
import { SettingsModal, loadGeneral } from "./components/SettingsModal";
import type { GeneralSettings } from "./components/SettingsModal";
import { loadTuner, TUNER_KEY } from "./components/TunerPanel";
import type { TunerSettings } from "./components/TunerPanel";
import type { DrumState } from "./components/DrumPanel";
import { LooperPanel, loadLooper } from "./components/LooperPanel";
import type { LooperSettings } from "./components/LooperPanel";
import { PushLog } from "./components/PushLog";
import { MSG } from "./i18n/messages";

const DRUM_KEY = "gp100.drum.v2";
const MASTER_KEY = "gp100.master.v1";

function loadDrum(): DrumState {
  try {
    const raw = localStorage.getItem(DRUM_KEY);
    if (raw) return JSON.parse(raw) as DrumState;
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
  const [ppTypeName, setPpTypeName] = useState("…");
  const [masterVol, setMasterVol] = useState(() => {
    try {
      const raw = localStorage.getItem(MASTER_KEY);
      return raw ? (Number(raw) || 99) : 99;
    } catch {
      return 99;
    }
  });
  const [drum, setDrum] = useState<DrumState>(loadDrum);
  const [general, setGeneral] = useState<GeneralSettings>(loadGeneral);
  const [looper, setLooper] = useState<LooperSettings>(loadLooper);
  const [arrangeMode, setArrangeMode] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [celebrate, setCelebrate] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [drumOpen, setDrumOpen] = useState(false); // popover do drum (subiu do TopBar p/ precedência do Esc)
  const [recRequest, setRecRequest] = useState(0); // pulso do atalho R (looper)
  const [tuner, setTuner] = useState<TunerSettings>(loadTuner);
  const [, setLooperPlaying] = useState(false); /* looper reporta tocar/gravar */
  const pushRef = useRef<HTMLDetailsElement>(null); // drawer "pushes do device" (Esc fecha)

  // info + biblioteca no mount
  useEffect(() => {
    void deviceInfo()
      .then(setInfo)
      .catch(() => setInfo(null));
    // Biblioteca de fábrica = artefato gerado do all.prst (presetData.ts).
    // `device_preset_library` volta a alimentar esta lista na integração real.
  }, []);

  // abre um preset (select real no device; fallback local do device.ts)
  const openPreset = useCallback(async (target: number) => {
    try {
      await deviceSelectPreset(target);
      const b = await deviceBoard(target);
      setPp(b.pp);
      setPresetName(b.name);
      setPpTypeName(b.ppTypeName);
      setErr(null);
    } catch (e) {
      // Usuário vê mensagem amigável; o detalhe técnico fica no console.
      console.error("openPreset falhou:", e);
      setErr(MSG.errOpenPreset);
    }
  }, []);

  // ◀ ▶ reproduzem a coluna do patch do app oficial: 0..98 em ciclo.
  // Efeito FORA do updater (updater tem que ser puro — StrictMode chama 2×).
  const stepPreset = useCallback(
    (delta: 1 | -1) => {
      setPp((cur) => (cur + delta + 99) % 99);
    },
    [],
  );
  useEffect(() => {
    void openPreset(pp);
  }, [pp, openPreset]);

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
      if (!settingsOpen) setDrum((d) => ({ ...d, on: !d.on }));
    },
    onLooperRec: () => {
      if (!settingsOpen) setRecRequest((n) => n + 1);
    },
    onEscape: () => {
      if (settingsOpen) setSettingsOpen(false);
      else if (drumOpen) setDrumOpen(false);
      else pushRef.current?.removeAttribute("open");
    },
  });

  const connected = info != null;


  // faixa de progresso só durante o boot (não ocupa layout permanente);
  // stage pode vir de um beat pendente após o fim — congelar em 100%/fim
  const stageNow = booting ? boot.stage : null;

  // viewport única: navbar → meio (scroll próprio) → rodapé fixos;
  // só o MEIO rola (a página inteira nunca cresce — layout de "mesa")
  return (
    <main
      style={{
        height: "100vh",
        background: "var(--bg)",
        color: "var(--text)",
        padding: "var(--space-8) var(--space-20) var(--space-12)",
        display: "grid",
        gridTemplateRows: "auto 1fr auto",
        gap: "var(--space-12)",
        overflow: "hidden",
      }}
    >
      <TopBar
        connected={connected}
        mock={info?.backend === "mock"}
        presetLabel={`P${String(pp + 1).padStart(2, "0")} ${presetName}`}
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

      {/* boot/erros: faixas transitórias FORA do scroll (sempre visíveis) */}

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
        <div role="alert" style={{ background: "#2a1414", border: "1px solid #5b2626", color: "#ffb3b3", padding: "8px 12px", borderRadius: 8, fontSize: 13 }}>
          {err}
        </div>
      )}      {/* MEIO da mesa (única área que rola): looper no topo, pushes,
          biblioteca (300px, lista rolável) à esquerda e palco à direita */}
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

        <details ref={pushRef} style={{ border: "1px solid #232932", borderRadius: 10, background: "var(--bg-raised)" }}>
          <summary style={{ cursor: "pointer", padding: "8px 12px", fontSize: 12, color: "var(--text-muted)", fontFamily: "var(--font-mono)" }}>
            {`${MSG.pushSummary} (${log.length})`}
          </summary>
          <PushLog log={log} onClear={clear} />
        </details>

        {/* linha biblioteca ↔ pedalboard: alturas iguais (stretch), sem lacunas */}
        <div className="shell-main">
          <LibraryPanel currentPp={pp} onSelect={(target) => void openPreset(target)} />
          <EmptyBoard
            pp={pp}
            presetName={presetName}
            ppTypeName={ppTypeName}
            celebrate={celebrate}
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
            onReorder={() => {
              /* a reordenação real chega quando os pedais forem renderizados */
            }}
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
    </main>
  );
}
