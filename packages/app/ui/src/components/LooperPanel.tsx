/**
 * LooperPanel — o loop da GP-100 como MÁQUINA DE FITA, em alumínio anodizado
 * violeta (issue #12). O deck inteiro (rolos, cabeçotes, capstan e o caminho
 * da fita) é UM SVG só: separados em vários <svg> os pedaços não se encaixam
 * quando a coluna muda de largura e a fita não consegue "nascer" num rolo e
 * morrer no outro.
 *
 * A peça que não existia e que dá o carácter da máquina: o PACOTE DE FITA é
 * dirigido pelo estado (o supply esvazia e o take-up enche conforme os
 * segundos passam). A máquina conta o que está gravando.
 *
 * A cor também mudou: a versão anterior era sépia/bege — um deck de 1990
 * dentro de uma casca violeta. O metal agora é violeta (a identidade do
 * projeto) e o âmbar fica reservado ao ANALÓGICO: a fita e a luz de fundo do
 * VU. Violeta é digital, âmbar é analógico — é a regra do painel inteiro.
 *
 * Dados REAIS extraídos:
 *  - firmware V2.1 (menu LOOPER): Rec VOL · Play VOL · Pre/Post · P-VOL;
 *  - specs oficiais Valeton: 90 s em PRE / 45 s em POST (stereo com efeitos),
 *    24-bit · 44.1 kHz · SNR 110 dB; Rec level 0–99.
 *
 * PRÉVIA LOCAL: o looper é estado do device (a escrita via USB ainda não
 * está implementada); volumes/rota ficam em localStorage, transporte é de sessão.
 */
import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { MSG } from "../i18n/messages";

export interface LooperSettings {
  recVol: number;
  playVol: number;
  pVol: number;
  pre: boolean; // true = PRE (90 s) · false = POST (45 s, com efeitos)
}

const KEY = "gp100.looper.v1";
export const LOOP_SECONDS_PRE = 90;
export const LOOP_SECONDS_POST = 45;

export function loadLooper(): LooperSettings {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) return { ...{ recVol: 80, playVol: 80, pVol: 80, pre: true }, ...(JSON.parse(raw) as Partial<LooperSettings>) };
  } catch {
    /* teste */
  }
  return { recVol: 80, playVol: 80, pVol: 80, pre: true };
}

type Mode = "idle" | "rec" | "play" | "dub" | "stop";

/* ── geometria do deck (unidades do viewBox 560×170) ────────────────────
   Proporção 3,3:1 DE PROPÓSITO. A versão anterior era 2,7:1 e o painel
   ficou 90% mais alto que o looper antigo — a máquina enchia a dobra da
   tela. Deck largo e baixo é o que uma máquina de fita de verdade é. */
const DECK_W = 560;
const DECK_H = 170;
const REEL_CY = 80;
const SUPPLY_CX = 100;
const TAKEUP_CX = 460;
const REEL_R = 76; // flange
const PACK_MIN = 32; // rolo vazio
const PACK_MAX = 66; // rolo cheio
const TAPE_Y = 154; // linha da fita nos cabeçotes
const HEAD_X0 = 210; // início da reta da fita sobre os cabeçotes
const HEAD_X1 = 350; // fim da reta
const HEAD_TOP = 104;
const HEAD_BOT = 166;
const HEAD_W = 34;
const HEAD2_X0 = 312;
/** fração gravada (0–1) → raio do pacote de fita de um rolo. Dirigido só
    pelos SEGUNDOS, nunca por `hasTape`: `hasTape` responde "existe loop
    gravado?", que é estado de sessão, enquanto o rolo tem que mostrar a
    fita SAINDO de um lado e CHEGANDO ao outro enquanto grava. Uma máquina
    parada aparece com o supply cheio e o take-up vazio — enfiada, com a
    fita montada. */
const packOf = (at: number): number => PACK_MIN + (PACK_MAX - PACK_MIN) * at;

/* ── 7 segmentos (contador) ─────────────────────────────────────────────
   Ordem: a, b, c, d, e, f, g — o desenho canônico. */
