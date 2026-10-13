#!/usr/bin/env node
// Wrapper do `beforeDevCommand` (#193): o Tauri CLI aborta o `tauri dev` quando
// o comando sai com código ≠ 0, e um 2º vite recusa a 5173 (`strictPort: true`
// no vite.config) — com um dev server UI-only já vivo, o `tauri dev` morria.
// O CLI não tem flag para pular o comando quando a devUrl já responde.
//
// Este script só dispara o dev server quando a `build.devUrl` do
// tauri.conf.json NÃO responde; com servidor vivo ele sai 0 e o Tauri segue
// para o build, reusando o vite que já está no ar. `strictPort` não muda.
import { readFileSync } from "node:fs";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const aqui = path.dirname(fileURLToPath(import.meta.url));
const conf = JSON.parse(
  readFileSync(path.resolve(aqui, "..", "api", "tauri.conf.json"), "utf8"),
);
const devUrl = conf?.build?.devUrl;
if (!devUrl) {
  console.error("[dev-if-needed] build.devUrl ausente no tauri.conf.json");
  process.exit(1);
}

/** Qualquer resposta HTTP (até 404) prova que há servidor na devUrl. */
async function responde(url, timeoutMs = 2000) {
  const ctrl = new AbortController();
  const t = setTimeout(() => ctrl.abort(), timeoutMs);
  try {
    await fetch(url, { signal: ctrl.signal });
    return true;
  } catch {
    return false;
  } finally {
    clearTimeout(t);
  }
}

if (await responde(devUrl)) {
  console.log(
    `[dev-if-needed] ${devUrl} já responde — reusando o dev server (exit 0)`,
  );
  process.exit(0);
}

console.log(`[dev-if-needed] ${devUrl} mudo — subindo o dev server (pnpm dev)`);
// Comando em string única (shell: true concatena args — DEP0190); o shell
// também resolve o pnpm.cmd do Windows sem path de binário.
const filho = spawn("pnpm dev", {
  cwd: aqui,
  stdio: "inherit",
  shell: true,
});
filho.on("exit", (codigo, sinal) => process.exit(codigo ?? (sinal ? 1 : 0)));
