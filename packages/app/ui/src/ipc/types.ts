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
  /** pp corrente (u16 BE no fio; aqui como número). */
  currentPp: number;
  /** Nome do pp corrente. */
  currentName: string;
  /** ppType (nº do tipo "Rock" etc. — semântica no dicionário do core). */
  currentPpType: number;
  /** Slots de IR com CRC de fábrica (mock: 20). */
  irSlotsWithCrc: number;
}

/** Etapas do script de boot (§13.10) — literais do backend (BootProgressDto). */
export type BootStage =
  | "tables"
  | "scan"
  | "probe"
  | "setlist"
  | "names"
  | "keepalive";

/** Beat de progresso do boot (evento `device://progress`, camelCase). */
export interface BootProgress {
  /** Etapa corrente do script (§13.10). */
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
  /** Nº de transações de boot+scan executadas com sucesso. */
  transactions: number;
}

/** Entrada de log de pushes (evento `device://push` — log da UI). */
export interface PushLogEntry {
  /** Hex cru da mensagem (F0…F7). */
  hex: string;
  /** Instante da chegada (Date.now() do front, só para exibição). */
  at: number;
}

/** Estado de tela canônico (docs/UI_DESIGN.md §5). */
export type ScreenState<D> =
  | { kind: "idle" }
  | { kind: "loading"; from?: ScreenState<D> }
  | { kind: "empty"; hint: string }
  | { kind: "error"; message: string; retry: () => void }
  | { kind: "ready"; data: D };
