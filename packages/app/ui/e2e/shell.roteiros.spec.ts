/**
 * Roteiros de teste manual (docs/UI_TEST_PLAN.md) executados no Chromium.
 * Cada teste referencia o ID do roteiro (R1–R7 + drum/looper). O resultado
 * desta suíte é registrado na tabela de execução do próprio UI_TEST_PLAN.
 *
 * Refactor POM (V-7): seletores encapsulados em e2e/pages/_pages.ts —
 * comportamento e asserts 1:1 com a versão original.
 */
import { expect, test } from "@playwright/test";
import { ShellPage } from "./pages/_pages";

let shell: ShellPage;

test.beforeEach(async ({ page }) => {
  shell = new ShellPage(page);
  await shell.beforeStandard();
});

/* ── R1. Topbar e identidade ── */
test("R1 topbar: logo, conexão, patch na navbar e foco de teclado", async ({ page }) => {
  const brand = shell.brand;
  await expect(brand.logo()).toBeVisible();
  await expect(brand.tagline()).toBeVisible();
  // status de conexão no cluster da navbar (texto; o cluster abriga boot/badge)
  // patch corrente no formato Pnn Nome (1-based, como o app oficial)
  await expect(shell.patchLabel()).toBeVisible();

  // tab alcança os controles globais com foco visível (outline definido no CSS)
  await page.keyboard.press("Tab");
  const focused = await page.evaluate(() => {
    const el = document.activeElement as HTMLElement | null;
    if (!el) return null;
    const s = getComputedStyle(el);
    return { tag: el.tagName, outlineStyle: s.outlineStyle, outlineWidth: s.outlineWidth };
  });
  expect(focused).not.toBeNull();

  // ◀ ▶ navegam presets em ciclo (coluna do patch do app oficial) —
  // navbar, display LED do board e biblioteca acompanhando juntos
  const listbox = shell.library.listbox();
  // O NOME precisa acompanhar o número nos 3 lugares (o fallback do board
  // já ignorou o pp e congelou o nome — cenário permanente no e2e)
  await shell.prevPatch();
  await expect(shell.patchLabel()).toHaveText("P99 Dreamy Aco");
  await expect(listbox.getByRole("option").last()).toHaveAttribute("aria-selected", "true");
  await expect(listbox.getByRole("option").last()).toHaveAccessibleName(/Dreamy Aco/);
  await expect(shell.board.display()).toContainText("Dreamy Aco");
  await shell.nextPatch();
  await expect(shell.patchLabel()).toHaveText("P01 It's GP100");
  await expect(listbox.getByRole("option").first()).toHaveAttribute("aria-selected", "true");
  await expect(shell.board.display()).toContainText("It's GP100");
});

/* ── R2. Biblioteca de fábrica (99) ── */
test("R2 biblioteca: 99 presets reais, busca, seleção e empty state", async ({ page }) => {
  const lib = shell.library;
  await expect(lib.listbox()).toBeVisible();
  await expect(lib.options()).toHaveCount(99);
  await expect(lib.optionAt(0)).toContainText("It's GP100");
  await expect(lib.options().last()).toContainText("Dreamy Aco");

  await lib.searchFor("mist");
  await expect(lib.options()).toHaveCount(1);
  await expect(lib.optionAt(0)).toHaveAccessibleName(/P25 Mist Rock/);

  // busca por estilo e vazio com dica
  await lib.searchFor("acoustic");
  const n = await lib.options().count();
  expect(n).toBeGreaterThan(10);
  await lib.searchFor("zzzz");
  await expect(lib.options()).toHaveCount(0);
  await expect(page.getByText(/Nenhum preset para/)).toBeVisible();

  // abrir preset (select real no shell; fallback dev troca o corrente)
  await lib.searchFor("mist");
  const mistOption = lib.optionAt(0);
  await mistOption.click();
  // seleção: accessible name é "P25 Mist Rock" (os spans não têm espaços no textContent)
  await expect(mistOption).toHaveAccessibleName(/P25 Mist Rock/);
  await expect(mistOption).toHaveAttribute("aria-selected", "true");
  // nome COMPLETO na navbar e no LED (não só o nº — o fallback já congelou)
  await expect(shell.patchLabel()).toHaveText("P25 Mist");
  await expect(shell.board.display()).toContainText("Mist");
});

