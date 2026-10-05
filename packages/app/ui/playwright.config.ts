/**
 * Playwright (e2e) — roteiros de teste manual executados no Chromium.
 * docs/UI_TEST_PLAN.md é a fonte dos roteiros (R1–R6 + drum/looper);
 * cada teste cita o ID do roteiro. Reusa o `pnpm dev` se já estiver no ar.
 */
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  fullyParallel: false,
  // MEDIDO (8 nucleos): com 4 workers cada tecla levava 270-500 ms — sao 4
  // navegadores renderizando o app inteiro ao mesmo tempo, e o `keydown`
  // espera a main thread. Com 2 workers cai para 76-217 ms e o tempo de
  // parede quase nao muda (1,0 -> 1,1 min): o gargalo e o arquivo mais serial,
  // nao a quantidade de worker. Mais worker aqui so compra latencia ruim.
  workers: 2,
  // As baselines visuais têm JOB PRÓPRIO no CI (platforma e artefato próprios),
  // e elas nao entram na suite de comportamento: sao 50 dos 78 testes e, por
  // arquivo, rodam em SERIE num worker so — 134 s dos 176 s de parede. Deixá-las
  // aqui faz o job de shellesperar por elas E roubar cpu dos testes que
  // exercitam comportamento (foi assim que o R-ATALHOS ficou lento e flake).
  // O job de visual passa o caminho do arquivo, então continua vendo tudo.
  testIgnore: process.env.SKIP_VISUAL === "1" ? ["**/visual.spec.ts"] : [],
  reporter: [["list"]],
  use: {
    baseURL: "http://localhost:5173",
    viewport: { width: 1440, height: 900 },
  },
  // Baselines da regressão visual (visual.spec.ts): por PLATAFORMA (fontes
  // divergem entre Windows/Linux) e por viewport (o nome do teste inclui).
  // Animações congeladas + tolerância 1% para antialiasing estável.
  snapshotPathTemplate: "{testDir}/__screenshots__/{arg}-{platform}{ext}",
  expect: {
    toHaveScreenshot: {
      animations: "disabled",
      caret: "hide",
      maxDiffPixelRatio: 0.01,
    },
  },
  webServer: {
    command: "pnpm dev --port 5173 --strictPort",
    url: "http://localhost:5173",
    reuseExistingServer: true,
    timeout: 60_000,
  },
  projects: [{ name: "chromium", use: { browserName: "chromium" } }],
});
