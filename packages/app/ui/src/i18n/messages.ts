/**
 * i18n/messages — FONTE ÚNICA dos textos de usuário da casca. Regras:
 *  - todo texto NOVO de usuário entra aqui (nada de string solta no JSX);
 *    o lint custom `local/no-user-literals` (eslint.config.js) trava JSXText
 *    e atributos title/placeholder/aria-label/label com literal;
 *  - proibido vocabulário interno (issues, fases, protocolo, capturas, nomes de
 *    doc) — o usuário não precisa do nosso roadmap;
 *  - navbar consolidada: conexão, boot, patch e drums vivem no banner; não
 *    existe seção de conexão duplicada na página;
 *  - estado do device honesto: "prévia local" quando a escrita ainda não
 *    chega ao hardware; desabilitado quando o canal de escrita não existe;
 *  - estrutura plana e tipada: pronta para virar dicionário por idioma
 *    (pt-BR é a base; APP Language do Settings já prevê en/es/zh).
 */
import { FACTORY_PRESETS } from "../artifacts/presetData";
import { DRUM_BEATS, DRUM_GENRES } from "../artifacts/drumData";
import { FX_MODULES } from "../artifacts/fxData";
import { version as REACT_VERSION } from "react";

/* Números exibidos ao usuário derivados dos ARTEFATOS gerados (nunca
 * transcritos à mão — quando o dicionário regenera, a UI acompanha). */
const PRESET_COUNT = FACTORY_PRESETS.length;
const DRUM_COUNT = DRUM_GENRES.reduce((n, g) => n + g.styles.length, 0);
const DRUM_GENRE_COUNT = DRUM_GENRES.length;
const DRUM_BEAT_RANGE = `${DRUM_BEATS[0]}…${DRUM_BEATS[DRUM_BEATS.length - 1]}`;
const FX_COUNT = Object.values(FX_MODULES).reduce((n, a) => n + a.length, 0);
const CTRL_COUNT = Object.values(FX_MODULES).reduce(
  (n, a) => n + a.reduce((m, alg) => m + alg.knobs.length + alg.switches.length + alg.comboxes.length, 0),
  0,
);

