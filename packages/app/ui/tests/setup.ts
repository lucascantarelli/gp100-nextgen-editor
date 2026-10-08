/**
 * Guarda do `act(...)` (issue #142).
 *
 * O React avisa por `console.error` quando um update de estado acontece FORA
 * do `act(...)`: o teste passa, mas o aviso costuma esconder corrida
 * assíncrona real que o teste não observa — e, com a suíte grande, o ruído
 * mascara aviso NOVO (a auditoria de 07/10 contou 44).
 *
 * Aqui o aviso deixa de ser ruído e vira FALHA, em dois lugares:
 *  1. no teste em que ele aconteceu (`afterEach`);
 *  2. nos que vazaram ENTRE testes — um timer/promessa pendente de um teste
 *     que já acabou disparando durante o próximo (é o caso que a suíte
 *     completa expôs: o aviso aparecia atribuído a um teste de IPC puro, que
 *     nem renderiza React). Esses são cobrados no fim do arquivo (`afterAll`),
 *     porque não pertencem a nenhum teste específico.
 *
 * A correção é sempre no disparo — envolver em `act()`/`await act()` (ou
 * esperar com um helper que já envolve), nunca silenciar o console.
 */
import { afterAll, afterEach, beforeEach, expect, vi } from "vitest";

const AVISO = "not wrapped in act";

let avisos: string[] = [];
const vazados: string[] = [];
let dentroDeTeste = false;

beforeEach(() => {
  avisos = [];
  dentroDeTeste = true;
  const original = console.error;
  vi.spyOn(console, "error").mockImplementation((...args: unknown[]) => {
    const primeiro = typeof args[0] === "string" ? args[0] : "";
    if (primeiro.includes(AVISO)) {
      if (dentroDeTeste) avisos.push(primeiro);
      else vazados.push(primeiro);
    }
    // O console segue funcionando: erro de verdade continua aparecendo.
    original(...(args as Parameters<typeof console.error>));
  });
});

afterEach(() => {
  dentroDeTeste = false;
  vi.restoreAllMocks();
  expect(
    avisos,
    `update de estado fora do act(...) — envolva o disparo em act() (issue #142). Avisos: ${avisos.length}`,
  ).toEqual([]);
});

afterAll(() => {
  expect(
    vazados,
    `update fora do act vazou ENTRE testes (timer/promessa de um teste anterior) — ` +
      `cancele o que ficou pendente no teardown (issue #142). Vazamentos: ${vazados.length}`,
  ).toEqual([]);
});
