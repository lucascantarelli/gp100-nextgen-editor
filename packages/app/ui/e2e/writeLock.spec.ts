/**
 * Build de LEITURA — a face (A) da #126 (decisão do owner, 06/10) na tela.
 *
 * O que este spec prova é o que a DoD da issue pede: **num build sem
 * `write-verified`, todo botão que grava no aparelho nasce desabilitado COM O
 * MOTIVO**, em vez de deixar o dono descobrir a recusa depois do clique. O
 * knob do modal, o envio de IR e o envio de SnapTone são os três canais de
 * escrita que a UI tem; o quarto (`save_preset`) já era travado pelo
 * `FieldDiagPanel` desde a #115.
 *
 * O quinto — trocar de preset — NÃO é travado de propósito: `select_preset`
 * sai como `WireKind::Read` no fio (é o mesmo select com que o boot varre os
 * 199 presets para LER), então o build de leitura segue navegando. Está
 * medido no core (`tests/write_gate.rs`) e justificado no PR da #126.
 *
 * **Como o teste monta o build de leitura.** O roteiro roda no navegador, sem
 * a feature compilada; o gancho `gp100.debug.writeVerified` (mesmo desenho do
 * `failDevice`) faz o fallback local relatar `writeVerified: false`. O que
 * ele prova é a POLÍTICA DA TELA — a trava do fio é do `real.rs` e segue
 * sendo provada pelo `write_gate.rs` + pelo build de campo da CI.
 */
import { expect, test } from "@playwright/test";
import { ShellPage } from "./pages/_pages";

/** Substring do `MSG.writeLockedHint` — o motivo que a tela tem que mostrar. */
const MOTIVO = "gravar no aparelho está bloqueado nesta build";
/** O mesmo texto como `RegExp`: o `title` do botão é a frase INTEIRA. */
const MOTIVO_RE = /gravar no aparelho está bloqueado nesta build/;
/** Sinal de que o `device_info` chegou com escrita liberada (ADR-5/mock). */
const LIBERADO = "Gravação liberada nesta instalação.";

let shell: ShellPage;

test.beforeEach(async ({ page }) => {
  shell = new ShellPage(page);
});

/** 4 chunks de 15B — o formato que o fio aceita (§13.7). */
const IR_OK = Buffer.from(Array.from({ length: 60 }, (_, i) => (i * 7 + 3) & 0xff));
const CLO = Buffer.from(Array.from({ length: 40 }, (_, i) => (i * 7 + 3) & 0xff));

/** Abre o modal do PRE (P01 = C-Wah, 3 knobs reais) e devolve o diálogo. */
async function abreModal(page: import("@playwright/test").Page) {
  const pedal = shell.board.slot(1, "PRE").locator('svg[role="group"]');
  // a trava de mover no repouso: sem isso o clique pertence ao drag (R7)
  await shell.board.expectMoverPressed(false);
  await pedal.locator('svg[role="img"]').first().click();
  const dlg = page.getByRole("dialog", { name: /Edição do pedal/ });
  await expect(dlg).toBeVisible();
  return dlg;
}

test("knob: o modal abre travado, com o motivo na tela e sem slider", async ({ page }) => {
  await shell.readOnlyBuild(); // ANTES do load: o mount lê o storage
  await shell.beforeStandard();

  const dlg = await abreModal(page);

  // o motivo é VISÍVEL no cabeçalho — não só num tooltip de controle morto
  await expect(dlg.getByText(MOTIVO).first()).toBeVisible();
  // nenhum controle interativo: os knobs viraram display (role img)
  await expect(dlg.locator('svg[role="slider"]')).toHaveCount(0);
  await expect(dlg.locator('svg[role="button"]')).toHaveCount(0);
  await expect(dlg.locator('svg[role="img"]')).toHaveCount(3);
  // a caixa de valor do modal existe e não entra em modo edição
  await expect(dlg.getByLabel("Valor (Enter para editar)").first()).toBeDisabled();

  // e a LEITURA continua: o valor do knob segue na tela (o fio não é travado)
  await expect(dlg.getByLabel("Valor (Enter para editar)").first()).not.toHaveValue("");

  // Esc fecha — a trava não é uma armadilha
  await page.keyboard.press("Escape");
  await expect(dlg).toHaveCount(0);
});

test("IR: importar e escolher slot deixam o envio desabilitado COM O MOTIVO", async () => {
  await shell.readOnlyBuild();
  await shell.beforeStandard();
  const dlg = await shell.openIrLab();
  await expect(dlg.getByText(MOTIVO).first()).toBeVisible();

  await dlg.locator('input[type="file"]').setInputFiles({
    name: "vintage_4x12.ir",
    mimeType: "application/octet-stream",
    buffer: IR_OK,
  });
  await expect(dlg.getByText("vintage_4x12").first()).toBeVisible();

  // slot escolhido: sem a política o botão ESTARIA habilitado (é o caso do
  // teste de controle) — o `disabled` daqui é da política, não da falta de slot
  await dlg.locator("select").selectOption("0");

  const enviar = dlg.getByRole("button", { name: /^Enviar .* para o slot do aparelho/ });
  await expect(enviar).toBeDisabled();
  await expect(enviar).toHaveAttribute("title", MOTIVO_RE);
});

test("SnapTone: importar e escolher slot deixam o envio desabilitado COM O MOTIVO", async () => {
  await shell.readOnlyBuild();
  await shell.beforeStandard();
  const dlg = await shell.openSnapTone();
  await expect(dlg.getByText(MOTIVO).first()).toBeVisible();

  await dlg.locator('input[type="file"][accept=".clo"]').setInputFiles({
    name: "modelo.clo",
    mimeType: "application/octet-stream",
    buffer: CLO,
  });
  await expect(dlg.getByText("modelo", { exact: true }).first()).toBeVisible();

  // a primeira caixa de seleção é da LINHA do tom (o A/B vem depois)
  await dlg.locator("select").first().selectOption("1");

  const enviar = dlg.getByRole("button", { name: /^Enviar .* para o slot do aparelho/ });
  await expect(enviar).toBeDisabled();
  await expect(enviar).toHaveAttribute("title", MOTIVO_RE);
});

test("sem o gancho os controles seguem vivos — o mock do ADR-5 libera (controle)", async ({ page }) => {
  // MESMOS passos sem o gancho: sem este teste, um `disabled` PERMANENTE
  // passaria por política de escrita.
  await shell.beforeStandard();
  // sinal de que o device_info CHEGOU liberado (sem ele o default é travado)
  await expect(page.locator("main")).toContainText(LIBERADO);

  const dlg = await abreModal(page);
  await expect(dlg.getByText(MOTIVO)).toHaveCount(0);
  await expect(dlg.locator('svg[role="slider"]')).toHaveCount(3);
  await page.keyboard.press("Escape");

  const ir = await shell.openIrLab();
  await ir.locator('input[type="file"]').setInputFiles({
    name: "vintage_4x12.ir",
    mimeType: "application/octet-stream",
    buffer: IR_OK,
  });
  await ir.locator("select").selectOption("0");
  const enviar = ir.getByRole("button", { name: /^Enviar .* para o slot do aparelho/ });
  await expect(enviar).toBeEnabled();
  await expect(enviar).not.toHaveAttribute("title", MOTIVO_RE);
});
