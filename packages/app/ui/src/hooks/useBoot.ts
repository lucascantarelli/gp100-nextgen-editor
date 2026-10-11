/**
 * useBoot — dispara o `device_boot`, VALIDA o relatório (#161) e consome
 * `device://progress` (barra de progresso; boot nunca trava a UI).
 *
 * Estado canônico: o boot vive num ScreenState próprio; o
 * progresso é um NUMBER % (throttle a ~30 fps por rAF) — 2297 beats por
 * boot não podem renderizar 2297 vezes (ui-ux-practices: feedback <100 ms
 * e zero jank).
 *
 * **Gate do relatório (#161):** `ready` só nasce de um relatório que
 * passa em [`validaRelatorioBoot`] — inventário e nomes coerentes com o
 * catálogo. Falhou → `error` nomeando QUAL leitura falhou (a casca não
 * monta com dado incoerente). O detalhe técnico vai pro console; a UI
 * recebe só texto amigável (contrato do e2e: nada de "debug:" no alert).
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { deviceBoot, onBootProgress } from "../ipc/device";
import type { BootProgress, BootReport, ScreenState } from "../ipc/types";
import { MSG } from "../i18n/messages";
import { PRESET_COUNT } from "../i18n/facts";

/** Etapa em PT-BR para exibição (textos do catálogo central de mensagens). */
export const BOOT_STAGE_LABEL: Record<BootProgress["stage"], string> = {
  tables: MSG.bootStages.tables,
  scan: MSG.bootStages.scan,
  probe: MSG.bootStages.probe,
  state5: MSG.bootStages.state5,
  names: MSG.bootStages.names,
  keepalive: MSG.bootStages.keepalive,
};

/**
 * Qual leitura do boot falhou na validação (#161) — o erro NOMEIA a
 * leitura, não culpa "o device" genérico: progresso do script, inventário
 * de presets ou nomes. Discriminante tipado: a UI decide o texto por ele.
 * (Sem `export` de propósito — o consumidor é o próprio módulo; o gate
 * de deadcode cobra o export sem uso externo.)
 */
type BootLeituraInvalida =
  | { leitura: "progresso" }
  | { leitura: "inventario"; lido: number; esperado: number }
  | { leitura: "nomes"; lido: number; esperado: number };

/** Teto sanidade: o script do aparelho é ~2299 transações — acima disso o
 *  número não é leitura, é lixo (e `NaN`/float/negativo caem aqui também). */
const TRANSACOES_MAX = 10_000;

/** Inventário esperado do catálogo atual: 99 de fábrica + 99 de usuário. */
export const INVENTARIO_ESPERADO = PRESET_COUNT * 2;

/**
 * Validação PURA do relatório do boot (#161) — função pura testável com
 * vetores (válida / inválida / vazia). O contrato:
 *
 *   1. `transactions` inteiro em [1, 10_000] — o boot rodou de fato;
 *   2. `presets` === catálogo atual (198) — o scan leu o inventário INTEIRO;
 *   3. `names` === `presets` — todo pp tem nome lido (o "198/198").
 *
 * Qualquer violação devolve a leitura culpada (`null` = passou). Os
 * números medidos são do PRÓPRIO relatório — nada é esperado em silêncio.
 */
export function validaRelatorioBoot(
  report: BootReport | null | undefined,
): BootLeituraInvalida | null {
  const inteiro = (n: unknown): n is number => typeof n === "number" && Number.isInteger(n);
  const t = report?.transactions;
  if (!inteiro(t) || t < 1 || t > TRANSACOES_MAX) return { leitura: "progresso" };
  const presets = report?.presets;
  if (!inteiro(presets) || presets !== INVENTARIO_ESPERADO) {
    return {
      leitura: "inventario",
      lido: inteiro(presets) ? presets : 0,
      esperado: INVENTARIO_ESPERADO,
    };
  }
  const names = report?.names;
  if (!inteiro(names) || names !== presets) {
    return { leitura: "nomes", lido: inteiro(names) ? names : 0, esperado: presets };
  }
  return null;
}

