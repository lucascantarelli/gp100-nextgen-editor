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
