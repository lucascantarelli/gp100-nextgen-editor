/**
 * ipc/diag — a porta do DIAGNÓSTICO DE CAMPO: as operações que respondem "o que
 * está no aparelho", "grava o que você fez" e "isto é o que sairia pelo fio".
 *
 * **POR QUE ESTA PORTA É SEPARADA DA DELE (`ipc/device.ts`).** A de device é a
 * porta do dia a dia da edição — info, boot, board, biblioteca, select, knob. A
 * de diagnóstico é outra COORTE, com outro contrato: devolve hexadecimal em vez
 * de estado, uma delas NÃO é idempotente (gravar duas vezes é pior que falhar),
 * e uma delas é uma SESSÃO de arquivo, não uma transação. Misturar as duas
 * fez a porta do dia a dia passar do teto de linhas sem nenhuma decisão nova —
 * e a revisão do corte de um teto é sempre decisão consciente.
 *
 * **O que NÃO foi duplicado:** a execução (`runCommand`) e a política
 * (`IDEMPOTENTE`/`semRetry`) vêm da porta de device — são uma coisa só, com um
 * prazo só. O que é próprio desta porta é o *conteúdo* de cada operação: a
 * forma do fallback e o que se faz com o hex que volta.
 *
 * **O hex vem do BACKEND, nunca é montado aqui.** É o que garante que a prévia e
 * o envio real não possam divergir — inclusive na trava de valor por faixa, que
 * vale igual para os dois. Um dry-run que aceitasse um valor perigoso e dissesse
 * "tudo certo" seria o pior dos dois mundos.
 */
import type { DumpReport, PreviewFrame, PreviewOp } from "./types";
import { IDEMPOTENTE, debugFail, inTauri, runCommand, semRetry } from "./device";

/**
 * `device_save_preset` — **salva o preset no aparelho** (§13.12).
 *
 * É o que PERSISTE. O `set-param` é fire-and-forget (§13.11, D4) — sem isto a
 * mexida morre com a sessão, e até agora o usuário do app conseguia mexer e
 * perder.
 *
 * **Sem retry.** Reenviar um `save` grava de novo: não é idempotente, e um
 * backend lento transformaria um clique em dois. A recuperação é o botão ↻ do
 * usuário, que ele sabe que só grava uma vez.
 */
export async function deviceSavePreset(
  pp: number,
  ppType: number,
  name: string,
): Promise<void> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    await runCommand(
      "save",
      semRetry(
        "save grava de novo no aparelho: repetir não é idempotente e um " +
          "backend lento viraria dois cliques em dois saves.",
      ),
      () => invoke("device_save_preset", { pp, ppType, name }),
    );
    return;
  }
  await runCommand("save", semRetry("fallback local"), async () => {
    debugFail("save");
  });
}

/**
 * `device_dump_preset` — **lê o preset do aparelho** (§13.9): meta6 + as 8
 * páginas de estado, em hex cru.
 *
 * É a leitura de campo: o mesmo dump que o script de campo usou para provar a
 * escrita (o valor do knob mudou na página 0, com o CRC recalculado). É
 * retentável — ler de novo não muda nada no aparelho.
 */
export async function deviceDumpPreset(pp: number): Promise<DumpReport> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return runCommand("dump", IDEMPOTENTE, () =>
      invoke<DumpReport>("device_dump_preset", { pp }),
    );
  }
  // Fallback dev/teste: as MESMAS 9 transações do §13.9, com payload sintético
  // — a FORMA é o que o contrato pede, e um shape diferente aqui ensinaria o
  // componente a lidar com duas formas.
  return runCommand("dump", IDEMPOTENTE, async () => {
    debugFail("dump");
    return {
      pp,
      meta6: "000C1C0100",
      pages: Array.from({ length: 8 }, () => "00".repeat(196)),
    };
  });
}

/**
 * `device_log_session` — **liga o log de fio da sessão** (schema P4).
 *
 * É o MESMO formato que o utilitário de terminal grava e que os comparadores de
 * campo leem. Fecha o ciclo pelo app: a sessão acontece no editor, o arquivo
 * vai para quem analisa — sem depender do binário de terminal.
 */
export async function deviceLogSession(path: string): Promise<boolean> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return runCommand("log_start", IDEMPOTENTE, () =>
      invoke<boolean>("device_log_session", { path }),
    );
  }
  return runCommand("log_start", IDEMPOTENTE, async () => {
    debugFail("log_start");
    return true;
  });
}

/** `device_log_stop` — desliga o log de fio (a sessão segue; só o log para). */
export async function deviceLogStop(): Promise<boolean> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return runCommand("log_stop", IDEMPOTENTE, () => invoke<boolean>("device_log_stop"));
  }
  return runCommand("log_stop", IDEMPOTENTE, async () => true);
}

/**
 * `device_log_path` — **em que arquivo o log de fio está gravando** (ou `null`).
 *
 * O caminho **não é do front**: no build de campo o backend liga o log sozinho
 * na abertura, com o nome que ele escolhe (diretório de dados + carimbo do
 * horário). A tela pergunta em vez de assumir — é o que deixa o operador ver o
 * arquivo sem abrir o terminal, e o que impede o botão de oferecer "gravar" por
 * cima de uma sessão que já está em disco.
 *
 * No fallback (browser) não existe backend gravando: a resposta honesta é
 * `null`, e não um caminho inventado.
 */
export async function deviceLogPath(): Promise<string | null> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return runCommand("log_path", IDEMPOTENTE, () =>
      invoke<string | null>("device_log_path"),
    );
  }
  return runCommand("log_path", IDEMPOTENTE, async () => null);
}

/**
 * `device_log_reveal` — **abre o gerenciador de arquivos com o log selecionado**.
 *
 * É o fim do fluxo de entrega da sessão de campo (#135): o painel já diz EM
 * QUE arquivo a gravação está; este command leva o arquivo até o suporte, sem
 * o operador caçar a pasta na mão.
 *
 * O caminho **não é argumento**: quem decide é o backend, que é o dono do log
 * ativo — o front não manda o explorer para lugar nenhum.
 *
 * No fallback (browser) não existe desktop para abrir, e a operação é **dita**,
 * não fingida: lança, o painel mostra a mensagem, e ninguém acredita que a
 * pasta abriu.
 */
export async function deviceLogReveal(): Promise<void> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    await runCommand("log_reveal", IDEMPOTENTE, () =>
      invoke<void>("device_log_reveal"),
    );
    return;
  }
  return runCommand("log_reveal", IDEMPOTENTE, async () => {
    throw new Error("abrir a pasta do log existe só no aplicativo desktop");
  });
}

/**
 * `device_preview` — **o que o aparelho receberia, sem receber**.
 *
 * O hex vem do BACKEND, nunca montado aqui: a porta única proíbe o front de
 * montar SysEx, e é por isso que a prévia e o envio não podem divergir.
 */
export async function devicePreview(op: PreviewOp): Promise<PreviewFrame[]> {
  if (inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return runCommand("preview", IDEMPOTENTE, () =>
      invoke<PreviewFrame[]>("device_preview", { op }),
    );
  }
  return runCommand("preview", IDEMPOTENTE, async () => {
    debugFail("preview");
    return [{ label: "12/10000002", hex: "F021257F47502D64" }];
  });
}