/** Texto amigável do problema, nomeando a leitura (§8: nada de jargão). */
export function mensagemBootInvalido(p: BootLeituraInvalida): string {
  if (p.leitura === "progresso") return MSG.bootInvalidoProgresso;
  if (p.leitura === "inventario") return MSG.bootInvalidoInventario(p.lido, p.esperado);
  return MSG.bootInvalidoNomes(p.lido, p.esperado);
}

export function useBoot() {
  const [state, setState] = useState<ScreenState<BootReport>>({ kind: "idle" });
  const [progress, setProgress] = useState<number | null>(null);
  const [stage, setStage] = useState<BootProgress["stage"] | null>(null);
  // refs do throttle: último frame agendado + beat pendente
  const raf = useRef<number | null>(null);
  const pending = useRef<BootProgress | null>(null);
  // FIX (auditoria 01/10): a promise do deviceBoot pode resolver DEPOIS do
  // unmount (teste que termina antes do boot acabar) — sem esta guarda o
  // setState roda com o ambiente já destruído e o React estoura
  // `ReferenceError: window is not defined` ao resolver a prioridade do
  // update (vitest marcou como unhandled error e o job ui-rust do macOS
  // caiu: run 36937323423). Em produção é a mesma regra: componente morto
  // não recebe update.
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    onBootProgress((p) => {
      pending.current = p;
      if (raf.current === null) {
        raf.current = requestAnimationFrame(() => {
          raf.current = null;
          const last = pending.current;
          if (!last) return;
          setProgress(Math.round((last.done / last.total) * 100));
          setStage(last.stage);
        });
      }
    }).then((un) => {
      if (disposed) un();
      else unlisten = un;
    });
    return () => {
      disposed = true;
      unlisten?.();
      if (raf.current !== null) cancelAnimationFrame(raf.current);
    };
  }, []);

  const [origin, setOrigin] = useState<"auto" | "manual">("auto");

  const startBoot = useCallback((src: "auto" | "manual" = "manual") => {
    setOrigin(src);
    setState({ kind: "loading" });
    setProgress(0);
    deviceBoot()
      .then((report) => {
        if (!alive.current) return; // promessa resolveu após o unmount
        // #161: relatório fora do contrato vira ERRO nomeando a leitura —
        // a casca só monta com o aparelho LIDO (inventário e nomes coerentes).
        const problema = validaRelatorioBoot(report);
        if (problema != null) {
          console.error("boot: relatório fora do contrato", problema, report);
          setProgress(null);
          setState({
            kind: "error",
            message: mensagemBootInvalido(problema),
            retry: startBoot,
          });
          return;
        }
        setProgress(100);
        setState({ kind: "ready", data: report });
      })
      .catch((e: unknown) => {
        if (!alive.current) return; // idem (evita update em componente morto)
        // O detalhe técnico ("debug: …", erro cru do invoke) vai pro CONSOLE
        // — a UI só vê texto amigável (contrato do e2e: nunca "debug:" no alert).
        console.error("boot falhou", e);
        setProgress(null);
        setState({
          kind: "error",
          message: MSG.connBootError,
          retry: startBoot,
        });
      });
  }, []);

  // boot AUTOMÁTICO no mount: o app detecta o device sozinho ao abrir
 // (o estado "off" da navbar já mostra o aguardo); o botão Boot da
  // navbar continua disponível para re-escanear explicitamente. A ref
  // evita rodar duas vezes sob StrictMode (mesma instância, 2 efeitos).
  const autoRan = useRef(false);
  useEffect(() => {
    if (autoRan.current) return;
    autoRan.current = true;
    startBoot("auto");
  }, [startBoot]);

  const reset = useCallback(() => {
    setState({ kind: "idle" });
    setProgress(null);
    setStage(null);
  }, []);

  return { state, progress, stage, origin, startBoot, reset };
}
