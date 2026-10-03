#!/usr/bin/env python3
"""make_sdist.py — tarball de FONTE com o front ja compilado (insumo do PKGBUILD).

POR QUE ESTE SCRIPT EXISTE. O `packaging/arch/PKGBUILD` (#29) compila a partir
de `gp100-nextgen-editor-<versão>.tar.gz`. Dois furos fizeram esse caminho nunca
funcionar, ambos fechados aqui:

1. **O tarball nao era publicado.** O `dist-cli` empacota o BINARIO e o
   `dist-ui` os instaladores, mas ninguem produzia o tarball de fonte que o
   PKGBUILD declara em `source=()`. `makepkg` procuraria um arquivo inexistente.

2. **`ui/dist` nao existe no tarball do git.** `packages/app/ui/dist/` esta no
   `.gitignore` (é produto de build), e o `tauri.conf.json` aponta
   `frontendDist` para ele. O `tauri::generate_context!()` faz `panic!` quando o
   diretorio nao existe (`tauri-codegen`, `context.rs:185-191`) — o build morre
   com um erro de codegen, nao com um erro de leitura util.

Duas saidas foram considered: (a) o PKGBUILD instala o node/npm e compila o
front a cada `makepkg`; (b) o tarball ja embarca o `ui/dist`. Escolhemos (b):

- **Velocidade**: o AUR nao compila a arvore de testes do front a cada build.
- **Reprodutibilidade**: o `ui/dist` que o Arch empacota e BYTE A BYTE o que o
  CI gerou e o `.deb` usa. Um pacote Arch que recompila o Vite pode sair
  diferente do `.deb` da mesma versao — e ai o usuario instala "a mesma
  versao" e recebe binarios diferentes.
- **O `pnpm-lock.yaml` e lockfileVersion 9.0**: o `npm` do Arch nao o le, entao
  `npm ci` nem seria uma opcao honesta. Instalar o `pnpm` so para ler um
  lockfile que o resto do repo nao usa seria introduzir um segundo gerenciador
  na cadeia de release.

O tarball e REPRODUZIVEL: `--sort=name` + mtime fixo + owner numerico, entao o
mesmo commit gera o mesmo sha256. Isso e o que permite `sha256sums` fixo no
PKGBUILD em vez de `SKIP`.

Uso:
    python3 scripts/make_sdist.py --version 0.2.0 --out dist-pkg
    python3 scripts/make_sdist.py --version 0.2.0 --build-front   # compila antes
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import os
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path


# Mtime fixo (2020-01-01T00:00:00Z) + owner/group 0 + ordem estavel: sao os tres
# fatores que mais movem o sha256 de um tar entre duas execucoes identicas.
# Sem isso, `sha256sums` no PKGBUILD teria de ser `SKIP` para sempre.
MTIME_FIXO = 1_577_836_800

# O que entra no tarball. Deliberadamente MINIMO: o PKGBUILD so precisa do
# necessario para `cargo build --release --locked` do shell e para montar o
# pacote. Entrar com `analysis/`, `files/` ou os testes do front incharia o
# tarball e arrastaria material que o `.gitignore` existe para nao distribuir.
INCLUDE_ARQUIVOS = [
    "Cargo.toml",
    "Cargo.lock",
    "README.md",
    "LICENSE",
    "rust-toolchain.toml",
]
INCLUDE_DIRETORIOS = [
    "packages/app/api",
    # `packages/core` e dependencia de CAMINHO do crate do Tauri
    # (`gp100-core = { path = "../../core" }`). Sem ela o `cargo build` morre em
    # "failed to load manifest for dependency gp100-core" ANTES de qualquer outra
    # verificacao -- provado com `cargo metadata --locked` no tarball (achado #72,
    # terceiro furo do mesmo caminho).
    "packages/core",
    "packaging/arch",
]
# O front entra so pelo PRODUTO compilado (`dist/`), nunca pelo fonte.
FRONT_DIST = "packages/app/ui/dist"

# Fonte que o pacote declara e que o AUR nao deve precisar clonar.
EXCLUIR_DE = [
    "target",
    "gen",
    "node_modules",
    "test-results",
    "coverage",
    "__pycache__",
]


def log(msg: str) -> None:
    """`print` em PT-BR sem quebrar o console cp1252 do host Windows."""
    print(f"[sdist] {msg}", file=sys.stdout)


def err(msg: str) -> None:
    print(f"[sdist] ERRO: {msg}", file=sys.stderr)
    sys.exit(1)


def compilar_front(root: Path) -> None:
    """Roda o build do front (pnpm) — o passo que o tarball do git nao traz."""
    ui = root / "packages/app/ui"
    if not (ui / "node_modules").is_dir():
        log("instalando o front (pnpm install --frozen-lockfile)")
        subprocess.run(
            ["pnpm", "install", "--frozen-lockfile"], cwd=ui, check=True
        )
    log("compilando o front (pnpm build)")
    subprocess.run(["pnpm", "run", "build"], cwd=ui, check=True)


def coletar(front_dist: Path) -> dict[str, bytes]:
    """Monta o mapa caminho->bytes do tarball, ja aplicando as exclusoes."""
    arquivos: dict[str, bytes] = {}

    for rel in INCLUDE_ARQUIVOS:
        src = ROOT / rel
        if src.is_file():
            arquivos[rel] = src.read_bytes()
        else:
            log(f"aviso: {rel} ausente, seguindo sem ele")

    for rel in INCLUDE_DIRETORIOS:
        base = ROOT / rel
        if not base.is_dir():
            err(f"diretorio obrigatorio ausente: {rel}")
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = [d for d in dirnames if d not in EXCLUIR_DE]
            for nome in filenames:
                src = Path(dirpath) / nome
                destino = str(src.relative_to(ROOT)).replace(os.sep, "/")
                arquivos[destino] = src.read_bytes()

    if not front_dist.is_dir():
        err(
            f"{FRONT_DIST} ausente — o build do shell morre com panic do "
            "frontendDist. Rode com --build-front, ou faca o build do front antes."
        )
    n_front = 0
    for dirpath, _dirnames, filenames in os.walk(front_dist):
        for nome in filenames:
            src = Path(dirpath) / nome
            destino = str(src.relative_to(ROOT)).replace(os.sep, "/")
            arquivos[destino] = src.read_bytes()
            n_front += 1
    log(f"front embutido: {n_front} arquivos de {FRONT_DIST}")

    return arquivos


def escrever_tarball(destino: Path, arquivos: dict[str, bytes], prefixo: str) -> str:
    """Grava o .tar.gz de forma reproduzivel e devolve o sha256.

    ⚠️ O `tarfile.open(mode="w:gz")` NAO serve: o gzip embutido grava o HORA
    ATUAL no cabecalho, entao dois tarballs identicos saindo de segundos
    diferentes tem sha256 diferente (provado na primeira execucao do script:
    `c6b1bf38…` e `2cb8dda6…` do mesmo commit). Por isso o gzip e instanciado
    explicitamente com `mtime=0` e `filename=""` — o nome do arquivo tambem
    entra no header evaries com a pasta de saida.
    """
    buffer = io.BytesIO()
    with gzip.GzipFile(
        filename="", mode="wb", compresslevel=9, fileobj=buffer, mtime=0
    ) as gz:
        with tarfile.open(fileobj=gz, mode="w") as tar:
            for caminho in sorted(arquivos):  # ordem estavel = hash estavel
                dados = arquivos[caminho]
                info = tarfile.TarInfo(f"{prefixo}/{caminho}")
                info.size = len(dados)
                info.mtime = MTIME_FIXO
                info.mode = 0o755 if caminho.endswith((".sh", ".py")) else 0o644
                info.uid = info.gid = 0
                info.uname = info.gname = "root"
                tar.addfile(info, io.BytesIO(dados))
    bruto = buffer.getvalue()
    destino.parent.mkdir(parents=True, exist_ok=True)
    destino.write_bytes(bruto)
    return hashlib.sha256(bruto).hexdigest()


def main() -> int:
    global ROOT

    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--version", required=True, help="versao sem o v (ex.: 0.2.0)")
    ap.add_argument("--out", default="dist-pkg", help="pasta de saida")
    ap.add_argument(
        "--build-front",
        action="store_true",
        help="roda pnpm install/build antes de empacotar",
    )
    args = ap.parse_args()

    ROOT = Path(__file__).resolve().parent.parent
    versao = args.version.lstrip("v")
    if not all(p.isdigit() for p in versao.split(".")):
        err(f"versao invalida: {args.version!r}")

    front_dist = ROOT / FRONT_DIST
    if args.build_front:
        compilar_front(ROOT)
    elif not front_dist.is_dir():
        err(f"{FRONT_DIST} ausente (use --build-front)")

    arquivos = coletar(front_dist)
    nome = f"gp100-nextgen-editor-{versao}.tar.gz"
    destino = Path(args.out) / nome
    digest = escrever_tarball(destino, arquivos, f"gp100-nextgen-editor-{versao}")

    # O `SHA256SUMS.txt` e o mesmo formato que o job `dist-ui` ja publica.
    (destino.parent / "SHA256SUMS.txt").write_text(
        f"{digest}  {nome}\n", encoding="utf-8", newline="\n"
    )

    log(f"{len(arquivos)} arquivos -> {destino} ({destino.stat().st_size} bytes)")
    log(f"sha256: {digest}")
    log("fixe este digest em packaging/arch/PKGBUILD -> sha256sums=()")
    return 0


ROOT = Path(__file__).resolve().parent.parent

if __name__ == "__main__":
    sys.exit(main())