/**
 * ES — dicionário Español. `DictPatch` de propósito: o que faltar aqui cai
 * no pt-BR (merge profundo em `../messages.ts`), então um idioma pela metade
 * nunca mostra chave crua — degrada para a língua garantida.
 *
 * O que NÃO é traduzido, e por quê:
 *  - termos que o próprio device imprime (PRE/POST, REC, PLAY, DUB, STOP,
 *    BPM, REW, FREQ/Q/GAIN, SET, e as abas de Settings) — traduzi-los
 *    desalinearia a UI do que está na frente do usuário, que precisa bater
 *    com o painel de-hardware na mão;
 *  - marca e nome de modelo (GP-100, Valeton, Mock Device) e os nomes de
 *    ritmo/efeito que vêm dos artefatos gerados.
 */
import type { DictPatch } from "./pt-BR";
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

export const ES: DictPatch = {
  /* ── marca / topbar ── */
  brand: "GP-100",
  brandMono: "GP",
  tagline: "editor no oficial",
  connShortOn: "on",
  connShortOff: "off",
  mockBadge: "Mock Device",
  patchLabel: "patch",
  patchPrevAria: "Patch anterior",
  patchPrevTitle: "Patch anterior (P01–P99 en ciclo)",
  patchNextAria: "Patch siguiente",
  patchNextTitle: "Patch siguiente (P01–P99 en ciclo)",
  drumChipPre: "drum · ",
  drumChipAria: (style: string, bpm: number, beat: string) =>
    `Batería (drum): ${style}, ${bpm} BPM, compás ${beat} — abrir la gestión de ritmos`,
  drumToggleAria: "Reproducir o detener el ritmo de la batería",
  drumBpmDownAria: "Bajar el BPM del ritmo",
  drumBpmUpAria: "Subir el BPM del ritmo",
  masterLabel: "master",
  masterAria: "Volumen master",
  arrangeLabel: "🔒",
  arrangeLabelOpen: "🔓",
  arrangeAria: "Bloqueo de mover pedales — arrastrar y soltar solo cuando está desbloqueado",
  arrangeTitle: "Desbloquea para arrastrar pedales (protege el ajuste de los knobs)",
  killLabel: "⭘ kill",
  killLabelOn: "⏻ killed",
  killAria: "Kill switch (silenciar master, batería y looper)",
  killTitle: "Silencio global: apaga el master y la batería — haz clic otra vez para restaurar",
  openSettingsAria: "Abrir ajustes",

  /* ── navbar (cluster de conexión: estado corto + badge + arranque) ── */
  connClusterAria: "Conexión y arranque",

  /* ── afinador del escenario ── */
  tunerAria: "Afinador",
  tunerPowerAria: "Encender o apagar la monitorización de afinación",
  tunerPowerIcon: "♪",
  tunerPowerTitle: (on: boolean) =>
    on
      ? "Monitor de afinación ENCENDIDO — muestra lo que tocas en tiempo real (en el device: mantén los 2 footswitches)"
      : "Monitor de afinación apagado — haz clic para seguir la afinación en tiempo real (en el device: mantén los 2 footswitches)",
  tunerModeAria: "Modo del afinador — alternar entre bypass, thru y mute",
  tunerModeTitle:
    "Lo que hace el device con la señal mientras afinas: bypass (seco), thru (con efecto) o mute (silencio)",
  tunerModeLabel: (mode: "bypass" | "thru" | "mute") => mode,
  tunerRefLabel: "ref",
  tunerRefAria: "Tono de referencia (A4) en hercios, de 435 a 445",
  tunerRefTitle: "REF PITCH: 435–445 Hz (estándar 440 Hz)",
  tunerRefValue: (hz: number) => `${hz}Hz`,
  tunerDemoLabel: "▶ demo",
  tunerDemoRunning: "■ demo",
  tunerDemoAria: "Reproducir la demo del afinador con un tono sintético",
  tunerDemoTitle:
    "Muestra sintética barriendo ±30 cents en A2 con el motor real del afinador — no es audio del device",
  tunerIdleNote: "—",
  tunerFlatMark: "♭",
  tunerSharpMark: "♯",

  /* ── navbar (arranque = reescanear; el app ya detecta el device solo) ── */
  rescanAria: "Reescanear el device",
  rescanTitle: "Reescanear (arranque): el app detecta el device solo al abrir — usa esto para reescanear",
  bootProgressAria: "Progreso del arranque",
  bootStarting: "Iniciando",
  bootStages: {
    tables: "Tablas de IR",
    scan: "Escaneo de presets",
    probe: "Sonda banco 02",
    setlist: "Setlist",
    names: "Nombres",
    keepalive: "Keepalive",
  },
  connBootError: "Falló el arranque del device — revisa la conexión e inténtalo de nuevo.",

  /* ── board ── */
  boardAria: "Pedalboard (9 huecos de la cadena)",
  slotAria: (n: number, fam: string) => `Slot ${n}: ${fam}`,
  slotEmpty: "vacío",
  slotArrange: "arrastra ⇄",
  stageFooter: "9 slots · orden de la señal →",
  stageFooterArrange: "arrastra ⇄ para intercambiar dos pedales de posición",
  stageIn: "⏻ IN",
  stageOut: "OUT ⏻",

  /* ── biblioteca ── */
  libAria: "Biblioteca de presets",
  libTabsAria: "Tipo de patch",
  libTabFactory: "Factory Patch",
  libTabUser: "User Patch",
  libListAria: "Presets de fábrica",
  searchPlaceholder: "Buscar nombre, nº o estilo…",
  searchAria: "Buscar presets por nombre, número o estilo",
  searchEmpty: (q: string) => `Ningún preset para “${q}”. Consejo: busca por estilo (Rock, Funk…) o por número.`,
  libRowTitle: (name: string, type: string) => `Abrir “${name}” (${type})`,
  libPp: (pp: number) => `P${String(pp + 1).padStart(2, "0")}`,
  libUserPp: (i: number) => `U${String(i + 1).padStart(2, "0")}`,

  /* patches de usuario (PRÉVIA LOCAL — escribir en el device no tiene canal) */
  userPatchNewPlaceholder: "Nombra y guarda el patch actual…",
  userPatchNameAria: "Nombre del patch de usuario a guardar",
  userPatchSave: "Guardar",
  userPatchListAria: "Patches de usuario",
  userPatchEmpty: "Todavía no hay patches guardados. Ajusta los pedales y guarda lo que quieras conservar.",
  userPatchRowTitle: (name: string, from: string) => `Abrir “${name}” (viene de ${from})`,
  userPatchDeleteAria: (name: string) => `Eliminar el patch “${name}”`,
  userPatchNote: "En el archivo de la biblioteca (SQLite), no en el navegador. Guardar en el device depende del canal USB.",
  userPatchDefaultName: (n: number) => `Mi patch ${n + 1}`,
  userBankType: "User",

  /* ── historial del patch (issue #113 — biblioteca versionada) ── */
  histOpenAria: (name: string) => `Abrir el historial de “${name}”`,
  histTitle: "Historial del patch",
  histAria: "Historial del patch: versiones, diferencias y restauración",
  histClose: "Cerrar el historial",
  histEmpty: "Este patch aún no tiene historial. Guarda el patch otra vez para crear la primera versión.",
  histBefore: "antes",
  histAfter: "después",
  histPickVersion: "elige una versión",
  histPickTwo: "Elige dos versiones para ver qué cambió entre ellas.",
  histCurrent: "· actual",
  histRestoreAll: (seq: number) => `Restaurar el patch entero en la versión ${seq}`,
  histRestoreAllAction: "restaurar todo",
  histRestoreKnob: (knob: string, from: string | null, to: string | null) => `Restaurar ${knob} de ${to} a ${from}`,
  histRestoreKnobAction: "restaurar",
  histIdentical: "Esas dos versiones son idénticas: no cambió nada entre ellas.",
  histDiffTitle: "Qué cambió:",
  histSlot: (slot: number, name: string) => `slot ${slot + 1} · ${name}`,
  histAlgorithm: (from: string, to: string) => `algoritmo: ${from} → ${to}`,
  histSlotState: (on: boolean) => (on ? "activo" : "inactivo"),
  histArrow: (from: string | null, to: string | null) => `${from ?? "—"} → ${to ?? "—"}`,
  histSlotsAdded: (list: string) => `Slots que solo existen en la versión nueva: ${list}`,
  histSlotsRemoved: (list: string) => `Slots que solo existen en la versión antigua: ${list}`,
  histRestored: (seq: number) => `Listo: el patch pasó a ser la versión ${seq}. La anterior sigue en el historial.`,

  /* lista de efectos del módulo (issue #19 — "Effects List" del app oficial) */
  effectListTitle: "Effects List",
  effectListAria: (fam: string) => `Lista de efectos del módulo ${fam}`,
  effectListCount: (n: number) => `${n} efectos`,
  effectListSearchPlaceholder: "Buscar efecto…",
  effectListSearchAria: "Buscar los efectos del módulo por nombre",
  effectListEmpty: (q: string) => `Ningún efecto para “${q}”.`,
  effectListPick: (name: string) => `Cambiar el efecto de este pedal a ${name}`,
  effectListCurrent: (name: string) => `${name} — efecto actual de este pedal`,
  effectListNote: "Vista previa local: cambiar el efecto todavía no se escribe en el device.",

  /* ── drum ── */
  drumNote: `${DRUM_COUNT} ritmos en ${DRUM_GENRE_COUNT} géneros · metrónomo incluido · vista previa local`,
  drumPanelAria: "Gestión de ritmos de la batería (drum)",
  drumTitle: "Batería · ritmos",
  drumCloseAria: "Cerrar la gestión de ritmos",
  drumGenreLabel: "Género",
  drumStyleLabel: "Ritmo",
  drumBpmLabel: "BPM",
  drumBeatLabel: "Compás",
  drumVolLabel: "Volumen",
  drumSpeedLabel: "Speed",

  /* ── looper ── */
  looperPlate: "GP-100 · TAPE LOOPER · STEREO",
  looperAria: "Looper (máquina de cinta)",
  looperSpecs: "loop estéreo 24-bit · 44.1 kHz · SNR 110 dB · vista previa local",
  tapeState: (secs: number, mode: "PRE" | "POST") => `cinta: ${secs}s (${mode})`,
  tapeEmpty: "cinta: vacía",
  looperMode: { rec: "REC", play: "PLAY", dub: "DUB", stop: "STOP", ready: "LISTO", empty: "VACÍA" },
  looperTransportAria: "Transporte del looper",
  looperTimerAria: "Posición de la cinta",
  looperRewAria: "Rebobinar al inicio del loop (REW)",
  looperStopAria: "Parar (STOP)",
  looperPlayAria: "Reproducir el loop (PLAY)",
  looperRecAria: "Grabar el loop (REC)",
  looperRecStopAria: "Parar la grabación y reproducir",
  looperDubAria: "Superponer (overdub)",
  looperClearAria: "Borrar la cinta (pide confirmación)",
  looperClearConfirmAria: "Confirmar el borrado de la cinta",
  looperClearBtn: "✕",
  looperClearConfirmBtn: "ok?",
  looperRecVol: "Rec VOL",
  looperPlayVol: "Play VOL",
  looperPVol: "P-VOL",
  looperRoute: "Ruta",
  looperPreBtn: "PRE · 90s",
  looperPostBtn: "POST · 45s",
  looperPreAria: "Looper en PRE (90 segundos, sin efectos grabados)",
  looperPostAria: "Looper en POST (45 segundos, con efectos)",
  looperVuAria: (channel: string, active: boolean) =>
    `VU meter canal ${channel}${active ? " con señal" : " en reposo"}`,
  looperDeckAria: (spinning: boolean) =>
    `Deck de cinta: rollo de alimentación, cabezales y cabrestante, rollo de recogida${spinning ? ", en movimiento" : ", parado"}`,
  reelSupply: "Supply",
  reelTakeup: "Take-up",

  /* ── pushes ── */
  pushTitle: "Pushes del device",
  pushSummary: "pushes del device",
  pushEmpty: "Todavía no llegó ningún push.",
  pushClear: "Limpiar",
  pushListAria: "Registro de pushes",
  pushRepeats: "Repeticiones seguidas de este push",
  pushRepeatMark: "×",

  /* ── diagnóstico de campo (capacidades que vieram del binario de línea de comandos) ── */
  diagTitle: "Diagnóstico de campo",
  diagSummary: "diagnóstico",
  diagAria: "Diagnóstico de campo: con qué habla esta instalación del aparato",
  diagBackendMock: "Simulación — ningún aparato conectado",
  diagBackendNone: "Sin aparato",
  diagNoDeviceHint: "Lo que edites aquí se queda en la pantalla.",
  diagBackendReal: "Aparato conectado",
  diagWriteLocked:
    "Instalación de solo lectura: escribir en el aparato queda bloqueado antes de la conexión USB.",
  diagWriteOpen: "Escritura liberada en esta instalación.",
  writeLockedHint: "Instalación de solo lectura: escribir en el aparato está bloqueado en esta compilación.",
  diagPpLabel: "Patch",
  diagPpTypeLabel: "Tipo",
  diagNameLabel: "Nombre",
  diagSave: "Grabar en el aparato",
  diagSaveDone: "Grabado en el aparato.",
  diagDump: "Leer lo que hay en el aparato",
  diagDumpTitle: "Lo que el aparato tiene en este patch",
  diagDumpMeta6: "Identificación",
  diagDumpPages: (n: number) => `${n} páginas de estado`,
  diagLogStart: "Grabar el tráfico",
  diagLogStop: "Parar la grabación",
  diagLogPathLabel: "Archivo del log",
  diagLogHint: "Graba todo lo que entra y sale por USB, para que el soporte lo analice después.",
  diagLogOn: (path: string) => `Grabando en ${path}`,
  diagPreviewBtn: "Ver lo que se enviaría",
  diagPreviewTitle: "Lo que se enviaría, sin enviarlo",
  diagPreviewEmpty: "Nada que mostrar.",
  diagPreviewCount: (n: number) => `${n} bloques`,
  diagCopyAria: "Copiar el código hexadecimal",
  diagCopyBtn: "copiar",
  diagCopied: "copiado",

  /* ── ajustes ── */
  previewBadge: "vista previa local",
  settingsTitle: "Settings",
  settingsDialogAria: "Ajustes",
  settingsTabsAria: "Pestañas de ajustes",
  settingsCloseAria: "Cerrar los ajustes",
  settingsTabs: {
    General: "General",
    "Global EQ": "Global EQ",
    About: "About",
    "Info Frame": "Info Frame",
    Help: "Help",
    "Release Note": "Release Note",
  },
  inputLevelLabel: "Input Level",
  inputLevelAria: "Nivel de entrada",
  normalLevelLabel: "Normal Level",
  normalLevelAria: "Nivel normal",
  usbAudioLabel: "USB Audio",
  usbAudioAria: "USB Audio",
  hintModeLabel: "Hint Mode",
  hintModeAria: "Modo de dicas",
  engineerModeLabel: "Modo ingeniero",
  engineerModeSub: "los tooltips de los knobs muestran el addr/code/ctrl del comando SET",
  engineerModeAria: "Modo ingeniero — tooltips con el addr, code y ctrl del comando SET",
  hintLeft: "Left",
  hintRight: "Right",
  tapTempoLabel: "Tap Tempo Mode",
  tapTempoLabels: { pre: "PRE", mod: "MOD", dly: "DLY" },
  footswitchLabel: "Footswitch Mode",
  footswitchSub: "modo de los footswitches del hardware",
  footswitchAria: "Footswitch mode (disponible en una próxima versión)",
  footswitchSoon: "— disponible en una próxima versión —",
  languageLabel: "APP Language",
  languageAria: "Idioma del app",
  globalEqIntro:
    "Ecualización global del device: 5 bandas (FREQ · Q · GAIN) y cortes de graves/agudos. Los controles estarán disponibles en una próxima versión.",
  eqBandGroupAria: "Banda del Global EQ",
  eqBandBtn: (n: number) => `B${n}/5`,
  eqFreqLabel: (n: number) => `Band ${n} FREQ`,
  eqQLabel: (n: number) => `Band ${n} Q`,
  eqGainLabel: (n: number) => `Band ${n} GAIN`,
  eqFreqAria: (n: number) => `Banda ${n} freq`,
  eqQAria: (n: number) => `Banda ${n} Q`,
  eqGainAria: (n: number) => `Banda ${n} ganancia`,
  lcutLabel: "L-CUT FREQ",
  hcutLabel: "H-CUT FREQ",
  lcutAria: "L-CUT freq",
  hcutAria: "H-CUT freq",
  aboutIntro:
    "GP-100 NextGen Editor — editor no oficial e independiente para la pedalera Valeton GP-100. Sin vínculo con Valeton.",
  aboutDataAria: "Datos del device en esta versión",
  aboutData: [
    ["Biblioteca", `${PRESET_COUNT} presets de fábrica (de la memoria del device)`],
    ["Batería", `${DRUM_COUNT} ritmos en ${DRUM_GENRE_COUNT} géneros + metrónomo · compases ${DRUM_BEAT_RANGE}`],
    ["Looper", "estéreo 24-bit · 44.1 kHz · SNR 110 dB — 90 s PRE / 45 s POST"],
    ["Catálogo", `${FX_COUNT} algoritmos / ${CTRL_COUNT} controles mapeados`],
  ],
  infoFrameIntro: "Identificación del device, leída al conectar.",
  infoFrameAria: "Identificación del device",
  infoFrame: [
    ["Firmware", "V2.1"],
    ["Software", "1.2.0"],
    ["Modelo", "Valeton GP-100 Multi-Effects Processor"],
    ["Conexión", "USB"],
  ],
  helpShortcutsIntro: "Atajos globales de teclado (funcionan en cualquier sitio de la casca):",
  helpKeySpace: "Espacio",
  helpKeyR: "R",
  helpKeyEsc: "Esc",
  helpShortcutSpace: "Batería: reproducir/detener el ritmo seleccionado (drum play/stop)",
  helpShortcutR: "Looper: REC — igual que el botón ● (grabar → reproducir → overdub)",
  helpShortcutEsc: "Cierra el panel abierto desde el nivel superior hasta la base: Settings → Drum → pushes",
  helpTableAria: "Atajos de teclado",
  helpNotes:
    "Notas: los atajos de transporte no se disparan mientras escribes en una búsqueda, un BPM o un select; Espacio sobre un botón enfocado activa ESE botón (comportamiento nativo de accesibilidad); Ctrl/Alt/⌘ + tecla se ignora; con el modal de ajustes abierto, Espacio y R quedan inactivos y Esc cierra el modal.",
  helpControlsIntro: "Controles:",
  helpControls: [
    "Tab recorre todos los controles; el foco siempre es visible",
    "Afinador (cabecera del escenario): ♪ enciende/apaga el monitor (LED verde/rojo); el modo y REF PITCH siempre están visibles; ▶ demo enciende el monitor y reproduce una muestra sintética",
    "Esc cierra este modal (global — desde cualquier foco)",
  ],

  /* ── Help → Información del sistema (datos reales del entorno) ── */
  sysInfoTitle: "Información del sistema",
  sysInfoAria: "Versiones del app y del entorno",
  sysInfoHint: "Solo lectura — nada de esto se envía a servidores.",
  sysInfo: [
    ["Versión del app", "0.1.0"],
    ["React", REACT_VER],
    ["Backend", "simulado (mock) — el device real alimenta todo por USB"],
    ["Firmware del device", "V2.1 (vista previa local)"],
    ["Navegador/SO", UA],
  ],
  sysNote:
    "En la app instalada (Tauri), la versión viene del paquete; en el navegador es la versión de desarrollo.",
  releaseNote:
    `Versión de vista previa: biblioteca de fábrica (${PRESET_COUNT} presets), batería con ${DRUM_COUNT} ritmos, looper de cinta (90 s PRE / 45 s POST), afinador de escenario con monitorización continua (LED verde/rojo, REF PITCH 435–445 Hz), atajos de teclado y layout responsivo. El board ya dibuja los pedales del preset — en el escenario cada pedal es compacto y los knobs son de SOLO LECTURA (muestran el valor de cada control): haz clic en el pedal para abrir la edición ampliada, y lo que cambies ahí aparece en el escenario al instante.`,

  /* ── pedales ── */
  pedalKind: {
    Pre: "Preamp",
    Drive: "Drive",
    Amp: "Amp",
    Gate: "Gate",
    Cabinet: "Cabinete",
    EQ: "EQ",
    Mod: "Mod",
    Delay: "Delay",
    Reverb: "Reverb",
  },
  pedalGroupAria: (kind: string, name: string, on: boolean) =>
    `Pedal ${kind} — ${name} (${on ? "encendido" : "apagado"})`,
  pedalToggleOff: "Apagar el efecto",
  pedalToggleOn: "Encender el efecto",
  pedalValueAria: "Valor (Enter para editar)",
  pedalValueEditAria: "Valor del knob (Enter aplica, Esc cancela)",
  pedalExpandTitle: "Los valores del pedal en el escenario son de solo lectura — haz clic (o Enter) para editar",
  pedalModalAria: (name: string) => `Edición del pedal ${name}`,
  pedalModalHint: "Esc cierra · los cambios se aplican al instante",
  pedalModalCloseAria: "Cerrar la edición del pedal",
  brandPlate: "GP-100",
  onSuffix: " ON",

  /* ── errores amigables (el detalle técnico va a la consola) ── */
  errOpenPreset: "No se pudo abrir el preset — inténtalo de nuevo.",
  errSelectPreset: "El device no aceptó el cambio de preset — la UI sigue en el preset actual.",
  errSetParam: "El device no aceptó el ajuste del knob — inténtalo de nuevo.",
  /* ── biblioteca (#26): archivo local, no el device ── */
  errLibrarySearch: "No se pudo leer la biblioteca — inténtalo de nuevo.",
  errLibraryStats: "No se pudieron leer los números de la biblioteca.",
  errLibrarySave: "No se pudo guardar el patch — el archivo de la biblioteca no respondió.",
  errLibraryDelete: "No se pudo borrar el patch de la biblioteca.",
  errLibraryImport: "La biblioteca no aceptó el archivo importado — no se grabó nada.",
  errLibraryExport: "No se pudo exportar la biblioteca.",

  /* ── biblioteca: rótulos de la UI ── */
  libSearchPlaceholder: "Buscar por nombre, número o estilo...",
  libSearchAria: "Buscar en la biblioteca de presets",
  libSearchClear: "Limpiar la búsqueda",
  libFilterAll: "Todos los estilos",
  libFilterAria: "Filtrar la biblioteca por estilo",
  libExport: "Exportar",
  libExportAria: "Exportar la biblioteca a un archivo",
  libImport: "Importar",
  libImportAria: "Importar una biblioteca desde un archivo",
  libEmpty: "Ningún preset coincide con este filtro.",
  libEmptySearch: (q: string) => `Nada encontrado para “${q}”.`,
  libStats: (total: number, user: number, schema: number) =>
    `${total} presets · ${user} míos · esquema v${schema}`,
  libMigrated: (n: number) =>
    `${n} de tus patches pasaron del almacenamiento local a la biblioteca.`,
  libImportDone: (inserted: number, replaced: number, skipped: number) =>
    `Importados: ${inserted} nuevos, ${replaced} actualizados, ${skipped} ya estaban.`,
  libImportRejected: "Ese archivo no es una biblioteca de este editor — no se importó nada.",

  /* -- SnapTone/NAM (#25): el .clo y el audio que renderiza el Suite -- */
  errToneRead: "No se pudieron leer los tonos — inténtalo de nuevo.",
  errToneWrite: "No se pudo guardar el tono — el archivo no respondió.",
  errToneImport: "El tono no se importó — no se grabó nada.",
  errToneSlot: "No se pudo asignar el slot.",
  errToneSend: "Falló el envío al aparato.",

  toneBtn: "SnapTone",
  toneBtnAria: "Abrir el gestor de tonos SnapTone",
  toneTitle: "Tonos SnapTone",
  toneIntro:
    "Importa el archivo .clo que el Valeton Suite genera a partir de un .nam. Esta app no convierte .nam: la conversión es del Suite.",
  toneNamePlaceholder: "Nombre del tono",
  toneNameAria: "Nombre del tono importado",
  toneImportClo: "Importar .clo",
  toneImportCloAria: "Importar un modelo .clo convertido por el Suite",
  toneImportWav: "Importar audio .wav",
  toneImportWavAria: "Adjuntar el audio que el Suite renderizó para este tono",
  toneEmpty: "Todavía no hay tonos importados.",
  toneListAria: "Tonos importados",
  toneRowTitle: (nombre: string, kb: number) => `${nombre} — ${kb} KB`,
  toneSlotNone: "sin slot",
  toneSlotLabel: (n: number) => `Slot ${n}`,
  toneSlotAria: (n: number) => `Slot ${n} del aparato`,
  toneAssignAria: (nombre: string, n: number) => `Poner “${nombre}” en el slot ${n}`,
  toneClearSlot: "Quitar del aparato",
  toneClearSlotAria: (nombre: string) => `Quitar “${nombre}” del aparato`,
  toneDeleteAria: (nombre: string) => `Borrar el tono “${nombre}”`,
  toneRenameAria: "Renombrar el tono",
  toneSend: "Enviar al aparato",
  toneSendAria: (nombre: string) => `Enviar “${nombre}” al slot del aparato`,
  toneSending: "Enviando…",
  toneSent: (bloques: number, bytes: number) =>
    `Enviado: ${bloques} bloques, ${bytes} bytes, un ACK por bloque.`,
  toneCabTitle: "SnapTone apaga el CAB",
  toneCabWarning:
    "Al activar SnapTone, el aparato desactiva el módulo CAB. Si usas el CAB, desactívalo a propósito antes de enviar.",
  toneAbTitle: "Comparar A/B",
  toneAbHint:
    "El audio es el nam_output_wav.wav que renderiza el Suite. La app no ejecuta el modelo: reproducir aquí es escuchar el audio del tono.",
  toneAbEmpty: "Elige los dos lados",
  toneAbA: "A",
  toneAbB: "B",
  tonePlay: "Escuchar",
  tonePlayAria: (lado: string, nombre: string) => `Escuchar el lado ${lado}: ${nombre}`,
  toneNoPreview: "Sin audio",
  toneStats: (total: number, slots: number, usados: number) =>
    `${total} tonos · ${usados} de ${slots} slots en uso`,

  /* ── laboratorio de IRs (#24): el `.ir` del dueño y los 20 slots del aparato ── */
  errIrRead: "No se pudieron leer los IR — inténtalo de nuevo.",
  errIrWrite: "No se pudo guardar el IR — el archivo no respondió.",
  errIrImport: "El IR no se importó — no se guardó nada.",
  errIrSlot: "No se pudo asignar el slot.",
  errIrSend: "Falló el envío al aparato.",
  errIrDevice: "No se pudo leer la tabla de IRs del aparato.",

  irBtnAria: "Abrir tu contenido (tonos SnapTone, laboratorio de IRs y el preset como archivo)",

  /* ── el preset como archivo (#114): JSON versionado + hoja de timbre ── */
  presetFileTitle: "Preset como archivo",
  presetFileHint:
    "El preset de fábrica que está en el escenario, en dos archivos: el JSON versionado (que vuelve a ser un preset) y la hoja de timbre para imprimir.",
  presetFileTarget: (rotulo: string, nombre: string) => `Exportando ${rotulo} ${nombre}`,
  presetFileExport: "Exportar JSON",
  presetFileExportAria: "Exportar el preset a un archivo JSON versionado",
  presetFileSheet: "Hoja de timbre (PDF)",
  presetFileSheetAria: "Exportar la hoja de timbre del preset en PDF",
  presetFileSheetOff: "solo en la app",
  presetFileSheetHint:
    "La hoja de timbre la genera el motor de PDF de la app: fuera de ella no existe.",
  presetFileImport: "Importar JSON",
  presetFileImportAria: "Importar un preset desde un archivo JSON",
  presetFileImported: (rotulo: string, nombre: string, slots: number) =>
    `Cadena importada: ${rotulo} ${nombre}, ${slots} slots en el escenario.`,
  presetFileRejected: (motivo: string) => `Archivo rechazado: ${motivo}`,
  presetFileFailed: "No se pudo generar el archivo — inténtalo de nuevo.",
  presetFileClose: "Cerrar el preset como archivo",
  presetFileNote:
    "Importar no graba en el aparato: la cadena va al escenario, y guardar en el GP-100 sigue siendo un acto aparte.",

  /* ── asistente de gain staging (#115): dónde está cada control en el rango ── */
  errGainReport: "No se pudo leer el informe de ganancia — inténtalo de nuevo.",
  gainTitle: "Asistente de gain staging",
  gainClose: "Cerrar el asistente de gain staging",
  gainTarget: (rotulo: string, nombre: string) => `Analizando ${rotulo} ${nombre}`,
  gainHint:
    "Dónde está cada control de ganancia y de nivel en el rango declarado del aparato, y dónde se acumula en la cadena. Es una ESTIMACIÓN: el asistente no mide audio.",
  gainCarregando: "Leyendo la cadena…",
  gainRisco: "Riesgo de saturación",
  gainRiscoNivel: { baixo: "BAJO", medio: "MEDIO", alto: "ALTO" },
  gainFolgaMenor: "menor margen",
  gainSemFolga: "sin control de nivel",
  gainFolgaDe: (slot: number, pct: string) => `en el ${slot}º lugar: ${pct}`,
  gainFolga: (pct: string) => `margen ${pct}`,
  gainModulo: (slot: number, familia: string, nombre: string) => `${slot}º · ${familia} · ${nombre}`,
  gainDesligado: "apagado (fuera de la cuenta)",
  gainSemControle: "sin control de ganancia ni de nivel",
  gainLeitura: (knob: string, valor: string, lo: number, hi: number) =>
    `${knob} = ${valor} · rango ${lo}–${hi}`,
  gainPosicao: (pct: string) => `${pct} del rango`,
  gainNoTeto: "en el tope",
  gainIgnorados: (lista: string) => `fuera de la cuenta (sin posición): ${lista}`,
  gainForaDoDicionario: (slots: string) => `Slots fuera del diccionario: ${slots}`,
  gainAjusteTitulo: "Orden de ajuste sugerido",
  gainAjusteVazio: "Nada en el tope — no hay qué ajustar.",
  gainAjusteItem: (familia: string, knob: string, valor: string) =>
    `${familia}: ${knob} (está en ${valor})`,
  gainMetodoTitulo: "Método y limitación",
  gainGanho: "Ganancia",
  gainSaida: "Salida",
  gainMix: "Mezcla (fuera)",
  gainTetoEm: (pct: string) => `tope en ${pct}`,
  gainNaoEscreve:
    "El asistente NO escribe en el aparato: señala dónde la cadena está en el tope, y la decisión (y el ajuste) es tuya.",

  /* La PUERTA que abre las dos pantallas. Las opciones llevan los TÍTULOS de
     las pantallas (`toneTitle`/`irTitle`) a propósito: el dueño lee el mismo
     nombre en el menú y arriba del panel que abrió. */
  contentMenuTitle: "Contenido",
  contentMenuHint: "Archivos que guardas en la app y envías al aparato.",
  contentMenuClose: "Cerrar",
  irTitle: "Laboratorio de IRs",
  irIntro:
    "Importa el archivo .ir que exportó el dueño del pedal. El aparato tiene 20 slots de User IR; lo que tiene ahora está justo debajo.",
  irDeviceTitle: "Lo que hay en el aparato",
  irDeviceHint:
    "Lectura del propio aparato (20 slots). Un slot que no aparece aquí puede haberse grabado desde el panel de hardware — por eso enviar a un slot ocupado pide confirmación.",
  irDeviceAria: "Slots de User IR del aparato",
  irDeviceEmpty: "vacío",
  irDeviceSlotTitle: (slot: number, nombre: string) => `Slot ${slot}: ${nombre}`,
  irDeviceHas: (nombre: string) => `en el aparato: ${nombre}`,
  irReadDevice: "Releer el aparato",
  irReadingDevice: "Leyendo…",
  irNamePlaceholder: "Nombre del IR",
  irNameAria: "Nombre del IR importado",
  irImport: "Importar .ir",
  irImportAria: "Importar un archivo .ir del disco",
  irEmpty: "Ningún IR importado todavía.",
  irListAria: "IRs importados",
  irRowTitle: (nombre: string, kb: number) => `${nombre} — ${kb} KB`,
  irSlotNone: "sin slot",
  irSlotLabel: (n: number) => `Slot ${n}`,
  irSlotAria: (n: number) => `Slot ${n} del aparato`,
  irRenameAria: "Renombrar el IR",
  irSend: "Enviar al aparato",
  irSendAria: (nombre: string) => `Enviar “${nombre}” al slot del aparato`,
  irSending: "Enviando…",
  irNotEnviable: "El aparato no envía este archivo: el tamaño debe ser múltiplo de 15 bytes.",
  irOverwriteTitle: "Ese slot ya tiene un IR",
  irOverwriteText: (nombre: string, slot: number) =>
    `El slot ${slot} del aparato tiene “${nombre}”. Enviar aquí reemplaza el IR grabado.`,
  irOverwriteEmpty: "Este slot del aparato está vacío.",
  irDeleteAria: (nombre: string) => `Borrar el IR “${nombre}” de la biblioteca`,
  irDelete: "Borrar",
  irDeleteWarnTitle: "Borra solo aquí",
  irDeleteWarnText:
    "Esto borra el IR del archivo de este editor. Lo que ya está grabado en el aparato sigue ahí: borrar el slot del aparato es un comando que todavía no existe.",
  irCancel: "Cancelar",
  irSent: (chunks: number, bytes: number) =>
    `Enviado: ${chunks} bloques de 15 bytes, ${bytes} bytes, un ACK por bloque.`,
  irStats: (total: number, slots: number, usados: number) =>
    `${total} IRs · ${usados} de ${slots} slots en uso`,

  /* A/B con prueba a ciegas (#116) */
  abTitle: "A/B del patch",
  abIntro:
    "Compara dos versiones del MISMO patch: el escenario cambia al instante y los knobs que cambiaron van al aparato. El blind oculta qué lado suena hasta tu respuesta.",
  abEmpty:
    "Abre un patch de usuario con DOS versiones en el historial (guarda una vez con el patch abierto): el A/B compara el mismo patch en dos instantes, no dos patches cualquiera.",
  abVersao: (seq: number) => `v${seq}`,
  abLado: (q: string) => `Lado ${q}`,
  abSoando: "sonando ahora",
  abOuvir: (q: string) => `Escuchar el lado ${q}`,
  abTrocar: "Cambiar de lado",
  abPalpite: (q: string) => `Es el lado ${q}`,
  abPergunta: "Escucha y elige: ¿qué lado está sonando?",
  abResposta: (q: string, ok: boolean) =>
    ok ? `Era el lado ${q} — acertaste.` : `Era el lado ${q} — esta vez no.`,
  abBlind: "Prueba a ciegas (oculta el lado hasta la respuesta)",
  abDeNovo: "Escuchar de nuevo",
  abNivel: (q: string, n: string) => `Lado ${q}: ${n}`,
  abDelta: (d: string) => `Diferencia de nivel (A − B): ${d} puntos`,
  abNivelMetodo:
    "Nivel = media de las posiciones de los controles de SALIDA de la cadena (0–100), la misma clasificación del asistente de gain. El aparato no expone dB por SysEx: el número dice qué tan abierto está el botón, no qué suena — por eso la calibración iguala botones, no ganancia.",
  abNivelSemControle:
    "Un lado no tiene control de nivel: no hay nada que calibrar, y la comparación solo vale si el volumen del amplificador queda igual.",
  abCalibrar: "Igualar nivel",
  abAlgoritmos: (n: number) =>
    `${n} slot(s) con algoritmo cambiado no llegan al aparato (el change-effect 0x47 no tiene formato capturado) — el cambio vale en pantalla.`,
  abErrLoad: "No se pudieron leer las versiones de este patch.",
  abRelatoNada: "Los dos lados son idénticos: no hay nada que cambiar.",
  abRelatoLocal: (n: number) =>
    `${n} knob(s) cambiaron en el escenario; en esta build de solo lectura el aparato no los recibe (ADR-5).`,
  abRelatoEnviado: (n: number) => `${n} knob(s) enviados al aparato.`,
  abRelatoCalibra: (n: number, antes: string, depois: string) =>
    `${n} control(es) de nivel igualado(s): diferencia de ${antes} a ${depois} puntos.`,

  errRetry: "Reintentar",
  errRetryAria: "Reintentar la operación que falló",
};