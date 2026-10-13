#!/usr/bin/env python3
"""
Export orfao: simbolo exportado em `src/` que NENHUM outro arquivo de
`src/`, `tests/` ou `e2e/` referencia.

Por que este arquivo existe: cobertura 100% em `src/design/tokens.ts` e
`src/artifacts/drumData.ts` criava a impressao de cobertura total, mas
cobertura mede que a linha EXECUTOU, nao que o contrato e consumivel. Um
arquivo 100% coberto cujos exports ninguem chama e o pior dos dois: parece
testado, nao e usado (achado #78).

Duas classes, e elas pedem correcoes diferentes:
  - `export` sem nenhum consumidor -> superficie de API inflada. Tira-se o
    `export`, ou justifica-se em comentario (o gate aceita as justificadas).
  - valor literal que DEVERIA ser derivado (um total, uma contagem) -> o
    numero e certo hoje e silenciosamente errado amanha. Deriva-se do dado.

Regras de precisao (o erro aqui seria acusar codigo inocente):
  - `export default` conta como nome, mas nao e interestingo para orfao;
  - re-export (`export { x } from`) e `export * from` sao pulados (nao declaram
    nada local);
  - um simbolo usado SO dentro do proprio arquivo e `exportado` e orfao de
    `export` (nao de codigo): o codigo e usado, a superficie nao;
  - `--allowlist` em `DEADCODE_ALLOWLIST` marca o simbolo como justificado,
    com o motivo obrigatorio (justificativa sem motivo e silenciosa).

Uso: python3 scripts/check_deadcode.py   (exit 1 na primeira violacao)
     python3 scripts/check_deadcode.py --list   (so imprime)
"""
from __future__ import annotations

import os
import re
import sys
from typing import NamedTuple

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
UI = os.path.join(REPO, "packages", "app", "ui")

# Raizes varridas: quem pode CONSUMIR um export de src/.
CONSUMIDORES = ("src", "tests", "e2e")

# Export justificado: simbolo -> motivo. Vazio = nada e justificado.
DEADCODE_ALLOWLIST: dict[str, str] = {}

# `export const X` / `export function X` / `export class X` / `export enum X`
DECL = re.compile(
    r"^export\s+(?:declare\s+)?"
    r"(?:const|let|var|function|class|enum|abstract\s+class)\s+([A-Za-z_$][\w$]*)",
    re.M,
)
# `export type X` / `export interface X`
DECL_TIPO = re.compile(r"^export\s+(?:type|interface)\s+([A-Za-z_$][\w$]*)", re.M)
# Re-exports: nao declaram simbolo local, entao nao sao orfaos.
REEXPORT = re.compile(r"^export\s+(?:\*|\{[^}]*\})\s*(?:as\s+\w+\s*)?from\b", re.M)
IDENT = re.compile(r"[A-Za-z_$][\w$]*")

SKIP_DIRS = {"node_modules", "dist", "coverage", ".git", "playwright-report", "test-results"}


class Achado(NamedTuple):
    """Um export de src/ que ninguem consome."""
    arquivo: str
    simbolo: str
    tipo: str
    uso_interno: bool


def arquivos() -> list[str]:
    """Todos os .ts/.tsx sob as raizes de consumidor, menos os ignorados."""
    achados: list[str] = []
    for raiz in CONSUMIDORES:
        base = os.path.join(UI, raiz)
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
            for nome in filenames:
                if nome.endswith((".ts", ".tsx")):
                    achados.append(os.path.join(dirpath, nome))
    return sorted(achados)


def rel(caminho: str) -> str:
    return os.path.relpath(caminho, UI).replace("\\", "/")


def le(caminho: str) -> str:
    with open(caminho, encoding="utf-8") as fh:
        return fh.read()


def main() -> int:
    todos = arquivos()
    textos = {p: le(p) for p in todos}
    apenas_src = [p for p in todos if rel(p).startswith("src/")]

    # 1. Coleta os simbolos declarados como `export` dentro de src/.
    declarados: dict[str, tuple[str, str, str]] = {}  # simbolo -> (arquivo, tipo, texto)
    for p in apenas_src:
        txt = textos[p]
        # Remove as linhas de re-export antes de casar declaracoes.
        limpo = REEXPORT.sub("", txt)
        for m in DECL.finditer(limpo):
            declarados.setdefault(m.group(1), (rel(p), "valor", txt))
        for m in DECL_TIPO.finditer(limpo):
            declarados.setdefault(m.group(1), (rel(p), "tipo", txt))

    # 2. Conta referencias FORA do arquivo que declara.
    externos: dict[str, int] = {}
    for simbolo in declarados:
        alvo = declarados[simbolo][0]
        # `export default` nao einterestingo para orfao: quem importa leva o
        # valor, nao o nome. Fora por construcao.
        if simbolo == "default":
            externos[simbolo] = 1
            continue
        padrao = rf"\b{re.escape(simbolo)}\b"
        externos[simbolo] = sum(
            len(re.findall(padrao, textos[p])) for p in todos if rel(p) != alvo
        )

    # 3. Consumo interno: o simbolo e usado no proprio arquivo alem da declaracao?
    orphans: list[Achado] = []
    for simbolo, (arquivo, tipo, txt) in sorted(declarados.items()):
        if externos[simbolo] > 0:
            continue
        if simbolo in DEADCODE_ALLOWLIST:
            continue
        # remove a propria linha de declaracao antes de contar uso interno
        sem_decl = DECL.sub("", txt)
        sem_decl = DECL_TIPO.sub("", sem_decl)
        uso_interno = bool(re.search(rf"\b{re.escape(simbolo)}\b", sem_decl))
        orphans.append((arquivo, simbolo, tipo, uso_interno))

    modo_lista = "--list" in sys.argv

    for arquivo, simbolo, tipo, uso_interno in orphans:
        if uso_interno:
            motivo = "so neste arquivo -> o codigo e usado, o `export` nao e"
        else:
            motivo = "nenhuma referencia em src/, tests/ ou e2e/"
        marca = "ORFAO"
        if modo_lista:
            print(f"{marca:5} {arquivo:38} {simbolo:26} {motivo}")
        else:
            print(f"{marca:5} {arquivo}: '{simbolo}' sem consumidor — {motivo}")

    if orphans and not modo_lista:
        print("\nFALHAS:")
        for arquivo, simbolo, tipo, uso_interno in orphans:
            print(f" - {arquivo}: {simbolo} ({tipo})")
        print(
            "\nCorrecao por classe:\n"
            "  * valor literal que devia ser derivado -> derive do dado;\n"
            "  * tipo/constante de uso interno -> tire o `export`;\n"
            "  * API publica real -> justifique em DEADCODE_ALLOWLIST com o motivo."
        )
        return 1

    print(
        f"\n{len(declarados)} exports em src/, {len(orphans)} orfaos"
        f"{' (--list)' if modo_lista else ''}: superficie de API justificada."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
