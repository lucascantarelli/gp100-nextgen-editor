/**
 * Tipos de IPC — espelhos manuais dos DTOs do backend (M1.0: `device_info`).
 * O ADR do M1.0 pode trocar por geração (ts-rs/specta); o CONTRATO destes
 * tipos (campos/semântica) não muda.
 */

/** Resultado do command `device_info` (M1.0 — mock). */
export interface DeviceInfo {
  /** Backend ativo: mock por default (política ADR-4/ADR-5). */
  backend: "mock" | "real";
  /** Nº de presets do estado (mock: all.prst = 99). */
  presetCount: number;
  /** pp corrente (u16 BE no fio; aqui como número). */
  currentPp: number;
  /** Nome do pp corrente. */
  currentName: string;
  /** ppType (nº do tipo "Rock" etc. — semântica no dicionário do core). */
  currentPpType: number;
  /** Slots de IR com CRC de fábrica (mock: 20). */
  irSlotsWithCrc: number;
}

/** Estado de tela canônico (docs/UI_DESIGN.md §5). */
export type ScreenState<D> =
  | { kind: "idle" }
  | { kind: "loading"; from?: ScreenState<D> }
  | { kind: "empty"; hint: string }
  | { kind: "error"; message: string; retry: () => void }
  | { kind: "ready"; data: D };
