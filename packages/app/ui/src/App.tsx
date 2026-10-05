/**
 * App — casca do editor em VIEWPORT ÚNICA: navbar (logo/conexão/boot/
 * patch/DRUM/Master+kill) → meio com scroll próprio (looper, pushes,
 * biblioteca + palco) → rodapé da pedaleira (IN · marca · ⚙ · OUT).
 * A biblioteca de FÁBRICA é real (99 presets do all.prst; abrir = select
 * no device), o board mostra os 9 lugares (display LED + navbar com o
 * patch) e o modal Settings persiste local (badge "prévia local").
 * O boot vive INTEGRADO à navbar: progresso e erro aparecem só durante
 * o boot/falha (a seção de conexão permanente foi removida: duplicava a
 * navbar). A trava ⇄ mover pertence ao PALCO (EmptyBoard); o rodapé
 * é o chassi da pedaleira (IN · GP · OUT) — ações globais na navbar.
 *
 * O board é REAL (`device_board`): o palco desenha os 9 PEDAIS REAIS da
 * cadeia numa fileira (visão completa; o modelo de cada efeito vem do
 * catálogo fxModels). Knob numérico vira `device_set_param`;
 * toggle e switch/combox ainda são prévia local (sem comando no protocolo).
 *
 * Atalhos globais (doc na aba Help do Settings): Espaço = drum play/stop,
 * R = REC do looper, Esc = fecha o painel do topo (Settings → Drum → pushes).
 *
 * ── POR QUE ESTE ARQUIVO É FINO (#82) ─────────────────────────────────────
 * A #82 mediu 514 linhas aqui e 1027 no teste que o cobre — o sintoma de um
 * componente que passou a hora de virar hook. O corte não foi "arquivo
 * pequeno", foi **coesão**:
 *
 *   - `useStage` — o que está no palco e as operações sobre isso (abrir
 *     preset/patch, knob, efeito, footswitch, o banner de erro com retry).
 *     Patch de usuário e preset de fábrica são o mesmo `BoardView`, então as
 *     operações dos dois vivem juntas.
 *   - `usePrefs` — o que sobrevive ao fechar a janela (master, drum, tuner,
 *     looper, settings/idioma), todo com persistência local.
 *
 * O que fica AQUI é o que só o App tem: o boot, o log de pushes, os atalhos,
 * quais modais estão abertos e o JSX. `docs/ARCHITECTURE.md` tem o mapa.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import type { DeviceInfo } from "./ipc/types";
import { deviceInfo } from "./ipc/device";
import { useBoot, BOOT_STAGE_LABEL } from "./hooks/useBoot";
import { usePushLog } from "./hooks/usePushLog";
import { useGlobalShortcuts } from "./hooks/useGlobalShortcuts";
import { useStage } from "./hooks/useStage";
import { useTones } from "./hooks/useTones";
import { useIrs } from "./hooks/useIrs";
import { useContentMenu } from "./hooks/useContentMenu";
import { usePrefs } from "./hooks/usePrefs";
import { TopBar } from "./components/TopBar";
import { BootProgressBar } from "./components/BootProgressBar";
import { ErrorBanner } from "./components/ErrorBanner";
import { LibraryPanel } from "./components/LibraryPanel";
import { Stage } from "./components/Stage";
import { SettingsModal } from "./components/SettingsModal";
import { PedalModal } from "./components/PedalModal";
import { LooperPanel } from "./components/LooperPanel";
import { SnapTonePanel } from "./components/SnapTonePanel";
import { IrLabPanel } from "./components/IrLabPanel";
import { PushLog } from "./components/PushLog";
import { FieldDiagPanel } from "./components/FieldDiagPanel";
import { MSG } from "./i18n/messages";

export default function App() {
  const boot = useBoot();
  const { log, clear } = usePushLog();
  const prefs = usePrefs();
  const { masterVol, setMasterVol, drum, setDrum, tuner, setTuner, looper, setLooper, general, onChangeGeneral } = prefs;

  const [info, setInfo] = useState<DeviceInfo | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [drumOpen, setDrumOpen] = useState(false); // modal do drum (subiu do TopBar p/ precedência do Esc)
  const [recRequest, setRecRequest] = useState(0); // pulso do atalho R (looper)
  const [arrangeMode, setArrangeMode] = useState(false);
  const [celebrate, setCelebrate] = useState(false);
  /** Slot da cadeia em edição ampliada (PedalModal; null = fechado). */
  const [editing, setEditing] = useState<number | null>(null);
  const pushRef = useRef<HTMLDetailsElement>(null); // drawer "pushes do device" (Esc fecha)

  // O palco é abre-e-fecha: trocar de preset/patch fecha a edição ampliada.
  // O callback precisa ser ESTÁVEL (o `openPreset` é dependência de efeito),
  // por isso é um useCallback vazio de dependências.
  const onPresetChanged = useCallback(() => setEditing(null), []);
  const stage = useStage(onPresetChanged);
  // O gestor de tons (#25) tem o estado no hook inteiro: a lista, o A/B e o
  // envio em andamento vivem la porque sao coisas que sobrevivem ao fechar e
  // reabrir a tela — perder o "enviando…" ao clicar fora mandaria um segundo
  // stream para o aparelho.
  const tones = useTones();
  // O laboratorio de IRs (#24) tem o mesmo dono-do-estado: a lista do dono, a
  // tabela que o APARELHO relata e o envio em andamento (que pode levar
  // minutos) vivem no hook porque precisam sobreviver ao fechar e reabrir a
  // tela — perder o "enviando…" mandaria um segundo stream para o aparelho.
  const irs = useIrs();
  // A PORTA do conteudo (#24/#25) fica no hook porque e ela que decide qual das
  // duas telas abre: um botao na navbar estouraria o overflow em 1280.
  const conteudo = useContentMenu({ tones, irs });
  const {
    pp,
    presetName,
    board,
    openUserId,
    lib,
    err,
    openPreset,
    stepPreset,
    openUserPatch,
    saveUserPatch,
    deleteUserPatch,
    applyKnob,
    onKnobReset,
    onChangeEffect,
    onToggle,
  } = stage;

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
          <BootProgressBar
            progress={boot.progress}
            stageLabel={stageNow ? BOOT_STAGE_LABEL[stageNow] : MSG.bootStarting}
          />
        )}
        {boot.state.kind === "error" && (
          <div role="alert" style={{ background: "color-mix(in srgb, var(--error) 12%, transparent)", border: "1px solid color-mix(in srgb, var(--error) 45%, transparent)", color: "var(--error)", borderRadius: "var(--space-8)", padding: "var(--space-8) var(--space-12)", fontSize: "var(--text-sm)" }}>
            {MSG.connBootError}
          </div>
        )}
        {err != null && <ErrorBanner message={err.message} onRetry={err.retry} />}
      </div>

      {/* MEIO da mesa: looper no topo, pushes, biblioteca (248px, com a
          LISTA rolável por dentro) à esquerda e palco à direita — o meio
          nunca tem scroll próprio: quem rola é a página inteira */}
      <div className="shell-content">
        <LooperPanel
          settings={looper}
          recRequest={recRequest}
          onChange={setLooper}
          onPlayingChange={() => { /* looper reporta tocar/gravar */ }}
        />

        <details ref={pushRef} className="gp-surface gp-surface--flat" style={{ overflow: "hidden" }}>
          <summary style={{ cursor: "pointer", padding: "8px 12px", fontSize: 12, color: "var(--text-muted)", fontFamily: "var(--font-mono)" }}>
            {`${MSG.pushSummary} (${log.length})`}
          </summary>
          <PushLog log={log} onClear={clear} />
        </details>
        <FieldDiagPanel info={info} />

        {/* linha biblioteca ↔ pedalboard: alturas iguais (stretch), sem lacunas */}
        <div className="shell-main">
          <LibraryPanel
            currentPp={pp}
            bankDoPalco={board?.bank ?? "factory"}
            lib={lib}
            currentUserId={openUserId}
            onOpenFactory={(target) => void openPreset(target)}
            onOpenUser={(id, index) => void openUserPatch(id, index)}
            onSave={saveUserPatch}
            onDelete={deleteUserPatch}
            onOpenContent={conteudo.abrir}
          />
          <Stage
            board={board}
            celebrate={celebrate}
            engineer={general.engineerMode}
            arrangeMode={arrangeMode}
            onToggleArrange={() => setArrangeMode((v) => !v)}
            tuner={tuner}
            onTunerChange={setTuner}
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
      {tones.aberto && <SnapTonePanel tones={tones} />}
      {irs.aberto && <IrLabPanel irs={irs} />}
      {conteudo.node}
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
