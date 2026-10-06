/**
 * useAb — o A/B entre duas versões do MESMO patch (#116, VISION §9 item 8).
 *
 * **De onde vêm A e B.** Do histórico versionado (#113): a versão que o
 * registro corrente espelha (A) e a anterior (B) — o mesmo par que o painel
 * de histórico compara por padrão, e por isso o A/B já abre com dois lados
 * que significam alguma coisa. Comparar dois presets quaisquer seria só
 * "abrir outro preset", que o editor já faz; o diferencial é comparar o MESMO
 * patch em dois instantes.
 *
 * **O que a troca FAZ, e o que ela não faz.** Na ordem:
 *
 *   1. **palco** — a cadeia do lado escolhido entra em cena (instantâneo,
 *      mesmo caminho do import de arquivo #114: desenhar não tem fio);
 *   2. **aparelho** — os knobs que DIFEREM saem por `set_param` (§13.11), um
 *      a um, SÓ nos slots cujo algoritmo é o mesmo nos dois lados: o `0x47`
 *      (change-effect) não tem formato capturado (BLOCKERS 10b), então
 *      trocar de algoritmo **não vai para o aparelho** — e a tela diz.
 *
 * O passo 2 é **escrita**: na face (A) da release (ADR-5) ele nasce
 * desabilitado com o motivo, como o knob, o IR e o SnapTone (#126). A troca
 * continua acontecendo na tela, e o painel deixa escrito que o aparelho não
 * recebeu — dizer "enviado" ali seria mentira.
 *
 * **O blind é um estado da TELA.** Armado, este hook não muda nada no que
 * toca; o que muda é o que o painel mostra — rótulo, versão e relatório só
 * voltam depois da resposta, e é o teste de UI que prova que nada vaza antes
 * dela.
 */
import { useCallback, useRef, useState } from "react";
import { libraryGet } from "../ipc/library";
import { libraryVersion, libraryVersions } from "../ipc/history";
import type { LibraryVersionRow } from "../ipc/history";
import { deviceSetParam } from "../ipc/device";
import { calibracao, deltaNivel, nivel, troca, algoritmosTrocados } from "../abLevel";
import type { BoardView } from "../ipc/types";
import { MSG } from "../i18n/messages";

/** Um lado do A/B: a linha do histórico + a cadeia materializada. */
interface AbLado {
  row: LibraryVersionRow;
  board: BoardView;
}

/** Qual lado do A/B. */
type AbQual = "a" | "b";

interface AbErr {
  message: string;
  retry: () => void;
}

interface AbArgs {
  /** Patch aberto no palco — sem ele não há histórico a comparar. */
  openUserId: string | null;
  /** Rótulo do banco (U01…), herdado do palco: os dois lados são o MESMO patch. */
  rotulo: string;
  /** Escrita liberada nesta build (ADR-5 / face (A) da #126). */
  podeGravar: boolean;
  /** Põe a cadeia do lado escolhido no palco (o `useStage` é o dono). */
  aplica: (board: BoardView) => void;
}

