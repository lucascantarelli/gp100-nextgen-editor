/**
 * useContentMenu — o dono da PORTA do conteudo (#25/#24/#114).
 *
 * **Por que isto e um hook e nao um `useState` na casca.** A escolha entre as
 * telas (tons, IRs, preset em arquivo) e estado de sessao: decide qual painel
 * fica no ar. Deixar isto no `App.tsx` incharia a casca com um bloco de JSX mais
 * os callbacks que montam o modal — e a #82 deixou a casca em 300 linhas DEPOIS
 * do corte. O gate de tamanho cobraria a conta com um `--allow`, que e
 * exatamente a excecao que o proprio script do gate manda nao acumular.
 *
 * **O que o hook NAO sabe dos tons e dos IRs.** Ele nao conhece nenhum dos dois:
 * recebe as manoplas de `abrir` e escolhe qual chamar. Assim o menu nao ganha
 * import de `useTones`/`useIrs` e o teste do menu so depende dos botoes.
 *
 * **O preset em arquivo e a EXCECAO, e e deliberado.** As outras duas telas
 * guardam estado que precisa sobreviver ao modal (o A/B do tom, um envio em
 * andamento), entao sao hooks proprios; a tela do preset em arquivo nao tem
 * estado nenhum para preservar — ela le o preset DO PALCO no instante em que
 * abre e devolve o importado por um callback. Pôr um terceiro hook na casca so
 * para montar um modal sem estado seria pagar linha no `App.tsx` por nada.
 */
import { useCallback, useState } from "react";
import { ContentMenu } from "../components/ContentMenu";
import { PresetFilePanel } from "../components/PresetFilePanel";
import type { BoardView } from "../ipc/types";

/** O preset que a tela de arquivo exporta (o do palco) e onde o importado cai. */
interface PresetNoPalco {
  /** Nº do preset de fabrica aberto no palco. */
  pp: number;
  /** Aplica no palco a cadeia de um arquivo importado. */
  onImport: (board: BoardView) => void;
}

interface ContentMenuAlvos {
  /** Abre o gestor de tons (SnapTone/NAM). */
  tones: { abrir: () => void };
  /** Abre o laboratorio de IRs. */
  irs: { abrir: () => void };
  /** O preset do palco + a porta de volta do arquivo importado. */
  preset: PresetNoPalco;
}

export function useContentMenu({ tones, irs, preset }: ContentMenuAlvos) {
  const [aberto, setAberto] = useState(false);
  const [presetAberto, setPresetAberto] = useState(false);
  const fechar = useCallback(() => setAberto(false), []);
  /** Abre a porta do conteudo (botao unico no rodape da biblioteca). */
  const abrir = useCallback(() => setAberto(true), []);
  // Escolher um destino FECHA a porta: as telas sao modais exclusivos (dois
  // overlays empilhados nao tem quem feche um sem o outro) e o dono espera
  // voltar para a mesa depois de escolher.
  const abrirTons = useCallback(() => {
    setAberto(false);
    tones.abrir();
  }, [tones]);
  const abrirIrs = useCallback(() => {
    setAberto(false);
    irs.abrir();
  }, [irs]);
  // A tela do preset em arquivo nasce AQUI (e nao num `abrir` de fora) porque
  // ela nao tem estado para preservar — ver o cabecalho do arquivo.
  const abrirPreset = useCallback(() => {
    setAberto(false);
    setPresetAberto(true);
  }, []);
  const fecharPreset = useCallback(() => setPresetAberto(false), []);

  const node = aberto ? (
    <ContentMenu onTones={abrirTons} onIrs={abrirIrs} onPreset={abrirPreset} onClose={fechar} />
  ) : presetAberto ? (
    <PresetFilePanel pp={preset.pp} onImport={preset.onImport} onClose={fecharPreset} />
  ) : null;

  return { abrir, node };
}
