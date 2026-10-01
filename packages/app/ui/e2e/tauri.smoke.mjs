/**
 * Smoke e2e do shell TAURI REAL (tauri-driver + WebKitWebDriver no Linux CI).
 * Prova que a casca sobe no WEBVIEW (não só no browser): binário gp100-ui
 * debug (frontendDist embutido, backend mock) → tauri-driver :4444 → Selenium.
 *
 * Receita oficial (v2.tauri.app/develop/tests/webdriver): capabilities
 * `tauri:options: { application }` + `browserName: "wry"`; no CI roda sob
 * `xvfb-run` com WEBKIT_DISABLE_DMABUF_RENDERER=1 (software rendering).
 *
 * Uso local/CI (após `cargo build -p gp100-ui` em packages/app/api):
 *   APP_PATH=/caminho/gp100-ui node e2e/tauri.smoke.mjs
 * Saída: exit 0 = casca bootou no webview e renderizou os painéis.
 */
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Builder, By, Capabilities, until } from "selenium-webdriver";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "..", "..", "..");

/** Resolve o binário do shell (env APP_PATH vence; senão, paths padrão). */
function resolveAppBinary() {
  const candidates = [
    process.env.APP_PATH,
    path.resolve(repoRoot, "packages/app/api/target/debug/gp100-ui"),
    path.resolve(repoRoot, "packages/app/api/target/debug/gp100_ui"),
  ].filter(Boolean);
  const found = candidates.find((p) => existsSync(p));
  if (!found) {
    console.error(`✗ binário do gp100-ui não encontrado (tentei: ${candidates.join(", ")})`);
    process.exit(1);
  }
  return found;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Espera o tauri-driver aceitar conexões na porta. */
async function waitForPort(port, tries = 40) {
  const net = await import("node:net");
  for (let i = 0; i < tries; i += 1) {
    const ok = await new Promise((resolve) => {
      const sock = net.default.connect(port, "127.0.0.1");
      sock.on("connect", () => {
        sock.end();
        resolve(true);
      });
      sock.on("error", () => resolve(false));
    });
    if (ok) return;
    await sleep(250);
  }
  throw new Error(`tauri-driver não abriu a porta ${port}`);
}

const application = resolveAppBinary();
console.log(`▸ shell: ${application}`);

// tauri-driver (wrapper) — o binário vem de ~/.cargo/bin (cargo install)
const tauriDriver = spawn(path.resolve(os.homedir(), ".cargo", "bin", "tauri-driver"), [], {
  stdio: ["ignore", "inherit", "inherit"],
});
let exiting = false;
tauriDriver.on("exit", (code) => {
  if (!exiting) {
    console.error(`✗ tauri-driver saiu cedo (code ${code})`);
    process.exit(1);
  }
});

let driver;
try {
  await waitForPort(4444);

  const capabilities = new Capabilities();
  capabilities.set("tauri:options", { application });
  capabilities.setBrowserName("wry");
  driver = await new Builder().withCapabilities(capabilities).usingServer("http://127.0.0.1:4444/").build();

  // 1. casca bootou no webview: banner com a identidade do app
  await driver.wait(until.elementLocated(By.css('[role="banner"]')), 20_000);
  const banner = await driver.findElement(By.css('[role="banner"]')).getText();
  if (!banner.includes("GP-100 NextGen")) throw new Error(`banner inesperado: ${banner}`);

  // 2. device mock conectado (o smoke não precisa de hardware)
  await driver.wait(until.elementLocated(By.css('[role="status"]')), 20_000);

  // 3. os painéis existem no DOM do webview
  for (const [label, sel] of [
    ["board", '[aria-label="Pedalboard (9 lugares da cadeia)"]'],
    ["looper", '[aria-label="Looper (máquina de fita)"]'],
    ["biblioteca", '[aria-label="Biblioteca de presets"]'],
  ]) {
    await driver.findElement(By.css(sel)); // lança se não existir
    console.log(`  ✓ painel ${label} renderizado no webview`);
  }

  // 4. biblioteca REAL (artefato do all.prst) chega inteira ao webview
  const options = await driver.findElements(By.css('[role="option"]'));
  if (options.length !== 99) throw new Error(`biblioteca com ${options.length} opções (esperado 99)`);
  console.log("  ✓ biblioteca com os 99 presets de fábrica");

  console.log("✅ SMOKE TAURI: casca bootou no webview e renderizou a casca completa");
} catch (err) {
  console.error("✗ smoke tauri falhou:", err.message ?? err);
  process.exitCode = 1;
} finally {
  exiting = true;
  if (driver) await driver.quit().catch(() => {});
  tauriDriver.kill();
}
