/**
 * Smoke e2e do shell TAURI REAL (tauri-driver + WebKitWebDriver no Linux CI).
 * Prova que a casca sobe no WEBVIEW (não só no browser): binário gp100-ui
 * debug (build SEM aparelho — #150: o app nunca monta mock) → tauri-driver
 * :4444 → Selenium.
 *
 * **(#150/#161) o objeto do smoke é o GATE de boot sem aparelho:** a
 * casca NÃO monta com o boot falho — o webview mostra só a navbar + o
 * painel de erro com o MOTIVO que o backend declarou (nunca uma lista de
 * fábrica embutida), LED off, sem barra eterna, e as ações "Refazer o
 * boot"/"Reconectar" recuperam até o boot falhar de novo. O
 * cenário DeviceGone do #48 (device morto NO MEIO do boot via
 * `GP100_DEBUG_FAULT`) não existe mais aqui: esse env só é lido pelo
 * MockDevice, que deixou de ser montado pelo app — a mesma asserção de
 * recuperação é exercida pela falha imediata de boot sem sessão.
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
 * Saída: exit 0 = o webview subiu, o gate de boot barrou a casca sem
 * aparelho e provou motivo + ações + recuperação com honestidade.
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

/** Resolve o binário do tauri-driver: env TAURI_DRIVER vence; senão o PATH
 *  (imagem ci-linux instala em /usr/local/bin); senão ~/.cargo/bin, que é onde
 *  o `cargo install` põe no runner hospedado e no host local.
 *  Lição #41: dentro do container o HOME é /github/home — resolver só por
 *  $HOME dava `spawn /github/home/.cargo/bin/tauri-driver ENOENT` mesmo com o
 *  driver instalado. */