const SEG_LIT: Record<string, number[]> = {
  "0": [0, 1, 2, 3, 4, 5],
  "1": [1, 2],
  "2": [0, 1, 6, 4, 3],
  "3": [0, 1, 6, 2, 3],
  "4": [5, 6, 1, 2],
  "5": [0, 5, 6, 2, 3],
  "6": [0, 5, 6, 4, 2, 3],
  "7": [0, 1, 2],
  "8": [0, 1, 2, 3, 4, 5, 6],
  "9": [0, 1, 2, 3, 5, 6],
};
/** segmento horizontal (hexágono de pontas): a, g, d */
const hSeg = (cx: number, cy: number, hw = 13, ht = 3.2, t = 4): string =>
  `${cx - hw},${cy} ${cx - hw + t},${cy - ht} ${cx + hw - t},${cy - ht} ${cx + hw},${cy} ${cx + hw - t},${cy + ht} ${cx - hw + t},${cy + ht}`;
/** segmento vertical: b, c, e, f */
const vSeg = (cx: number, cy: number, hh = 10, ht = 3.2, t = 4): string =>
  `${cx},${cy - hh} ${cx + ht},${cy - hh + t} ${cx + ht},${cy + hh - t} ${cx},${cy + hh} ${cx - ht},${cy + hh - t} ${cx - ht},${cy - hh + t}`;
const DIGIT_SEGS = [
  hSeg(18, 4), // a
  vSeg(31, 17.5), // b
  vSeg(31, 44.5), // c
  hSeg(18, 58), // d
  vSeg(5, 44.5), // e
  vSeg(5, 17.5), // f
  hSeg(18, 31), // g
];
const DIGIT_STEP = 42; // passo entre dígitos
const DIGIT_X = [0, DIGIT_STEP, 104, 104 + DIGIT_STEP];

function SevenSeg({ text }: { text: string }) {
  /* A posição do dígito NÃO é o índice do caractere: o "mm:ss" tem 5
     caracteres e só 4 dígitos, então indexar pelo caractere colocava a
     vírgula no meio da lista e o último dígito caía fora — voltando
     para x=0 e se sobrepondo ao primeiro. Os segundos saíam AO CONTRÁRIO
     ("00:05" virava "50:00"). Aqui um contador próprio sóanda pelos
     dígitos. */
  let d = 0;
  return (
    <>
      {/* o texto acessível é o mm:ss real (é o que o leitor de tela e o
          teste leem); os segmentos são a pele, não a segunda informação */}
      <span className="lp-sr">{text}</span>
      <g aria-hidden="true">
        {text.split("").map((ch, i) => {
          if (ch === ":") {
            return (
              <g key={i}>
                <circle cx="91" cy="19" r="3.4" fill="#c084fc" />
                <circle cx="91" cy="43" r="3.4" fill="#c084fc" />
              </g>
            );
          }
          const x = DIGIT_X[d] ?? 0;
          d += 1;
          return (
            <g key={i} transform={`translate(${x},0)`}>
              {DIGIT_SEGS.map((pts, s) => (
                <polygon key={s} points={pts} fill={SEG_LIT[ch]?.includes(s) ? "#c084fc" : "#2a2038"} />
              ))}
            </g>
          );
        })}
      </g>
    </>
  );
}

/** um rolo: flange escovado, pacote de fita quente e 3 raios por cima.
    Sem furos recortados no flange — a fita aparece pelos VAZOS ENTRE OS
    RAIOS, que é como o rolo de verdade mostra o material enrolado. */
