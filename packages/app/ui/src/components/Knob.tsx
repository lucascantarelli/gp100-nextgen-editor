/**
 * Knob SVG — controle rotativo paramétrico do pedal: arrasto vertical
 * ajusta o valor dentro do range do dicionário; duplo clique reseta ao
 * default; setas ↑/↓ e PageUp/PageDown no teclado; switch/combox cicla
 * as opções por clique. Acessível (role slider, aria-valuenow, foco
 * visível) e sem dependências externas.
 *
 * `locked` = controles do PALCO (U-3): o pedal compacto é DISPLAY — mostra
 * o valor de cada knob (e o tooltip), mas quem ajusta é o modal de edição
 * (`role="img"`, sem foco e sem handlers). Assim o clique em qualquer ponto
 * do pedal abre a edição, sem disputar o gesto do knob.
 */
import { useCallback, useRef } from "react";
import type { BoardKnob } from "../ipc/types";

interface Props {
  knob: BoardKnob;
  size?: number;
  accent?: string;
  /** Modo engenheiro: enriquece o tooltip com o endereço de memória do comando SET. */
  engineer?: boolean;
  /** Só leitura (palco): sem foco, sem handlers — a edição mora no modal. */
  locked?: boolean;
  /** Endereço de memória do comando SET (ex.: "10 01 00 02") — p/ tooltip e etiqueta. */
  addr?: string;
  /** effectCode do algoritmo em hex (ex.: "0x0700006e"). */
  codeHex?: string;
  /** Notifica o novo valor (string cru, mesmo formato do preset). */
  onChange: (pos: number, value: string) => void;
  onReset: (pos: number) => void;
}

/** Formata com 1 decimal quando o range é fracionário. */
function formatValue(v: number, frac: boolean): string {
  return frac ? (Math.round(v * 10) / 10).toFixed(1) : String(Math.round(v));
}

