/**
 * useHistory — o estado do HISTÓRICO do patch (issue #113).
 *
 * **O que este hook é.** A lista de versões de UM patch, o diff entre duas
 * delas e as duas restaurações (total e pontual). Ele é o dono da política de
 * erro, do par selecionado e do relato — o componente só desenha.
 *
 * **A regra do par selecionado, e por que ela é do hook.** Comparar é escolha
 * do DONO, e a escolha precisa sobreviver a um reload da lista: se o par
 * morasse no componente, o primeiro `restaura` (que recarrega a lista) faria a
 * seleção sumir, e o diff sumiria junto — que é o oposto do que a pessoa foi
 * fazer ali.
 *
 * **Falha tem ação, igual no `useStage` e no `useLibrary`.** A lista vem do
 * SQLite, e banco em disco pode sumir (HD externo, antivírus, permissão). O
 * dono precisa de um caminho de volta, e um banner que só diz "erro" não é
 * caminho de volta.
 *
 * **Leitura bem-sucedida NÃO limpa o banner.** O mesmo motivo do `useLibrary`:
 * o diff dispara sozinho quando o par muda, e limpar ali apagaria o erro de
 * uma gravação que falhou um instante antes. Quem limpa é o RETRY, e só quando
 * a repetição dá certo.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import {
  libraryDiff,
  libraryRestoreKnob,
  libraryRestoreVersion,
  libraryVersions,
} from "../ipc/history";
import type { ChainDiff, LibraryVersionRow } from "../ipc/history";
import { MSG } from "../i18n/messages";

/** Erro amigável + AÇÃO de recuperação (mesmo contrato do `useStage`). */
interface HistoryError {
  message: string;
  retry: () => void;
}

/** O que o painel de histórico consome. */
export interface History {
  /** Linhas do histórico, da mais nova para a mais antiga. */
  versoes: LibraryVersionRow[];
  /** Diff entre `antes` e `depois`; `null` = ainda não escolhido (ou vazio). */
  diff: ChainDiff | null;
  /** Id da versão "antes" escolhida. */
  antes: number | null;
  /** Id da versão "depois" escolhida. */
  depois: number | null;
  /** Patch aberto (o histórico de outro patch zera o que está na tela). */
  patchId: string | null;
  /** Abre o histórico de um patch e escolhe o par mais útil por padrão. */
  abre: (presetId: string) => void;
  /** Fecha o histórico (volta ao estado inicial). */
  fecha: () => void;
  /** Troca uma ponta do par; o diff é recalculado. */
  compara: (antes: number | null, depois: number | null) => void;
  /** Restauração total. `false` = falhou (o banner já explica). */
  restaura: (id: number) => Promise<boolean>;
  /** Restauração pontual de um knob. */
  restauraKnob: (id: number, slot: number, pos: number) => Promise<boolean>;
  /** Relato da última restauração (a tela diz "virou a versão N"). */
  relato: string | null;
  /** Erro com retry, ou `null`. */
  err: HistoryError | null;
  /** Uma escrita em andamento — os botões desabilitam enquanto ela corre. */
  ocupado: boolean;
  /** Limpa o erro. */
  clearErr: () => void;
}

/**
 * O par que a pessoa vai querer ver primeiro: a ÚLTIMA versão contra a
 * penúltima. É o que responde "o que eu mudei desde a última vez que mexi".
 *
 * `@param versaoAnterior` existe porque o hook não deve supor posição de
 * array: quem chama tem a lista e escolhe o que é o "antes".
 */
function parPadrao(versoes: LibraryVersionRow[]): { antes: number | null; depois: number | null } {
  const [depois, antes] = versoes;
  return { antes: antes?.id ?? null, depois: depois?.id ?? null };
}

/**
 * @param onMudou chamado depois de uma restauração que deu certo — é quem
 *   reabre o patch no palco (o board precisa refletir a cadeia nova) e quem
 *   relê a lista da biblioteca.
 */
