/**
 * POM (V-7) — Page Objects dos e2e. Cada page object conhece UMA superfície
 * da UI (navbar, biblioteca, board, looper, drawer do drum, modal Settings)
 * e expõe SEMÂNTICA (abrir/gravar/alternar), nunca seletores crus nos specs.
 *
 * REGRA DA REFACTOR: comportamento 1:1 — mesmo goto/asserts dos specs
 * originais, apenas realocados. O beforeStandard() mantém o contrato comum
 * dos specs (`goto + banner visível`).
 */
import { expect, type Locator, type Page } from "@playwright/test";

/* ── Raiz: app inteiro + fluxos globais (boot, pushes, atalhos) ── */

export class ShellPage {
  readonly page: Page;
  readonly banner: Locator;
  readonly alerts: Locator;
  readonly bootProgress: Locator;
  readonly rescanButton: Locator;
  readonly pushDetails: Locator;
  readonly pushSummary: Locator;
  readonly library: LibraryPage;
  readonly board: BoardPage;
  readonly looper: LooperPage;
  readonly drum: DrumPage;
  readonly settings: SettingsDialog;
  readonly tuner: TunerPage;
  readonly brand: BrandPage;

  constructor(page: Page) {
    this.page = page;
    this.banner = page.getByRole("banner");
    this.alerts = page.getByRole("alert");
    this.bootProgress = page.locator('[aria-label="Progresso do boot"]');
    this.rescanButton = this.banner.getByRole("button", { name: "Reescanear device" });
    this.pushDetails = page.locator("details");
    this.pushSummary = page.getByText(/pushes do device/);
    this.library = new LibraryPage(page);
    this.board = new BoardPage(page);
    this.looper = new LooperPage(page);
    this.drum = new DrumPage(page);
    this.settings = new SettingsDialog(page);
    this.tuner = new TunerPage(page);
    this.brand = new BrandPage(this);
  }

  /** contrato comum dos specs: goto + banner visível */
  async beforeStandard(): Promise<void> {
    await this.page.goto("/");
    await expect(this.banner).toBeVisible();
  }

  async goto(): Promise<void> {
    await this.page.goto("/");
  }

  async reload(): Promise<void> {
    await this.page.reload();
    await expect(this.banner).toBeVisible();
  }

  /**
   * Ganho de teste: falha simulada de device ANTES do load (o mount lê o
   * storage). Valores (src/ipc/device.ts):
   *   - `info|boot|board|select|set_param|all` — falha SEMPRE;
   *   - `<op>:<n>` — falha TRANSITÓRIA (retry/backoff cobre; issue #20);
   *   - `boot-mid` — device cai NO MEIO do boot (progresso real e então erro).
   */
  async failDevice(op: string): Promise<void> {
    await this.page.addInitScript((o) => {
      localStorage.setItem("gp100.debug.failDevice", o);
    }, op);
  }

  /**
   * Arma o gancho DEPOIS do load (o mount já rodou limpo) — para cenários
   * de falha do MEIO da sessão (ex.: select só falha no clique do usuário).
   */
  async armFailDevice(op: string): Promise<void> {
    await this.page.evaluate((o) => localStorage.setItem("gp100.debug.failDevice", o), op);
  }

  /** Desarma o gancho no meio do teste (prova a recuperação). */
  async clearFailDevice(): Promise<void> {
    await this.page.evaluate(() => localStorage.removeItem("gp100.debug.failDevice"));
  }

  /** Ação de recuperação do banner de erro (botão DENTRO do role="alert"). */
  alertRetry(): Locator {
    return this.page
      .getByRole("alert")
      .getByRole("button", { name: "Tentar novamente a operação que falhou" });
  }

  /** patch corrente exibido na navbar: Pnn + nome (1-based, app oficial) */
  patchLabel(): Locator {
    return this.banner.getByText(/^P\d{2}/);
  }