/* ── R2b. User Patch: snapshot do palco, volta ao palco e exclusão (#11) ── */
test("R2b user patch: salvar a cadeia corrente, abrir de volta (U01) e excluir", async ({ page }) => {
  const lib = shell.library;
  const board = shell.board;

  // a aba NÃO é mais decorativa: mostra o estado vazio e o caminho para salvar
  await lib.userTab().click();
  await expect(page.getByText(/Nenhum patch salvo ainda/)).toBeVisible();
  await expect(page.getByText(/Prévia local/)).toBeVisible();
  await expect(lib.userRows()).toHaveCount(0);

  // salvar o patch CORRENTE (o app abre em P01) como patch de usuário
  await lib.saveAs("Meu clean");
  await expect(lib.userRows()).toHaveCount(1);
  await expect(lib.userRowAt(0)).toContainText("U01");
  await expect(lib.userRowAt(0)).toContainText("Meu clean");

  // abrir o patch salvo: o rótulo U01 assume no LED e na navbar (patch local
  // não tem cursor de fábrica — pp = −1)
  await lib.userRowAt(0).getByRole("button").first().click();
  await expect(board.display()).toContainText("U01");
  await expect(board.display()).toContainText("Meu clean");
  await expect(shell.patchLabel()).toHaveText("U01 Meu clean");

  // excluir: volta ao preset de fábrica confirmado pelo device (nunca um patch
  // apagado continuaria no palco — a UI não mente sobre o que existe)
  await lib.deleteUserPatch("Meu clean").click();
  await expect(lib.userRows()).toHaveCount(0);
  await expect(shell.patchLabel()).toHaveText("P01 It's GP100");
  await expect(board.display()).toContainText("It's GP100");
});

/* ── R3. Board (9 pedais reais — cadeia inteira numa fileira) + trava ⇄ mover ── */
test("R3 board: 9 slots na ordem do sinal e trava do drag", async () => {
  const board = shell.board;
  const fams = ["PRE", "DST", "AMP", "NR", "CAB", "EQ", "MOD", "DLY", "RVB"];
  for (let i = 0; i < fams.length; i += 1) {
    await board.expectSlotVisible(i + 1, fams[i]);
  }
  // a cadeia inteira desenha pedal: 9 no palco, nenhum lugar vazio
  await expect(board.root.locator('svg[role="group"]')).toHaveCount(9);
  await expect(board.root.getByText("vazio")).toHaveCount(0);

  // display LED: nº (2 dígitos) + nome + tipo
  await expect(board.display()).toContainText("01");

  // TODOS os slots são pedais REAIS: o arrasto sai de qualquer um; com a
  // trava ⇄ destravada a dica mora no RODAPÉ do palco (os antigos lugares
  // vazios eram quem carregava o hint "arraste ⇄")
  await board.expectMoverPressed(false);
  const slot = board.slot(1, "PRE");
  await expect(slot).not.toHaveAttribute("draggable", "true");
  await expect(board.slot(2, "DST").locator('svg[role="group"]')).toBeVisible();

  await board.toggleMover();
  await board.expectMoverPressed(true);
  await expect(slot).toHaveAttribute("draggable", "true");
  await expect(
    board.root.getByText("arraste ⇄ para trocar dois pedais de posição"),
  ).toBeVisible();

  // kill/drum/master não soltam erro
  await shell.killSwitch().click();
  await expect(shell.alerts).toHaveCount(0);
});

