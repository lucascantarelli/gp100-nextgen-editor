import {
  CTRL_COUNT,
  DRUM_BEAT_RANGE,
  DRUM_COUNT,
  DRUM_GENRE_COUNT,
  FX_COUNT,
  PRESET_COUNT,
  REACT_VER,
  UA,
} from "../facts";

export const PT_BR = {
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
  /** chip = IDENTIDADE do ritmo (abre o modal de gestão); play/stop e BPM
   *  viraram controles próprios da navbar (hierarquia nova do chip).
   *  O prefixo some em ≤1024 (`.nb-chip-pre`) — o nome do estilo fica */
  drumChipPre: "drum · ",
  drumChipAria: (style: string, bpm: number, beat: string) =>
    `Bateria (drum): ${style}, ${bpm} BPM, compasso ${beat} — abrir gestão de ritmos`,
  /** play/stop DIRETO na navbar — não exige o modal de gestão aberto */
  drumToggleAria: "Tocar ou parar o ritmo da bateria",
  /** stepper de BPM direto na navbar (40–240, mesmo clamp do modal) */
  drumBpmDownAria: "Diminuir BPM do ritmo",
  drumBpmUpAria: "Aumentar BPM do ritmo",
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
  stageFooterArrange: "arraste ⇄ para trocar dois pedais de posição",
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
  libUserPp: (i: number) => `U${String(i + 1).padStart(2, "0")}`,

  /* patches de usuário (PRÉVIA LOCAL — a escrita no device não tem canal) */
  userPatchNewPlaceholder: "Nomear e salvar o patch atual…",
  userPatchNameAria: "Nome do patch de usuário a salvar",
  userPatchSave: "Salvar",
  userPatchListAria: "Patches de usuário",
  userPatchEmpty: "Nenhum patch salvo ainda. Ajuste os pedais e salve o que quiser guardar.",
  userPatchRowTitle: (name: string, from: string) => `Abrir “${name}” (veio de ${from})`,
  userPatchDeleteAria: (name: string) => `Excluir o patch “${name}”`,
  userPatchNote: "No arquivo da biblioteca (SQLite), não no navegador. Salvar no device depende do canal USB.",
  userPatchDefaultName: (n: number) => `Meu patch ${n + 1}`,
  userBankType: "User",

  /* lista de efeitos do módulo (issue #19 — "Effects List" do app oficial) */
  effectListTitle: "Effects List",
  effectListAria: (fam: string) => `Lista de efeitos do módulo ${fam}`,
  effectListCount: (n: number) => `${n} efeitos`,
  effectListSearchPlaceholder: "Buscar efeito…",
  effectListSearchAria: "Buscar efeito do módulo por nome",
  effectListEmpty: (q: string) => `Nenhum efeito para “${q}”.`,
  effectListPick: (name: string) => `Trocar o efeito deste pedal para ${name}`,
  effectListCurrent: (name: string) => `${name} — efeito atual deste pedal`,
  effectListNote: "Prévia local: trocar o efeito ainda não é escrito no device.",

  /* ── drum ── */
  drumNote: `${DRUM_COUNT} ritmos em ${DRUM_GENRE_COUNT} gêneros · metrônomo incluído · prévia local`,
  drumPanelAria: "Gestão de ritmos da bateria (drum)",
  drumTitle: "Bateria · ritmos",
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
  /* o deck é uma ilustração única: o nome acessível descreve o mecanismo
     inteiro (rolos + cabeçotes + capstan) e o estado de movimento */
  looperDeckAria: (spinning: boolean) =>
    `Deck de fita: rolo de alimentação, cabeçotes e capstan, rolo de recolhimento${spinning ? ", em movimento" : ", parado"}`,
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
  languageAria: "Idioma do app",
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
    ["React", REACT_VER],
    ["Backend", "simulado (mock) — o device real alimenta tudo na integração USB"],
    ["Firmware do device", "V2.1 (prévia local)"],
    ["Navegador/SO", UA],
  ] as ReadonlyArray<readonly [string, string]>,
  sysNote:
    "No app instalado (Tauri), a versão do app vem do pacote; em navegador é a versão de desenvolvimento.",
  releaseNote:
    `Versão de prévia: biblioteca de fábrica (${PRESET_COUNT} presets), bateria com ${DRUM_COUNT} ritmos, looper de fita (90 s PRE / 45 s POST), afinador de palco com monitor contínuo (LED verde/vermelho, REF PITCH 435–445 Hz), atalhos de teclado e layout responsivo. O board já renderiza os pedais do preset — no palco cada pedal é compacto e os knobs são SÓ LEITURA (mostram o valor de cada controle): clicar no pedal abre a edição ampliada, e o que se ajusta lá aparece no palco na hora.`,

  /* ── pedais (textos catálogados antes dos componentes entrarem no board) ── */
  /** Tipo do pedal por família da cadeia. As CHAVES são as siglas do device
   *  (PRE/DST/AMP… viram `kind` em Pedal.tsx); o valor é o texto que aparece
   *  no palco e nos aria-labels.
   *
   *  pt-BR devolve os mesmos loanwords que a UI já mostrava em inglês
   *  (Pre, Drive, Amp, Gate, Cabinet, Mod, Delay, Reverb) — trocar isso por
   *  "Pré"/"Cabine" mudaria o conteúdo em português por decisão editorial,
   *  não por falta de i18n. Quem tem palavra própria traduz aqui. */
  pedalKind: {
    Pre: "Pre",
    Drive: "Drive",
    Amp: "Amp",
    Gate: "Gate",
    Cabinet: "Cabinet",
    EQ: "EQ",
    Mod: "Mod",
    Delay: "Delay",
    Reverb: "Reverb",
  },
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

  /* ── biblioteca (#26): arquivo local, nao device ── */
  errLibrarySearch: "Não foi possível ler a biblioteca — tente novamente.",
  errLibraryStats: "Não foi possível ler os números da biblioteca.",
  errLibrarySave: "Não foi possível salvar o patch — o arquivo da biblioteca não respondeu.",
  errLibraryDelete: "Não foi possível apagar o patch da biblioteca.",
  errLibraryImport: "A biblioteca não aceitou o arquivo importado — nada foi gravado.",
  errLibraryExport: "Não foi possível exportar a biblioteca.",

  /* ── biblioteca: rótulos da UI ── */
  libSearchPlaceholder: "Buscar por nome, nº ou estilo…",
  libSearchAria: "Buscar na biblioteca de presets",
  libSearchClear: "Limpar busca",
  libFilterAll: "Todos os estilos",
  libFilterAria: "Filtrar a biblioteca por estilo",
  libExport: "Exportar",
  libExportAria: "Exportar a biblioteca para um arquivo",
  libImport: "Importar",
  libImportAria: "Importar uma biblioteca de um arquivo",
  libEmpty: "Nenhum preset com esse filtro.",
  libEmptySearch: (q: string) => `Nada encontrado para “${q}”.`,
  libStats: (total: number, user: number, schema: number) =>
    `${total} presets · ${user} meus · esquema v${schema}`,
  libMigrated: (n: number) =>
    `${n} patches seus foram movidos do armazenamento local para a biblioteca.`,
  libImportDone: (inserted: number, replaced: number, skipped: number) =>
    `Importado: ${inserted} novos, ${replaced} atualizados, ${skipped} já existiam.`,
  libImportRejected: "O arquivo não é uma biblioteca deste editor — nada foi importado.",

  /* ── SnapTone/NAM (#25): o `.clo` e o audio que o Suite renderiza ── */
  errToneRead: "Não foi possível ler os tons — tente novamente.",
  errToneWrite: "Não foi possível gravar o tom — o arquivo não respondeu.",
  errToneImport: "O tom não foi importado — nada foi gravado.",
  errToneSlot: "Não foi possível atribuir o slot.",
  errToneSend: "O envio ao aparelho falhou.",

  toneBtn: "SnapTone",
  toneBtnAria: "Abrir o gestor de tons SnapTone",
  toneTitle: "Tons SnapTone",
  toneIntro:
    "Importe o arquivo .clo que o Valeton Suite gera a partir de um .nam. Este app não converte .nam: a conversão é do Suite.",
  toneNamePlaceholder: "Nome do tom",
  toneNameAria: "Nome do tom importado",
  toneImportClo: "Importar .clo",
  toneImportCloAria: "Importar um modelo .clo convertido pelo Suite",
  toneImportWav: "Importar áudio .wav",
  toneImportWavAria: "Anexar o áudio que o Suite renderiza para este tom",
  toneEmpty: "Nenhum tom importado ainda.",
  toneListAria: "Tons importados",
  toneRowTitle: (nome: string, kb: number) => `${nome} — ${kb} KB`,
  toneSlotNone: "sem slot",
  toneSlotLabel: (n: number) => `Slot ${n}`,
  toneSlotAria: (n: number) => `Slot ${n} do aparelho`,
  toneAssignAria: (nome: string, n: number) => `Colocar “${nome}” no slot ${n}`,
  toneClearSlot: "Tirar do aparelho",
  toneClearSlotAria: (nome: string) => `Tirar “${nome}” do aparelho`,
  toneDeleteAria: (nome: string) => `Apagar o tom “${nome}”`,
  toneRenameAria: "Renomear o tom",
  toneSend: "Enviar ao aparelho",
  toneSendAria: (nome: string) => `Enviar “${nome}” para o slot do aparelho`,
  toneSending: "Enviando…",
  toneSent: (blocos: number, bytes: number) =>
    `Enviado: ${blocos} blocos, ${bytes} bytes, um ACK por bloco.`,
  toneCabTitle: "SnapTone desliga o CAB",
  toneCabWarning:
    "Ao ligar o SnapTone, o módulo CAB é desligado pelo aparelho. Se você usa o CAB, desligue-o de propósito antes de enviar.",
  toneAbTitle: "Comparar A/B",
  toneAbHint:
    "O áudio é o nam_output_wav.wav que o Suite renderiza. O app não roda o modelo: tocar aqui é ouvir o áudio do tom.",
  toneAbEmpty: "Escolha os dois lados",
  toneAbA: "A",
  toneAbB: "B",
  tonePlay: "Ouvir",
  tonePlayAria: (lado: string, nome: string) => `Ouvir o lado ${lado}: ${nome}`,
  toneNoPreview: "Sem áudio",
  toneStats: (total: number, slots: number, usados: number) =>
    `${total} tons · ${usados} de ${slots} slots em uso`,

  errRetry: "Tentar de novo",
  errRetryAria: "Tentar novamente a operação que falhou",
} as const;

export type Dict = typeof PT_BR;

/**
 * O que um idioma traduzido pode sobrescrever do dicionário base.
 *
 * O `as const` acima é o que impede um texto de deviate do contrato: todo
 * rótulo é literal (`"PRONTO"`, `"Supply"`), e `Dict` é o tipo que os ~15
 * componentes já conhecem. O problema é que `"PRONTO"` como tipo BLOQUEIA o
 * espanhol: nenhum valor cabe em `"PRONTO"`, nem `"LISTO"`, nem `"PRONTO"`.
 *
 * `DictPatch` alarga para `string` o *conteúdo* dos rótulos, mantendo intacta
 * a *forma*: nome da chave, aridade e tipos dos parâmetros das funções, e o
 * tipo dos arrays de dados. Ou seja, traduzir continua sendo impossível de
 * quebrar a tipagem em qualquer ponto do app — só não pode mais escrever
 * exatamente a palavra que o português usou.
 */
export type DictPatch = {
  [K in keyof Dict]?: Dict[K] extends (...args: infer A) => unknown
    ? (...args: A) => string
    : Dict[K] extends ReadonlyArray<unknown>
      ? Dict[K]
      : Dict[K] extends object
        ? { [K2 in keyof Dict[K]]?: string }
        : string;
};
