/**
 * useSyncState — o estado de sincronização com o APARELHO (#164).
 *
 * Nasce `idle` (sem leitura ainda: sem número falso) e é semeado pelo MESMO
 * `device_info` que o resto da UI lê — o chip nunca tem uma segunda fonte
 * de verdade. Depois, o evento `device://sync` é a fonte: cada transição
 * (`pending` na escrita, `sync`/`failed` no read-back, `offline` na queda)
 * chega do actor, que é quem toca o fio.
 *
 * O vocabulário é o do backend (`SyncStatus`): este hook não traduz nem
 * inventa estado — entrega o que o device afirmou, com o detalhe humano de
 * `failed`/`offline`.
 *
 * Contrato de teardown (mesmo padrão dos outros listeners do app): o
 * unlisten só vale para quem ainda está vivo — um unlisten que chega depois
 * do cleanup chama a si mesmo, e um evento que chega depois do cleanup não
 * grava estado.
 */
import { useEffect, useState } from "react";
import type { SyncEvent } from "../ipc/types";
import { deviceInfo, onDeviceSync } from "../ipc/device";

export function useSyncState(): SyncEvent {
  const [st, setSt] = useState<SyncEvent>({ status: "idle", changed: false, detail: "" });

  // Semente: o estado que o BACKEND já tem (o app pode ter sido reaberto no
  // meio de uma sessão). O guarda `idle` faz a verdade mais NOVA vencer:
  // se um evento chegar antes do device_info resolver, o evento fica.
  useEffect(() => {
    let vivo = true;
    void deviceInfo()
      .then((i) => {
        if (!vivo) return;
        setSt((cur) =>
          cur.status === "idle" ? { status: i.sync, changed: false, detail: i.syncDetail } : cur,
        );
      })
      .catch(() => {
        /* sem device_info o chip fica honesto em `idle` */
      });
    return () => {
      vivo = false;
    };
  }, []);

  useEffect(() => {
    let vivo = true;
    let un: (() => void) | undefined;
    void onDeviceSync((ev) => {
      if (vivo) setSt(ev);
    }).then((u) => {
      if (vivo) un = u;
      else u();
    });
    return () => {
      vivo = false;
      un?.();
    };
  }, []);

  return st;
}