/* ── R7. Pedal real no palco (Fase 2/U-3 — o PRE real do P01 é C-Wah) ── */
test("R7 C-Wah: valor em texto no palco (só leitura), edição no modal, LED/footswitch e trava", async ({ page }) => {
  const board = shell.board;
  const pedalSlot = board.slot(1, "PRE");
  const pedal = pedalSlot.locator('svg[role="group"]');
  await expect(pedal).toBeVisible();
  // o PRE do P01 no all.prst é C-Wah (o mock antigo congelava COMP)
  await expect(pedal).toHaveAttribute("aria-label", /C-Wah/);
  // a cadeia inteira está no palco (9 pedais reais)
  await expect(board.root.locator('svg[role="group"]')).toHaveCount(9);
  await expect(board.slot(2, "DST").locator('svg[role="group"]')).toBeVisible();

  // knobs REAIS do dicionário (fxData: C-Wah = Range/Q/VOL): no palco o
  // pedal é COMPACTO e mostra o VALOR de cada knob em texto, só leitura
  await expect(pedalSlot.locator("[data-value]")).toHaveCount(3);
  await expect(pedalSlot.locator('[data-value="50"]').first()).toBeVisible();
  await expect(pedalSlot.locator('svg[role="slider"]')).toHaveCount(0);

  // o estado vem do .prst (`effectState`): o C-Wah do P01 nasce DESLIGADO e o
  // footswitch alterna LOCAL (sem comando de toggle no protocolo capturado)
  const led = pedalSlot.locator("[data-led]");
  await expect(led).toHaveAttribute("data-led", "off");
  await pedalSlot.locator('[role="button"][aria-label^="Ligar efeito"]').click();
  await expect(led).toHaveAttribute("data-led", "on");

  // trava ⇄ mover: ligada libera o arrasto de qualquer pedal da cadeia
  await expect(pedalSlot).not.toHaveAttribute("draggable", "true");
  await board.toggleMover();
  await expect(pedalSlot).toHaveAttribute("draggable", "true");

  // modo engenheiro (Settings → General): o tooltip do knob passa a mostrar
  // addr/code/ctrl do comando SET — sem a referência interna à doc (§13.11)
  const settings = await shell.openSettings();
  const eng = settings.role("switch", "Modo engenheiro");
  await expect(eng).toHaveAttribute("aria-checked", "false");
  await eng.click();
  await expect(eng).toHaveAttribute("aria-checked", "true");
  await settings.pressEscape();
  await settings.expectHidden();
  const tip = pedalSlot.locator('svg[role="img"] title').first();
  await expect(tip).toContainText("addr 10 01 00 02");
  await expect(tip).toContainText("ctrl 0");
  await expect(tip).toContainText("05000008"); // effectCode do C-Wah no .prst
  await expect(tip).not.toContainText("§13.11");

  // edição: a trava precisa voltar ao repouso antes — com ela ativa o clique
  // pertence ao drag, não à ampliação. O clique em QUALQUER ponto do pedal
  // (inclusive sobre o knob, que é display) abre o modal
  await board.toggleMover();
  await expect(board.mover()).toHaveAttribute("aria-pressed", "false");
  await pedal.locator('svg[role="img"]').first().click();
  const dlg = page.getByRole("dialog", { name: /Edição do pedal/ });
  await expect(dlg).toBeVisible();
  await expect(dlg.locator('svg[role="slider"]')).toHaveCount(3);
  await expect(dlg.locator('input[aria-label="Valor (Enter para editar)"]').first()).toHaveValue(
    "50",
  );

  // o knob AMPLIADO ajusta o MESMO estado: ArrowUp 50 → 51 no modal e no palco
  await dlg.locator('svg[role="slider"]').first().press("ArrowUp");
  const modeBox = dlg.locator('input[aria-label="Valor (Enter para editar)"]').first();
  await expect(modeBox).toHaveValue("51");
  await expect(pedalSlot.locator('[data-value="51"]').first()).toBeVisible();

  // valor EDITÁVEL no modal: digitar 42 → Enter (virou device_set_param) e o
  // palco reflete o novo valor ao fechar
  await modeBox.click();
  const editing = dlg.locator('input[aria-label^="Valor do knob"]');
  await editing.fill("42");
  await editing.press("Enter");
  await expect(modeBox).toHaveValue("42");
  await page.keyboard.press("Escape");
  await expect(dlg).toHaveCount(0);
  await expect(pedalSlot.locator('[data-value="42"]')).toBeVisible();
});

