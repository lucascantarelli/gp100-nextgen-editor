/**
 * useGain — o estado do assistente de gain staging (#115).
 *
 * **Só há UM estado aqui: o relatório em andamento.** O assistente não mexe no
 * palco, não mexe no aparelho e não guarda nada entre aberturas: quem abre de
 * novo pede uma leitura nova (`carrega`). Por isso o hook é pequeno e o
 * `aberto` NÃO mora nele — quem abre e fecha é o dono da porta (`useContentMenu`),
 * que já decide qual tela está no ar.
 *
 * **Falha tem ação (#20).** A leitura pode falhar (arquivo/embutido), e o
 * relatório é o que o dono veio ver: o erro vira `{message, retry}` e o retry
 * relê o MESMO pp — nunca um spinner sem saída.
 */
import { useCallback, useState } from "react";
import { MSG } from "../i18n/messages";
import { presetGainReport } from "../ipc/gain";
import type { Relatorio } from "../ipc/gain";

/** Erro com ação: a mensagem é do catálogo e o retry refaz a leitura. */
interface ErroGain {
  message: string;
  retry: () => void;
}

export interface Gain {
  /** O relatório lido (null enquanto a primeira leitura não volta). */
  relatorio: Relatorio | null;
  /** A leitura está em andamento? */
  carregando: boolean;
  /** Falha com ação de retry. */
  err: ErroGain | null;
  /** (Re)lê o relatório do preset `pp`. Chamado ao abrir a tela. */
  carrega: () => void;
}

export function useGain(pp: number): Gain {
  const [relatorio, setRelatorio] = useState<Relatorio | null>(null);
  const [carregando, setCarregando] = useState(false);
  const [err, setErr] = useState<ErroGain | null>(null);

  const carregaAlvo = useCallback((alvo: number) => {
    setCarregando(true);
    void presetGainReport(alvo)
      .then((r) => {
        setRelatorio(r);
        setErr(null);
      })
      .catch((e: unknown) => {
        // O detalhe técnico fica no console; a tela mostra a mensagem amigável.
        console.error("preset_gain_report falhou:", e);
        setErr({ message: MSG.errGainReport, retry: () => carregaAlvo(alvo) });
      })
      .finally(() => setCarregando(false));
  }, []);

  const carrega = useCallback(() => carregaAlvo(pp), [carregaAlvo, pp]);

  return { relatorio, carregando, err, carrega };
}
