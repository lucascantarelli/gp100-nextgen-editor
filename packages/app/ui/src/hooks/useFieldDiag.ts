/**
 * useFieldDiag — o estado do painel de diagnóstico de campo.
 *
 * Este hook é a **fina camada de orquestração** sobre as portas de
 * `ipc/diag`. Ele não monta SysEx e não decide política: quem faz isso é o
 * backend (`devicePreview`, `deviceSavePreset`), e o hook só guarda o que
 * voltou para a tela mostrar.
 *
 * **Os valores de destino moram aqui, e não no componente.** Gravar, ler o
 * dump e descrever o envio são três operações sobre o MESMO preset, e as três
 * precisam concordar sobre qual preset é. Se cada uma guardasse o seu próprio
 * campo de entrada, o operador mudava o número no formulário do dump, lia o
 * aparelho e o botão de gravar mandava o outro número — sem aviso nenhum.
 *
 * **Por que o log de fio é uma sessão e não um arquivo solto.** O log
 * registra o que entra e sai pela USB num arquivo que o suporte lê. Ligar e
 * desligar no meio da sessão é o que permite capturar só o trecho que
 * interessa, sem reiniciar o aparelho e sem perder o que já foi gravado.
 *
 * `info` entra como argumento (e não é buscado aqui) para que o painel
 * reflita o backend **que a sessão já conhece**: uma segunda leitura de
 * `device_info` custaria um punhado de transações no aparelho real e poderia
 * discordar do que a tela já mostrou. Ele semeia os campos; depois é o operador
 * que decide para onde apontam.
 */
import { useCallback, useEffect, useState } from "react";

import {
  deviceDumpPreset,
  deviceLogPath,
  deviceLogReveal,
  deviceLogSession,
  deviceLogStop,
  devicePreview,
  deviceSavePreset,
} from "../ipc/diag";
import type { DeviceInfo, DumpReport, PreviewFrame } from "../ipc/types";

/** O log de fio sem caminho é inútil: o arquivo é o que o operador entrega. */
const DEFAULT_LOG_PATH = "gp100-sessao.jsonl";

/**
 * O que o painel sabe fazer — e o que está em voo agora.
 *
 * Sem `export`: quem consome é o componente, e ele pega o tipo pelo valor de
 * retorno do hook. Expor aqui seria uma superfície pública sem consumidor.
 */
interface FieldDiag {
  /** Patch de destino: pp (0-based no fio), tipo e nome. */
  pp: number;
  ppType: number;
  name: string;
  setPp: (pp: number) => void;
  setPpType: (ppType: number) => void;
  setName: (name: string) => void;
  /** Caminho do arquivo de log (campo do operador). */
  logFile: string;
  setLogFile: (path: string) => void;
  /** Último dump lido do aparelho, ou `null` se ainda não leu. */
  dump: DumpReport | null;
  /** Frames do preview, ou `null` se ainda não pediu. */
  preview: PreviewFrame[] | null;
  /**
   * O log de fio está gravando?
   *
   * Inclui o log que o **build de campo liga sozinho** na abertura (#130): a
   * tela pergunta ao backend em vez de assumir que começou desligado.
   */
  logging: boolean;
  /** Caminho do log atual (para o operador saber onde vai o arquivo). */
  logPath: string | null;
  /** O preset foi gravado nesta sessão? (o botão não repete sozinho) */
  saved: boolean;
  /** Operação em voo — desabilita os botões em vez de duplicar clique. */
  busy: null | "save" | "dump" | "log" | "preview" | "reveal";
  /** Última mensagem de erro (a UI mostra e não engole). */
  error: string | null;
  /** Grava o preset no aparelho. Não tem retry: gravar duas vezes é pior. */
  save: () => Promise<boolean>;
  /** Lê o preset do aparelho (identificação + as 8 páginas). */
  dumpPreset: () => Promise<void>;
  /** Liga/desliga o log de fio da sessão. */
  toggleLog: () => Promise<void>;
  /**
   * Abre o gerenciador de arquivos com o LOG ATIVO selecionado (#135).
   *
   * O caminho é do backend: o front só clica — é o que impede este botão de
   * levar o operador para um lugar qualquer do disco.
   */
  revealLog: () => Promise<void>;
  /** Pede os frames que o aparelho receberia, sem receber. */
  previa: () => Promise<void>;
  /** Limpa o dump e o preview (não mexe no log — desligar é explícito). */
  clear: () => void;
}