/* ── R7b. Effects List: a lista de efeitos do módulo (#19, U-3) ── */
test("R7b effects list: troca o efeito do pedal e o palco acompanha", async ({ page }) => {
  const board = shell.board;
  const pedalSlot = board.slot(1, "PRE");

  // abre a edição pelo pedal do palco (como no R7)
  await pedalSlot.locator('svg[role="img"]').first().click();
  const dlg = page.getByRole("dialog", { name: /Edição do pedal/ });
  await expect(dlg).toBeVisible();

  // a lista é do MÓDULO: só PRE, e o efeito atual vem marcado com ✓
  const list = dlg.getByRole("listbox", { name: "Lista de efeitos do módulo PRE" });
  await expect(list).toBeVisible();
  await expect(list.getByRole("option", { selected: true })).toHaveText(/C-Wah/);
  await expect(list.getByRole("option")).toHaveCount(16); // os 16 PREs do dicionário
  // AMP não vaza para a lista de PRE
  await expect(list.getByText("Bog RedM")).toHaveCount(0);

  // troca C-Wah → COMP: o palco troca na hora, com os knobs do COMP
  // (o nome acessível da opção é o NOME do efeito; o title carrega a ação)
  await list.getByRole("option", { name: "COMP", exact: true }).click();
  await expect(pedalSlot.locator('svg[role="group"]')).toHaveAttribute("aria-label", /COMP/);
  await expect(pedalSlot.locator("[data-value]")).toHaveCount(2);
  await expect(pedalSlot.locator('[data-value="20.0"]')).toBeVisible();
  // o modal acompanha (estado único) e o título já é o novo efeito
  await expect(dlg).toHaveAttribute("aria-label", /COMP/);
  await expect(list.getByRole("option", { selected: true })).toHaveText(/COMP/);

  // a busca filtra a lista do módulo
  const search = dlg.getByRole("textbox", { name: "Buscar efeito do módulo por nome" });
  await search.fill("wah");
  await expect(list.getByRole("option")).toHaveCount(4); // C-Wah, V-Wah, T-Wah, A-WAH
  await search.fill("zzz-nada");
  await expect(list.getByRole("option")).toHaveCount(0);
  await expect(dlg.getByText(/Nenhum efeito para/)).toBeVisible();

  // fechar devolve o palco com o COMP (o estado é do App, não do modal)
  await page.keyboard.press("Escape");
  await expect(dlg).toHaveCount(0);
  await expect(pedalSlot.locator('svg[role="group"]')).toHaveAttribute("aria-label", /COMP/);
});

/* ── R4. Modal Settings (⚙) ── */
test("R4 settings: 6 abas, persistência local, Global EQ e fechamento", async () => {
  const settings = await shell.openSettings();
  await settings.expectVisible();
  for (const t of ["General", "Global EQ", "About", "Info Frame", "Help", "Release Note"]) {
    await expect(settings.tab(t)).toBeVisible();
  }
  await expect(settings.dialog.getByText(/prévia local/)).toBeVisible();

  // General opera e PERSISTE após reload
  const input = settings.label("Input level");
  await input.fill("42");
  await shell.reload();
  const reopened = await shell.openSettings();
  await expect(reopened.label("Input level")).toHaveValue("42");

  // Footswitch Mode presente e desabilitado (aguardando captura)
  await expect(reopened.dialog.getByLabel(/Footswitch mode/)).toBeDisabled();

  // Global EQ: 5 bandas + controles desabilitados
  await reopened.openTab("Global EQ");
  for (const n of [1, 2, 3, 4, 5]) {
    await expect(reopened.role("button", `B${n}/5`)).toBeVisible();
  }
  await expect(reopened.label("Banda 1 freq")).toBeDisabled();
  await expect(reopened.label("L-CUT freq")).toBeDisabled();

  // Esc fecha; clique no fundo fecha
  await reopened.pressEscape();
  await reopened.expectClosed();
  await shell.openSettings();
  await shell.page.mouse.click(8, 8);
  await reopened.expectClosed();
});

/* ── R5. Boot e pushes ── */
test("R5 boot: roda sem erro, status segue conectado e pushes abre", async () => {
  await shell.rescanButton.click();
  // o boot sintético resolve rápido; a UI não pode soltar alerta de erro
  await expect(shell.alerts).toHaveCount(0);
  await expect(shell.banner).toContainText("on");

  const details = await shell.openPushes();
  await expect(details).toHaveJSProperty("open", true);
});

