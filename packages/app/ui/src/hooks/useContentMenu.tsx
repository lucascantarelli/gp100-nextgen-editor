/**
 * useContentMenu — o dono da PORTA do conteudo (#25/#24/#114/#115/#116).
 *
 * **Por que isto e um hook e nao um `useState` na casca.** A escolha entre as
 * telas (tons, IRs, preset em arquivo, assistente de gain staging, A/B com
 * blind) e estado de sessao: decide qual painel fica no ar. Deixar isto no `App.tsx` incharia a casca com um bloco de JSX mais
 * os callbacks que montam o modal — e a #82 deixou a casca em 300 linhas DEPOIS
 * do corte. O gate de tamanho cobraria a conta com um `--allow`, que e
 * exatamente a excecao que o proprio script do gate manda nao acumular.
 *
 * **O que o hook NAO sabe dos tons e dos IRs.** Ele nao conhece nenhum dos dois:
 * recebe as manoplas de `abrir` e escolhe qual chamar. Assim o menu nao ganha
 * import de `useTones`/`useIrs` e o teste do menu so depende dos botoes.
 *
 * **Três telas montam AQUI, e e deliberado.** Tons e IRs guardam estado que
 * precisa sobreviver ao modal (o A/B do tom, um envio em andamento), entao sao
 * hooks proprios cuja vida a casca possui. O preset em arquivo e o assistente de
 * gain staging (#115) nao: os dois leem o preset DO PALCO no instante em que
 * abrem (o gain rele a cada abertura) e nada entre uma abertura e a seguinte
 * precisa sobreviver. O A/B (#116) monta aqui pelos DOIS motivos: a casca esta
 * no teto (300/300) e nao paga linha por mais um modal, e a tela precisa do
 * `aplica` (a mesma porta de volta do import) e do `podeGravar` da porta. A
 * leitura do par de versoes — e com ela uma SESSOA NOVA de blind — roda na
 * abertura, junto com o gancho do gain: sem isto o hook leria (e poria no
 * palco) na abertura do app, com a tela fechada.
 */
import { useCallback, useState } from "react";
import { ContentMenu } from "../components/ContentMenu";
import { PresetFilePanel } from "../components/PresetFilePanel";
import { GainPanel } from "../components/GainPanel";
import { AbPanel } from "../components/AbPanel";
import { useGain } from "./useGain";
import { useAb } from "./useAb";
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
  /** O A/B (#116): patch aberto, rotulo do palco, escrita e o `openImported`. */
  ab: { openUserId: string | null; rotulo: string; podeGravar: boolean };
}

export function useContentMenu({ tones, irs, preset, ab: abAlvos }: ContentMenuAlvos) {
  const [aberto, setAberto] = useState(false);
  const [presetAberto, setPresetAberto] = useState(false);
  const [gainAberto, setGainAberto] = useState(false);
  const [abAberto, setAbAberto] = useState(false);
  // O assistente (#115) analisa o MESMO preset que o palco desenha. O hook da
  // leitura mora aqui (e nao na casca) pelo mesmo motivo do painel do preset em
  // arquivo: a tela nasce como destino da porta, sem estado para preservar — o
  // App.tsx nao paga linha por ela.
  const gain = useGain(preset.pp);
  // O A/B (#116) mora aqui pela mesma razão das outras telas da porta: a
  // casca está no teto e a tela precisa do `aplica`/`podeGravar` vindo de lá.
  // A leitura do par (e a sessão nova de blind) roda no `abrirAb`, não no
  // mount — ver o cabeçalho do arquivo.
  const ab = useAb({
    openUserId: abAlvos.openUserId,
    rotulo: abAlvos.rotulo,
    podeGravar: abAlvos.podeGravar,
    aplica: preset.onImport,
  });
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
  // Abrir o assistente DISPARA a leitura: ele nao tem "estado de leitura" a
  // preservar, e reler a cada abertura cobre o preset ter mudado no palco.
  const { carrega: carregaGain } = gain;
  const abrirGain = useCallback(() => {
    setAberto(false);
    setGainAberto(true);
    carregaGain();
  }, [carregaGain]);
  const fecharGain = useCallback(() => setGainAberto(false), []);
  // Abrir o A/B DISPARA a leitura do par (igual ao gain): o hook fica montado
  // com a casca, entao sem este gatilho ele leria — e mudaria o palco — na
  // abertura do app, com a tela fechada. Cada abertura tambem e uma sessão
  // NOVA de blind: `carrega` rearma o par e desarma o blind, que é o que se
  // espera de "começar a ouvir de novo".
  const { carrega: carregaAb } = ab;
  const abrirAb = useCallback(() => {
    setAberto(false);
    setAbAberto(true);
    carregaAb();
  }, [carregaAb]);
  const fecharAb = useCallback(() => setAbAberto(false), []);

  const node = aberto ? (
    <ContentMenu
      onTones={abrirTons}
      onIrs={abrirIrs}
      onPreset={abrirPreset}
      onGain={abrirGain}
      onAb={abrirAb}
      onClose={fechar}
    />
  ) : presetAberto ? (
    <PresetFilePanel pp={preset.pp} onImport={preset.onImport} onClose={fecharPreset} />
  ) : gainAberto ? (
    <GainPanel gain={gain} pp={preset.pp} onClose={fecharGain} />
  ) : abAberto ? (
    <AbPanel ab={ab} podeGravar={abAlvos.podeGravar} onClose={fecharAb} />
  ) : null;

  return { abrir, node };
}
