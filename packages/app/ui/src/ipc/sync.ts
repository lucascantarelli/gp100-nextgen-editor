/**
 * ipc/sync — porta de SINCRONIZAÇÃO device ↔ app (#164).
 *
 * Mesmo contrato da porta de device: nenhum componente chama `invoke`/
 * `listen` direto — o command e o evento vivem aqui, o fallback (dev/
 * browser sem webview) usa o MESMO shape do backend real.
 *
 * O estado de sync é o que o BACKEND afirma (idle/sync/pending/failed/
 * offline + `changed` do read-back); o "tempo real" deste protocolo é
 * poll + read-back (`docs/PROTOCOL.md` §sync), e esta porta não inventa
 * contadores nem eventos que o device não emitiu.
 */
import type { SyncEvent } from "./types";
import { debugFail, IDEMPOTENTE, inTauri, runCommand } from "./device";

/** Subscrições locais de `device://sync` (fallback dev/teste; Tauri usa o próprio). */
const syncListeners = new Set<(ev: SyncEvent) => void>();

/**
 * Evento `device://sync` (#164) — transição do estado de sincronização
 * (idle/sync/pending/failed/offline + `changed` do read-back). Devolve o
 * unlisten (chamar no cleanup do hook).
 */
export async function onDeviceSync(cb: (ev: SyncEvent) => void): Promise<() => void> {
  if (inTauri()) {
    const { listen } = await import("@tauri-apps/api/event");
    const un = await listen<SyncEvent>("device://sync", (e) => cb(e.payload));
    return un;
  }
  syncListeners.add(cb);
  return () => {
    syncListeners.delete(cb);
  };
}

/**
 * Produtor do `device://sync` NO FALLBACK — e o único ponto que emite sem
 * o backend (jsdom não tem webview; os testes e o dev-browser usam o MESMO
 * shape do evento real, nunca um shape de teste).
 */
export function emitSync(ev: SyncEvent): void {
  syncListeners.forEach((l) => l(ev));
}

/**
 * `device_sync_now` (#164) — agenda um read-back IMEDIATO do preset
 * corrente (a ação "Reler agora" do chip de sincronização). Só leitura no
 * backend; o desfecho (sync/failed/offline + `changed`) chega pelo evento
 * `device://sync`, não pela resposta deste command.
 */
export async function deviceSyncNow(): Promise<void> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    // Retentável: reler o preset corrente é idempotente (toda leitura é).
    await runCommand("sync_now", IDEMPOTENTE, () => invoke("device_sync_now"));
    return;
  }
  await runCommand("sync_now", IDEMPOTENTE, async () => {
    debugFail("sync_now");
    // O mock local "relê" e responde sincronizado — mesma verdade do
    // MockDevice do backend após um read-back sem mutação.
    emitSync({ status: "sync", changed: false, detail: "" });
  });
}