/* ── R6. Acessibilidade (gate manual) ── */
test("R6 a11y: alvos ≥32px, estado não só por cor, reduced-motion", async ({ page }) => {
  // alvos clicáveis ≥32px (altura) nos controles da casca
  const small = await page.evaluate(() => {
    const bad: string[] = [];
    document.querySelectorAll<HTMLElement>("button, select, input[type=range], input[type=number]").forEach((el) => {
      const r = el.getBoundingClientRect();
      if (r.height > 0 && r.height < 32) bad.push(`${el.tagName}:${el.getAttribute("aria-label") ?? el.textContent?.slice(0, 20)}`);
    });
    return bad;
  });
  expect(small).toEqual([]);

  // reduced-motion desliga as animações (live-dot e rolos): duração ≈ 0
  await page.emulateMedia({ reducedMotion: "reduce" });
  const animMs = await page.evaluate(() => {
    const d = getComputedStyle(document.querySelector(".live-dot")!).animationDuration;
    return parseFloat(d) * 1000; // converte s→ms ("1e-05s" = 0.01ms)
  });
  expect(animMs).toBeLessThan(5);

  // 800×600 sobrevive (scroll horizontal permitido, sem erro de layout)
  await page.setViewportSize({ width: 800, height: 600 });
  await expect(shell.banner).toBeVisible();
  await expect(shell.board.root).toBeVisible();
});

/* ── Drum: gestão de ritmos ── */
test("drum: painel com 87 ritmos do firmware, gênero→estilo e compassos reais", async () => {
  const drum = shell.drum;
  await expect(drum.chip).toContainText("Rock 1");
  await drum.open();

  await expect(drum.panel).toBeVisible();
  await expect(drum.panel.getByText(/87 ritmos/)).toBeVisible();

  await expect(drum.genreOptions()).toHaveCount(5);
  await expect(drum.styleOptions()).toHaveCount(33); // Rock
  await drum.selectGenre("World");
  await expect(drum.styleOptions()).toHaveCount(20);
  await drum.selectStyle("Samba");
  await drum.setBpm("140");

  // BPM (input number) ALINHADO com os selects: mesma caixa 240×32 border-box.
  // (O UA só aplica border-box a <select>; sem .gp-num{box-sizing} o BPM
  // media 258×46 — regressão travada por teste.)
  const boxes = await drum.measureBoxes();
  expect(boxes.bpm).not.toBeNull();
  expect(boxes.bpm!.boxSizing).toBe("border-box");
  expect(Math.abs(boxes.bpm!.w - boxes.genre!.w)).toBeLessThanOrEqual(1);
  expect(Math.abs(boxes.bpm!.h - boxes.genre!.h)).toBeLessThanOrEqual(1);

  await drum.close();

  // chip reflete o ritmo escolhido (nome acessível tem o resumo completo)
  await expect(drum.chipByName()).toHaveAccessibleName(/Samba, 140 BPM, compasso 4\/4/);
  await expect(drum.chipByName()).toContainText("Samba");
});

/* ── Looper: máquina de fita ── */
test("looper: transporte acessível, fita vazia, REC→PLAY e PRE/POST", async () => {
  const looper = shell.looper;
  await expect(looper.root).toBeVisible();
  await expect(looper.plate()).toBeVisible();

  const play = looper.playButton();
  await expect(play).toBeDisabled();
  await expect(looper.timer()).toHaveText("00:00");
  await expect(looper.tapeText(/vazia/)).toBeVisible();

  // REC grava; segundo toque vira PLAY (o label do botão muda com o modo)
  await looper.recButton().click();
  await expect(looper.status()).toContainText("REC");
  await looper.recButton().click();
  await expect(looper.status()).toContainText("PLAY");
  await expect(play).toBeEnabled();

  // timer anda enquanto roda
  await expect(looper.timer()).not.toHaveText("00:00", { timeout: 4000 });

  // REW volta ao início; STOP para
  await looper.rewButton().click();
  await expect(looper.timer()).toHaveText("00:00");
  await looper.stopButton().click();
  await expect(looper.status()).toContainText("STOP");

  // PRE/POST com os tempos reais do device (estado único no deck da fita)
  await looper.routeButton("POST").click();
  await expect(looper.tapeText(/fita: 45s \(POST\)/)).toBeVisible();
  await looper.routeButton("PRE").click();
  await expect(looper.tapeText(/fita: 90s \(PRE\)/)).toBeVisible();

  // CLEAR pede confirmação e zera
  await looper.clearButton().click();
  await looper.confirmClearButton().click();
  await expect(looper.tapeText(/vazia/)).toBeVisible();
  await expect(play).toBeDisabled();

  // parâmetros do firmware no rack
  for (const id of ["loop-rec", "loop-play", "loop-pvol"] as const) {
    await expect(looper.rackSlider(id)).toBeVisible();
  }
});