export function useAb({ openUserId, rotulo, podeGravar, aplica }: AbArgs) {
  const [a, setA] = useState<AbLado | null>(null);
  const [b, setB] = useState<AbLado | null>(null);
  const [lado, setLado] = useState<AbQual>("a");
  const [blind, setBlind] = useState(false);
  const [resposta, setResposta] = useState<AbQual | null>(null);
  const [relato, setRelato] = useState<string | null>(null);
  const [err, setErr] = useState<AbErr | null>(null);
  const [ocupado, setOcupado] = useState(false);
  // A cadeia que está em cena: o palco pode ter sido mexido depois de abrir,
  // e a próxima troca parte do que OUVE, não da versão guardada.
  const emCenaRef = useRef<BoardView | null>(null);
  const contadorRef = useRef(0);

  /** Materializa a cadeia de uma versão (`payload` é `BoardSlot[]` serializado). */
  const materializa = useCallback(
    async (row: LibraryVersionRow, ppType: number, nomePatch: string): Promise<AbLado | null> => {
      const v = await libraryVersion(row.id);
      if (v?.payload == null) return null;
      let slots: BoardView["slots"];
      try {
        slots = JSON.parse(v.payload) as BoardView["slots"];
      } catch {
        return null; // payload corrompido: o lado não existe, o par não forma
      }
      return {
        row,
        board: {
          pp: -1,
          name: v.name || nomePatch,
          ppType,
          ppTypeName: MSG.userBankType,
          slots,
          bank: "user",
          ppLabel: rotulo,
        },
      };
    },
    [rotulo],
  );

  /**
   * Carrega o par padrão (A = a versão corrente, B = a anterior) e põe A no
   * palco — o aparelho já está com ela, então a primeira troca é a única que
   * tem o que levar.
   */
  const carrega = useCallback(async () => {
    const meu = ++contadorRef.current;
    if (openUserId == null) {
      setA(null);
      setB(null);
      setErr(null);
      return;
    }
    try {
      const rec = await libraryGet(openUserId);
      const lista = await libraryVersions(openUserId);
      if (meu !== contadorRef.current) return;
      if (rec == null || lista.length < 2) {
        // Sem duas versões não há A/B — e o painel diz isso com o motivo,
        // não com um botão morto.
        setA(null);
        setB(null);
        setErr(null);
        return;
      }
      const [primeira, segunda] = lista;
      const ladoA = await materializa(primeira, rec.ppType, rec.name);
      const ladoB = await materializa(segunda, rec.ppType, rec.name);
      if (meu !== contadorRef.current) return;
      if (ladoA == null || ladoB == null) {
        setA(null);
        setB(null);
        return;
      }
      setA(ladoA);
      setB(ladoB);
      setLado("a");
      setBlind(false);
      setResposta(null);
      setRelato(null);
      setErr(null);
      emCenaRef.current = ladoA.board;
      aplica(ladoA.board);
    } catch (e) {
      if (meu !== contadorRef.current) return;
      console.error("useAb: carregar versões falhou:", e);
      setErr({ message: MSG.abErrLoad, retry: () => void carrega() });
    }
  }, [openUserId, materializa, aplica]);

  /** Escolhe um dos lados: palco primeiro, depois o aparelho. */
  const escolhe = useCallback(
    async (qual: AbQual) => {
      const alvo = qual === "a" ? a : b;
      if (alvo == null) return;
      const de = emCenaRef.current;
      aplica(alvo.board);
      emCenaRef.current = alvo.board;
      setLado(qual);
      setResposta(null);
      setErr(null);
      if (de == null) return;
      const cmds = troca(de, alvo.board);
      // Slots cujo algoritmo mudou: a troca VALOR na tela (o palco já desenhou)
      // mas não sai pelo fio — e a tela diz, em vez de fingir que foi completo.
      const fora = algoritmosTrocados(de, alvo.board).length;
      const aviso = fora > 0 ? ` ${MSG.abAlgoritmos(fora)}` : "";
      if (cmds.length === 0) {
        setRelato(fora > 0 ? MSG.abAlgoritmos(fora) : MSG.abRelatoNada);
        return;
      }
      if (!podeGravar) {
        // Face (A): troca na TELA e nada sai. "Enviado" seria mentira.
        setRelato(MSG.abRelatoLocal(cmds.length) + aviso);
        return;
      }
      setOcupado(true);
      try {
        for (const c of cmds) {
          await deviceSetParam(c.slot + 1, c.code, c.pos, Number(c.valor));
        }
        setRelato(MSG.abRelatoEnviado(cmds.length) + aviso);
      } catch (e) {
        console.error("useAb: set_param falhou:", e);
        setErr({ message: MSG.errSetParam, retry: () => void escolhe(qual) });
      } finally {
        setOcupado(false);
      }
    },
    [a, b, aplica, podeGravar],
  );

  /**
   * Iguala o nível: leva os controles de saída COMPARTILHADOS do lado mais
   * ALTO para os valores do lado mais BAIXO — nunca o contrário, porque subir
   * o mais baixo é a mesma falha de comparar volume.
   *
   * O lado que calibrado é o que está soando recebe os `set_param` AGORA; o
   * outro só muda em memória, e os valores vão junto na próxima troca. Um
   * controle que só um lado tem não entra (não há com o que igualá-lo) e
   * sobra como residual no relatório.
   */
  const calibra = useCallback(async () => {
    if (a == null || b == null || !podeGravar) return;
    const na = nivel(a.board);
    const nb = nivel(b.board);
    if (na == null || nb == null || na === nb) return;
    const altoEhA = na > nb;
    const alto = altoEhA ? a : b;
    const baixo = altoEhA ? b : a;
    const ajustes = calibracao(alto.board, baixo.board);
    if (ajustes.length === 0) return;
    const valorDe = (c: { slot: number; pos: number }): string | undefined =>
      baixo.board.slots.find((s) => s.slot === c.slot)?.knobs.find((k) => k.pos === c.pos)?.value;
    const board: BoardView = {
      ...alto.board,
      slots: alto.board.slots.map((s) => {
        const muda = ajustes.filter((c) => c.slot === s.slot);
        if (muda.length === 0) return s;
        const por = new Map(muda.map((c) => [c.pos, c]));
        return {
          ...s,
          knobs: s.knobs.map((k) => {
            const c = por.get(k.pos);
            const novo = c == null ? undefined : valorDe(c);
            return novo == null || novo === k.value ? k : { ...k, value: novo };
          }),
        };
      }),
    };
    const antes = deltaNivel(a.board, b.board);
    const soando = (altoEhA ? lado === "a" : lado === "b") || emCenaRef.current == null;
    if (soando) {
      aplica(board);
      emCenaRef.current = board;
      setOcupado(true);
      try {
        for (const c of ajustes) {
          const valor = valorDe(c);
          if (valor == null) continue;
          await deviceSetParam(c.slot + 1, board.slots.find((s) => s.slot === c.slot)!.code, c.pos, Number(valor));
        }
      } catch (e) {
        console.error("useAb: calibração falhou:", e);
        setErr({ message: MSG.errSetParam, retry: () => void calibra() });
        return;
      } finally {
        setOcupado(false);
      }
    }
    const novoA = altoEhA ? { ...a, board } : a;
    const novoB = altoEhA ? b : { ...b, board };
    setA(novoA);
    setB(novoB);
    const depois = deltaNivel(novoA.board, novoB.board);
    setRelato(MSG.abRelatoCalibra(ajustes.length, (antes ?? 0).toFixed(1), (depois ?? 0).toFixed(1)));
  }, [a, b, lado, podeGravar, aplica]);

  /** "Trocar de lado" do blind: o rótulo não diz para onde — é o ponto. */
  const trocaLado = useCallback(() => void escolhe(lado === "a" ? "b" : "a"), [escolhe, lado]);

  const armaBlind = useCallback((on: boolean) => {
    setBlind(on);
    setResposta(null);
    setRelato(null);
  }, []);

  /** Responde o blind: a tela só volta a nomear os lados DAQUI para frente. */
  const responde = useCallback((qual: AbQual) => setResposta(qual), []);

  const reinicia = useCallback(() => setResposta(null), []);

  return {
    a,
    b,
    lado,
    blind,
    resposta,
    acertou: resposta != null && resposta === lado,
    relato,
    err,
    ocupado,
    disponivel: a != null && b != null,
    carrega,
    escolhe,
    calibra,
    trocaLado,
    armaBlind,
    responde,
    reinicia,
    clearErr: useCallback(() => setErr(null), []),
  };
}

export type Ab = ReturnType<typeof useAb>;
