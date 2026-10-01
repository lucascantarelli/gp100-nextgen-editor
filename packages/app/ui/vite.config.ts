/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Base "./" é obrigatória para o Tauri carregar o bundle de file:// em
// produção; o bloco `test` roda o vitest em jsdom (App/hooks/ipc).
export default defineConfig({
  plugins: [react()],
  base: "./",
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { target: "es2022", outDir: "dist" },
  test: {
    environment: "jsdom",
    include: ["tests/**/*.test.{ts,tsx}"],
    /* 15s: sob instrumentação de coverage o mount do App (99 presets +
     * boot simulado em lotes) passa de 5s — piso para o gate não piscar. */
    testTimeout: 15_000,
    /*
     * GATE DE COVERAGE (V-7): `pnpm test:coverage` falha se as linhas/funções
     * caírem de 85% — o mesmo gate roda na CI (build-front action). Branques
     * cobrem o que sobra de difícil exercício no jsdom (baselines e2e cuidam
     * do visual; o shell Tauri, do smoke).
     */
    coverage: {
      provider: "v8",
      reporter: ["text", "html"],
      include: ["src/**"],
      /* Honesto por construção: main.tsx é bootstrap sem lógica; design.css
       * não é código; e o VU é coberto por e2e (rAF por frame, jsdom sem rAF
       * estável). Tudo que TEM lógica testável fica incluído. */
      exclude: [
        "src/main.tsx",
        "src/design/design.css",
        "src/components/VuPanel.tsx",
      ],
      thresholds: {
        statements: 85,
        functions: 85,
        lines: 85,
        /* Branches: medido, sem gate — ganho passo a passo; mapa dos furos
         * está em docs/ROADMAP.md (V-7, mapa de edge cases de IPC). */
        branches: 0,
      },
    },
  },
});
