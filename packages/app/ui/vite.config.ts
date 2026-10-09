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
    /* POOL vmThreads (#145, F-10): o pool padrão pagava um processo novo por
     * arquivo — a suíte de 38 arquivos criava o jsdom 38 vezes (113s de
     * ambiente, 39% do tempo rastreado). vmThreads cria o ambiente UMA vez
     * por worker reaproveitando o contexto V8, mantendo o ISOLAMENTO por
     * arquivo (o vitest é quem recomenda esta rota; `isolate: false` também
     * resolve o tempo mas dividi o ambiente entre arquivos — não quisemos
     * isso). Medição antes/depois no docs/INDEX.md §6. */
    pool: "vmThreads",
    include: ["tests/**/*.test.{ts,tsx}"],
    /* GUARDA DO act(...) (#142): o setup falha o teste quando o React avisa
     * que um update de estado aconteceu fora do act; o aviso e sintoma de
     * corrida assincrona real, nao ruido para silenciar. */
    setupFiles: ["tests/setup.ts"],
    /* 15s: sob instrumentação de coverage o mount do App (99 presets +
     * boot simulado em lotes) passa de 5s — piso para o gate não piscar. */
    testTimeout: 15_000,
    /* TETO DE WORKERS (#79): sem isto o vitest abre um worker por ARQUIVO e
     * cada um paga ~9s de spawn + ambiente jsdom. Quando este teto foi
     * escrito a suíte tinha 15 arquivos; em 05/10/2026 ela tem 28, e o
     * sintoma original VOLTOU: a rodada de `test:coverage` cai com UM
     * `testTimeout` que troca de arquivo a cada execução (i18n.test.tsx numa,
     * shortcuts.test.tsx na outra) — se a falha fosse real, ela ficaria
     * parada no mesmo lugar. 4 é o paralelismo dos runners do CI
     * (ubuntu-24.04, 4 cores), então local e CI passam a ter a mesma carga.
     *
     * Subir este número economiza tempo até o ponto em que a suíte fica
     * imprevisível; aqui a economia é ESTABILIDADE. **Se você está lendo isto
     * por causa de um `testTimeout` intermitente, a resposta não é subir o
     * `testTimeout`** — é remedir: a suíte cresceu de novo. */
    maxWorkers: 4,
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
      /* Honesto por construção: main.tsx é bootstrap sem lógica; as folhas de
       * estilo não são código (o `design.css` já estava, o `looper.css`
       * entrava com 0% e só poluía o relatório — #79); e o VU é coberto por
       * e2e (rAF por frame, jsdom sem rAF estável). Tudo que TEM lógica
       * testável fica incluído. */
      exclude: [
        "src/main.tsx",
        "src/design/design.css",
        "src/design/looper.css",
      ],
      thresholds: {
        statements: 85,
        functions: 85,
        lines: 85,
        /* Branches: medido, sem gate — ganho passo a passo; mapa dos furos
         * está em docs/ROADMAP.md (V-7, mapa de edge cases de IPC). */
        branches: 0,

        /* TETO POR ARQUIVO (#79 parte 2). O agregado de 85% escondia a
         * distribuição: um arquivo a 69% (TunerPanel) e outro a 100% davam
         * 91% no agregado, e o buraco ficava invisível no relatório.
         *
         * O branch é cobrado só onde há LÓGICA: nos dicionários de i18n
         * (`facts.ts`/`es.ts` a 50% de branch) o branch mede regra de plural
         * e fallback de idioma, não caminho de código esquecido — e um teto
         * ali seria teatro. `effects.ts` entra no agregado só.
         */
        "src/components/**": { statements: 75, branches: 75 },
        "src/hooks/**": { statements: 75, branches: 75 },
        "src/ipc/**": { statements: 75, branches: 70 },
        "src/tuner/**": { statements: 75, branches: 75 },
        "src/design/**": { statements: 85, branches: 85 },
      },
    },
  },
});
