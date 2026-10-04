/**
 * useLibrary — o estado da biblioteca do dono (issue #26).
 *
 * A #26 tira os patches de usuário do `localStorage` e os põe no banco. Este
 * hook é o dono dessa transição, e ele carrega os invariantes que antes eram
 * promessas implícitas espalhadas pelo `userPatches.ts` (que agora é só a
 * fronteira entre o retrato e o registro do banco):
 *
 * - **A busca é do device, não é um filtro local.** O texto vai para o banco.
 *   Isso importa quando a lista deixar de caber na memória: filtrar no front
 *   seria mentir sobre o custo.
 * - **Falha tem ação.** Como no `useStage` (issue #20), todo erro vira
 *   `{message, retry}` — a biblioteca em disco pode sumir (HD externo,
 *   antivírus, permissão) e o dono precisa de um caminho de volta.
 * - **A migração é idempotente e só apaga a chave depois do import.** Um app
 *   fechado no meio da migração não perde patch; rodar de novo não duplica.
 *
 * **Quem manda no banner.** Uma leitura bem-sucedida NÃO limpa o erro: a busca
 * dispara sozinha (debounce, troca de aba) e limpar ali apagaria o erro de uma
 * gravação que falhou um instante antes — o dono veria a falha sumir sem ter
 * feito nada. Quem limpa é o RETRY, e só quando a repetição dá certo.
 *
 * O que NÃO mora aqui: o palco (`App`) continua dono do que está aberto. A
 * biblioteca só diz o que existe; quem abre é o palco.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import {
  libraryDelete,
  libraryExport,
  libraryGet,
  libraryImport,
  librarySave,
  librarySearch,
  libraryStats,
  lerLegadoCru,
  migraLegado,
} from "../ipc/library";
import type { ImportReport, LibraryPreset, LibraryQuery, LibraryRecord, LibraryStats } from "../ipc/library";
import { MSG } from "../i18n/messages";

/** Erro amigável + AÇÃO de recuperação (mesmo contrato do `useStage`). */
interface LibraryError {
  message: string;
  retry: () => void;
}

/** Banco visível na biblioteca (a aba aberta no painel). */
export type Banco = "factory" | "user";

/** Filtro de estilo: `null` = todos. */
type FiltroEstilo = number | null;

/**
 * Retry que só limpa o banner se a REPETIÇÃO deu certo.
 *
 * O `if` não é vaidade: sem ele, um retry que falha de novo terminaria com o
 * banner limpo (o `limpa()` viria depois do novo `setErr`), e o dono ficaria
 * sem nenhuma pista de que a segunda tentativa também falhou.
 */
function refeito(limpa: () => void, op: () => Promise<boolean>): () => void {
  return () => {
    void (async () => {
      if (await op()) limpa();
    })();
  };
}

/** O que o painel da biblioteca consome. */
export interface Library {
  /** Banco visível (a aba aberta). A lista é SEMPRE deste banco. */
  banco: Banco;
  /** Troca a aba — e com ela a lista. */
  setBanco: (b: Banco) => void;
  /** Resultados da busca atual, do banco visível. */
  rows: LibraryPreset[];
  /** Está buscando agora (a lista antiga continua visível). */
  buscando: boolean;
  /** Texto digitado na caixa. */
  texto: string;
  /** Escreve na caixa (debounce depois). */
  setTexto: (t: string) => void;
  /** Estilo selecionado (`null` = todos). */
  estilo: FiltroEstilo;
  /** Troca o estilo (o id do tipo; `null` volta para "todos"). */
  setEstilo: (e: FiltroEstilo) => void;
  /** Números da biblioteca (rodapé). */
  stats: LibraryStats | null;
  /** Erro com retry, ou `null`. */
  err: LibraryError | null;
  /** Limpa o erro. */
  clearErr: () => void;
  /**
   * Lê UM registro COM a cadeia. `null` = não existe mais (o dono apagou o
   * patch em outro lugar — a UI volta para a fábrica em vez de abrir vazio).
   */
  carrega: (id: string) => Promise<LibraryRecord | null>;
  /** Apaga um registro e re-lê a lista. */
  apaga: (id: string) => Promise<void>;
  /** Grava (ou regrava) um registro e re-lê a lista. */
  salva: (p: Parameters<typeof librarySave>[0]) => Promise<void>;
  /** Exporta e devolve o JSON (para o botão "baixar"). */
  exportar: () => Promise<string | null>;
  /** Importa um envelope; `replace` decide o conflito. */
  importar: (json: string, replace: boolean) => Promise<ImportReport | null>;
  /** Havia patches no `localStorage` que foram para o banco? */
  migrouLegado: boolean;
}

