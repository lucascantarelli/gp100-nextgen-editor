/**
 * useIrs — o estado do laboratório de IRs (issue #24).
 *
 * O hook é o DONO da tela: quem abre o quadro, quem relê a tabela que o
 * APARELHO relata, quem trava o botão durante o envio. O componente só desenha.
 *
 * **Duas listas, e elas não são a mesma coisa.** `board` é a BIBLIOTECA do dono
 * (o que ele importou para o arquivo); `deviceSlots` é o que o aparelho tem
 * gravado agora, lido do fio (`list_user_irs`, §13.12). Mostrar só a primeira
 * deixaria o dono acreditar que o slot 3 está vazio quando o aparelho tem um
 * IR de outra sessão ali — e o próximo envio sobrescreve.
 *
 * **O envio é longo e por isso tem dono.** Na captura, 296 chunks levaram ~5 s;
 * um IR de 300 KB são ~20.000 chunks e quase 6 minutos. O estado `enviando`
 * existe para que o botão fique travado e o texto mude: sem ele, o dono clica
 * de novo e dispara dois streams — e o último chunk duplicado (§13.7, o
 * marcador de fim) fecha a segunda transferência no meio da primeira.
 */
import { useCallback, useEffect, useState } from "react";
import {
  deviceIrTable,
  irAssignSlot,
  irBoard,
  irDelete,
  irImport,
  irRename,
  irSend,
} from "../ipc/ir";
import type { DeviceIrSlot, Ir, IrBoard, IrSendReport } from "../ipc/ir";
import { MSG } from "../i18n/messages";

/** Erro amigável + AÇÃO de recuperação (mesmo contrato do `useStage`). */
interface IrError {
  message: string;
  retry: () => void;
}

/** O que o painel do laboratório consome. */
export interface Irs {
  /** O quadro da BIBLIOTECA (lista + slots). `null` = ainda não chegou. */
  board: IrBoard | null;
  /** O que o APARELHO tem gravado agora (`list_user_irs`). `null` = não lido. */
  deviceSlots: DeviceIrSlot[] | null;
  /** Erro com retry, ou `null`. */
  err: IrError | null;
  /** O painel está aberto? */
  aberto: boolean;
  /** Abre o painel e recarrega as DUAS listas. */
  abrir: () => void;
  /** Fecha o painel. */
  fechar: () => void;
  /** Relê a tabela do aparelho (sozinha: o device muda fora do app). */
  relendoDevice: boolean;
  /** Pede a releitura da tabela do aparelho agora. */
  relêDevice: () => Promise<void>;
  /** Importa um `.ir` e relê o quadro. */
  importa: (nome: string, blob: number[]) => Promise<Ir | null>;
  /** Renomeia e relê o quadro. */
  renomeia: (id: string, nome: string) => Promise<void>;
  /** Atribui (ou desliga) o slot e relê o quadro. */
  atribui: (id: string, slot: number | null) => Promise<void>;
  /** Apaga e relê o quadro. */
  apaga: (id: string) => Promise<void>;
  /** Id do IR em envio agora (`null` = o aparelho está livre). */
  enviando: string | null;
  /** Envia ao aparelho; o relatório fica em `relatorio`. */
  envia: (id: string) => Promise<void>;
  /** Relatório do último envio. */
  relatorio: IrSendReport | null;
}

