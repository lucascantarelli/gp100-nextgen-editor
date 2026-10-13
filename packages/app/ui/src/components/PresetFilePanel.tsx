/**
 * PresetFilePanel — o preset em arquivo (issue #114).
 *
 * **O que esta tela FAZ.** Leva o preset de FÁBRICA que está no palco para fora
 * em dois formatos que um humano lê — o JSON versionado (que volta a ser um
 * `.prst`) e a folha de timbre em PDF (para imprimir, anotar e voltar) — e
 * traz um JSON de volta para o palco. Os dois sentidos moram aqui de propósito:
 * é o par que a issue promete ("exportar e reimportar"), e uma tela só para cada
 * sentido faria o dono procurar a volta em outro lugar.
 *
 * **Por que ela mora atrás da porta do conteúdo (∿) e não na navbar.** A navbar
 * tem ~11px de folga em 1280 (qualquer botão novo a estoura, é o achado do
 * SnapTone na #25) e o rodapé da biblioteca já tem três botões — um quarto
 * estoura a linha, o rodapé vira duas fileiras, a coluna cresce por `stretch` e
 * o pedalboard desce (18 baselines visuais). Um destino a mais no modal da porta
 * não move um pixel da tela de repouso: é o mesmo caminho que a #25/#24 usaram.
 *
 * **Exporta o preset de FÁBRICA, e o Importar NÃO grava no aparelho.** O JSON
 * sai do `all.prst` embutido (é o arquivo que o palco já desenha) e a importação
 * devolve um `BoardView` para o palco — gravar no GP-100 continua sendo um ato
 * separado, com a trava do build de campo (ADR-4/ADR-5). A tela diz isso em
 * texto, porque um botão chamado "Importar" ao lado de um aparelho conectado
 * sugere o contrário.
 *
 * **A folha de timbre se anuncia quando não existe.** Fora do app (browser) não
 * há motor de PDF; em vez de um botão que sempre falha, ele fica DESABILITADO
 * com a explicação no `title` — o mesmo tratamento do `writeVerified` do build
 * de campo.
 */
import { useRef, useState } from "react";
import type { CSSProperties } from "react";
import { FACTORY_PRESETS } from "../artifacts/presetData";
import { MSG } from "../i18n/messages";
import type { BoardView } from "../ipc/types";
import {
  folhaDeTimbreDisponivel,
  presetExportJson,
  presetExportToneSheet,
  presetImportJson,
} from "../ipc/preset";

interface Props {
  /** preset de fábrica do palco — o que esta tela exporta */
  pp: number;
  /** aplica no palco a cadeia de um arquivo importado */
  onImport: (board: BoardView) => void;
  /** fecha a tela */
  onClose: () => void;
}

const overlay: CSSProperties = {
  position: "fixed",
  inset: 0,
  background: "rgba(0,0,0,.55)",
  display: "grid",
  placeItems: "center",
  zIndex: 50,
};
const sheet: CSSProperties = {
  background: "var(--bg)",
  color: "var(--text)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  borderRadius: 12,
  padding: "var(--space-16)",
  width: "min(520px, 94vw)",
  maxHeight: "86vh",
  overflowY: "auto",
  display: "grid",
  gap: "var(--space-12)",
};
const head: CSSProperties = { display: "flex", alignItems: "center", gap: "var(--space-8)" };
const h2t: CSSProperties = { margin: 0, fontSize: "var(--text-lg)" };
const p: CSSProperties = { margin: 0, fontSize: "var(--text-sm)", color: "var(--text-muted)" };
const note: CSSProperties = { ...p, fontSize: "var(--text-xs)" };
const btn: CSSProperties = {
  background: "transparent",
  border: "1px solid color-mix(in srgb, var(--accent) 45%, transparent)",
  color: "var(--accent-text)",
  borderRadius: 8,
  padding: "var(--space-8) var(--space-12)",
  minHeight: 32,
  cursor: "pointer",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  textAlign: "left",
};
const fecha: CSSProperties = {
  ...btn,
  borderColor: "transparent",
  color: "var(--text-muted)",
  marginLeft: "auto",
  minWidth: 32,
  textAlign: "center",
};
const banner: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "minmax(0, 1fr) auto",
  gap: "var(--space-4)",
  alignItems: "center",
  fontSize: "var(--text-xs)",
  color: "var(--text-muted)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  borderRadius: 8,
  padding: "var(--space-8)",
};
const errBanner: CSSProperties = {
  ...banner,
  color: "var(--danger, #f66)",
  borderColor: "color-mix(in srgb, var(--danger, #f66) 45%, transparent)",
};

/** Grava um arquivo de texto (download). O input é invisível por isso. */
function baixaTexto(nome: string, texto: string, tipo: string): void {
  const url = URL.createObjectURL(new Blob([texto], { type: tipo }));
  const a = document.createElement("a");
  a.href = url;
  a.download = nome;
  a.click();
  URL.revokeObjectURL(url);
}

