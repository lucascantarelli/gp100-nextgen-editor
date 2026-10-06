/**
 * M3-2 — o preset em arquivo: exportar e reimportar (issue #114).
 *
 * Este é o passo da DoD que pede a tela: **"e2e: exportar um preset de fábrica
 * e reimportar, com a cadeia preservada"**. O roteiro faz exatamente isso, e o
 * passo do MEIO é o que dá sentido à palavra "preservada": antes de reimportar,
 * o palco TROCA de preset — sem isso, o arquivo poderia estar vazio e a cadeia
 * continuaria "preservada" por não ter saído do lugar.
 *
 * **O roteiro roda no navegador**, fora do webview do Tauri, e por isso
 * exercita o fallback em memória da porta (`ipc/preset.ts`). É o mesmo caminho
 * do `pnpm dev`; contra o core Rust, a prova do round-trip byte-idêntico é a
 * suíte Rust (`packages/core/tests/preset_json_roundtrip.rs`), e a diferença
 * entre os dois está escrita no cabeçalho da porta — o front não tem o `.prst`.
 */
import { Buffer } from "node:buffer";
import { readFile, writeFile } from "node:fs/promises";
import { expect, test } from "@playwright/test";
import { ShellPage } from "./pages/_pages";

let shell: ShellPage;

test.beforeEach(async ({ page }) => {
  shell = new ShellPage(page);
  await shell.beforeStandard();
});

/** O pedal do slot `n` da família `fam` — o `aria-label` traz o NOME real. */
function pedal(n: number, fam: string) {
  return shell.board.slot(n, fam).locator('svg[role="group"]');
}

test("M3-2: exportar um preset de fábrica e reimportar preserva a cadeia", async ({
  page,
}, testInfo) => {
  // 1. o preset do boot (P01) abre num C-Wah no primeiro lugar da cadeia
  await expect(pedal(1, "PRE")).toHaveAttribute("aria-label", /C-Wah/);

  // 2. exporta o JSON pela porta do conteúdo (o ∿ do rodapé da biblioteca)
  const painel = await shell.openPresetFile();
  const [download] = await Promise.all([page.waitForEvent("download"), painel.exportJson().click()]);
  expect(download.suggestedFilename()).toBe("gp100.preset.P01.json");

  const json = await readFile((await download.path())!, "utf8");
  const env = JSON.parse(json) as {
    format: string;
    version: number;
    preset: { pp: number; slots: unknown[] };
  };
  // o arquivo é VERSIONADO: é o que autoriza um build futuro a recusá-lo
  expect(env.format).toBe("gp100.preset.chain");
  expect(env.version).toBe(1);
  expect(env.preset.pp).toBe(0);
  expect(env.preset.slots).toHaveLength(9);

  // 3. fecha a tela e TROCA de preset: a cadeia em cena deixa de ser a do P01
  await painel.close().click();
  await shell.nextPatch();
  await expect(pedal(1, "PRE")).toHaveAttribute("aria-label", /COMP/);

  // 4. reimporta o MESMO arquivo que saiu do app
  const arquivo = testInfo.outputPath("gp100.preset.P01.json");
  await writeFile(arquivo, json);
  const painel2 = await shell.openPresetFile();
  await painel2.fileInput().setInputFiles(arquivo);
  await expect(painel2.status()).toContainText("Cadeia importada: P01");
  await expect(painel2.status()).toContainText("9 slots no palco");

  // 5. a cadeia do P01 está de VOLTA no palco, e a navbar diz qual é
  await painel2.close().click();
  await expect(pedal(1, "PRE")).toHaveAttribute("aria-label", /C-Wah/);
  await expect(shell.patchLabel()).toHaveText(/^P01/);
});

test("M3-2: arquivo que não é um preset nosso é recusado com o motivo", async () => {
  const painel = await shell.openPresetFile();

  // A folha de timbre se anuncia indisponível fora do app (não há motor de PDF
  // no navegador) — o dono vê o porquê em vez de um botão que sempre falha.
  await expect(painel.toneSheet()).toBeDisabled();

  // o arquivo vai em memória: o caso aqui é o CONTEÚDO, não o caminho
  await painel.fileInput().setInputFiles({
    name: "gp100.library.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify({ format: "gp100.library", version: 1 })),
  });
  await expect(painel.status()).toContainText("Arquivo recusado");
  await expect(painel.status()).toContainText("gp100.library");

  // e o palco NÃO mudou: recusar um arquivo não pode deixar o palco em dúvida
  await painel.close().click();
  await expect(pedal(1, "PRE")).toHaveAttribute("aria-label", /C-Wah/);
});
