/**
 * useContentMenu — o dono da PORTA do conteudo (#25/#24).
 *
 * **Por que isto e um hook e nao um `useState` na casca.** A escolha entre as
 * duas telas (tons ou IRs) e estado de sessao: sobrevive ao fechar/reabrir a
 * janela e decide qual painel fica no ar. Deixar isto no `App.tsx` incharia a
 * casca com um bloco de 12 linhas de JSX mais tres callbacks que so montam o
 * modal — e a #82 deixou a casca em 300 linhas DEPOIS do corte. O gate de
 * tamanho cobraria a conta com um `--allow`, que e exatamente a excecao que o
 * proprio script do gate manda nao acumular.
 *
 * **O que o hook NAO sabe.** Ele nao conhece os tons nem os IRs: recebe as duas
 * manoplas de `abrir` e escolhe qual chamar. Assim o menu nao ganha import de
 * `useTones`/`useIrs` e o teste do menu so depende dos botoes.
 */
import { useCallback, useState } from "react";
import { ContentMenu } from "../components/ContentMenu";

interface ContentMenuAlvos {
  /** Abre o gestor de tons (SnapTone/NAM). */
  tones: { abrir: () => void };
  /** Abre o laboratorio de IRs. */
  irs: { abrir: () => void };
}

export function useContentMenu({ tones, irs }: ContentMenuAlvos) {
  const [aberto, setAberto] = useState(false);
  const fechar = useCallback(() => setAberto(false), []);
  /** Abre a porta do conteudo (botao unico no rodape da biblioteca). */
  const abrir = useCallback(() => setAberto(true), []);
  // Escolher um destino FECHA a porta: os dois paineis sao modais exclusivos
  // (dois overlays empilhados nao tem quem feche um sem o outro) e o dono
  // espera voltar para a mesa depois de escolher.
  const abrirTons = useCallback(() => {
    setAberto(false);
    tones.abrir();
  }, [tones]);
  const abrirIrs = useCallback(() => {
    setAberto(false);
    irs.abrir();
  }, [irs]);

  const node = aberto ? (
    <ContentMenu onTones={abrirTons} onIrs={abrirIrs} onClose={fechar} />
  ) : null;

  return { abrir, node };
}