function resolveTauriDriver() {
  const explicit = process.env.TAURI_DRIVER;
  if (explicit && existsSync(explicit)) return explicit;
  for (const dir of (process.env.PATH || "").split(path.delimiter)) {
    for (const name of ["tauri-driver", "tauri-driver.exe"]) {
      const candidate = path.join(dir, name);
      if (existsSync(candidate)) return candidate;
    }
  }
  return path.resolve(os.homedir(), ".cargo", "bin", "tauri-driver");
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

/* #150/#161 — build SEM aparelho: o app não monta mock, o boot falha (actor
 * sem sessão) e o GATE tem que mostrar o motivo do backend, derrubar o LED,
 * sumir com a barra de progresso e manter as ações (Refazer o boot /
 * Reconectar) operáveis. O boot saudável (2297 transações) NÃO é esperado
 * aqui: no webview do CI ele levaria ~65 s (medição da run 36999196897) e o
 * objeto do smoke é a recuperação. */

// tauri-driver (wrapper) — PATH primeiro (imagem de CI), ~/.cargo/bin como
// fallback (cargo install do runner hospedado / host local).
const driverBin = resolveTauriDriver();
console.log(`▸ driver: ${driverBin}`);
const tauriDriver = spawn(driverBin, [], {
  stdio: ["ignore", "inherit", "inherit"],
  env: process.env,
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

  // RACE do CI (container ci-linux, run 36953492697): a porta do tauri-driver
  // abre ANTES do WebKitWebDriver NATIVO responder — a 1ª criação de sessão
  // morria com `Connection refused (os error 111)` ("Error serving connection"
  // no proxy). Retenta por até ~60s: o native driver leva alguns segundos no
  // software rendering. Esperar só a porta 4444 não basta.
  const SESSION_DEADLINE = Date.now() + 60_000;
  for (let tentativa = 1; !driver; tentativa++) {
    try {
      driver = await new Builder().withCapabilities(capabilities).usingServer("http://127.0.0.1:4444/").build();
    } catch (err) {
      if (Date.now() > SESSION_DEADLINE) throw err;
      console.log(`… sessão ainda não acordou (tentativa ${tentativa}: ${err.message ?? err}) — retry`);
      await sleep(3000);
    }
  }

  // 1. casca bootou no webview: banner com a identidade do app.
  // 60s: o webview do CI roda em SOFTWARE RENDERING (xvfb + DRI3 indisponível)
  // e o primeiro paint pode passar de 20s (run 36937323423 estourou o antigo).
  await driver.wait(until.elementLocated(By.css('[role="banner"]')), 60_000);
  const banner = await driver.findElement(By.css('[role="banner"]')).getText();
  // A marca no DOM é `MSG.brand` = "GP-100" (o "NextGen Editor" é só o TÍTULO
  // da janela). O assert antigo procurava "GP-100 NextGen" NO BANNER — nunca
  // existiu ali, e o smoke ficou vermelho desde que nasceu (issue #39).
  if (!banner.includes("GP-100")) throw new Error(`banner sem a marca (MSG.brand): ${banner}`);

  // 2. cluster de conexão da navbar (o smoke não precisa de hardware)
  await driver.wait(
    until.elementLocated(By.css('[role="status"][aria-label="Conexão e boot"]')),
    20_000,
  );

  // 3. (#161) o GATE de boot: sem aparelho o boot FALHA e a casca NÃO
  // monta — nenhum painel, nenhuma lista (a casca só nasce com relatório
  // validado).
  for (const [label, sel] of [
    ["board", '[aria-label="Pedalboard (9 lugares da cadeia)"]'],
    ["looper", '[aria-label="Looper (máquina de fita)"]'],
    ["biblioteca", '[aria-label="Patches do aparelho"]'],
  ]) {
    if ((await driver.findElements(By.css(sel))).length > 0) {
      throw new Error(`painel ${label} montado com o boot falho (o gate #161 deveria barrar)`);
    }
  }
  const options = await driver.findElements(By.css('[role="option"]'));
  if (options.length !== 0) {
    throw new Error(`biblioteca com ${options.length} opções (esperado 0 — sem aparelho não há lista)`);
  }
  console.log("  ✓ gate de boot: casca NÃO monta sem aparelho (sem painéis, sem lista)");

  // 4. (#150/#161) o erro mostra o MOTIVO que o backend declarou + as duas
  // ações: Refazer o boot (sempre) e Reconectar (backend "none").
  await driver.wait(until.elementLocated(By.css('[role="alert"]')), 60_000);
  await driver.wait(
    until.elementLocated(By.xpath('//*[contains(., "Aparelho não conectado")]')),
    20_000,
  );
  await driver.findElement(
    By.xpath('//*[@role="alert"]//button[contains(., "Refazer o boot")]'),
  );
  await driver.findElement(
    By.xpath('//*[@role="alert"]//button[contains(., "Reconectar")]'),
  );
  console.log("  ✓ gate de boot: motivo do backend + ações Refazer o boot e Reconectar");

  // 5. sem barra eterna com o boot falho (o trap do "spinner eterno" #48)
  if ((await driver.findElements(By.css('[role="progressbar"]'))).length > 0) {
    throw new Error("barra de progresso viva com o boot falho (spinner eterno)");
  }

  // XPath negativo: o alerta NÃO pode vazar o detalhe do transporte
  // (o texto técnico mora no Rust; a UI mostra só a mensagem do catálogo).
  const leak = By.xpath(
    '//*[@role="alert" and (contains(., "MockFault") or contains(., "die-after"))]',
  );
  if ((await driver.findElements(leak)).length > 0) {
    throw new Error("detalhe técnico do transporte vazou no alerta do boot");
  }

  // 6. Recuperação pela AÇÃO do gate: "Refazer o boot" roda um boot novo
  // que falha igual — o device morto não ressuscita, mas o actor segue de
  // pé e a UI responde. É esta 2ª falha que FIXA o estado observado: LED
  // off (sem corrida com o device_info do mount), NENHUMA barra de
  // progresso eterna, alerta de volta e o ⟳ da navbar habilitado. Se
  // qualquer um faltar, o smoke falha — é o trap do "spinner eterno" (#48).
  const rescan = await driver.findElement(By.css('[aria-label="Reescanear device"]'));
  await driver.wait(async () => rescan.isEnabled(), 30_000);
  const refazer = await driver.findElement(
    By.xpath('//*[@role="alert"]//button[contains(., "Refazer o boot")]'),
  );
  await refazer.click();
  await driver.wait(async () => {
    const alerta = (await driver.findElements(By.css('[role="alert"]'))).length > 0;
    const habilitado = await rescan.isEnabled();
    const semBarra = (await driver.findElements(By.css('[role="progressbar"]'))).length === 0;
    const ledOff =
      (await driver.findElements(By.css('[role="status"][aria-label="Conexão e boot"] .idle-dot')))
        .length > 0;
    return alerta && habilitado && semBarra && ledOff;
  }, 30_000);
  console.log("  ✓ pós-refazer: LED off, sem barra eterna e ⟳ da navbar vivo");

  console.log("✅ SMOKE TAURI: webview no ar + gate de boot honesto sem aparelho (falha → motivo → ações → recuperação)");
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
