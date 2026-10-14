#!/usr/bin/env python3
"""sync_version.py — uma versao, cinco manifests, zero divergencia.

POR QUE ESTE SCRIPT EXISTE. A cadeia de release (`.github/actions/version-bump`
e o job `Release · RC`) bumpeava tres manifests: `Cargo.toml` (raiz),
`packages/app/api/Cargo.toml` e `version.json`. Ficavam de fora dois:

- **`packages/app/api/tauri.conf.json`** — e o que o Tauri usa como versao do
  BUNDLE. Sem bump, cortar `v0.2.0` produz um `.msi`/`.dmg`/`.deb`/`.AppImage`
  que se apresenta como `0.1.0` (#73). O risco estava escrito no
  `docs/planejamento/RELEASE_PLAN.md:26` desde o plano de release e nunca foi tratado.
- **`packages/app/ui/package.json`** — o front embutido no app.

Por que nao bastava `sed`: `"version"` aparece em varias posicoes nesses dois
arquivos. No `tauri.conf.json`, um `sed` generico pega a PRIMEIRA ocorrencia
`^version = "..."`-style, que nao e a chave do bundle. Aqui cada manifest e
parseado como a estrutura que ele e (TOML por linha anchors, JSON por
estrutura) e a edicao e conferida.

O `version` tambem aparece nos LOCK FILES, que guardam a versao dos crates de
CAMINHO: bump sem eles deixa a arvore que `--locked` recusa compilar (#231).
O gate cobre os dois lados — os 5 manifests E os pacotes locais dos 2 locks.

Uso:
    python3 .github/scripts/sync_version.py --check          # gate: divergencia = erro
    python3 .github/scripts/sync_version.py --set 0.2.0      # aplica em todos (manifests + locks)
    python3 .github/scripts/sync_version.py                  # sem flag: mostra o estado
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent.parent

# (caminho, modo, ancora) — `ancora` diz qual chave ler/escrever em cada arquivo.
MANIFESTS: list[tuple[str, str, str]] = [
    ("Cargo.toml", "toml-anchor", "version"),
    ("packages/app/api/Cargo.toml", "toml-anchor", "version"),
    ("packages/app/api/tauri.conf.json", "json", "version"),
    ("packages/app/ui/package.json", "json", "version"),
    ("version.json", "json", "version"),
]

RE_ANCHOR = r'^(version\s*=\s*")([^"]+)(")'

# OS LOCK FILES TAMBEM CARREGAM A VERSAO — e o bump sem eles deixa a arvore
# INCONSISTENTE (#231). Os `Cargo.lock` guardam a versao dos crates de CAMINHO
# (`gp100-core`, `gp100-cli`, `gp100-library`, `gp100-ui`); com o lock velho,
# `cargo metadata --locked` e `cargo build --release --locked` recusam atualizar
# o lock e morrem ("cannot update the lock file … because --locked was passed") —
# e sao exatamente esses dois os consumidores de release: o passo `dist-sdist`
# do CI (dentro do tarball) e o `build()` do `packaging/arch/PKGBUILD`.
#
# A reescrita e DE LINHA, e so nos pacotes LOCAIS. Pacote de caminho e o que
# nao tem `source = ...` no lock (terceiro sempre tem) — por isso a lista nao
# precisa ser mantida a mao, e uma dependencia de caminho nova entra sozinha.
LOCKS: list[str] = ["Cargo.lock", "packages/app/api/Cargo.lock"]

RE_PKG_INICIO = re.compile(r"^\[\[package\]\]\s*$", re.M)
RE_PKG_NOME = re.compile(r'^name\s*=\s*"([^"]+)"', re.M)
RE_PKG_VERSAO = re.compile(r'^(version\s*=\s*")[^"]+(")', re.M)
RE_PKG_SOURCE = re.compile(r"^source\s*=", re.M)


def _blocos_de_pacote(texto: str) -> list[tuple[int, int]]:
    """Faixas (inicio, fim) de cada bloco `[[package]]` do lock."""
    marcas = [m.start() for m in RE_PKG_INICIO.finditer(texto)]
    limites = marcas + [len(texto)]
    return [(limites[i], limites[i + 1]) for i in range(len(marcas))]


def locais_do_lock(texto: str) -> dict[str, str]:
    """`nome -> versao` dos pacotes LOCAIS (sem `source`) declarados no lock."""
    locais: dict[str, str] = {}
    for inicio, fim in _blocos_de_pacote(texto):
        bloco = texto[inicio:fim]
        if RE_PKG_SOURCE.search(bloco):
            continue  # terceiro: o bump de versao nao encosta aqui
        nome = RE_PKG_NOME.search(bloco)
        versao = RE_PKG_VERSAO.search(bloco)
        if nome and versao:
            locais[nome.group(1)] = versao.group(0).split('"')[1]
    return locais


def escrever_lock(caminho: Path, versao: str) -> int:
    """Reescreve a versao dos pacotes locais no lock. Devolve quantos mudou.

    Grava em BYTES (`newline` do repo e LF — mesma armadilha do `escrever`).
    """
    texto = caminho.read_bytes().decode("utf-8")
    pedacos: list[str] = []
    anterior = 0
    mudou = 0
    for inicio, fim in _blocos_de_pacote(texto):
        bloco = texto[inicio:fim]
        if not RE_PKG_SOURCE.search(bloco) and RE_PKG_NOME.search(bloco):
            novo_bloco, n = RE_PKG_VERSAO.subn(
                rf"\g<1>{versao}\g<2>", bloco, count=1
            )
            if n == 1 and novo_bloco != bloco:
                mudou += 1
            bloco = novo_bloco
        pedacos.append(texto[anterior:inicio])
        pedacos.append(bloco)
        anterior = fim
    pedacos.append(texto[anterior:])
    caminho.write_bytes("".join(pedacos).encode("utf-8"))
    return mudou


def divergencias_de_lock(caminho: Path, esperada: str) -> list[str]:
    """Pacotes locais do lock que NAO estao na versao esperada (#231)."""
    if not caminho.is_file():
        return [f"{caminho.name}: lock ausente"]
    return [
        f"{caminho.as_posix()}: `{nome}` em {versao} (esperado {esperada})"
        for nome, versao in sorted(locais_do_lock(caminho.read_text(encoding="utf-8")).items())
        if versao != esperada
    ]


def log(msg: str) -> None:
    print(f"[versao] {msg}", file=sys.stdout)


def err(msg: str) -> None:
    print(f"[versao] ERRO: {msg}", file=sys.stderr)
    sys.exit(1)


def ler(caminho: str) -> str | None:
    """Le a versao de um manifest, ou None se ele nao estiver no formato esperado."""
    alvo = ROOT / caminho
    if not alvo.is_file():
        return None
    if caminho.endswith(".json"):
        try:
            return json.loads(alvo.read_text(encoding="utf-8")).get("version")
        except json.JSONDecodeError:
            return None
    m = re.search(RE_ANCHOR, alvo.read_text(encoding="utf-8"), re.M)
    return m.group(2) if m else None


def escrever(caminho: str, versao: str) -> None:
    """Escreve em BYTES-preservando o resto do arquivo.

    O `.gitattributes` do repo exige LF; abrir em modo texto no Windows
    traduziria `\n` -> `\r\n` e sujaria o diff inteiro. Por isso `newline=""`
    no texto e gravacao em bytes — armadilha ja mordida neste repo.
    """
    alvo = ROOT / caminho
    if caminho.endswith(".json"):
        # ⚠️ NAO usar `json.dumps(indent=2)`: ele reescreve o arquivo INTEIRO e
        # desfaz a formatacao compacta que o `tauri.conf.json` usa
        # (`"targets": ["nsis", ...]` viraria uma linha por item). Prova: rodar
        # `--set` com a MESMA versao produzia um diff de 23 linhas em um arquivo
        # que nao tinha mudado de valor. O bump de versao nao pode sujar o diff
        # — o `.gitattributes` exige LF e o repo valoriza diff enxuto.
        texto = alvo.read_bytes().decode("utf-8")
        novo, n = re.subn(
            r'("version"\s*:\s*")[^"]+(")', rf"\g<1>{versao}\g<2>", texto, count=1
        )
        if n != 1:
            err(f"{caminho}: chave `\"version\"` nao encontrada (ou ambigua)")
        alvo.write_bytes(novo.encode("utf-8"))
        return
    texto = alvo.read_bytes().decode("utf-8")
    novo, n = re.subn(RE_ANCHOR, rf"\g<1>{versao}\g<3>", texto, count=1, flags=re.M)
    if n != 1:
        err(f"{caminho}: ancora `version` nao encontrada (ou ambigua)")
    alvo.write_bytes(novo.encode("utf-8"))


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--check", action="store_true", help="falha se divergirem")
    ap.add_argument("--set", dest="alvo", help="versao a aplicar em todos")
    args = ap.parse_args()

    estado = {c: ler(c) for c, _, _ in MANIFESTS}
    ausentes = [c for c, v in estado.items() if v is None]

    if args.alvo:
        versao = args.alvo.lstrip("v")
        # Aceita `X.Y.Z` e `X.Y.Z-rc.N`: a cadeia de release grava a rc no
        # MESMO passo quebumpeia os manifests (`0.2.0-rc.1`), e um validador
        # estrito aqui faria o job de release morrer num passo que funcionava.
        import re as _re

        if not _re.fullmatch(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", versao):
            err(f"versao invalida: {args.alvo!r} (esperado X.Y.Z ou X.Y.Z-rc.N)")
        for caminho, _, _ in MANIFESTS:
            escrever(caminho, versao)
        log(f"versao {versao} aplicada em {len(MANIFESTS)} manifests")
        # Os locks entram no MESMO passo (#231): sem isso a arvore bumpada nao
        # compila `--locked` (tarball do dist-sdist e PKGBUILD do Arch).
        for rel in LOCKS:
            alvo = ROOT / rel
            if not alvo.is_file():
                err(f"lock obrigatorio ausente: {rel}")
            n = escrever_lock(alvo, versao)
            log(f"{rel}: {n} pacote(s) local(is) em {versao}")
        return 0

    for caminho, versao in estado.items():
        marca = "  " if versao else "!!"
        log(f"{marca} {caminho:38} {versao or '<ilegivel>'}")

    distintos = {v for v in estado.values() if v is not None}
    if ausentes:
        err(f"manifesto(s) ausente(s) ou sem `version`: {', '.join(ausentes)}")

    # Os locks tambem sao MANIFESTO da versao (#231): o crate de caminho que
    # ficou para tras quebra `--locked` em silencio ate a distribuicao rodar.
    if len(distintos) == 1:
        esperada = next(iter(distintos))
        for rel in LOCKS:
            alvo = ROOT / rel
            achados = locais_do_lock(
                alvo.read_text(encoding="utf-8")
            ) if alvo.is_file() else {}
            log(
                f"  {rel:38} "
                + ", ".join(f"{k}={v}" for k, v in sorted(achados.items()))
            )
        pendentes = [
            d for rel in LOCKS for d in divergencias_de_lock(ROOT / rel, esperada)
        ]
        if pendentes:
            msg = (
                "LOCK DEFASADO (#231): "
                + "; ".join(pendentes)
                + ". Uma arvore bumpada que nao compila `--locked` quebra o "
                "tarball do dist-sdist e o PKGBUILD do Arch. Rode: "
                "python3 .github/scripts/sync_version.py --set <versao>"
            )
            if args.check:
                err(msg)
            log(msg)
            return 1

    if len(distintos) > 1:
        msg = (
            f"VERSIONES DIVERGENTES: {len(distintos)} valores em "
            f"{len(MANIFESTS)} manifests -> {sorted(distintos)}. "
            "Um instalador com versao errada e o sintoma (#73). "
            "Rode: python3 .github/scripts/sync_version.py --set <versao>"
        )
        if args.check:
            err(msg)
        log(msg)
        return 1

    log(f"todos os {len(MANIFESTS)} manifests em {distintos.pop()}")
    return 0


if __name__ == "__main__":
    sys.exit(main())