  /** connection status curto (on/off) */
  statusText(text: string): Locator {
    return this.banner.getByText(text, { exact: true });
  }

  async prevPatch(): Promise<void> {
    await this.banner.getByRole("button", { name: "Patch anterior" }).click();
  }

  async nextPatch(): Promise<void> {
    await this.banner.getByRole("button", { name: "Próximo patch" }).click();
  }

  killSwitch(): Locator {
    return this.page.getByRole("button", { name: /^Kill switch/ });
  }

  masterVolume(): Locator {
    return this.page.getByLabel("Master volume");
  }

  /** abre o drawer de pushes e devolve o <details> */
  async openPushes(): Promise<Locator> {
    await this.pushSummary.click();
    return this.pushDetails;
  }

  async openSettings(): Promise<SettingsDialog> {
    await this.page.getByRole("button", { name: "Abrir configurações" }).click();
    await expect(this.settings.dialog).toBeVisible();
    return this.settings;
  }
}

/* ── Navbar: logo/tagline/branding (estáticos) ── */
export class BrandPage {
  constructor(private readonly shell: ShellPage) {}

  logo(): Locator {
    return this.shell.page.getByText("GP-100").first();
  }

  tagline(): Locator {
    return this.shell.page.getByText("editor não-oficial");
  }
}

/* ── Biblioteca de presets (aside esquerdo) ── */
export class LibraryPage {
  readonly page: Page;
  readonly root: Locator;

  constructor(page: Page) {
    this.page = page;
    this.root = page.getByRole("complementary", { name: "Biblioteca de presets" });
  }

  /** listbox dos 99 de fábrica (o aside é o complementary; a listbox, a lista) */
  listbox(): Locator {
    return this.page.getByRole("listbox", { name: /Presets de fábrica/ });
  }

  search(): Locator {
    return this.page.getByRole("textbox", { name: /Buscar preset/ });
  }

  async searchFor(q: string): Promise<void> {
    await this.search().fill(q);
  }

  options(): Locator {
    return this.listbox().getByRole("option");
  }

  optionAt(index: number): Locator {
    return this.options().nth(index);
  }

  userTab(): Locator {
    return this.page.getByRole("tab", { name: "User Patch" });
  }

  /** seleciona pelo texto do accessible name (ex.: /P25 Mist Rock/) */
  async select(re: RegExp): Promise<Locator> {
    const opt = this.options().filter({ hasText: re }).first();
    await opt.click();
    return opt;
  }
}

/* ── Board/palco (região) + trava ⇄ mover ── */
export class BoardPage {
  readonly page: Page;
  readonly root: Locator;

  constructor(page: Page) {
    this.page = page;
    this.root = page.getByRole("region", { name: "Pedalboard (9 lugares da cadeia)" });
  }

  boardRegion(): Locator {
    return this.root;
  }

  /** região Pedalboard por prefixo (para specs que casam /Pedalboard/) */
  region(): Locator {
    return this.page.getByRole("region", { name: /Pedalboard/ });
  }

  slot(n: number, fam: string): Locator {
    return this.root.getByLabel(`Slot ${n}: ${fam}`);
  }

  display(): Locator {
    return this.root.getByRole("status");
  }

  async expectSlotVisible(n: number, fam: string): Promise<void> {
    await expect(this.slot(n, fam)).toBeVisible();
  }

  /** trava de mover (cabeçalho do palco): cadeado 🔒/🔓 abaixo do display do patch */
  mover(): Locator {
    return this.page.getByRole("button", { name: /Trava de mover pedais/ });
  }

  async expectMoverPressed(pressed: boolean): Promise<void> {
    await expect(this.mover()).toHaveAttribute("aria-pressed", String(pressed));
  }

  async toggleMover(): Promise<void> {
    await this.mover().click();
  }
}

/* ── Looper (região) ── */
export class LooperPage {
  readonly page: Page;
  readonly root: Locator;

