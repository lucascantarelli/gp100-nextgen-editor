/**
 * Smoke e2e do shell TAURI REAL (tauri-driver + WebKitWebDriver no Linux CI).
 * Prova que a casca sobe no WEBVIEW (não só no browser): binário gp100-ui
 * debug (backend mock) → tauri-driver :4444 → Selenium.
 *
 * ⚠️ BUILD DEBUG CARREGA O `devUrl` (http://localhost:5173) — o dist embutido
 * só é usado em release. Por isso o job serve o dist de produção ali
 * (`vite preview --port 5173`) antes deste script; sem isso o webview mostra
 * "Could not connect to localhost: Connection refused" (causa raiz fechada na
 * run 36943668915 pelo page source do diagnóstico deste script).
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
/* e2e/ → ui/ → app/ → packages/ → RAIZ do repo (4 níveis).
   Com 3 níveis o repoRoot caía em `packages/` e o candidato virava
   `packages/packages/app/api/...` — o smoke falhava sempre no CI
   (regressão achada na auditoria de 01/10). */
const repoRoot = path.resolve(here, "..", "..", "..", "..");

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

  // 1. casca bootou no webview: banner com a identidade do app.
  // 60s: o webview do CI roda em SOFTWARE RENDERING (xvfb + DRI3 indisponível)
  // e o primeiro paint pode passar de 20s (run 36937323423 estourou o antigo).
  await driver.wait(until.elementLocated(By.css('[role="banner"]')), 60_000);
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
  // DIAGNÓSTICO (o smoke nunca esteve verde — job criado em 30/09):
  // "página vazia" (front não carregou) ≠ "DOM sem banner" (app montou e
  // falhou no meio) ≠ "webview nem pintou". O page source separa os casos.
  if (driver) {
    try {
      const src = await driver.getPageSource();
      console.error(`—— page source (${String(src).length} chars, primeiros 2000) ——`);
      console.error(String(src).slice(0, 2000));
    } catch (e) {
      console.error("(sem page source:", e.message ?? e, ")");
    }
  }
  process.exitCode = 1;
} finally {
  exiting = true;
  if (driver) await driver.quit().catch(() => {});
  tauriDriver.kill();
}
