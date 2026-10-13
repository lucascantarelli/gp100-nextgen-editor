/**
 * SnapTonePanel — o gestor de tons SnapTone/NAM (issue #25).
 *
 * **O que esta tela FAZ e o que ela NÃO faz.** Importa o `.clo` que o Valeton
 * Suite gera, nomeia, escolhe um dos 5 slots do aparelho, manda para o
 * aparelho e compara dois tons em A/B tocando o áudio que o Suite renderiza.
 * Ela NÃO converte `.nam` (o motor NAM mora no exe da Valeton — o texto de
 * introduction diz isso ao dono, em vez de deixar ele procurar o botão certo).
 *
 * **O aviso do CAB é parte da tela, não um detalhe.** As notas de release do
 * aparelho (V2.1) são explícitas: *"If the SnapTone function is enabled, the
 * CAB module will be disabled."* O dono que usa o CAB perde o som sem aviso se
 * o app mandar o tom calado. Por isso o texto fica VISÍVEL na tela, e o botão
 * de enviar pede confirmação com o mesmo texto — um aviso que só aparece no
 * log é um aviso que ninguém lê.
 *
 * **O A/B é por SLOT, não por tom.** A pergunta do dono é "o que o aparelho
 * tem no slot 3 contra o do slot 4", e é assim que o A/B do próprio aparelho
 * funciona: um lado ligado, o outro desligado. Guardar o id do tom aqui faria a
 * tela desatualizar sozinha depois de um reimport, que troca o id sem mudar o
 * slot.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { MSG } from "../i18n/messages";
import type { Tones } from "../hooks/useTones";
import type { Tone } from "../ipc/tones";
import { toneSetPreview } from "../ipc/tones";

const box: CSSProperties = {
  position: "fixed",
  inset: 0,
  background: "color-mix(in srgb, #000 60%, transparent)",
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
  width: "min(760px, 92vw)",
  maxHeight: "88vh",
  overflowY: "auto",
  display: "grid",
  gap: "var(--space-12)",
};
const h2t: CSSProperties = { margin: 0, fontSize: "var(--text-lg)" };
const p: CSSProperties = { margin: 0, fontSize: "var(--text-sm)", color: "var(--text-muted)" };
const btn: CSSProperties = {
  background: "transparent",
  border: "1px solid color-mix(in srgb, var(--accent) 45%, transparent)",
  color: "var(--accent-text)",
  borderRadius: 8,
  padding: "6px 10px",
  minHeight: 32,
  cursor: "pointer",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  whiteSpace: "nowrap",
};
const btnOff: CSSProperties = { ...btn, borderColor: "transparent", color: "var(--text-muted)" };
const input: CSSProperties = {
  background: "var(--bg)",
  border: "1px solid color-mix(in srgb, var(--text-muted) 30%, transparent)",
  color: "var(--text)",
  borderRadius: 8,
  padding: "7px 10px",
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  minHeight: 32,
  minWidth: 0,
};
const linha: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "minmax(0, 1fr) auto",
  gap: "var(--space-8)",
  alignItems: "center",
  borderTop: "1px solid color-mix(in srgb, var(--text-muted) 18%, transparent)",
  paddingTop: "var(--space-8)",
};
const nomeCell: CSSProperties = {
  fontSize: "var(--text-sm)",
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
};
/* O aviso do CAB é a única coisa AMARELA da tela — é para chamar o olho sem
   parecer erro (não é erro: é uma consequência que o dono escolhe). */
const aviso: CSSProperties = {
  ...p,
  color: "var(--accent-text)",
  border: "1px solid color-mix(in srgb, var(--accent) 45%, transparent)",
  borderRadius: 8,
  padding: "var(--space-8)",
  display: "grid",
  gap: "var(--space-4)",
};
const erro: CSSProperties = {
  ...p,
  color: "var(--danger)",
  border: "1px solid color-mix(in srgb, var(--danger) 45%, transparent)",
  borderRadius: 8,
  padding: "var(--space-8)",
};
const relatorio: CSSProperties = { ...p, color: "var(--text)" };

/** KB arredondados — o `.clo` tem ~2,7 KB e o dono não quer contar bytes. */
function kb(bytes: number): number {
  return Math.round(bytes / 1024);
}

/** Lê um arquivo do disco como array de bytes (o que o serde espera). */
async function bytesDoArquivo(f: File): Promise<number[]> {
  return Array.from(new Uint8Array(await f.arrayBuffer()));
}

