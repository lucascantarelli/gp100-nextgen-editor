/**
 * useTones — o estado do gestor de SnapTone/NAM (issue #25).
 *
 * O hook é o DONO da tela: quem abre o quadro, quem guarda o que está em A e
 * em B, quem trava o botão durante o envio. O componente só desenha.
 *
 * **O envio é longo e por isso tem dono.** 143 blocos com o settle de 250 ms
 * entre eles (§4/§5) dão dezenas de segundos; o estado `enviando` existe para
 * que o botão fique travado e o texto mude. Sem ele, o dono clica de novo e
 * dispara dois streams no aparelho — e a segunda transferência começa no meio
 * da primeira.
 *
 * **A/B são DOIS SLOTS, não dois lados de um mesmo controle.** A comparação
 * que interessa aqui é "qual tom está no aparelho, no slot 3, contra o do
 * slot 4" — e o `A/B` do aparelho é exatamente isso: um som ligado e outro
 * desligado. Guardar os lados como número de slot (e não como id de tom) faz a
 * tela refletir o que está no aparelho mesmo depois de um reimport, que troca o
 * id do tom sem mudar o slot.
 *
 * **O áudio vem do WAV que o Suite renderiza.** O `.clo` é o modelo
 * convertido, e o motor NAM mora no exe da Valeton (`BLOCKERS.md`) — o app não
 * reimplementa inferência. O Suite deixa do lado do `.clo` o
 * `nam_output_wav.wav`, que é o ÁUDIO que aquele modelo produz; é esse arquivo
 * que o player toca. Sem ele, o botão fica desabilitado com o motivo escrito
 * (e não some): "comparar sem áudio" é informação, não sumiço.
 */
import { useCallback, useEffect, useState } from "react";
import {
  toneAssignSlot,
  toneBoard,
  toneDelete,
  toneDoSlot,
  toneImport,
  toneRename,
  toneSend,
} from "../ipc/tones";
import type { Tone, ToneBoard, ToneSendReport, ToneWithModel } from "../ipc/tones";
import { MSG } from "../i18n/messages";

/** Erro amigável + AÇÃO de recuperação (mesmo contrato do `useStage`). */
interface ToneError {
  message: string;
  retry: () => void;
}

/** O que o painel do gestor consome. */
export interface Tones {
  /** O quadro (lista + slots). `null` = ainda não chegou. */
  board: ToneBoard | null;
  /** Erro com retry, ou `null`. */
  err: ToneError | null;
  /** O painel está aberto? */
  aberto: boolean;
  /** Abre o painel e carrega o quadro. */
  abrir: () => void;
  /** Fecha o painel. */
  fechar: () => void;
  /** Importa um `.clo` (e o WAV de preview, se o dono escolher). */
  importa: (nome: string, model: number[], preview?: number[]) => Promise<Tone | null>;
  /** Renomeia e relê o quadro. */
  renomeia: (id: string, nome: string) => Promise<void>;
  /** Atribui (ou desliga) o slot e relê o quadro. */
  atribui: (id: string, slot: number | null) => Promise<void>;
  /** Apaga e relê o quadro. */
  apaga: (id: string) => Promise<void>;
  /** Id do tom em envio agora (`null` = o aparelho está livre). */
  enviando: string | null;
  /** Envia ao aparelho; o relatório fica em `relatorio`. */
  envia: (id: string) => Promise<void>;
  /** Relatório do último envio. */
  relatorio: ToneSendReport | null;
  /** Slot do lado A do A/B (`null` = vazio). */
  ladoA: number | null;
  /** Slot do lado B do A/B (`null` = vazio). */
  ladoB: number | null;
  /** Escolhe o slot de um lado do A/B. */
  chooseLado: (lado: "a" | "b", slot: number | null) => void;
  /** Toca o áudio de um slot; devolve a URL do objeto ou `null` (sem preview). */
  toca: (slot: number) => Promise<string | null>;
  /** Slot que está tocando agora (`null` = parado). */
  tocando: number | null;
  /** Avisa que o aparelho está tocando. */
  setTocando: (slot: number | null) => void;
}

