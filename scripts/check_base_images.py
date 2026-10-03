#!/usr/bin/env python3
"""
Confere se TODA imagem base citada pelos Dockerfiles do CI existe no registro.

Por que isso e um script e nao o build: o build so descobre a tag invalida
DEPOIS de subir as camadas anteriores, e a falha sai como
`docker.io/library/rust:bookworm-slim: not found` no meio de um log de build,
sem dizer que o problema era uma tag montada por concatenacao. Foi
exatamente o que travou o bootstrap da `ci-base`: um ARG `DEBIAN_VERSION`
com default `bookworm-slim` remontado em `FROM rust:${DEBIAN_VERSION}`, quando
a tag oficial e `slim-bookworm` (sufixo depois). O `7 ci · imagens` gastava o
runner inteiro para falhar em 3s e o consome morria depois com
`manifest unknown`, sem nenhuma ligacao com a causa.

Aqui o custo e um HEAD por imagem, no comeco do job, e a falha diz o arquivo, a
linha e a tag.

Uso: python3 scripts/check_base_images.py [<dockerfile> ...]
"""
from __future__ import annotations

import json
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path

DOCKERFILES = sorted(Path(".github/docker").glob("*/Dockerfile"))

# `FROM x AS y`, `FROM x`, e `COPY --from=x` (imagem estage de outro registro).
RE_FROM = re.compile(r"^\s*FROM\s+(\S+)", re.IGNORECASE | re.MULTILINE)
RE_COPY_FROM = re.compile(r"^\s*COPY\s+--from=(\S+)", re.IGNORECASE | re.MULTILINE)
RE_ARG = re.compile(r"^\s*ARG\s+([A-Za-z_][A-Za-z0-9_]*)(?:=(.*))?\s*$")
RE_VAR = re.compile(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}")

# Nome de stage declarado no proprio arquivo (`FROM golang:1.22 AS build`) nao
# vem do registro.
RE_STAGE_AS = re.compile(r"^\s*FROM\s+\S+\s+AS\s+(\S+)", re.IGNORECASE | re.MULTILINE)

ACCEPT = ",".join(
    [
        "application/vnd.oci.image.index.v1+json",
        "application/vnd.oci.image.manifest.v1+json",
        "application/vnd.docker.distribution.manifest.list.v2+json",
        "application/vnd.docker.distribution.manifest.v2+json",
    ]
)


def split_ref(ref: str) -> tuple[str, str]:
    """devolve (host/repo, tag) -- default tag `latest`."""
    if "/" not in ref.split(":")[0]:
        ref = f"library/{ref}"  # imagem oficial do Docker Hub
    name, _, tag = ref.rpartition(":")
    if not name or "/" in tag:
        name, tag = ref, "latest"
    return name, tag


def hub_token(repo: str) -> str:
    url = (
        "https://auth.docker.io/token?service=registry.docker.io"
        f"&scope=repository:{repo}:pull"
    )
    with urllib.request.urlopen(url, timeout=30) as r:
        return json.load(r)["token"]


def ghcr_token(repo: str) -> str:
    url = f"https://ghcr.io/token?scope=repository:{repo}:pull"
    with urllib.request.urlopen(url, timeout=30) as r:
        return json.load(r)["token"]


def probe(ref: str) -> tuple[bool, str]:
    name, tag = split_ref(ref)
    if name.startswith("ghcr.io/"):
        host, repo = "ghcr.io", name[len("ghcr.io/") :]
        url = f"https://ghcr.io/v2/{repo}/manifests/{tag}"
        get_token = ghcr_token
    else:
        repo = name
        url = f"https://registry-1.docker.io/v2/{repo}/manifests/{tag}"
        get_token = hub_token
    req = urllib.request.Request(
        url, headers={"Authorization": f"Bearer {get_token(repo)}", "Accept": ACCEPT}
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return r.status == 200, f"HTTP {r.status}"
    except urllib.error.HTTPError as e:
        return False, f"HTTP {e.code}"
    except Exception as e:  # rede caiu: NAO e prova de que a tag nao existe
        return False, f"indisponivel ({type(e).__name__})"


def args_of(text: str) -> dict[str, str]:
    """Default de cada `ARG NOME=valor`. Sem isso `FROM ${RUST_IMAGE}` vira
    `${RUST_IMAGE}` no probe e a tag real nunca e conferida -- que e como a tag
    invalida passa batida."""
    out: dict[str, str] = {}
    for line in text.splitlines():
        if line.lstrip().startswith("#"):
            continue
        m = RE_ARG.match(line)
        if m and m.group(2) is not None:
            out[m.group(1)] = m.group(2).strip().strip('"').strip("'")
    return out


def expand(ref: str, env: dict[str, str]) -> str:
    for _ in range(5):  # ${A:-${B}} simplificado: nao ha aninhamento real aqui
        new = RE_VAR.sub(lambda m: env.get(m.group(1), m.group(0)), ref)
        if new == ref:
            break
        ref = new
    return ref


def refs_of(path: Path) -> list[tuple[int, str]]:
    text = path.read_text(encoding="utf-8")
    env = args_of(text)
    stages = {m.group(1).lower() for m in RE_STAGE_AS.finditer(text)}
    out: list[tuple[int, str]] = []
    for i, line in enumerate(text.splitlines(), start=1):
        if line.lstrip().startswith("#"):
            continue
        for rx in (RE_FROM, RE_COPY_FROM):
            m = rx.match(line)
            if m:
                ref = expand(m.group(1), env)
                if "${" in ref:
                    print(f"AVISO {path}:{i} `{m.group(1)}` ficou com ARG sem default")
                    continue
                if ref.lower() not in stages:
                    out.append((i, ref))
    return out


def main(argv: list[str]) -> int:
    paths = [Path(a) for a in argv] if argv else DOCKERFILES
    if not paths:
        print("nenhum Dockerfile encontrado")
        return 0

    failures: list[str] = []
    unknown: list[str] = []
    for path in paths:
        refs = refs_of(path)
        if not refs:
            print(f"OK   {path} (sem imagem externa)")
            continue
        print(f"--    {path}")
        for lineno, ref in refs:
            ok, why = probe(ref)
            mark = "OK  " if ok else "ERR "
            print(f"  {mark}L{lineno}: {ref} -> {why}")
            if ok:
                continue
            if why.startswith("indisponivel"):
                # Rede/registro fora do ar: nao reprova o build por speculacao.
                unknown.append(f"{path}:{lineno} {ref} ({why})")
            else:
                failures.append(f"{path}:{lineno} `{ref}` nao existe no registro ({why})")

    for u in unknown:
        print(f"AVISO nao verificado (registro indisponivel): {u}")
    if failures:
        print("\nFALHAS:")
        for f in failures:
            print(f" - {f}")
        print(
            "\nTags do Docker Hub oficial sao `sufixo-suite` (`slim-bookworm`), "
            "nao `suite-sufixo` (`bookworm-slim`). Verifique a tag no registro."
        )
        return 1
    print("\ntodas as imagens base existem no registro")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