/** Grava os BYTES do PDF (a folha de timbre vem do core como `Vec<u8>`). */
function baixaBytes(nome: string, bytes: Uint8Array): void {
  const url = URL.createObjectURL(new Blob([bytes as BlobPart], { type: "application/pdf" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = nome;
  a.click();
  URL.revokeObjectURL(url);
}

/** Erro com AÇÃO (#20): todo banner desta tela tem o que fazer depois dele. */
interface ErroTela {
  message: string;
  retry: () => void;
}

export function PresetFilePanel({ pp, onImport, onClose }: Props) {
  const [relato, setRelato] = useState<string | null>(null);
  const [erro, setErro] = useState<ErroTela | null>(null);
  const arquivoRef = useRef<HTMLInputElement>(null);

  const rotulo = MSG.libPp(pp);
  const nome = FACTORY_PRESETS.find((p) => p.pp === pp)?.name ?? "";
  const temFolha = folhaDeTimbreDisponivel();

  // Exportar é uma LEITURA do arquivo embutido: se falhar, repetir é a ação
  // certa (foi o `runCommand` que tentou de novo; a tela oferece a re-tentativa
  // manual, como o banner do #20).
  const exportaJson = (): void => {
    void presetExportJson(pp)
      .then((json) => baixaTexto(`gp100.preset.${rotulo}.json`, json, "application/json"))
      .catch(() => setErro({ message: MSG.presetFileFailed, retry: exportaJson }));
  };

  const exportaFolha = (): void => {
    void presetExportToneSheet(pp)
      .then((bytes) => baixaBytes(`gp100.preset.${rotulo}.pdf`, bytes))
      .catch(() => setErro({ message: MSG.presetFileFailed, retry: exportaFolha }));
  };

  // Importar é ler um arquivo do DONO: o que pode dar errado é o arquivo não
  // ser um preset nosso, e aí a ação é escolher outro — não repetir o mesmo.
  // Por isso a recusa vai no relato (com o motivo do core), não no banner de
  // erro: um botão "Tentar de novo" ao lado de um arquivo errado só repete o
  // erro.
  const importa = async (f: File | undefined): Promise<void> => {
    if (!f) return;
    setErro(null);
    try {
      const board = await presetImportJson(await f.text());
      onImport(board);
      setRelato(MSG.presetFileImported(MSG.libPp(board.pp), board.name, board.slots.length));
    } catch (e) {
      setRelato(MSG.presetFileRejected(e instanceof Error ? e.message : String(e)));
    }
  };

  return (
    <div style={overlay} onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div role="dialog" aria-modal="true" aria-label={MSG.presetFileTitle} style={sheet}>
        <div style={head}>
          <h2 style={h2t}>{MSG.presetFileTitle}</h2>
          <button style={fecha} onClick={onClose} aria-label={MSG.presetFileClose}>
            ✕
          </button>
        </div>

        <p style={p}>{MSG.presetFileHint}</p>
        <p style={note}>{MSG.presetFileTarget(rotulo, nome)}</p>

        <button style={btn} onClick={exportaJson} aria-label={MSG.presetFileExportAria}>
          {MSG.presetFileExport}
        </button>

        {/* desabilitada (e explicada) fora do app: é lá que vive o motor de PDF */}
        <button
          style={btn}
          onClick={exportaFolha}
          disabled={!temFolha}
          title={temFolha ? MSG.presetFileSheetAria : MSG.presetFileSheetHint}
          aria-label={MSG.presetFileSheetAria}
        >
          {MSG.presetFileSheet}
          {temFolha ? "" : ` — ${MSG.presetFileSheetOff}`}
        </button>

        <button style={btn} onClick={() => arquivoRef.current?.click()} aria-label={MSG.presetFileImportAria}>
          {MSG.presetFileImport}
        </button>

        {/* o input fica fora da tela: o botão é o alvo visível e o input
            continua alcançável por teclado/leitor de tela */}
        <input
          ref={arquivoRef}
          type="file"
          accept="application/json,.json"
          style={{ display: "none" }}
          aria-label={MSG.presetFileImportAria}
          onChange={(e) => void importa(e.target.files?.[0])}
        />

        {erro != null && (
          <div role="alert" style={errBanner}>
            <span>{erro.message}</span>
            <button style={btn} onClick={erro.retry}>
              {MSG.errRetry}
            </button>
          </div>
        )}
        {relato != null && (
          <div role="status" style={banner}>
            <span>{relato}</span>
            <button style={fecha} onClick={() => setRelato(null)} aria-label={MSG.presetFileClose}>
              ✕
            </button>
          </div>
        )}

        <p style={note}>{MSG.presetFileNote}</p>
      </div>
    </div>
  );
}
