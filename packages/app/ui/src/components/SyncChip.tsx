/**
 * SyncChip — o estado de sincronização com o APARELHO na navbar (#164).
 *
 * O chip NÃO mente: os rótulos são os estados que o BACKEND afirma
 * (idle/sync/pending/failed/offline), sem contadores inventados — o "tempo
 * real" deste protocolo é poll + read-back (o knob físico não emite SysEx;
 * `docs/tecnico/PROTOCOL.md` §sync), e a interface diz isso em vez de esconder.
 *
 * Erro tem AÇÃO (contrato do #20, estendido ao sync): `failed` oferece
 * "Reler" (`device_sync_now` — read-back imediato do preset corrente) e
 * `offline` oferece "Reconectar" (`device_conectar` — re-enumera as
 * portas). Os estados sem erro mostram só o rótulo. O detalhe humano de
 * failed/offline vai no `title` do chip — o operador lê o motivo sem abrir
 * o painel de diagnóstico.
 *
 * O chip se conecta SOZINHO (mesmo padrão do `BootBanners`, que chama
 * `deviceConectar`): o estado e as ações vivem aqui, o App só o monta —
 * passar 3 props pelo App estouraria o teto de 300 linhas
 * (`check_module_size`, docs/dev/ARCHITECTURE.md §4) para um detalhe de navbar.
 *
 * MSG é lido NO RENDER (o Proxy de idioma troca o catálogo inteiro): um
 * mapa de rótulos congelado em module scope congelaria o pt-BR.
 */
import type { CSSProperties } from "react";
import type { SyncStatus } from "../ipc/types";
import { deviceConectar } from "../ipc/device";
import { deviceSyncNow } from "../ipc/sync";
import { useSyncState } from "../hooks/useSyncState";
import { MSG } from "../i18n/messages";

/* mesma caixa dos outros botões da navbar (32px, border-box); `flexShrink: 0`
   para a ação nunca ser espremida — quem cede espaço é o rótulo */
const nbBtn: CSSProperties = { height: 32, boxSizing: "border-box", flexShrink: 0 };
const label: CSSProperties = {
  fontSize: "var(--text-xs)",
  color: "var(--text-muted)",
  fontFamily: "var(--font-mono)",
  letterSpacing: 0.8,
  textTransform: "uppercase",
  /* rótulo corta com ellipsis quando a célula do rack aperta, em vez de
     empurrar o ⟳ para fora do cluster (R5 no CI — fontes mais largas) */
  overflow: "hidden",
  textOverflow: "ellipsis",
  minWidth: 0,
};
const divider: CSSProperties = { width: 1, height: 20, background: "var(--border)", flex: "0 0 auto" };

/** Cor do rótulo: ok = sincronizado; `--error` = erro de verdade; o resto é neutro. */
function corDoEstado(status: SyncStatus): string {
  if (status === "sync") return "var(--ok)";
  if (status === "failed" || status === "offline") return "var(--error)";
  return "var(--text-muted)";
}

export function SyncChip() {
  const sync = useSyncState();
  const rotulo: Record<SyncStatus, string> = {
    idle: MSG.syncIdle,
    sync: MSG.syncSync,
    pending: MSG.syncPending,
    failed: MSG.syncFailed,
    offline: MSG.syncOffline,
  };
  return (
    <span
      role="status"
      aria-label={`${MSG.syncClusterAria}: ${rotulo[sync.status]}`}
      title={sync.detail || undefined}
      style={{ display: "flex", alignItems: "center", gap: "var(--space-8)", minWidth: 0, overflow: "hidden" }}
    >
      <span className={sync.status === "sync" ? "live-dot" : "idle-dot"} aria-hidden="true" />
      <span style={{ ...label, color: corDoEstado(sync.status) }}>{rotulo[sync.status]}</span>
      {sync.status === "failed" && (
        <>
          <span style={divider} aria-hidden="true" />
          <button
            className="gp-btn"
            style={nbBtn}
            onClick={() => void deviceSyncNow()}
            aria-label={MSG.syncRelerAria}
            title={MSG.syncRelerTitle}
          >
            {MSG.syncReler}
          </button>
        </>
      )}
      {sync.status === "offline" && (
        <>
          <span style={divider} aria-hidden="true" />
          <button
            className="gp-btn"
            style={nbBtn}
            onClick={() => void deviceConectar()}
            aria-label={MSG.syncReconnectAria}
            title={MSG.syncReconnectTitle}
          >
            {MSG.syncReconnect}
          </button>
        </>
      )}
    </span>
  );
}