export const MSG = {
  /* ── marca / topbar ── */
  brand: "GP-100",
  brandMono: "GP",
  tagline: "editor não-oficial",
  connShortOn: "on",
  connShortOff: "off",
  mockBadge: "Mock Device",
  patchLabel: "patch",
  patchPrevAria: "Patch anterior",
  patchPrevTitle: "Patch anterior (P01–P99 em ciclo)",
  patchNextAria: "Próximo patch",
  patchNextTitle: "Próximo patch (P01–P99 em ciclo)",
  drumChipOn: (style: string) => `⏹ drum · ${style}`,
  drumChipOff: (style: string) => `⏵ drum · ${style}`,
  drumChipAria: (style: string, bpm: number, beat: string) =>
    `Bateria (drum): ${style}, ${bpm} BPM, compasso ${beat} — abrir gestão de ritmos`,
  /** info do drum EMPILHADA na navbar (BPM sobre compasso) — ocupa a
   *  metade da largura na horizontal e a navbar segue numa linha só */
  drumInfo: (bpm: number, beat: string) => `${bpm}\n${beat}`,
  masterLabel: "master",
  masterAria: "Master volume",
  arrangeLabel: "🔒",
  arrangeLabelOpen: "🔓",
  arrangeAria: "Trava de mover pedais — arrastar e soltar só quando destravado",
  arrangeTitle: "Destravar para arrastar pedais (protege o ajuste dos knobs)",
  killLabel: "⭘ kill",
  killLabelOn: "⏻ killed",
  killAria: "Kill switch (desligar master, bateria e looper)",
  killTitle: "Mute global: desliga o master e a bateria — clique de novo para restaurar",
  openSettingsAria: "Abrir configurações",

  /* ── navbar (cluster de conexão: status curto + badge + boot) ── */
  connClusterAria: "Conexão e boot",

  /* ── afinador do palco (ocupa o lugar do antigo VU; manual V1.8 p.5) ── */
  tunerAria: "Afinador",
  tunerPowerAria: "Ligar ou desligar a monitoração de afinação",
  /** botão VISUAL do monitor (sem texto): ícone ♪ + LED verde/vermelho */
  tunerPowerIcon: "♪",
  tunerPowerTitle: (on: boolean) =>
    on
      ? "Monitor de afinação LIGADO — mostra o que você toca em tempo real (no device: segure os 2 footswitches)"
      : "Monitor de afinação desligado — clique para acompanhar a afinação em tempo real (no device: segure os 2 footswitches)",
  tunerModeAria: "Modo do afinador — alternar entre bypass, thru e mute",
  tunerModeTitle: "O que o device faz com o sinal enquanto você afina: bypass (seco), thru (com efeito) ou mute (silencioso)",
  tunerModeLabel: (mode: "bypass" | "thru" | "mute") => mode,
  tunerRefLabel: "ref",
  tunerRefAria: "Pitch de referência (A4) em hertz, de 435 a 445",
  tunerRefTitle: "REF PITCH: 435–445 Hz (padrão 440 Hz)",
  tunerRefValue: (hz: number) => `${hz}Hz`,
  tunerDemoLabel: "▶ demo",
  tunerDemoRunning: "■ demo",
  tunerDemoAria: "Tocar demonstração do afinador com tom sintético",
  tunerDemoTitle: "Amostra sintética varrendo ±30 cents em A2 pelo motor real do afinador — não é áudio do device",
  tunerIdleNote: "—",
  tunerFlatMark: "♭",
  tunerSharpMark: "♯",

  /* ── navbar (boot = re-escanear; o app já detecta o device sozinho) ── */
  rescanAria: "Reescanear device",
  rescanTitle: "Reescanear (boot): o app detecta o device sozinho ao abrir — use para reescanear",
  bootProgressAria: "Progresso do boot",
  bootStarting: "Iniciando",
  bootStages: {
    tables: "Tabelas de IR",
    scan: "Scan de presets",
    probe: "Sonda banco 02",
    setlist: "Setlist",
    names: "Nomes",
    keepalive: "Keepalive",
  },
  connBootError: "Falha no boot do device — verifique a conexão e tente novamente.",

  /* ── board ── */
  boardAria: "Pedalboard (9 lugares da cadeia)",
  slotAria: (n: number, fam: string) => `Slot ${n}: ${fam}`,
  slotEmpty: "vazio",
  slotArrange: "arraste ⇄",
  stageFooter: "9 slots · ordem do sinal →",
  stageIn: "⏻ IN",
  stageOut: "OUT ⏻",

  /* ── biblioteca ── */
  libAria: "Biblioteca de presets",
  libTabsAria: "Tipo de patch",
  libTabFactory: "Factory Patch",
  libTabUser: "User Patch",
  libListAria: "Presets de fábrica",
  searchPlaceholder: "Buscar nome, nº ou estilo…",
  searchAria: "Buscar preset por nome, número ou estilo",
  searchEmpty: (q: string) => `Nenhum preset para “${q}”. Dica: busque por estilo (Rock, Funk…) ou nº.`,
  libRowTitle: (name: string, type: string) => `Abrir “${name}” (${type})`,
  libPp: (pp: number) => `P${String(pp + 1).padStart(2, "0")}`,
  userPatchEmpty: "Ainda não há patches de usuário nesta versão.",

  /* ── drum ── */
  drumNote: `${DRUM_COUNT} ritmos em ${DRUM_GENRE_COUNT} gêneros · metrônomo incluído · prévia local`,
  drumPanelAria: "Gestão de ritmos da bateria (drum)",
  drumTitle: "Bateria · ritmos",
  drumPlayLabel: "⏵ tocar",
  drumStopLabel: "⏹ parar",
  drumCloseAria: "Fechar gestão de ritmos",
  drumGenreLabel: "Gênero",
  drumStyleLabel: "Ritmo",
  drumBpmLabel: "BPM",
  drumBeatLabel: "Compasso",
  drumVolLabel: "Volume",
  drumSpeedLabel: "Speed",

  /* ── looper ── */
  looperPlate: "GP-100 · TAPE LOOPER · STEREO",
  looperAria: "Looper (máquina de fita)",
  looperSpecs: "loop stereo 24-bit · 44.1 kHz · SNR 110 dB · prévia local",
  tapeState: (secs: number, mode: "PRE" | "POST") => `fita: ${secs}s (${mode})`,
  tapeEmpty: "fita: vazia",
  looperMode: { rec: "REC", play: "PLAY", dub: "DUB", stop: "STOP", ready: "PRONTO", empty: "VAZIA" },
  looperTransportAria: "Transporte do looper",
  looperTimerAria: "Posição da fita",
  looperRewAria: "Retroceder ao início do loop (REW)",
  looperStopAria: "Parar (STOP)",
  looperPlayAria: "Tocar loop (PLAY)",
  looperRecAria: "Gravar loop (REC)",
  looperRecStopAria: "Parar gravação e tocar",
  looperDubAria: "Sobrepor (overdub)",
  looperClearAria: "Limpar fita (pede confirmação)",
  looperClearConfirmAria: "Confirmar limpar fita",
  looperClearBtn: "✕",
  looperClearConfirmBtn: "ok?",
  looperRecVol: "Rec VOL",
  looperPlayVol: "Play VOL",
  looperPVol: "P-VOL",
  looperRoute: "Rota",
  looperPreBtn: "PRE · 90s",
  looperPostBtn: "POST · 45s",
  looperPreAria: "Looper em PRE (90 segundos, sem efeitos gravados)",
  looperPostAria: "Looper em POST (45 segundos, com efeitos)",
  looperVuAria: (channel: string, active: boolean) =>
    `VU meter canal ${channel}${active ? " com sinal" : " em repouso"}`,
  reelAria: (label: string, spinning: boolean) =>
    `Rolo ${label}${spinning ? " girando" : " parado"}`,
  capstanTitle: "capstan",
  reelSupply: "Supply",
  reelTakeup: "Take-up",

  /* ── pushes ── */
  pushTitle: "Pushes do device",
  pushSummary: "pushes do device",
  pushEmpty: "Nenhum push recebido ainda.",
  pushClear: "Limpar",
  pushListAria: "Log de pushes",
  pushRepeats: "Repetições seguidas deste push",
  pushRepeatMark: "×",

  /* ── settings ── */
  previewBadge: "prévia local",
  settingsTitle: "Settings",
  settingsDialogAria: "Configurações",
  settingsTabsAria: "Abas de configuração",
  settingsCloseAria: "Fechar configurações",
  settingsTabs: {
    General: "General",
    "Global EQ": "Global EQ",
    About: "About",
    "Info Frame": "Info Frame",
    Help: "Help",
    "Release Note": "Release Note",
  },
  inputLevelLabel: "Input Level",
  inputLevelAria: "Input level",
  normalLevelLabel: "Normal Level",
  normalLevelAria: "Normal level",
  usbAudioLabel: "USB Audio",
  usbAudioAria: "USB Audio",
  hintModeLabel: "Hint Mode",
  hintModeAria: "Hint mode",
  engineerModeLabel: "Modo engenheiro",
  engineerModeSub: "tooltips dos knobs mostram addr/code/ctrl do comando SET",
  engineerModeAria: "Modo engenheiro — tooltips com addr, code e ctrl do comando SET",
  hintLeft: "Left",
  hintRight: "Right",
  tapTempoLabel: "Tap Tempo Mode",
  tapTempoLabels: { pre: "PRE", mod: "MOD", dly: "DLY" },
  footswitchLabel: "Footswitch Mode",
  footswitchSub: "modo dos footswitches do hardware",
  footswitchAria: "Footswitch mode (disponível em uma próxima versão)",
  footswitchSoon: "— disponível em uma próxima versão —",
  languageLabel: "APP Language",
  languageAria: "Idioma do app (disponível em uma próxima versão)",
  globalEqIntro:
    "Equalização global do device: 5 bandas (FREQ · Q · GAIN) e cortes de graves/agudos. Os controles ficam disponíveis em uma próxima versão.",
  eqBandGroupAria: "Banda do Global EQ",
  eqBandBtn: (n: number) => `B${n}/5`,
  eqFreqLabel: (n: number) => `Band ${n} FREQ`,
  eqQLabel: (n: number) => `Band ${n} Q`,
  eqGainLabel: (n: number) => `Band ${n} GAIN`,
  eqFreqAria: (n: number) => `Banda ${n} freq`,
  eqQAria: (n: number) => `Banda ${n} Q`,
  eqGainAria: (n: number) => `Banda ${n} ganho`,
  lcutLabel: "L-CUT FREQ",
  hcutLabel: "H-CUT FREQ",
  lcutAria: "L-CUT freq",
  hcutAria: "H-CUT freq",
  aboutIntro:
    "GP-100 NextGen Editor — editor não-oficial e independente para a pedaleira Valeton GP-100. Sem vínculo com a Valeton.",
  aboutDataAria: "Dados do device nesta versão",
  aboutData: [
    ["Biblioteca", `${PRESET_COUNT} presets de fábrica (da memória do device)`],
    ["Bateria", `${DRUM_COUNT} ritmos em ${DRUM_GENRE_COUNT} gêneros + metrônomo · compassos ${DRUM_BEAT_RANGE}`],
    ["Looper", "stereo 24-bit · 44.1 kHz · SNR 110 dB — 90 s PRE / 45 s POST"],
    ["Catálogo", `${FX_COUNT} algoritmos / ${CTRL_COUNT} controles mapeados`],
  ] as ReadonlyArray<readonly [string, string]>,
  infoFrameIntro: "Identificação do device, lida na conexão.",
  infoFrameAria: "Identificação do device",
  infoFrame: [
    ["Firmware", "V2.1"],
    ["Software", "1.2.0"],
    ["Modelo", "Valeton GP-100 Multi-Effects Processor"],
    ["Conexão", "USB"],
  ] as ReadonlyArray<readonly [string, string]>,
  helpShortcutsIntro: "Atalhos globais de teclado (funcionam em qualquer lugar da casca):",
  helpKeySpace: "Espaço",
  helpKeyR: "R",
  helpKeyEsc: "Esc",
  helpShortcutSpace: "Bateria: tocar/parar o ritmo selecionado (drum play/stop)",
  helpShortcutR: "Looper: REC — igual ao botão ● (gravar → tocar → overdub)",
  helpShortcutEsc: "Fecha o painel aberto do topo para a base: Settings → Drum → pushes",
  helpTableAria: "Atalhos de teclado",
  helpNotes:
    "Notas: os atalhos de transporte não disparam enquanto você digita em busca, BPM ou selects; Espaço sobre um botão focado ativa o PRÓPRIO botão (comportamento nativo de acessibilidade); Ctrl/Alt/⌘ + tecla é ignorado; com o modal Settings aberto, Espaço e R ficam inativos e Esc fecha o modal.",
  helpControlsIntro: "Controles:",
  helpControls: [
    "Tab navega todos os controles; foco visível",
    "Afinador (cabeçalho do palco): o ♪ liga/desliga o monitor (LED verde/vermelho); modo e REF PITCH ficam sempre visíveis; ▶ demo liga o monitor e toca uma amostra sintética",
    "Esc fecha este modal (global — de qualquer foco)",
  ] as string[],

  /* ── Help → Informações do sistema (dados reais do ambiente) ── */
  sysInfoTitle: "Informações do sistema",
  sysInfoAria: "Versões do app e do ambiente",
  sysInfoHint: "Só leitura — nada daqui é enviado a servidores.",
  sysInfo: [
    ["Versão do app", "0.1.0"],
    ["React", REACT_VERSION],
    ["Backend", "simulado (mock) — o device real alimenta tudo na integração USB"],
    ["Firmware do device", "V2.1 (prévia local)"],
    ["Navegador/SO", typeof navigator !== "undefined" ? navigator.userAgent : "-"],
  ] as ReadonlyArray<readonly [string, string]>,
  sysNote:
    "No app instalado (Tauri), a versão do app vem do pacote; em navegador é a versão de desenvolvimento.",
  releaseNote:
    `Versão de prévia: biblioteca de fábrica (${PRESET_COUNT} presets), bateria com ${DRUM_COUNT} ritmos, looper de fita (90 s PRE / 45 s POST), afinador de palco com monitor contínuo (LED verde/vermelho, REF PITCH 435–445 Hz), atalhos de teclado e layout responsivo. O board já renderiza os pedais do preset — no palco cada pedal é compacto e os knobs são SÓ LEITURA (mostram o valor de cada controle): clicar no pedal abre a edição ampliada, e o que se ajusta lá aparece no palco na hora.`,

  /* ── pedais (textos catálogados antes dos componentes entrarem no board) ── */
  pedalGroupAria: (kind: string, name: string, on: boolean) =>
    `Pedal ${kind} — ${name} (${on ? "ligado" : "desligado"})`,
  pedalToggleOff: "Desligar efeito",
  pedalToggleOn: "Ligar efeito",
  pedalValueAria: "Valor (Enter para editar)",
  pedalValueEditAria: "Valor do knob (Enter aplica, Esc cancela)",
  pedalExpandTitle: "Valores do pedal no palco são só leitura — clique (ou Enter) para editar",
  pedalModalAria: (name: string) => `Edição do pedal ${name}`,
  pedalModalHint: "Esc fecha · os ajustes aplicam na hora",
  pedalModalCloseAria: "Fechar edição do pedal",
  brandPlate: "GP-100",
  onSuffix: " ON",

  /* ── erros amigáveis (detalhe técnico vai pro console) ── */
  errOpenPreset: "Não foi possível abrir o preset — tente novamente.",
  errSelectPreset: "O device não aceitou a troca de preset — a UI segue no preset atual.",
  errSetParam: "O device não aceitou o ajuste do knob — tente novamente.",
  errRetry: "Tentar de novo",
  errRetryAria: "Tentar novamente a operação que falhou",
} as const;