  constructor(page: Page) {
    this.page = page;
    this.root = page.getByRole("region", { name: "Looper (máquina de fita)" });
  }

  rootRegion(): Locator {
    return this.root;
  }

  plate(): Locator {
    return this.root.getByText("GP-100 · TAPE LOOPER · STEREO");
  }

  playButton(): Locator {
    return this.root.getByRole("button", { name: /Tocar loop/ });
  }

  /** botão ●: REC / Parar gravação e tocar / Sobrepor, conforme o modo */
  recButton(): Locator {
    return this.root.getByRole("button", {
      name: /Gravar loop|Parar gravação e tocar|Sobrepor/,
    });
  }

  stopButton(): Locator {
    return this.root.getByRole("button", { name: /Parar/ });
  }

  rewButton(): Locator {
    return this.root.getByRole("button", { name: /Retroceder/ });
  }

  clearButton(): Locator {
    return this.root.getByRole("button", { name: /Limpar fita/ });
  }

  confirmClearButton(): Locator {
    return this.root.getByRole("button", { name: /Confirmar limpar fita/ });
  }

  routeButton(route: "PRE" | "POST"): Locator {
    return this.root.getByRole("button", { name: new RegExp(route) });
  }

  timer(): Locator {
    return this.root.getByRole("timer");
  }

  /** estado da fita (deck) — ÚNICO role=status do painel */
  status(): Locator {
    return this.root.getByRole("status").filter({ hasText: "●" });
  }

  tapeText(text: string | RegExp): Locator {
    return this.root.getByText(text);
  }

  rackSlider(id: "loop-rec" | "loop-play" | "loop-pvol"): Locator {
    return this.root.locator(`#${id}`);
  }

  rackSliderByLabel(label: string): Locator {
    return this.page.getByLabel(label, { exact: true });
  }

  /** span display ao lado do slider (#id + span) */
  sliderDisplay(id: "loop-rec" | "loop-play" | "loop-pvol"): Locator {
    return this.root.locator(`#${id} + span`);
  }
}

/* ── Drum: chip da navbar + MODAL de gestão (padrão Settings) ── */
export class DrumPage {
  readonly page: Page;
  readonly chip: Locator;
  /** play/stop DIRETO na navbar (não exige o modal) */
  readonly toggle: Locator;
  readonly panel: Locator;

  constructor(page: Page) {
    this.page = page;
    this.chip = page.getByRole("button", { name: /^Bateria \(drum\):/ });
    this.toggle = page.getByRole("button", { name: "Tocar ou parar o ritmo da bateria" });
    this.panel = page.getByRole("dialog", { name: /Gestão de ritmos/ });
  }

  /** chip alternativo com nome acessível longo (muda com o estado) */
  chipByName(): Locator {
    return this.page.getByRole("button", { name: /abrir gestão de ritmos/ });
  }

  async open(): Promise<void> {
    await this.chip.click();
    await expect(this.panel).toBeVisible();
  }

  async close(): Promise<void> {
    await this.panel.getByRole("button", { name: "Fechar gestão de ritmos" }).click();
  }

  async selectGenre(g: string): Promise<void> {
    await this.panel.locator("#drum-genre").selectOption(g);
  }

  async selectStyle(s: string): Promise<void> {
    await this.panel.locator("#drum-style").selectOption(s);
  }

  async setBpm(v: string): Promise<void> {
    await this.panel.locator("#drum-bpm").fill(v);
  }

  genreOptions(): Locator {
    return this.panel.locator("#drum-genre option");
  }

  styleOptions(): Locator {
    return this.panel.locator("#drum-style option");
  }

  volume(): Locator {
    return this.page.getByLabel("Volume", { exact: true });
  }

  speed(): Locator {
    return this.page.getByLabel("Speed", { exact: true });
  }

  async setVolume(v: string): Promise<void> {
    await this.volume().fill(v);
  }

