/**
 * Tipos de IPC — espelhos manuais dos DTOs do backend.
 * O ADR futuro pode trocar por geração (ts-rs/specta); o CONTRATO destes
 * tipos (campos/semântica) não muda — os testes serde do backend (camelCase)
 * travam os dois lados.
 */

/** Resultado do command `device_info` (mock). */
export interface DeviceInfo {
  /** Backend ativo: mock por default (política ADR-4/ADR-5). */
  backend: "mock" | "real";
  /** Nº de presets do estado (mock: all.prst = 99). */
  presetCount: number;
  /** pp corrente (u16 big-endian no protocolo; aqui como número). */
  currentPp: number;
  /** Nome do pp corrente. */
  currentName: string;
  /** ppType (nº do tipo "Rock" etc. — semântica no dicionário do core). */
  currentPpType: number;
  /** Slots de IR com CRC de fábrica (mock: 20). */
  irSlotsWithCrc: number;
}

/** Etapas do script de boot — literais do backend (BootProgressDto). */
export type BootStage =
  | "tables"
  | "scan"
  | "probe"
  | "setlist"
  | "names"
  | "keepalive";

/** Beat de progresso do boot (evento `device://progress`, camelCase). */
export interface BootProgress {
  /** Etapa corrente do script de boot. */
  stage: BootStage;
  /** Transações completas até agora. */
  done: number;
  /** Total esperado do script. */
  total: number;
  /** pp corrente após a transação (o scan avança a seleção). */
  currentPp: number;
}

/** Resultado do command `device_boot` (BootReportDto). */
export interface BootReport {
  /** Nº de transações de boot+scan executadas (fallback dev: total simulado). */
  transactions: number;
}

/** Um knob do pedal (spec do dicionário + valor do preset). */
export interface BoardKnob {
  name: string;
  pos: number;
  kind: "knob" | "switch" | "combox";
  /** [min, max] — knobs bidirecionais podem ter min > max (0 = centro). */
  range?: [number, number];
  options: string[];
  value?: string;
  default?: string;
}

/** Um slot da cadeia do board (x = 0..8, ordem do sinal). */
export interface BoardSlot {
  slot: number;
  family: "PRE" | "DST" | "AMP" | "NR" | "CAB" | "EQ" | "MOD" | "DLY" | "RVB";
  archetype:
    | "BUFFER"
    | "DISTORTION"
    | "AMPLIFIER"
    | "NOISEGATE"
    | "CABINET"
    | "EQ"
    | "MODULATION"
    | "DELAY"
    | "REVERB";
  name: string;
  /** Slug do algoritmo real (ex.: "green-od") — escolhe o MODELO do pedal. */
  variant: string;
  state: boolean;
  code: number;
  knobs: BoardKnob[];
}

/** Ordem fixa da cadeia de sinal (slots 0..8) — única fonte no front. */
export const CHAIN_FAMILIES = ["PRE", "DST", "AMP", "NR", "CAB", "EQ", "MOD", "DLY", "RVB"] as const;
export type ChainFamily = (typeof CHAIN_FAMILIES)[number];

/** Arquétipo de render por família da cadeia (mesma ordem de CHAIN_FAMILIES). */
export const ARCHETYPE_OF: Record<ChainFamily, BoardSlot["archetype"]> = {
  PRE: "BUFFER",
  DST: "DISTORTION",
  AMP: "AMPLIFIER",
  NR: "NOISEGATE",
  CAB: "CABINET",
  EQ: "EQ",
  MOD: "MODULATION",
  DLY: "DELAY",
  RVB: "REVERB",
};

/** Board do preset (o pedalboard renderizado no index). */
export interface BoardView {
  pp: number;
  name: string;
  ppType: number;
  ppTypeName: string;
  slots: BoardSlot[];
}

/** Entrada da biblioteca de presets (flight case). */
export interface PresetEntry {
  pp: number;
  name: string;
  ppTypeName: string;
}

/** Biblioteca completa + corrente do mock. */
export interface PresetLibrary {
  entries: PresetEntry[];
  currentPp: number;
}

/** Entrada de log de pushes (evento `device://push` — log da UI). */
export interface PushLogEntry {
  /** Hex cru da mensagem (F0…F7), já validado/normalizado pelo ipc/push. */
  hex: string;
  /** Instante da chegada mais recente (Date.now() do front, só exibição). */
  at: number;
  /** Repetições CONSECUTIVAS do mesmo hex (1 = linha única — boot repete muito). */
  count: number;
}

/** Estado de tela canônico: idle → loading → ready | error. */
export type ScreenState<D> =
  | { kind: "idle" }
  | { kind: "loading"; from?: ScreenState<D> }
  | { kind: "empty"; hint: string }
  | { kind: "error"; message: string; retry: () => void }
  | { kind: "ready"; data: D };