function Reel({ cx, packR }: { cx: number; packR: number }) {
  const spokes = [90, 210, 330];
  return (
    <g>
      <circle cx={cx} cy={REEL_CY} r={REEL_R} fill="#0b0e14" />
      {/* pacote de fita: anel quente entre o cubo e a borda */}
      <circle cx={cx} cy={REEL_CY} r={packR} fill={`url(#lp-tape-pack)`} />
      {[0.82, 0.68, 0.54, 0.42].map((k) => (
        <circle
          key={k}
          cx={cx}
          cy={REEL_CY}
          r={packR * k}
          fill="none"
          stroke="rgba(184,132,63,.32)"
          strokeWidth="1"
        />
      ))}
      <circle cx={cx} cy={REEL_CY} r={packR} fill="none" stroke="rgba(255,255,255,.18)" strokeWidth="1.5" />
      {/* flange: aro de metal por cima da fita, com canalheado */}
      <circle cx={cx} cy={REEL_CY} r={REEL_R} fill="none" stroke={`url(#lp-reel-metal)`} strokeWidth="16" />
      <circle cx={cx} cy={REEL_CY} r="70" fill="none" stroke="rgba(255,255,255,.1)" strokeWidth="1" />
      <circle cx={cx} cy={REEL_CY} r="63" fill="none" stroke="rgba(0,0,0,.3)" strokeWidth="1" />
      {/* raios: cônicos, do cubo à borda */}
      {spokes.map((deg) => {
        const rad = (deg * Math.PI) / 180;
        const px = Math.sin(rad);
        const py = -Math.cos(rad);
        const qx = Math.cos(rad);
        const qy = Math.sin(rad);
        const at = (r: number, o: number): string =>
          `${cx + px * r + qx * o},${REEL_CY + py * r + qy * o}`;
        return (
          <polygon
            key={deg}
            points={`${at(70, -9)} ${at(22, -11)} ${at(22, 11)} ${at(70, 7)}`}
            fill={`url(#lp-reel-metal)`}
            stroke="rgba(0,0,0,.35)"
            strokeWidth="0.8"
          />
        );
      })}
      {/* cubo + furo */}
      <circle cx={cx} cy={REEL_CY} r="22" fill={`url(#lp-reel-hub)`} stroke="rgba(0,0,0,.45)" strokeWidth="1.2" />
      <circle cx={cx} cy={REEL_CY} r="9" fill="#0a0c10" />
      {/* fio de luz na aresta do flange (luz de cima, como no resto) */}
      <circle cx={cx} cy={REEL_CY} r={REEL_R} fill="none" stroke="rgba(255,255,255,.22)" strokeWidth="1.6" />
    </g>
  );
}

/** ponto da escala do VU: ângulo em graus a partir do topo, + = horário.
    A escala VU real NÃO é linear: -20 ocupa um trecho largo e +3 um
    apêndice. Traduzir dB → ângulo por uma reta deixa o topo apertado. */
const VU_MIN = -30;
const VU_SPAN = 100; // graus
const vuAngle = (db: number): number => -VU_SPAN / 2 + ((db - VU_MIN) / (3 - VU_MIN)) * VU_SPAN;
const vuPoint = (deg: number, r: number): [number, number] => {
  const rad = (deg * Math.PI) / 180;
  return [70 + Math.sin(rad) * r, 76 - Math.cos(rad) * r];
};