export function useIrs(): Irs {
  const [board, setBoard] = useState<IrBoard | null>(null);
  const [deviceSlots, setDeviceSlots] = useState<DeviceIrSlot[] | null>(null);
  const [err, setErr] = useState<IrError | null>(null);
  const [aberto, setAberto] = useState(false);
  const [enviando, setEnviando] = useState<string | null>(null);
  const [relatorio, setRelatorio] = useState<IrSendReport | null>(null);
  const [relendoDevice, setRelendoDevice] = useState(false);

  const limpa = useCallback(() => setErr(null), []);

  /** Relê a biblioteca. O retry precisa saber se a REPETIÇÃO deu certo. */
  const recarrega = useCallback(async (): Promise<boolean> => {
    try {
      setBoard(await irBoard());
      return true;
    } catch (e) {
      console.error("ir_board falhou:", e);
      setErr({ message: MSG.errIrRead, retry: () => void recarrega().then((ok) => { if (ok) limpa(); }) });
      return false;
    }
  }, [limpa]);

  /**
   * Relê a tabela do APARELHO, que é uma transação de fio (20 leituras).
   *
   * Ela é separada da biblioteca de propósito: o device muda fora do app (o
   * dono importa um IR pelo painel de hardware, ou por outra sessão), e uma
   * leitura de arquivo diria que o slot está livre quando não está.
   */
  const relêDevice = useCallback(async (): Promise<void> => {
    setRelendoDevice(true);
    try {
      setDeviceSlots(await deviceIrTable());
    } catch (e) {
      console.error("list_user_irs falhou:", e);
      // Falha aqui NÃO é o mesmo que falhar a biblioteca: o dono ainda pode
      // importar e organizar o que tem, e a tela segue mostrando a lista dele.
      setErr({
        message: MSG.errIrDevice,
        retry: () => void relêDevice(),
      });
    } finally {
      setRelendoDevice(false);
    }
  }, []);

  useEffect(() => {
    void recarrega();
  }, [recarrega]);

  const abrir = useCallback(() => {
    setAberto(true);
    void recarrega();
    void relêDevice();
  }, [recarrega, relêDevice]);

  const fechar = useCallback(() => setAberto(false), []);

  const importa = useCallback(
    async (nome: string, blob: number[]) => {
      try {
        const linha = await irImport(nome, blob);
        await recarrega();
        limpa();
        return linha;
      } catch (e) {
        console.error("ir_import falhou:", e);
        // O motivo do banco vale mais que uma frase genérica: o caso comum
        // aqui é o tamanho que o fio recusa (§13.7), e o dono precisa do
        // número para entender por que o arquivo foi recusado.
        setErr({
          message: e instanceof Error && e.message ? e.message : MSG.errIrImport,
          retry: () => void importa(nome, blob),
        });
        return null;
      }
    },
    [limpa, recarrega],
  );

  const renomeia = useCallback(
    async (id: string, nome: string) => {
      try {
        await irRename(id, nome);
        await recarrega();
        limpa();
      } catch (e) {
        console.error("ir_rename falhou:", e);
        setErr({ message: MSG.errIrWrite, retry: () => void renomeia(id, nome) });
      }
    },
    [limpa, recarrega],
  );

  const atribui = useCallback(
    async (id: string, slot: number | null) => {
      try {
        await irAssignSlot(id, slot);
        await recarrega();
        limpa();
      } catch (e) {
        console.error("ir_assign_slot falhou:", e);
        // O erro do banco (slot ocupado por outro IR) é mais útil que uma
        // frase genérica: quem ocupa o slot é o que o dono precisa saber.
        const msg = e instanceof Error && e.message ? e.message : MSG.errIrSlot;
        setErr({ message: msg, retry: () => void atribui(id, slot) });
      }
    },
    [limpa, recarrega],
  );

  const apaga = useCallback(
    async (id: string) => {
      try {
        await irDelete(id);
        await recarrega();
        limpa();
      } catch (e) {
        console.error("ir_delete falhou:", e);
        setErr({ message: MSG.errIrWrite, retry: () => void apaga(id) });
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
        const rel = await irSend(id);
        setRelatorio(rel);
        await recarrega();
        // O aparelho mudou: a tabela dele é relida depois do envio, senão a
        // tela continuaria mostrando o nome antigo no slot.
        await relêDevice();
        limpa();
      } catch (e) {
        console.error("ir_send falhou:", e);
        setErr({
          message: e instanceof Error && e.message ? e.message : MSG.errIrSend,
          retry: () => void envia(id),
        });
      } finally {
        setEnviando(null);
      }
    },
    [enviando, limpa, recarrega, relêDevice],
  );

  return {
    board,
    deviceSlots,
    err,
    aberto,
    abrir,
    fechar,
    relendoDevice,
    relêDevice,
    importa,
    renomeia,
    atribui,
    apaga,
    enviando,
    envia,
    relatorio,
  };
}