export function Knob({
  knob,
  size = 64,
  accent = "#ffb020",
  engineer = false,
  locked = false,
  addr,
  codeHex,
  onChange,
  onReset,
}: Props) {
  const dragging = useRef<{ y: number; norm: number } | null>(null);

  const frac =
    knob.kind === "knob" &&
    knob.range != null &&
    (!Number.isInteger(knob.range[0]) || !Number.isInteger(knob.range[1]));

  // Normalizado 0..1 dentro da faixa (knobs bidirecionais min>max são
  // invertidos: 0 do protocolo = centro físico).
  const [min, max] = knob.range ?? [0, 100];
  const inverted = min > max;
  const lo = Math.min(min, max);
  const hi = Math.max(min, max);
  const num = Number(knob.value ?? knob.default ?? lo) || 0;
  const norm = hi === lo ? 0 : Math.min(1, Math.max(0, (num - lo) / (hi - lo)));

  const clamp = useCallback((n: number) => Math.min(1, Math.max(0, n)), []);

  const onPointerDown = (e: React.PointerEvent<SVGSVGElement>) => {
    if (knob.kind !== "knob") return;
    e.preventDefault();
    (e.target as Element).setPointerCapture?.(e.pointerId);
    dragging.current = { y: e.clientY, norm };
  };
  const onPointerMove = (e: React.PointerEvent<SVGSVGElement>) => {
    if (!dragging.current) return;
    const dy = dragging.current.y - e.clientY;
    const next = clamp(dragging.current.norm + dy / 260);
    const val = lo + next * (hi - lo);
    onChange(knob.pos, formatValue(inverted ? hi - next * (hi - lo) : val, frac));
  };
  const onPointerUp = () => {
    dragging.current = null;
  };

  // Teclado: passos de 1% (5% com Shift), Home/End nos extremos.
  const step = (dir: 1 | -1, big: boolean) => {
    const delta = (big ? 0.05 : 0.01) * dir;
    const next = clamp(norm + delta);
    onChange(
      knob.pos,
      formatValue(inverted ? hi - next * (hi - lo) : lo + next * (hi - lo), frac),
    );
  };

  // switch/combox: clique cicla opções
  const cycle = () => {
    if (knob.kind === "knob" || knob.options.length === 0) return;
    const idx = knob.options.findIndex((o) => o === knob.value);
    const next = knob.options[(idx + 1) % knob.options.length];
    onChange(knob.pos, next);
  };

  const angle = -135 + norm * 270;
  const c = size / 2;
  const r = c - 5;

  return (
    <svg
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      role={locked ? "img" : knob.kind === "knob" ? "slider" : "button"}
      aria-label={locked ? `${knob.name}: ${knob.value ?? "—"}` : knob.name}
      aria-valuenow={!locked && knob.kind === "knob" ? num : undefined}
      aria-valuemin={!locked && knob.kind === "knob" ? lo : undefined}
      aria-valuemax={!locked && knob.kind === "knob" ? hi : undefined}
      tabIndex={locked ? undefined : 0}
      style={{
        display: "block",
        outline: "none",
        cursor: locked ? "default" : knob.kind === "knob" ? "ns-resize" : "pointer",
        touchAction: locked ? undefined : "none",
      }}
      onPointerDown={locked ? undefined : onPointerDown}
      onPointerMove={locked ? undefined : onPointerMove}
      onPointerUp={locked ? undefined : onPointerUp}
      onKeyDown={(e) => {
        if (knob.kind !== "knob") return;
        if (e.key === "ArrowUp" || e.key === "ArrowRight") { e.preventDefault(); step(1, e.shiftKey); }
        if (e.key === "ArrowDown" || e.key === "ArrowLeft") { e.preventDefault(); step(-1, e.shiftKey); }
        if (e.key === "PageUp") { e.preventDefault(); step(1, true); }
        if (e.key === "PageDown") { e.preventDefault(); step(-1, true); }
        if (e.key === "Home") { e.preventDefault(); onChange(knob.pos, formatValue(lo, frac)); }
        if (e.key === "End") { e.preventDefault(); onChange(knob.pos, formatValue(hi, frac)); }
      }}
      onDoubleClick={locked ? undefined : () => onReset(knob.pos)}
      onClick={locked ? undefined : cycle}
    >
      <defs>
        <radialGradient id={`kb-${knob.pos}`} cx="35%" cy="30%">
          <stop offset="0%" stopColor="#4a4f58" />
          <stop offset="100%" stopColor="#191c22" />
        </radialGradient>
        {/* luz do design system (#9): luz em CIMA/esquerda — o vidro do
            knob recebe um gradiente de topo e um brilho especular */}
        <linearGradient id={`kshine-${knob.pos}`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#ffffff" stopOpacity="0.5" />
          <stop offset="55%" stopColor="#ffffff" stopOpacity="0.05" />
          <stop offset="100%" stopColor="#ffffff" stopOpacity="0" />
        </linearGradient>
      </defs>
      {/* marcações */}
      {[-135, -90, -45, 0, 45, 90, 135].map((a) => {
        const rad = (a * Math.PI) / 180;
        const x1 = c + Math.sin(rad) * (r + 1);
        const y1 = c - Math.cos(rad) * (r + 1);
        const x2 = c + Math.sin(rad) * (r + 4);
        const y2 = c - Math.cos(rad) * (r + 4);
        return <line key={a} x1={x1} y1={y1} x2={x2} y2={y2} stroke="#77808c" strokeWidth="1.2" />;
      })}
      {/* corpo (com sombra de contato projetada na placa) */}
      <circle
        cx={c}
        cy={c}
        r={r}
        fill={`url(#kb-${knob.pos})`}
        stroke="#0c0e12"
        strokeWidth="1.8"
        style={{ filter: "drop-shadow(0 2px 3px rgba(0,0,0,.5))" }}
      />
      {/* vidro iluminado por cima + brilho ESPECULAR pontual + recorte */}
      <circle cx={c} cy={c} r={r - 1} fill={`url(#kshine-${knob.pos})`} />
      <ellipse
        cx={c - r * 0.32}
        cy={c - r * 0.46}
        rx={r * 0.3}
        ry={r * 0.17}
        fill="#ffffff"
        opacity="0.22"
      />
      <circle
        cx={c}
        cy={c}
        r={r - 0.7}
        style={{ fill: "none", stroke: "var(--light-rim)" }}
        strokeWidth="1"
      />
      <circle cx={c} cy={c} r={Math.max(1, r - 14)} fill="none" stroke="#000" strokeOpacity="0.35" strokeWidth="1" />
      {/* indicador */}
      <line
        x1={c}
        y1={c}
        x2={c + Math.sin((angle * Math.PI) / 180) * (r - 7)}
        y2={c - Math.cos((angle * Math.PI) / 180) * (r - 7)}
        stroke={accent}
        strokeWidth="3.4"
        strokeLinecap="round"
      />
      <title>
        {engineer && addr
          ? `${knob.name}: ${knob.value ?? "—"} · SET · addr ${addr} · code ${codeHex ?? "?"} · ctrl ${knob.pos} · payload [code u32 LE][ctrl][00][f32 LE]`
          : locked
            ? `${knob.name}: ${knob.value ?? "—"} · clique no pedal para editar`
            : `${knob.name}: ${knob.value ?? "—"} · duplo clique = default`}
      </title>
    </svg>
  );
}