/** Atraso do debounce da busca: curto o bastante para parecer imediato. */
const DEBOUNCE_MS = 180;

/**
 * @param bancoInicial o banco que o PALCO abre. É só a semente: quem manda na
 *   lista é a aba escolhida pelo dono, e o painel é quem sincroniza as duas
 *   (trocar de aba relê; abrir um patch no palco segue para a aba dele).
 *
 * Se a consulta saísse da prop direto, clicar na aba do dono buscaria entre os
 * 99 de fábrica e a lista do dono apareceria vazia sem nenhuma mensagem — foi
 * exatamente o que o teste de integração pegou.
 */
export function useLibrary(bancoInicial: Banco = "factory"): Library {
  const [bancoVisivel, setBanco] = useState<Banco>(bancoInicial);
  const [rows, setRows] = useState<LibraryPreset[]>([]);
  const [buscando, setBuscando] = useState(false);
  const [texto, setTexto] = useState("");
  const [estilo, setEstilo] = useState<FiltroEstilo>(null);
  const [stats, setStats] = useState<LibraryStats | null>(null);
  const [err, setErr] = useState<LibraryError | null>(null);
  const [migrouLegado, setMigrouLegado] = useState(false);

  /** Limpa o banner. Estável, porque vive DENTRO dos retries. */
  const limpa = useCallback(() => setErr(null), []);

  /**
   * A consulta que será executada: banco visível + texto aparado + estilo.
   *
   * Fica num REF, e não só num objeto de render, porque é isso que permite
   * `recarrega()` ser estável: `salva`/`apaga`/`importar` precisam reler a
   * lista com o filtro que está na tela sem capturar o `texto` no `useCallback`
   * (que os re-criaria a cada tecla e re-dispararia a busca).
   */
  const consulta = useRef<LibraryQuery>({});
  consulta.current = {
    bank: bancoVisivel,
    text: texto.trim() || undefined,
    ppType: estilo ?? undefined,
  };

  /**
   * O `contador` é a razão de a busca não ter um "useEffect que volta atrás".
   * Cada digitação dispara uma requisição; as respostas chegam fora de ordem, e
   * sem isso a busca de "bl" (lenta) pode chegar DEPOIS da de "blink" (rápida)
   * e a lista mostrar o resultado velho. Só a última requisição escreve.
   */
  const contador = useRef(0);

  const busca = useCallback(
    async (q: LibraryQuery): Promise<boolean> => {
      const meu = ++contador.current;
      setBuscando(true);
      try {
        const resultado = await librarySearch(q);
        if (meu !== contador.current) return false; // resposta velha: não escreve
        setRows(resultado);
        // sucesso da LEITURA não limpa o banner (ver o doc do arquivo): quem
        // limpa é o retry de quem falhou
        return true;
      } catch (e) {
        console.error("library_search falhou:", e);
        if (meu !== contador.current) return false;
        setErr({ message: MSG.errLibrarySearch, retry: refeito(limpa, () => busca(q)) });
        return false;
      } finally {
        if (meu === contador.current) setBuscando(false);
      }
    },
    [limpa],
  );

  /**
   * Trocar de ABA limpa a lista.
   *
   * Sem isto, a busca do banco novo levava 180ms para voltar e a tela mostrava
   * as linhas do banco ANTIGO com o rótulo do novo: os 99 presets de fábrica
   * apareciam como "U01…U99" na aba do dono, e clicar num deles abria o patch
   * errado sem nenhum erro. Durante a busca de TEXTO (mesmo banco) a lista
   * antiga fica: é o mesmo conjunto, só com outro filtro, e piscar a cada tecla
   * seria pior.
   */
  useEffect(() => {
    setRows([]);
  }, [bancoVisivel]);

  // A busca acompanha a consulta. O debounce existe para a DIGITAÇÃO (uma
  // requisição por tecla, cada uma atravessando o IPC) — não para a primeira
  // carga, nem para a troca de aba ou de estilo, que sao cliques.
  //
  // A distinção é visível: com debounce na primeira busca, o painel abre
  // VAZIO por 180 ms a cada boot, e o dono vê uma biblioteca que ainda não
  // chegou. Sem isto, os testes que leem a lista depois de um microtask
  // perdiam a corrida com o timer.
  const primeiroRef = useRef(true);
  useEffect(() => {
    const primeiro = primeiroRef.current;
    primeiroRef.current = false;
    if (primeiro) {
      void busca(consulta.current);
      return;
    }
    const t = setTimeout(() => void busca(consulta.current), DEBOUNCE_MS);
    return () => clearTimeout(t);
    // A consulta vive num ref (por que), então as dependências são os
    // PRIMITIVOS que a formam — inclusive a aba, que também muda a busca.
  }, [texto, estilo, bancoVisivel, busca]);

  /** Relê a lista com o filtro QUE ESTÁ NA TELA (pós-gravação, pós-import). */
  const recarrega = useCallback(() => busca(consulta.current), [busca]);

  // Números: uma vez no mount e depois de cada escrita.
  const leStats = useCallback(async (): Promise<boolean> => {
    try {
      setStats(await libraryStats());
      return true;
    } catch (e) {
      console.error("library_stats falhou:", e);
      setErr({ message: MSG.errLibraryStats, retry: refeito(limpa, () => leStats()) });
      return false;
    }
  }, [limpa]);

  useEffect(() => {
    void leStats();
  }, [leStats]);

  // Migração do `localStorage` — UMA vez.
  //
  // A PERGUNTA ("há o que migrar?") é respondida de forma SÍNCRONA, antes de
  // qualquer `await`, e é ela que liga a flag. O motivo não é performance: em
  // StrictMode este efeito roda duas vezes, e a segunda só encontra a chave se
  // a primeira ainda não a apagou — então quem decidisse depois do `await`
  // mostraria "seus patches foram movidos" na metade dos boots.
  useEffect(() => {
    const cru = lerLegadoCru();
    if (cru === null) return;
    setMigrouLegado(true);

    let vivo = true;
    /** Um passo de migração: importar, reler a lista e os números. */
    const migraUmaVez = async (): Promise<boolean> => {
      await migraLegado();
      if (!vivo) return false;
      await recarrega();
      await leStats();
      return true;
    };

    void (async () => {
      try {
        // deu certo: some com o banner de uma tentativa anterior, senão o dono
        // não sabe se funcionou
        if (await migraUmaVez()) limpa();
      } catch (e) {
        console.error("migracao do localStorage falhou:", e);
        if (!vivo) return;
        setErr({ message: MSG.errLibrarySearch, retry: refeito(limpa, () => migraUmaVez()) });
      }
    })();
    return () => {
      vivo = false;
    };
  }, [leStats, limpa, recarrega]);

  const salva = useCallback(
    async (p: Parameters<typeof librarySave>[0]) => {
      try {
        await librarySave(p);
        await leStats();
        await recarrega();
        limpa();
      } catch (e) {
        console.error("library_save falhou:", e);
        setErr({ message: MSG.errLibrarySave, retry: () => void salva(p) });
      }
    },
    [leStats, limpa, recarrega],
  );

  /** Lê um registro com a cadeia (o palco precisa dos slots, a lista não tem). */
  const carrega = useCallback(
    async (id: string) => {
      try {
        const rec = await libraryGet(id);
        // leitura bem-sucedida limpa o banner: abrir um patch é a ação do dono
        // neste instante, e o erro que estiver na tela é de outra coisa
        limpa();
        return rec;
      } catch (e) {
        console.error("library_get falhou:", e);
        setErr({ message: MSG.errLibrarySearch, retry: refeito(limpa, () => carrega(id).then(() => true)) });
        return null;
      }
    },
    [limpa],
  );

  const apaga = useCallback(
    async (id: string) => {
      try {
        await libraryDelete(id);
        await leStats();
        await recarrega();
        limpa();
      } catch (e) {
        console.error("library_delete falhou:", e);
        setErr({ message: MSG.errLibraryDelete, retry: () => void apaga(id) });
      }
    },
    [leStats, limpa, recarrega],
  );

  const exportar = useCallback(async () => {
    try {
      const json = await libraryExport();
      limpa();
      return json;
    } catch (e) {
      console.error("library_export falhou:", e);
      setErr({ message: MSG.errLibraryExport, retry: () => void exportar() });
      return null;
    }
  }, [limpa]);

  const importar = useCallback(
    async (json: string, replace: boolean) => {
      try {
        const rel = await libraryImport(json, replace);
        await leStats();
        await recarrega();
        limpa();
        return rel;
      } catch (e) {
        console.error("library_import falhou:", e);
        setErr({ message: MSG.errLibraryImport, retry: () => void importar(json, replace) });
        return null;
      }
    },
    [leStats, limpa, recarrega],
  );

  return {
    banco: bancoVisivel,
    setBanco,
    rows,
    buscando,
    texto,
    setTexto,
    estilo,
    setEstilo,
    stats,
    err,
    clearErr: limpa,
    carrega,
    apaga,
    salva,
    exportar,
    importar,
    migrouLegado,
  };
}