function msg(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** Cria o estado do diagnóstico, semeado pelo que a sessão já sabe do device. */
export function useFieldDiag(info: DeviceInfo | null): FieldDiag {
  const [pp, setPp] = useState(0);
  const [ppType, setPpType] = useState(0);
  const [name, setName] = useState("");
  const [logFile, setLogFile] = useState(DEFAULT_LOG_PATH);
  const [dump, setDump] = useState<DumpReport | null>(null);
  const [preview, setPreview] = useState<PreviewFrame[] | null>(null);
  const [logging, setLogging] = useState(false);
  const [logPath, setLogPath] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [busy, setBusy] = useState<FieldDiag["busy"]>(null);
  const [error, setError] = useState<string | null>(null);

  // Semente do formulário: o patch que a sessão já tem aberto. Só entra uma
  // vez por device — se o `info` recarregar depois (re-boot, ⟳), o operador
  // pode estar mirado em outro patch e trocar o número na mão dele seria o
  // pior jeito de "ajudar".
  useEffect(() => {
    if (info == null) return;
    setPp(info.currentPp);
    setPpType(info.currentPpType);
    setName(info.currentName);
  }, [info]);

  // **O log do build de campo já está ligado quando esta tela abre.** Desde a
  // #130 o `run()` liga o log de fio sozinho no backend real, com o arquivo que
  // ele escolhe. Perguntar é o que impede o painel de dizer "nenhum log" (e o
  // botão de oferecer "gravar") por cima de uma sessão que já está em disco.
  useEffect(() => {
    let vivo = true;
    void deviceLogPath()
      .then((path) => {
        if (!vivo || path == null) return;
        setLogging(true);
        setLogPath(path);
      })
      .catch(() => {
        // Sem resposta do backend a tela fica no repouso: mostrar um arquivo que
        // a sessão não está gravando seria pior que não mostrar arquivo nenhum.
      });
    return () => {
      vivo = false;
    };
  }, []);

  const save = useCallback(async () => {
    setBusy("save");
    setError(null);
    try {
      await deviceSavePreset(pp, ppType, name);
      setSaved(true);
      return true;
    } catch (e) {
      setError(msg(e));
      return false;
    } finally {
      setBusy(null);
    }
  }, [pp, ppType, name]);

  const dumpPreset = useCallback(async () => {
    setBusy("dump");
    setError(null);
    try {
      setDump(await deviceDumpPreset(pp));
    } catch (e) {
      setError(msg(e));
    } finally {
      setBusy(null);
    }
  }, [pp]);

  const toggleLog = useCallback(async () => {
    setBusy("log");
    setError(null);
    try {
      if (logging) {
        await deviceLogStop();
        setLogging(false);
        // Parou de gravar: o caminho na tela some junto (senão a tela diria
        // "gravando em X" para um arquivo que parou de crescer).
        setLogPath(null);
      } else {
        await deviceLogSession(logFile);
        setLogging(true);
        setLogPath(logFile);
      }
    } catch (e) {
      setError(msg(e));
    } finally {
      setBusy(null);
    }
  }, [logging, logFile]);

  const previa = useCallback(async () => {    setBusy("preview");
    setError(null);
    try {
      setPreview(await devicePreview({ op: "save", pp, ppType, name }));
    } catch (e) {
      setError(msg(e));
      setPreview(null);
    } finally {
      setBusy(null);
    }
  }, [pp, ppType, name]);

  const revealLog = useCallback(async () => {
    setBusy("reveal");
    setError(null);
    try {
      await deviceLogReveal();
    } catch (e) {
      // Sem desktop (browser) ou sem handler de arquivo, a operação é DITA —
      // deixar o botão mudo seria o operador achando que a pasta abriu.
      setError(msg(e));
    } finally {
      setBusy(null);
    }
  }, []);

  const clear = useCallback(() => {
    setDump(null);
    setPreview(null);
  }, []);

  return {
    pp,
    ppType,
    name,
    setPp,
    setPpType,
    setName,
    logFile,
    setLogFile,
    dump,
    preview,
    logging,
    logPath,
    saved,
    busy,
    error,
    save,
    dumpPreset,
    toggleLog,
    revealLog,
    previa,
    clear,
  };
}