  async setSpeed(v: string): Promise<void> {
    await this.speed().fill(v);
  }

  /** box-sizing/medidas alinhadas dos selects × bpm (regressão do border-box) */
  async measureBoxes(): Promise<{
    bpm: { w: number; h: number; boxSizing: string } | null;
    genre: { w: number; h: number; boxSizing: string } | null;
    beat: { w: number; h: number; boxSizing: string } | null;
  }> {
    return this.panel.evaluate((root) => {
      const box = (sel: string) => {
        const el = root.querySelector(sel);
        if (!el) return null;
        const r = el.getBoundingClientRect();
        return { w: +r.width.toFixed(1), h: +r.height.toFixed(1), boxSizing: getComputedStyle(el).boxSizing };
      };
      return { bpm: box("#drum-bpm"), genre: box("#drum-genre"), beat: box("#drum-beat") };
    });
  }
}

/* ── Modal Settings (⚙) ── */
export class SettingsDialog {
  readonly page: Page;
  readonly dialog: Locator;

  constructor(page: Page) {
    this.page = page;
    this.dialog = page.getByRole("dialog", { name: "Configurações" });
  }

  async expectVisible(): Promise<void> {
    await expect(this.dialog).toBeVisible();
  }

  async expectHidden(): Promise<void> {
    await expect(this.dialog).toBeHidden();
  }

  async expectClosed(): Promise<void> {
    await expect(this.dialog).toHaveCount(0);
  }

  tab(name: string): Locator {
    return this.dialog.getByRole("tab", { name });
  }

  async openTab(name: string): Promise<void> {
    await this.tab(name).click();
  }

  label(label: string): Locator {
    return this.dialog.getByLabel(label);
  }

  role(role: "switch" | "checkbox" | "button" | "tab", name: string): Locator {
    return this.dialog.getByRole(role, { name });
  }

  /** span display ao lado do input ([aria-label] + span) */
  displayAfter(ariaLabel: string): Locator {
    return this.dialog.locator(`[aria-label="${ariaLabel}"] + span`);
  }

  async expectPersisted(patch: Record<string, unknown>): Promise<void> {
    const saved = await this.page.evaluate(() =>
      JSON.parse(localStorage.getItem("gp100.settings.general.v1") ?? "{}"),
    );
    expect(saved).toMatchObject(patch);
  }

  async pressEscape(): Promise<void> {
    await this.page.keyboard.press("Escape");
  }
}

/* ── afinador do palco (display sempre visível, no lugar do antigo VU) ── */
export class TunerPage {
  readonly group: Locator;

  constructor(page: Page) {
    this.group = page.getByRole("region", { name: /Pedalboard/ }).getByRole("group", { name: "Afinador" });
  }

  /** botão monitorar (liga/desliga a monitoração de afinação) */
  powerButton(): Locator {
    return this.group.getByRole("button", { name: /Ligar ou desligar a monitoração/ });
  }

  /** LED do próprio botão do monitor: verde ligado, vermelho desligado (#8) */
  powerLed(): Locator {
    return this.group.locator("[data-tuner-power-led]");
  }

  /** botão de modo (bypass/thru/mute) — SEMPRE presente, mesmo com o monitor off */
  modeButton(): Locator {
    return this.group.getByRole("button", { name: /Modo do afinador/ });
  }

  demoButton(): Locator {
    return this.group.getByRole("button", { name: /demonstração do afinador/ });
  }

  /** nota central exibida ("—" em repouso) */
  note(): Locator {
    return this.group.locator("[data-tuner-note]");
  }

  /** LED próprio do afinador (cinza/âmbar/verde/vermelho) */
  led(): Locator {
    return this.group.locator("[data-tuner-led]");
  }

  /** REF PITCH atual (ex.: "440Hz") */
  async refPitch(): Promise<string | null> {
    return this.group.locator("[data-tuner-ref]").textContent();
  }
}