export function useHistory(onMudou?: () => void): History {
  const [versoes, setVersoes] = useState<LibraryVersionRow[]>([]);
  const [diff, setDiff] = useState<ChainDiff | null>(null);
  const [antes, setAntes] = useState<number | null>(null);
  const [depois, setDepois] = useState<number | null>(null);
  const [patchId, setPatchId] = useState<string | null>(null);
  const [relato, setRelato] = useState<string | null>(null);
  const [err, setErr] = useState<HistoryError | null>(null);
  const [ocupado, setOcupado] = useState(false);

  /**
   * Contador de pedidos: a lista dispara sozinha (troca de par, restauração) e
   * as respostas podem chegar fora de ordem. Sem isto, o diff de um par lento
   * poderia chegar DEPOIS do de um par rápido e a tela mostraria a comparação
   * errada — que é pior do que não mostrar nada, porque ela parece certa.
   */
  const contador = useRef(0);
  const mudouRef = useRef(onMudou);
  mudouRef.current = onMudou;

  const recarrega = useCallback(async (presetId: string) => {
    const lista = await libraryVersions(presetId);
    setVersoes(lista);
    return lista;
  }, []);

  /** Lê o diff de um par. `null` quando o par não tem duas pontas. */
  const leDiff = useCallback(async (a: number | null, d: number | null) => {
    const meu = ++contador.current;
    if (a == null || d == null) {
      setDiff(null);
      return;
    }
    try {
      const d2 = await libraryDiff(a, d);
      if (meu !== contador.current) return;
      setDiff(d2);
    } catch {
      // Uma falha ao COMPARAR não pode derrubar a tela de histórico: a lista
      // continua válida e o dono ainda pode restaurar por ela. O que não pode
      // é o diff sumir sem explicação, então ele vira "não consegui ler".
      if (meu !== contador.current) return;
      setDiff(null);
    }
  }, []);

  const abre = useCallback(
    (presetId: string) => {
      setPatchId(presetId);
      setRelato(null);
      void (async () => {
        try {
          const lista = await recarrega(presetId);
          const par = parPadrao(lista);
          setAntes(par.antes);
          setDepois(par.depois);
          await leDiff(par.antes, par.depois);
        } catch (e) {
          console.error("library_versions falhou:", e);
          setErr({ message: MSG.errLibrarySearch, retry: () => abre(presetId) });
        }
      })();
    },
    [leDiff, recarrega],
  );

  const fecha = useCallback(() => {
    contador.current++;
    setPatchId(null);
    setVersoes([]);
    setDiff(null);
    setAntes(null);
    setDepois(null);
    setRelato(null);
  }, []);

  const compara = useCallback(
    (a: number | null, d: number | null) => {
      setAntes(a);
      setDepois(d);
      void leDiff(a, d);
    },
    [leDiff],
  );

  /** Releitura depois de uma escrita: a lista cresce e o par volta ao padrão. */
  const reescolhe = useCallback(
    async (presetId: string) => {
      const lista = await recarrega(presetId);
      const par = parPadrao(lista);
      setAntes(par.antes);
      setDepois(par.depois);
      await leDiff(par.antes, par.depois);
    },
    [leDiff, recarrega],
  );

  const restaura = useCallback(
    async (id: number): Promise<boolean> => {
      const alvo = patchId;
      if (!alvo) return false;
      setOcupado(true);
      try {
        const criada = await libraryRestoreVersion(id);
        await reescolhe(alvo);
        setRelato(MSG.histRestored(criada.seq));
        mudouRef.current?.();
        return true;
      } catch (e) {
        console.error("library_restore_version falhou:", e);
        setErr({ message: MSG.errLibrarySave, retry: () => void restaura(id) });
        return false;
      } finally {
        setOcupado(false);
      }
    },
    [patchId, reescolhe],
  );

  const restauraKnob = useCallback(
    async (id: number, slot: number, pos: number): Promise<boolean> => {
      const alvo = patchId;
      if (!alvo) return false;
      setOcupado(true);
      try {
        const criada = await libraryRestoreKnob(id, slot, pos);
        await reescolhe(alvo);
        setRelato(MSG.histRestored(criada.seq));
        mudouRef.current?.();
        return true;
      } catch (e) {
        console.error("library_restore_knob falhou:", e);
        setErr({ message: MSG.errLibrarySave, retry: () => void restauraKnob(id, slot, pos) });
        return false;
      } finally {
        setOcupado(false);
      }
    },
    [patchId, reescolhe],
  );

  /** O botão "voltar ao histórico" também limpa o banner depois de repetir. */
  const clearErr = useCallback(() => setErr(null), []);

  // fechar por tecla é do componente (ele conhece o elemento); aqui só
  // garantimos que a lista não fica carregando para o infinito depois de sair.
  useEffect(() => () => { contador.current++; }, []);

  return {
    versoes,
    diff,
    antes,
    depois,
    patchId,
    abre,
    fecha,
    compara,
    restaura,
    restauraKnob,
    relato,
    err,
    ocupado,
    clearErr,
  };
}