export function useTones(): Tones {
  const [board, setBoard] = useState<ToneBoard | null>(null);
  const [err, setErr] = useState<ToneError | null>(null);
  const [aberto, setAberto] = useState(false);
  const [enviando, setEnviando] = useState<string | null>(null);
  const [relatorio, setRelatorio] = useState<ToneSendReport | null>(null);
  const [ladoA, setLadoA] = useState<number | null>(null);
  const [ladoB, setLadoB] = useState<number | null>(null);
  const [tocando, setTocando] = useState<number | null>(null);

  const limpa = useCallback(() => setErr(null), []);

  /**
   * Relê o quadro. Contador de sequência para fora do quadro não é exagero
   * aqui (não há busca com debounce), mas o retry precisa saber se a
   * REPETIÇÃO deu certo — é o retry que limpa o banner, nunca a leitura.
   */
  const recarrega = useCallback(async (): Promise<boolean> => {
    try {
      setBoard(await toneBoard());
      return true;
    } catch (e) {
      console.error("tone_board falhou:", e);
      setErr({ message: MSG.errToneRead, retry: () => void recarrega().then((ok) => { if (ok) limpa(); }) });
      return false;
    }
  }, [limpa]);

  // O quadro é lido uma vez no mount: a tela mostra os números do arquivo mesmo
  // com o painel fechado, e reler só quando ele abre custaria uma leitura
  // garantidamente obsoleta (o dono mexe no arquivo em outro lugar).
  useEffect(() => {
    void recarrega();
  }, [recarrega]);

  const abrir = useCallback(() => {
    setAberto(true);
    void recarrega();
  }, [recarrega]);

  const fechar = useCallback(() => setAberto(false), []);

  const importa = useCallback(
    async (nome: string, model: number[], preview?: number[]) => {
      try {
        // o `preview` vai JUNTO: o A/B toca o `nam_output_wav.wav` do Suite, e
        // um áudio deixado para trás aqui é um botão de ouvir que nunca
        // funciona — o dono não tem como saber por que
        const linha = await toneImport(nome, model, preview);
        await recarrega();
        limpa();
        return linha;
      } catch (e) {
        console.error("tone_import falhou:", e);
        setErr({ message: MSG.errToneImport, retry: () => void importa(nome, model, preview) });
        return null;
      }
    },
    [limpa, recarrega],
  );

  const renomeia = useCallback(
    async (id: string, nome: string) => {
      try {
        await toneRename(id, nome);
        await recarrega();
        limpa();
      } catch (e) {
        console.error("tone_rename falhou:", e);
        setErr({ message: MSG.errToneWrite, retry: () => void renomeia(id, nome) });
      }
    },
    [limpa, recarrega],
  );

  const atribui = useCallback(
    async (id: string, slot: number | null) => {
      try {
        await toneAssignSlot(id, slot);
        await recarrega();
        limpa();
      } catch (e) {
        console.error("tone_assign_slot falhou:", e);
        // O erro do banco (slot ocupado por outro tom) é mais útil que uma
        // frase genérica: quem ocupa o slot é o que o dono precisa saber.
        const msg = e instanceof Error && e.message ? e.message : MSG.errToneSlot;
        setErr({ message: msg, retry: () => void atribui(id, slot) });
      }
    },
    [limpa, recarrega],
  );

  const apaga = useCallback(
    async (id: string) => {
      try {
        await toneDelete(id);
        await recarrega();
        limpa();
      } catch (e) {
        console.error("tone_delete falhou:", e);
        setErr({ message: MSG.errToneWrite, retry: () => void apaga(id) });
      }
    },
    [limpa, recarrega],
  );

  /**
   * Envia ao aparelho.
   *
   * O `if (enviando != null)` é a trava manual: o botão já fica desabilitado,
   * mas um duplo clique no mesmo evento chega aqui antes do re-render, e o
   * segundo stream sobrescreveria o primeiro no meio da transferência.
   */
  const envia = useCallback(
    async (id: string) => {
      if (enviando != null) return;
      setEnviando(id);
      setRelatorio(null);
      try {
        const rel = await toneSend(id);
        setRelatorio(rel);
        await recarrega();
        limpa();
      } catch (e) {
        console.error("tone_send falhou:", e);
        setErr({
          message: e instanceof Error && e.message ? e.message : MSG.errToneSend,
          retry: () => void envia(id),
        });
      } finally {
        setEnviando(null);
      }
    },
    [enviando, limpa, recarrega],
  );

  const chooseLado = useCallback((lado: "a" | "b", slot: number | null) => {
    if (lado === "a") setLadoA(slot);
    else setLadoB(slot);
  }, []);

  /**
   * A URL de áudio do slot, para o `<audio>` do painel.
   *
   * A revogação é o detalhe: cada play cria um `objectURL`, e sem revogar o
   * WebKit do WebView segura o WAV inteiro até a janela fechar — cinco
   * comparações de meio minuto de áudio são alguns MB que não volta.
   */
  const toca = useCallback(async (slot: number): Promise<string | null> => {
    try {
      const tom: ToneWithModel | null = await toneDoSlot(slot);
      if (tom == null || tom.preview == null || tom.preview.length === 0) return null;
      const bytes = new Uint8Array(tom.preview);
      const url = URL.createObjectURL(new Blob([bytes], { type: "audio/wav" }));
      return url;
    } catch (e) {
      console.error("tone_do_slot falhou:", e);
      setErr({ message: MSG.errToneRead, retry: () => void toca(slot) });
      return null;
    }
  }, []);

  return {
    board,
    err,
    aberto,
    abrir,
    fechar,
    importa,
    renomeia,
    atribui,
    apaga,
    enviando,
    envia,
    relatorio,
    ladoA,
    ladoB,
    chooseLado,
    toca,
    tocando,
    setTocando,
  };
}