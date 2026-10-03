/**
 * EN — dicionário English. `DictPatch` de propósito: o que faltar aqui
 * cai no pt-BR (merge profundo em `../messages.ts`), então um idioma pela
 * metade nunca mostra chave crua — degrada para a língua garantida.
 *
 * O que NÃO é traduzido, e por quê:
 *  - termos que o próprio device imprime (PRE/POST, REC, PLAY, DUB, STOP,
 *    BPM, REW, FREQ/Q/GAIN, SET) — traduzi-los desalinearia a UI do que está
 *    na frente do usuário;
 *  - marca e nome de modelo (GP-100, Valeton) e os nomes de ritmo/efeito que
 *    vêm dos artefatos gerados.
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

export const EN: DictPatch = {
  /* ── brand / topbar ── */
  brand: "GP-100",
  brandMono: "GP",
  tagline: "unofficial editor",
  connShortOn: "on",
  connShortOff: "off",
  mockBadge: "Mock Device",
  patchLabel: "patch",
  patchPrevAria: "Previous patch",
  patchPrevTitle: "Previous patch (P01–P99 cycling)",
  patchNextAria: "Next patch",
  patchNextTitle: "Next patch (P01–P99 cycling)",
  drumChipPre: "drum · ",
  drumChipAria: (style: string, bpm: number, beat: string) =>
    `Drum: ${style}, ${bpm} BPM, ${beat} time signature — open pattern manager`,
  drumToggleAria: "Play or stop the drum pattern",
  drumBpmDownAria: "Decrease the pattern BPM",
  drumBpmUpAria: "Increase the pattern BPM",
  masterLabel: "master",
  masterAria: "Master volume",
  arrangeLabel: "🔒",
  arrangeLabelOpen: "🔓",
  arrangeAria: "Pedal move lock — drag and drop only when unlocked",
  arrangeTitle: "Unlock to drag pedals (protects the knob values)",
  killLabel: "⭘ kill",
  killLabelOn: "⏻ killed",
  killAria: "Kill switch (mute master, drum and looper)",
  killTitle: "Global mute: turns off master and drum — click again to restore",
  openSettingsAria: "Open settings",

  /* ── navbar (connection cluster: short status + badge + boot) ── */
  connClusterAria: "Connection and boot",

  /* ── stage tuner ── */
  tunerAria: "Tuner",
  tunerPowerAria: "Turn the tuning monitor on or off",
  tunerPowerIcon: "♪",
  tunerPowerTitle: (on: boolean) =>
    on
      ? "Tuning monitor ON — shows what you play in real time (on the device: hold both footswitches)"
      : "Tuning monitor off — click to follow the tuning in real time (on the device: hold both footswitches)",
  tunerModeAria: "Tuner mode — toggle between bypass, thru and mute",
  tunerModeTitle:
    "What the device does with the signal while you tune: bypass (dry), thru (with effect) or mute (silent)",
  tunerModeLabel: (mode: "bypass" | "thru" | "mute") => mode,
  tunerRefLabel: "ref",
  tunerRefAria: "Reference pitch (A4) in hertz, from 435 to 445",
  tunerRefTitle: "REF PITCH: 435–445 Hz (440 Hz standard)",
  tunerRefValue: (hz: number) => `${hz}Hz`,
  tunerDemoLabel: "▶ demo",
  tunerDemoRunning: "■ demo",
  tunerDemoAria: "Play the tuner demo with a synthetic tone",
  tunerDemoTitle:
    "Synthetic sample sweeping ±30 cents on A2 through the real tuner engine — not device audio",
  tunerIdleNote: "—",
  tunerFlatMark: "♭",
  tunerSharpMark: "♯",

  /* ── navbar (boot = rescan; the app detects the device on its own) ── */
  rescanAria: "Rescan device",
  rescanTitle: "Rescan (boot): the app detects the device on its own at startup — use this to rescan",
  bootProgressAria: "Boot progress",
  bootStarting: "Starting",
  bootStages: {
    tables: "IR tables",
    scan: "Preset scan",
    probe: "Bank 02 probe",
    setlist: "Setlist",
    names: "Names",
    keepalive: "Keepalive",
  },
  connBootError: "Device boot failed — check the connection and try again.",

  /* ── board ── */
  boardAria: "Pedalboard (9 chain slots)",
  slotAria: (n: number, fam: string) => `Slot ${n}: ${fam}`,
  slotEmpty: "empty",
  slotArrange: "drag ⇄",
  stageFooter: "9 slots · signal order →",
  stageFooterArrange: "drag ⇄ to swap two pedals' positions",
  stageIn: "⏻ IN",
  stageOut: "OUT ⏻",

  /* ── library ── */
  libAria: "Preset library",
  libTabsAria: "Patch type",
  libTabFactory: "Factory Patch",
  libTabUser: "User Patch",
  libListAria: "Factory presets",
  searchPlaceholder: "Search name, number or style…",
  searchAria: "Search presets by name, number or style",
  searchEmpty: (q: string) => `No preset matches “${q}”. Tip: search by style (Rock, Funk…) or number.`,
  libRowTitle: (name: string, type: string) => `Open “${name}” (${type})`,
  libPp: (pp: number) => `P${String(pp + 1).padStart(2, "0")}`,
  libUserPp: (i: number) => `U${String(i + 1).padStart(2, "0")}`,

  /* user patches (LOCAL PREVIEW — writing to the device has no channel yet) */
  userPatchNewPlaceholder: "Name and save the current patch…",
  userPatchNameAria: "Name of the user patch to save",
  userPatchSave: "Save",
  userPatchListAria: "User patches",
  userPatchEmpty: "No patch saved yet. Adjust the pedals and save whatever you want to keep.",
  userPatchRowTitle: (name: string, from: string) => `Open “${name}” (from ${from})`,
  userPatchDeleteAria: (name: string) => `Delete the patch “${name}”`,
  userPatchNote: "Local preview: patches stay in this browser. Saving to the device depends on the USB channel.",
  userPatchDefaultName: (n: number) => `My patch ${n + 1}`,
  userBankType: "User",

  /* per-module effect list */
  effectListTitle: "Effects List",
  effectListAria: (fam: string) => `Effect list for the ${fam} module`,
  effectListCount: (n: number) => `${n} effects`,
  effectListSearchPlaceholder: "Search effect…",
  effectListSearchAria: "Search this module's effects by name",
  effectListEmpty: (q: string) => `No effect matches “${q}”.`,
  effectListPick: (name: string) => `Switch this pedal's effect to ${name}`,
  effectListCurrent: (name: string) => `${name} — this pedal's current effect`,
  effectListNote: "Local preview: switching the effect is not written to the device yet.",

  /* ── drum ── */
  drumNote: `${DRUM_COUNT} patterns in ${DRUM_GENRE_COUNT} genres · metronome included · local preview`,
  drumPanelAria: "Drum pattern manager",
  drumTitle: "Drum · patterns",
  drumCloseAria: "Close the pattern manager",
  drumGenreLabel: "Genre",
  drumStyleLabel: "Pattern",
  drumBpmLabel: "BPM",
  drumBeatLabel: "Time signature",
  drumVolLabel: "Volume",
  drumSpeedLabel: "Speed",

  /* ── looper ── */
  looperPlate: "GP-100 · TAPE LOOPER · STEREO",
  looperAria: "Looper (tape machine)",
  looperSpecs: "stereo 24-bit loop · 44.1 kHz · SNR 110 dB · local preview",
  tapeState: (secs: number, mode: "PRE" | "POST") => `tape: ${secs}s (${mode})`,
  tapeEmpty: "tape: empty",
  looperMode: { rec: "REC", play: "PLAY", dub: "DUB", stop: "STOP", ready: "READY", empty: "EMPTY" },
  looperTransportAria: "Looper transport",
  looperTimerAria: "Tape position",
  looperRewAria: "Rewind to the start of the loop (REW)",
  looperStopAria: "Stop (STOP)",
  looperPlayAria: "Play loop (PLAY)",
  looperRecAria: "Record loop (REC)",
  looperRecStopAria: "Stop recording and play",
  looperDubAria: "Overdub (overdub)",
  looperClearAria: "Clear tape (asks for confirmation)",
  looperClearConfirmAria: "Confirm clearing the tape",
  looperClearBtn: "✕",
  looperClearConfirmBtn: "ok?",
  looperRecVol: "Rec VOL",
  looperPlayVol: "Play VOL",
  looperPVol: "P-VOL",
  looperRoute: "Route",
  looperPreBtn: "PRE · 90s",
  looperPostBtn: "POST · 45s",
  looperPreAria: "Looper in PRE (90 seconds, no effects recorded)",
  looperPostAria: "Looper in POST (45 seconds, with effects)",
  looperVuAria: (channel: string, active: boolean) =>
    `VU meter channel ${channel}${active ? " with signal" : " at rest"}`,
  looperDeckAria: (spinning: boolean) =>
    `Tape deck: supply reel, heads and capstan, take-up reel${spinning ? ", moving" : ", stopped"}`,
  reelSupply: "Supply",
  reelTakeup: "Take-up",

  /* ── pushes ── */
  pushTitle: "Device pushes",
  pushSummary: "device pushes",
  pushEmpty: "No push received yet.",
  pushClear: "Clear",
  pushListAria: "Push log",
  pushRepeats: "Consecutive repeats of this push",
  pushRepeatMark: "×",

  /* ── settings ── */
  previewBadge: "local preview",
  settingsTitle: "Settings",
  settingsDialogAria: "Settings",
  settingsTabsAria: "Settings tabs",
  settingsCloseAria: "Close settings",
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
  engineerModeLabel: "Engineer mode",
  engineerModeSub: "knob tooltips show the addr/code/ctrl of the SET command",
  engineerModeAria: "Engineer mode — tooltips with the addr, code and ctrl of the SET command",
  hintLeft: "Left",
  hintRight: "Right",
  tapTempoLabel: "Tap Tempo Mode",
  tapTempoLabels: { pre: "PRE", mod: "MOD", dly: "DLY" },
  footswitchLabel: "Footswitch Mode",
  footswitchSub: "hardware footswitch behaviour",
  footswitchAria: "Footswitch mode (coming in a future version)",
  footswitchSoon: "— coming in a future version —",
  languageLabel: "APP Language",
  languageAria: "App language",
  globalEqIntro:
    "Device-wide equalisation: 5 bands (FREQ · Q · GAIN) plus low/high cuts. The controls become available in a future version.",
  eqBandGroupAria: "Global EQ band",
  eqBandBtn: (n: number) => `B${n}/5`,
  eqFreqLabel: (n: number) => `Band ${n} FREQ`,
  eqQLabel: (n: number) => `Band ${n} Q`,
  eqGainLabel: (n: number) => `Band ${n} GAIN`,
  eqFreqAria: (n: number) => `Band ${n} freq`,
  eqQAria: (n: number) => `Band ${n} Q`,
  eqGainAria: (n: number) => `Band ${n} gain`,
  lcutLabel: "L-CUT FREQ",
  hcutLabel: "H-CUT FREQ",
  lcutAria: "L-CUT freq",
  hcutAria: "H-CUT freq",
  aboutIntro:
    "GP-100 NextGen Editor — an independent, unofficial editor for the Valeton GP-100 pedalboard. Not affiliated with Valeton.",
  aboutDataAria: "Device data in this version",
  aboutData: [
    ["Library", `${PRESET_COUNT} factory presets (from device memory)`],
    ["Drum", `${DRUM_COUNT} patterns in ${DRUM_GENRE_COUNT} genres + metronome · time signatures ${DRUM_BEAT_RANGE}`],
    ["Looper", "24-bit stereo · 44.1 kHz · SNR 110 dB — 90 s PRE / 45 s POST"],
    ["Catalogue", `${FX_COUNT} mapped algorithms / ${CTRL_COUNT} mapped controls`],
  ],
  infoFrameIntro: "Device identification, read on connect.",
  infoFrameAria: "Device identification",
  infoFrame: [
    ["Firmware", "V2.1"],
    ["Software", "1.2.0"],
    ["Model", "Valeton GP-100 Multi-Effects Processor"],
    ["Connection", "USB"],
  ],
  helpShortcutsIntro: "Global keyboard shortcuts (work anywhere in the shell):",
  helpKeySpace: "Space",
  helpKeyR: "R",
  helpKeyEsc: "Esc",
  helpShortcutSpace: "Drum: play/stop the selected pattern",
  helpShortcutR: "Looper: REC — same as the ● button (record → play → overdub)",
  helpShortcutEsc: "Closes the panel open at the top, back down to the base: Settings → Drum → pushes",
  helpTableAria: "Keyboard shortcuts",
  helpNotes:
    "Notes: transport shortcuts do not fire while you type in a search box, a BPM field or a select; Space on a focused button activates THAT button (native accessibility behaviour); Ctrl/Alt/⌘ + key is ignored; with the Settings modal open, Space and R are disabled and Esc closes the modal.",
  helpControlsIntro: "Controls:",
  helpControls: [
    "Tab moves through every control; focus is always visible",
    "Tuner (stage header): ♪ turns the monitor on/off (green/red LED); mode and REF PITCH are always visible; ▶ demo turns the monitor on and plays a synthetic sample",
    "Esc closes this modal (global — from any focus)",
  ],
  sysInfoTitle: "System information",
  sysInfoAria: "App and environment versions",
  sysInfoHint: "Read only — none of this is sent to any server.",
  sysInfo: [
    ["App version", "0.1.0"],
    ["React", REACT_VER],
    ["Backend", "simulated (mock) — the real device feeds everything over USB"],
    ["Device firmware", "V2.1 (local preview)"],
    ["Browser/OS", UA],
  ],
  sysNote:
    "In the installed app (Tauri) the version comes from the package; in a browser it is the development version.",
  releaseNote:
    `Preview version: factory library (${PRESET_COUNT} presets), drum with ${DRUM_COUNT} patterns, tape looper (90 s PRE / 45 s POST), stage tuner with continuous monitoring (green/red LED, REF PITCH 435–445 Hz), keyboard shortcuts and responsive layout. The board already renders the pedals of the preset — on the stage each pedal is compact and the knobs are READ ONLY (they show each control's value): click the pedal to open the enlarged editor, and whatever you change there shows up on the stage right away.`,

  /* ── pedals ── */
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
    `${kind} pedal — ${name} (${on ? "on" : "off"})`,
  pedalToggleOff: "Turn the effect off",
  pedalToggleOn: "Turn the effect on",
  pedalValueAria: "Value (Enter to edit)",
  pedalValueEditAria: "Knob value (Enter applies, Esc cancels)",
  pedalExpandTitle: "Pedal values on the stage are read only — click (or press Enter) to edit",
  pedalModalAria: (name: string) => `Editing the ${name} pedal`,
  pedalModalHint: "Esc closes · changes apply immediately",
  pedalModalCloseAria: "Close pedal editing",
  brandPlate: "GP-100",
  onSuffix: " ON",

  /* ── friendly errors (technical detail goes to the console) ── */
  errOpenPreset: "Could not open the preset — try again.",
  errSelectPreset: "The device did not accept the preset switch — the UI stays on the current preset.",
  errSetParam: "The device did not accept the knob change — try again.",
  errRetry: "Try again",
  errRetryAria: "Retry the operation that failed",
};