function VuMeter({ channel, active }: { channel: "L" | "R"; active: boolean }) {
  return (
    <svg className="lp-vu" viewBox="0 0 140 86" role="img" aria-label={MSG.looperVuAria(channel, active)}>
      <defs>
        <linearGradient id={`lp-vu-bezel-${channel}`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#6b5a8f" />
          <stop offset="100%" stopColor="#241b36" />
        </linearGradient>
        {/* a face é MAIS CLARA NO TOPO (item 3 da issue): a lâmpada do
            medidor fica atrás do topo da escala, não do meio */}
        <linearGradient id={`lp-vu-face-${channel}`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#fff2cd" />
          <stop offset="38%" stopColor="#f6cf74" />
          <stop offset="100%" stopColor="#c98f2e" />
        </linearGradient>
        <clipPath id={`lp-vu-clip-${channel}`}>
          <rect x="5" y="5" width="130" height="76" rx="6" />
        </clipPath>
      </defs>
      <rect x="1" y="1" width="138" height="84" rx="8" fill={`url(#lp-vu-bezel-${channel})`} stroke="#0d0a14" strokeWidth="1.5" />
      <rect x="5" y="5" width="130" height="76" rx="6" fill={`url(#lp-vu-face-${channel})`} />
      <g clipPath={`url(#lp-vu-clip-${channel})`}>
        {/* zona vermelha a partir do +1 dB (onde a agulha encosta no
            limitador da escala VU real) */}
        <path
          d={`M ${vuPoint(vuAngle(1), 52).join(" ")} A 52 52 0 0 1 ${vuPoint(vuAngle(3), 52).join(" ")} L ${vuPoint(vuAngle(3), 42).join(" ")} A 42 42 0 0 0 ${vuPoint(vuAngle(1), 42).join(" ")} Z`}
          fill="#e04a4a"
          opacity=".5"
        />
        {[-30, -20, -10, -7, -5, -3, -1, 0, 1, 2, 3].map((v) => {
          const major = v % 10 === 0 || v === 3 || v === -30;
          const deg = vuAngle(v);
          const [x1, y1] = vuPoint(deg, major ? 52 : 48);
          const [x2, y2] = vuPoint(deg, 42);
          return <line key={v} x1={x1} y1={y1} x2={x2} y2={y2} stroke="#7a5216" strokeWidth={major ? 1.8 : 1} />;
        })}
        {[-20, -10, 0].map((v) => {
          const [x, y] = vuPoint(vuAngle(v), 34);
          return (
            <text key={v} x={x} y={y + 3} textAnchor="middle" fontSize="7.5" fill="#7a5216" fontFamily="var(--font-mono)">
              {v}
            </text>
          );
        })}
        {/* vidro: um risco de luz atravessando o visor */}
        <path d="M 8 81 L 44 5 L 62 5 L 26 81 Z" fill="#fff" opacity=".16" />
      </g>
      <g className="lp-vu-needle" style={active ? undefined : { transform: "rotate(-46deg)" }}>
        <line x1="70" y1="76" x2="70" y2="22" stroke="#2b1d08" strokeWidth="2" strokeLinecap="round" />
      </g>
      <circle cx="70" cy="76" r="5" fill="#2b1d08" />
      <circle cx="70" cy="76" r="2" fill="#7a5216" />
      <text x="13" y="77" fontSize="8" fontWeight="700" fill="#7a5216" fontFamily="var(--font-mono)">
        {channel}
      </text>
    </svg>
  );
}

export function LooperPanel({
  settings,
  onChange,
  recRequest = 0,
  onPlayingChange,
}: {
  settings: LooperSettings;
  onChange: (s: LooperSettings) => void;
  /** pulso do atalho global R (useGlobalShortcuts no App): incrementa p/ disparar REC */
  recRequest?: number;
  /** VU da navbar: true quando o loop está tocando/gravando (há "som") */
  onPlayingChange?: (playing: boolean) => void;
}) {
  const [mode, setMode] = useState<Mode>("idle");

  // VU reativo da navbar: qualquer transporte que produza som anima as barras
  useEffect(() => {
    onPlayingChange?.(mode === "rec" || mode === "play" || mode === "dub");
  }, [mode, onPlayingChange]);
  const [hasTape, setHasTape] = useState(false);
  const [secs, setSecs] = useState(0);
  const [confirmClear, setConfirmClear] = useState(false);
  const timer = useRef<number | null>(null);

  const maxSecs = settings.pre ? LOOP_SECONDS_PRE : LOOP_SECONDS_POST;
  const running = mode === "rec" || mode === "play" || mode === "dub";
  const set = (patch: Partial<LooperSettings>) => onChange({ ...settings, ...patch });

  useEffect(() => {
    if (!running) {
      if (timer.current != null) window.clearInterval(timer.current);
      timer.current = null;
      return;
    }
    timer.current = window.setInterval(() => {
      setSecs((s) => {
        if (mode === "rec" && !hasTape) {
          if (s + 1 >= maxSecs) {
            setHasTape(true);
            setMode("play");
            return 0;
          }
          return s + 1;
        }
        return (s + 1) % maxSecs;
      });
    }, 1000);
    return () => {
      if (timer.current != null) window.clearInterval(timer.current);
      timer.current = null;
    };
  }, [running, mode, hasTape, maxSecs]);

  const onRec = () => {
    setConfirmClear(false);
    if (mode === "rec") {
      setHasTape(true);
      setMode("play");
    } else if (mode === "play" || mode === "dub") {
      setMode("dub");
    } else {
      setMode("rec");
    }
  };
  // Atalho global R (App): dispara o MESMO onRec do botão ● (rec→play→dub).
  // ref mantém o handler sempre atual sem re-registrar o efeito.
  const onRecRef = useRef(onRec);
  useEffect(() => {
    onRecRef.current = onRec;
  });
  useEffect(() => {
    if (recRequest > 0) onRecRef.current();
  }, [recRequest]);

  const onPlay = () => {
    if (!hasTape) return;
    setMode(mode === "play" ? "stop" : "play");
  };
  const onStop = () => setMode("stop");
  const onRew = () => setSecs(0);
  const onClear = () => {
    if (!confirmClear) {
      setConfirmClear(true);
      return;
    }
    setConfirmClear(false);
    setHasTape(false);
    setMode("idle");
    setSecs(0);
  };

  const modeLabel =
    mode === "rec" ? MSG.looperMode.rec : mode === "play" ? MSG.looperMode.play : mode === "dub" ? MSG.looperMode.dub : mode === "stop" ? MSG.looperMode.stop : hasTape ? MSG.looperMode.ready : MSG.looperMode.empty;
  const mm = String(Math.floor(secs / 60)).padStart(2, "0");
  const ss = String(secs % 60).padStart(2, "0");

  /* fração gravada: dirige o pacote de fita dos dois rolos e o brilho do
     contador. `filled` é o quanto a fita JÁ SAÍU do supply — no repouso a
     máquina está enfiada (supply cheio, take-up vazio) e a cada segundo
     de gravação os dois rolos trocam de lugar. */
  const filled = Math.min(1, secs / maxSecs);
  const supplyPack = packOf(1 - filled);
  const takeupPack = packOf(filled);
  const spinClass = running ? "reel-spin" : "";
  const fastClass = mode === "rec" || mode === "dub" ? " reel-fast" : "";
  const tone = mode === "rec" || mode === "dub" ? "rec" : mode === "play" ? "play" : "";

  /* caminho da fita: sai do fundo do supply, desce aos cabeçotes, atravessa
     e sobe para o take-up. Os pontos de partida são os MESMOS raios dos
     pacotes — a fita cresce visivelmente conforme a gravação avança. */
  const supplyY = REEL_CY + supplyPack;
  const takeupY = REEL_CY + takeupPack;
  const tapePath = `M ${SUPPLY_CX} ${supplyY} C ${SUPPLY_CX + 78} ${supplyY}, ${HEAD_X0 - 78} ${TAPE_Y}, ${HEAD_X0} ${TAPE_Y} L ${HEAD_X1} ${TAPE_Y} C ${HEAD_X1 + 78} ${TAPE_Y}, ${TAKEUP_CX - 78} ${takeupY}, ${TAKEUP_CX} ${takeupY}`;

  return (
    <section className="lp" aria-label={MSG.looperAria}>
      {/* ── FAIXA DE CABEÇALHO: placa, estado, transporte, rota ── */}
      <div className="lp-head">
        <span className="lp-plate">{MSG.looperPlate}</span>
        <span className="lp-tape" role="status">
          {hasTape ? MSG.tapeState(maxSecs, settings.pre ? "PRE" : "POST") : MSG.tapeEmpty}
        </span>
        <span className={`lp-mode${tone ? ` lp-mode--${tone}` : ""}`} role="status">
          <span className="lp-mode-led" aria-hidden="true" />
          ● {modeLabel}
        </span>
        {/* rota PRE/POST: a posição ativa é a que acende dentro do sulco */}
        <span className="lp-route">
          <button
            type="button"
            className="lp-route-btn"
            onClick={() => set({ pre: true })}
            aria-pressed={settings.pre}
            aria-label={MSG.looperPreAria}
          >
            {MSG.looperPreBtn}
          </button>
          <button
            type="button"
            className="lp-route-btn"
            onClick={() => set({ pre: false })}
            aria-pressed={!settings.pre}
            aria-label={MSG.looperPostAria}
          >
            {MSG.looperPostBtn}
          </button>
        </span>
      </div>

      {/* ── DECK: rolos + cabeçotes + fita (um SVG só) ── */}
      <div className="lp-deck">
        <svg
          className="lp-deck-svg"
          viewBox={`0 0 ${DECK_W} ${DECK_H}`}
          role="img"
          aria-label={MSG.looperDeckAria(running)}
        >
          <defs>
            <linearGradient id="lp-reel-metal" x1="0.15" y1="0" x2="0.85" y2="1">
              <stop offset="0%" stopColor="#6d5a99" />
              <stop offset="45%" stopColor="#443663" />
              <stop offset="100%" stopColor="#221a33" />
            </linearGradient>
            <linearGradient id="lp-reel-hub" x1="0.2" y1="0" x2="0.8" y2="1">
              <stop offset="0%" stopColor="#8f7ac4" />
              <stop offset="100%" stopColor="#392c56" />
            </linearGradient>
            <radialGradient id="lp-tape-pack" cx="0.35" cy="0.3" r="0.8">
              <stop offset="0%" stopColor="#8a6236" />
              <stop offset="60%" stopColor="#4e3620" />
              <stop offset="100%" stopColor="#2b1d10" />
            </radialGradient>
          </defs>

          {/* ponte dos cabeçotes: chapa rebaixada onde a fita corre, com
              os quatro parafusos que prendem o conjunto no chassi */}
          <rect x="186" y="76" width="188" height="90" rx="10" fill="#1a1622" stroke="#2c2540" strokeWidth="1.5" />
          <rect x="192" y="82" width="176" height="7" rx="3.5" fill="#2f2745" opacity=".8" />
          {[
            [196, 86],
            [364, 86],
            [196, 156],
            [364, 156],
          ].map(([sx, sy]) => (
            <g key={`${sx}-${sy}`}>
              <circle cx={sx} cy={sy} r="4.5" fill="#2a2340" stroke="#120e1e" strokeWidth="1" />
              <rect x={sx - 3} y={sy - 0.8} width="6" height="1.6" rx="0.8" fill="#0d0a14" />
            </g>
          ))}
          {/* lâmpadas de armed: acendem em REC/DUB (o resto é sempre apagado) */}
          {[232, 328].map((lx) => (
            <circle
              key={lx}
              cx={lx}
              cy="94"
              r="4.5"
              fill={mode === "rec" || mode === "dub" ? "#f87171" : "#2a2340"}
            />
          ))}

          {/* cabeçotes: corpo com o entalhe por onde a fita passa */}
          {[HEAD_X0 + 2, HEAD2_X0].map((x0) => {
            const x1 = x0 + HEAD_W;
            const ncx = x0 + HEAD_W / 2;
            return (
              <g key={x0}>
                <path
                  d={`M ${x0} ${HEAD_BOT} L ${x0} ${HEAD_TOP + 6} Q ${x0} ${HEAD_TOP} ${x0 + 6} ${HEAD_TOP} L ${x1 - 6} ${HEAD_TOP} Q ${x1} ${HEAD_TOP} ${x1} ${HEAD_TOP + 6} L ${x1} ${HEAD_BOT} L ${ncx + 6} ${HEAD_BOT} L ${ncx + 6} ${TAPE_Y - 7} L ${ncx - 6} ${TAPE_Y - 7} L ${ncx - 6} ${HEAD_BOT} Z`}
                  fill="url(#lp-reel-metal)"
                  stroke="#0d0a14"
                  strokeWidth="1.2"
                />
                {/* face polida: o ponto onde a fita encosta */}
                <rect x={ncx - 3} y={TAPE_Y - 5} width="6" height="16" rx="2" fill="#9aa3b2" opacity=".55" />
              </g>
            );
          })}

          {/* fita POR CIMA dos cabeçotes: é assim que ela realmente passa —
              a face polida do cabeçote fica pressionada contra a fita. Com
              a fita atrás, a linha sumia no metal e o deck parecia solto. */}
          <path d={tapePath} fill="none" stroke="#2b1d10" strokeWidth="6" strokeLinecap="round" />
          <path d={tapePath} fill="none" stroke="#b8843f" strokeWidth="2" strokeLinecap="round" opacity=".9" />

          {/* capstan: o pino que a fita envolve, metal polido com brilho */}
          <circle cx="281" cy={TAPE_Y} r="13" fill="#241d33" stroke="#0d0a14" strokeWidth="1.4" />
          <circle cx="281" cy={TAPE_Y} r="8.5" fill="#d7dbe4" />
          <circle cx="278" cy={TAPE_Y - 3} r="2.6" fill="#fff" opacity=".85" />
          {/* flywheel do capstan: aro fino por trás, dá espessura ao pino */}
          <circle cx="281" cy={TAPE_Y} r="19" fill="none" stroke="#3a3054" strokeWidth="3" />

          <g className={`lp-reel--supply ${spinClass}${fastClass}`}>
            <Reel cx={SUPPLY_CX} packR={supplyPack} />
          </g>
          <g className={`lp-reel--takeup ${spinClass}${fastClass}`}>
            <Reel cx={TAKEUP_CX} packR={takeupPack} />
          </g>
        </svg>
        <div className="lp-reel-legend">
          <span className="lp-reel-label">{MSG.reelSupply}</span>
          <span className="lp-reel-label">{MSG.reelTakeup}</span>
        </div>
      </div>

      {/* ── MEDIÇÃO + TRANSPORTE ── */}
      <div className="lp-right">
        <div className="lp-meter">
          <svg
            className={`lp-counter${tone ? ` lp-counter--${tone}` : ""}`}
            viewBox="0 0 182 62"
            role="timer"
            aria-label={MSG.looperTimerAria}
          >
            <g className="lp-counter-digits">
              <SevenSeg text={`${mm}:${ss}`} />
            </g>
          </svg>
          <div className="lp-vus">
            <VuMeter channel="L" active={running} />
            <VuMeter channel="R" active={running} />
          </div>
        </div>

        <div className="lp-transport" role="group" aria-label={MSG.looperTransportAria}>
          <button type="button" className="lp-key" onClick={onRew} aria-label={MSG.looperRewAria}>
            <span className="lp-key-led" aria-hidden="true" />
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M3 5h2.5v14H3zM7 5l7 7-7 7zM15 5l7 7-7 7z" />
            </svg>
          </button>
          <button
            type="button"
            className="lp-key"
            onClick={onStop}
            aria-pressed={mode === "stop" || mode === "idle"}
            aria-label={MSG.looperStopAria}
          >
            <span className="lp-key-led" aria-hidden="true" />
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M6.5 6.5h11v11h-11z" />
            </svg>
          </button>
          <button
            type="button"
            className="lp-key lp-key--play"
            onClick={onPlay}
            aria-pressed={mode === "play"}
            aria-label={MSG.looperPlayAria}
            disabled={!hasTape}
          >
            <span className="lp-key-led" aria-hidden="true" />
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M7 4.5l12 7.5-12 7.5z" />
            </svg>
          </button>
          <button
            type="button"
            className="lp-key lp-key--rec"
            onClick={onRec}
            aria-pressed={mode === "rec" || mode === "dub"}
            aria-label={mode === "rec" ? MSG.looperRecStopAria : mode === "play" || mode === "dub" ? MSG.looperDubAria : MSG.looperRecAria}
          >
            <span className="lp-key-led" aria-hidden="true" />
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <circle cx="12" cy="12" r="6" />
            </svg>
          </button>
          <button
            type="button"
            className="lp-key lp-key--clear"
            onClick={onClear}
            aria-pressed={confirmClear}
            aria-label={confirmClear ? MSG.looperClearConfirmAria : MSG.looperClearAria}
          >
            <span className="lp-key-led" aria-hidden="true" />
            {confirmClear ? (
              <span className="lp-key-text">{MSG.looperClearConfirmBtn}</span>
            ) : (
              <svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round">
                <path d="M6.5 6.5l11 11M17.5 6.5l-11 11" />
              </svg>
            )}
          </button>
        </div>
      </div>

      {/* ── MIX: os três ganhos do menu LOOPER do firmware ── */}
      <div className="lp-mix">
        {(
          [
            ["loop-rec", MSG.looperRecVol, settings.recVol, "recVol"],
            ["loop-play", MSG.looperPlayVol, settings.playVol, "playVol"],
            ["loop-pvol", MSG.looperPVol, settings.pVol, "pVol"],
          ] as const
        ).map(([id, label, value, field]) => (
          <div className="lp-fader-cell" key={id}>
            <label className="lp-fader-cap" htmlFor={id}>
              {label}
            </label>
            <div className="lp-fader">
              {/* --r = fração real do curso (0..99) — o CSS usa a MESMA
                  fração para a ponta do preenchimento e para a posição
                  do capuz, então os dois não podem sair de registro */}
              <span
                className="lp-fader-fill"
                style={{ "--r": value / 99 } as CSSProperties}
                aria-hidden="true"
              />
              <input
                id={id}
                type="range"
                min={0}
                max={99}
                value={value}
                onChange={(e) => set({ [field]: Number(e.target.value) } as Partial<LooperSettings>)}
              />
              <span className="lp-fader-val">{value}</span>
            </div>
          </div>
        ))}
        <p className="lp-specs">{MSG.looperSpecs}</p>
      </div>
    </section>
  );
}