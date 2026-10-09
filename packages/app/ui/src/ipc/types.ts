/**
 * Tipos de IPC — espelhos manuais dos DTOs do backend.
 * O ADR futuro pode trocar por geração (ts-rs/specta); o CONTRATO destes
 * tipos (campos/semântica) não muda — os testes serde do backend (camelCase)
 * travam os dois lados.
 */

/** Resultado do command `device_info` (mock). */
export interface DeviceInfo {
  /**
   * Backend ativo: `mock` no build de desenvolvimento, `real` no build de
   * campo (`--features real-device`), `none` no app SEM aparelho (issue
   * #150). Nunca mais um literal do backend — ver `docs/REAL_DEVICE_GAP.md` §1.
   */
  backend: "mock" | "real" | "none";
  /**
   * Motivo humano quando `backend` é `"none"` (issue #150): "aparelho não
   * conectado via USB…" ou "build sem o transporte…". Vazio com a sessão
   * viva — a UI não inventa aviso onde o device responde.
   */
  detail: string;
  /**
   * Nº de presets. No mock, o do `all.prst` (99). No aparelho real, o
   * inventário que o boot percorreu — que hoje é o default `0..198`, não
   * uma contagem descoberta no aparelho (`REAL_DEVICE_GAP.md` §4.3).
   */
  presetCount: number;
  /** pp corrente (u16 big-endian no protocolo; aqui como número). Lido da Session nos dois backends. */
  currentPp: number;
  /**
   * Nome do pp corrente.
   *
   * **VAZIO com o aparelho real:** vem da página meta6 (`13010001`), cujo
   * layout ainda não foi decifrado. A UI precisa tratar vazio como
   * "desconhecido" e não como um preset sem nome — este é o contrato que a
   * integridade do editor depende.
   */
  currentName: string;
  /** ppType (nº do tipo "Rock" etc. — semântica no dicionário do core). Só no mock. */
  currentPpType: number;
  /** Slots de IR com CRC de fábrica (mock: 20). **Só no mock** — o fio não expõe CRC. */
  irSlotsWithCrc: number;
  /**
   * Tabela dos 20 User IRs (§13.12) **lida do device** — a fonte de
   * verdade de "o que está no aparelho", nos dois backends. Vazio antes do
   * boot.
   */
  irSlots: IrSlot[];
  /**
   * Binário compilado com `write-verified` (ADR-5)? `false` no build de
   * campo de leitura: os botões de escrita ficam desabilitados com
   * explicação, em vez de o operador descobrir a recusa depois de clicar.
   */
  writeVerified: boolean;
}

/**
 * Um frame que o aparelho **receberia**, sem receber (o `--dry-run` do CLI,
 * agora no backend).
 *
 * O hex é o mesmo que sairia pelo `send_raw`. E o backend que monta — o
 * front nunca monta SysEx (porta única: `ipc/` não importa o core) — então
 * o preview e o envio não podem divergir.
 */
export interface PreviewFrame {
  /** `func` + addr do frame (ex.: "12/10030002"). */
  label: string;
  /** O SysEx completo em hex. */
  hex: string;
}

/** A operação que `devicePreview` sabe descrever. */
export type PreviewOp =
  /** Um knob da cadeia (1 frame, §13.11). */
  | { op: "setParam"; slot: number; code: number; ctrl: number; value: number }
  /** A gravação do preset (9 frames, §13.12). */
  | { op: "save"; pp: number; ppType: number; name: string };

/** O que o `dump-preset` devolve: meta6 + as 8 páginas, em hex cru (§13.9). */
export interface DumpReport {
  /** pp que foi lido. */
  pp: number;
  /** Página meta6 do pp (6B). */
  meta6: string;
  /** Páginas 0..=7 e a final de 4B, na ordem do fio. */
  pages: string[];
}

/**
 * Um slot da tabela de User IRs, como o aparelho a relata (§13.12).
 *
 * Sem `export`: chega ao front por dentro de `DeviceInfo.irSlots`, e ninguém
 * importa o nome sozinho.
 */
interface IrSlot {
  /** Slot 0..=19. */
  slot: number;
  /** Nome ASCII do IR no aparelho ("" = vazio). */
  name: string;
}

/** Etapas do script de boot — literais do backend (BootProgressDto). */
type BootStage =
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
  /** Índice do preset de FÁBRICA (0..98, o cursor do device); −1 quando o
   *  patch aberto é de usuário (patches locais não existem no device). */
  pp: number;
  name: string;
  ppType: number;
  ppTypeName: string;
  slots: BoardSlot[];
  /** Banco de origem do patch (#11): fábrica (device) ou usuário (local). */
  bank: "factory" | "user";
  /** Etiqueta exibida na UI: "P25" (fábrica, 1-based) ou "U01" (usuário). */
  ppLabel: string;
}

/** Entrada da biblioteca de presets (flight case). */
interface PresetEntry {
  pp: number;
  name: string;
  ppTypeName: string;
}

/**
 * Biblioteca completa + pp corrente. **No aparelho (#150)**: as entradas são
 * o inventário medido (ADR-12) com nome vazio até o decode das páginas (#152)
 * — vazio é "o app ainda não sabe", e não um preset sem nome.
 */
export interface PresetLibrary {
  entries: PresetEntry[];
  currentPp: number;
}

/**
 * Rótulo honesto de um pp do APARELHO (ADR-12): até o mapeamento
 * banco→LED (P/F) ser medido em campo, o rótulo é o ENDEREÇO (`0x0000`/
 * `0x0100`) — dizer "P42" seria afirmar um banco que ninguém mediu ainda.
 */
export function rotuloPp(pp: number): string {
  return `0x${pp.toString(16).padStart(4, "0")}`;
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