interface Props {
  /** O estado do gestor (o hook é o dono). */
  tones: Tones;
  /**
   * Escrita liberada nesta build? (ADR-5; `false` = build de leitura, face
   * (A) da #126). Travado, o envio fica desabilitado COM O MOTIVO na tela e
   * no `title` — um SnapTone são ~143 blocos e o fio recusaria o primeiro
   * como erro depois do clique.
   */
  podeGravar?: boolean;
}

export function SnapTonePanel({ tones, podeGravar = true }: Props) {
  const [nome, setNome] = useState("");
  const [renomeando, setRenomeando] = useState<string | null>(null);
  const [confirmando, setConfirmando] = useState<string | null>(null);
  const [preview, setPreview] = useState<{ id: string; audio: HTMLAudioElement | null } | null>(null);
  const cloRef = useRef<HTMLInputElement>(null);
  const wavRef = useRef<HTMLInputElement>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);

  // O `<audio>` nasce UMA vez e muda de fonte; recriá-lo a cada play
  // reinicia a posição e dispara um pico de áudio no meio da comparação.
  useEffect(() => {
    const el = new Audio();
    audioRef.current = el;
    return () => {
      el.pause();
      el.src = "";
    };
  }, []);

  const board = tones.board;
  const slots = board?.slots ?? 5;
  const tons = board?.tones ?? [];

  /**
   * Importa o `.clo` escolhido e, se o dono tinha marcado um WAV, ele junto.
   *
   * O nome nasce do nome do ARQUIVO sem a extensão: pedir o nome de novo para
   * cada importação é um campo vazio a cada vez, e `bass_cab.clo` já é um nome
   * utilizável — o dono renomeia depois se quiser.
   */
  const escolheClo = useCallback(
    async (f: File | undefined) => {
      if (!f) return;
      const modelo = await bytesDoArquivo(f);
      const linha = await tones.importa(nome.trim() || f.name.replace(/\.clo$/i, ""), modelo);
      if (linha != null) {
        setNome("");
        // o áudio escolhido ANTES do `.clo` fica pendente: importar agora é
        // o que o dono quer fazer, e o WAV é um segundo passo
        if (wavPendente.current != null) {
          const wav = wavPendente.current;
          wavPendente.current = null;
          await toneSetPreview(linha.id, wav).catch(() => undefined);
          void tones.atribui(linha.id, null).catch(() => undefined);
        }
      }
    },
    [nome, tones],
  );

  /** O WAV marcado antes do `.clo` (o botão de áudio vem primeiro). */
  const wavPendente = useRef<number[] | null>(null);

  const escolheWav = useCallback(async (f: File | undefined) => {
    if (!f) return;
    wavPendente.current = await bytesDoArquivo(f);
    if (wavRef.current) wavRef.current.value = "";
  }, []);

  /** Envia ao aparelho — com a confirmação do CAB na frente. */
  const envia = useCallback(
    async (id: string) => {
      if (confirmando === id) {
        setConfirmando(null);
        await tones.envia(id);
        return;
      }
      setConfirmando(id);
    },
    [confirmando, tones],
  );

  /** Toca (ou para) o áudio do A/B de um slot. */
  const tocaLado = useCallback(
    async (lado: "a" | "b") => {
      const slot = lado === "a" ? tones.ladoA : tones.ladoB;
      if (slot == null) return;
      const el = audioRef.current;
      if (!el) return;
      if (tones.tocando === slot) {
        el.pause();
        tones.setTocando(null);
        setPreview(null);
        return;
      }
      const url = await tones.toca(slot);
      if (url == null) return;
      // revoga a URL anterior: cada play cria uma, e sem isso o WebKit segura
      // o WAV inteiro até a janela fechar
      if (preview != null && preview.audio == null) URL.revokeObjectURL(url);
      el.src = url;
      await el.play().catch(() => undefined);
      tones.setTocando(slot);
      setPreview({ id: `${lado}:${slot}`, audio: el });
    },
    [preview, tones],
  );

  if (!tones.aberto) return null;

  const nomeDoSlot = (slot: number): Tone | undefined =>
    tons.find((t) => t.slot === slot);

  return (
    <div style={box} onClick={(e) => { if (e.target === e.currentTarget) tones.fechar(); }}>
      <div role="dialog" aria-modal="true" aria-label={MSG.toneTitle} style={sheet}>
        <h2 style={h2t}>{MSG.toneTitle}</h2>
        <p style={p}>{MSG.toneIntro}</p>
        {/* o build de leitura diz na tela que não envia: um botão travado sem
            motivo obriga o dono a descobrir sozinho o que esta instalação é */}
        {!podeGravar && (
          <p style={{ ...p, color: "var(--accent-text)" }} role="status">{MSG.writeLockedHint}</p>
        )}

        {/* o aviso do CAB: visível ANTES de qualquer botão de enviar */}
        <div role="note" style={aviso}>
          <strong>{MSG.toneCabTitle}</strong>
          <span>{MSG.toneCabWarning}</span>
        </div>

        {tones.err != null && (
          <div role="alert" style={erro}>
            <span>{tones.err.message}</span>
            <button style={btnOff} onClick={tones.err.retry}>
              {MSG.errRetry}
            </button>
          </div>
        )}

        {/* importar: o nome é opcional — sem ele nasce do nome do arquivo */}
        <div style={{ display: "grid", gridTemplateColumns: "minmax(0, 1fr) auto auto", gap: "var(--space-8)", alignItems: "center" }}>
          <input
            style={input}
            value={nome}
            onChange={(e) => setNome(e.target.value)}
            placeholder={MSG.toneNamePlaceholder}
            aria-label={MSG.toneNameAria}
          />
          <button style={btnOff} aria-label={MSG.toneImportWavAria} title={MSG.toneImportWavAria} onClick={() => wavRef.current?.click()}>
            {MSG.toneImportWav}
          </button>
          <button style={btn} aria-label={MSG.toneImportCloAria} title={MSG.toneImportCloAria} onClick={() => cloRef.current?.click()}>
            {MSG.toneImportClo}
          </button>
        </div>
        <input
          ref={cloRef}
          type="file"
          accept=".clo"
          style={{ display: "none" }}
          aria-label={MSG.toneImportCloAria}
          onChange={(e) => void escolheClo(e.target.files?.[0])}
        />
        <input
          ref={wavRef}
          type="file"
          accept=".wav,audio/wav"
          style={{ display: "none" }}
          aria-label={MSG.toneImportWavAria}
          onChange={(e) => void escolheWav(e.target.files?.[0])}
        />

        {/* a lista */}
        <div role="list" aria-label={MSG.toneListAria} style={{ display: "grid", gap: "var(--space-8)" }}>
          {tons.length === 0 && <p style={p}>{MSG.toneEmpty}</p>}
          {tons.map((t) => (
            <div key={t.id} role="listitem" style={linha}>
              <div style={{ display: "grid", gap: 2, minWidth: 0 }}>
                {renomeando === t.id ? (
                  <input
                    style={input}
                    autoFocus
                    defaultValue={t.name}
                    aria-label={MSG.toneRenameAria}
                    onBlur={(e) => {
                      void tones.renomeia(t.id, e.target.value);
                      setRenomeando(null);
                    }}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                    }}
                  />
                ) : (
                  <span style={{ ...nomeCell, fontWeight: 700 }}>{t.name}</span>
                )}
                <span style={p}>{MSG.toneRowTitle(t.name, kb(t.bytes))}</span>
              </div>

              <div style={{ display: "flex", gap: "var(--space-4)", alignItems: "center", flexWrap: "wrap", justifyContent: "flex-end" }}>
                <select
                  style={input}
                  value={t.slot ?? ""}
                  aria-label={MSG.toneSlotAria(t.slot ?? 0)}
                  onChange={(e) => void tones.atribui(t.id, e.target.value === "" ? null : Number(e.target.value))}
                >
                  <option value="">{MSG.toneSlotNone}</option>
                  {Array.from({ length: slots }, (_, i) => i + 1).map((n) => (
                    <option key={n} value={n} disabled={nomeDoSlot(n) != null && nomeDoSlot(n)?.id !== t.id}>
                      {MSG.toneSlotLabel(n)}
                    </option>
                  ))}
                </select>
                <button style={btnOff} aria-label={MSG.toneRenameAria} title={MSG.toneRenameAria} onClick={() => setRenomeando(t.id)}>
                  ✎
                </button>
                <button
                  style={podeGravar ? btn : btnOff}
                  disabled={!podeGravar || t.slot == null || tones.enviando != null}
                  aria-label={MSG.toneSendAria(t.name)}
                  title={podeGravar ? MSG.toneSendAria(t.name) : MSG.writeLockedHint}
                  onClick={() => void envia(t.id)}
                >
                  {tones.enviando === t.id ? MSG.toneSending : MSG.toneSend}
                </button>
                <button style={btnOff} aria-label={MSG.toneDeleteAria(t.name)} title={MSG.toneDeleteAria(t.name)} onClick={() => void tones.apaga(t.id)}>
                  ✕
                </button>
              </div>
            </div>
          ))}
        </div>

        {/* a confirmação do CAB fica NO LUGAR do clique: o dono leu o aviso e
            agora decide. O texto é o mesmo do aviso do topo — duas palavras
            novas numa confirmação fariam o dono desconfiar do que ela diz. */}
        {confirmando != null && (
          <div role="alertdialog" aria-label={MSG.toneCabTitle} style={aviso}>
            <strong>{MSG.toneCabTitle}</strong>
            <span>{MSG.toneCabWarning}</span>
            <div style={{ display: "flex", gap: "var(--space-8)" }}>
              <button
                style={podeGravar ? btn : btnOff}
                disabled={!podeGravar}
                title={podeGravar ? undefined : MSG.writeLockedHint}
                onClick={() => void envia(confirmando)}
              >
                {MSG.toneSend}
              </button>
              <button style={btnOff} onClick={() => setConfirmando(null)}>
                {MSG.toneClearSlot}
              </button>
            </div>
          </div>
        )}

        {tones.relatorio != null && (
          <p role="status" style={relatorio}>
            {MSG.toneSent(tones.relatorio.blocks, tones.relatorio.bytes)}
          </p>
        )}

        {/* ── A/B ── */}
        <div style={{ display: "grid", gap: "var(--space-8)" }}>
          <h3 style={{ ...h2t, fontSize: "var(--text-md)" }}>{MSG.toneAbTitle}</h3>
          <p style={p}>{MSG.toneAbHint}</p>
          {(["a", "b"] as const).map((lado) => {
            const slot = lado === "a" ? tones.ladoA : tones.ladoB;
            const tom = slot == null ? undefined : nomeDoSlot(slot);
            const tocando = slot != null && tones.tocando === slot;
            return (
              <div key={lado} style={{ ...linha, borderTop: "1px solid color-mix(in srgb, var(--text-muted) 18%, transparent)" }}>
                <div style={{ display: "flex", gap: "var(--space-8)", alignItems: "center", minWidth: 0 }}>
                  <strong style={{ fontFamily: "var(--font-mono)" }}>{lado === "a" ? MSG.toneAbA : MSG.toneAbB}</strong>
                  <select
                    style={input}
                    value={slot ?? ""}
                    aria-label={MSG.toneAbEmpty}
                    onChange={(e) => tones.chooseLado(lado, e.target.value === "" ? null : Number(e.target.value))}
                  >
                    <option value="">{MSG.toneAbEmpty}</option>
                    {Array.from({ length: slots }, (_, i) => i + 1).map((n) => (
                      <option key={n} value={n}>
                        {`${MSG.toneSlotLabel(n)} — ${nomeDoSlot(n)?.name ?? MSG.toneSlotNone}`}
                      </option>
                    ))}
                  </select>
                </div>
                <button
                  style={tom == null ? btnOff : btn}
                  disabled={tom == null}
                  aria-label={tom == null ? MSG.toneNoPreview : MSG.tonePlayAria(lado === "a" ? MSG.toneAbA : MSG.toneAbB, tom.name)}
                  title={tom == null ? MSG.toneNoPreview : MSG.tonePlayAria(lado === "a" ? MSG.toneAbA : MSG.toneAbB, tom.name)}
                  onClick={() => void tocaLado(lado)}
                >
                  {tom == null ? MSG.toneNoPreview : tocando ? MSG.toneSending : MSG.tonePlay}
                </button>
              </div>
            );
          })}
        </div>

        <footer style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "var(--space-8)" }}>
          <p role="status" style={p}>
            {board != null ? MSG.toneStats(board.tones.length, board.slots, board.usados) : ""}
          </p>
          <button style={btnOff} onClick={tones.fechar}>
            {MSG.libSearchClear}
          </button>
        </footer>
      </div>
    </div>
  );
}