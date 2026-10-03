#!/usr/bin/env python3
"""
Gate de empacotamento (#27/#28/#29): a config do Tauri, os ícones e o job que
gera os instaladores precisam concordar — e nada disso pode apodrecer em
silêncio entre um tag e outro.

O QUE ESTE ARQUIVO PEGA (e por quê cada um já aconteceu ou é plausível):

- campo inexistente na `tauri.conf.json`: o bundler do Tauri só reclama no
  build da tag, que é a hora mais cara possível para descobrir;
- `bundle.icon` apontando para arquivo que não existe: o `.icns` faltando
  quebra o `dmg` no macOS e o `.ico` curto (só 32x32) quebra o instalador
  do Windows;
- `targets` da config sem o bundle que o `Distribuição · instalador` pede: o job
  passa `--bundles`, que SOBRESCREVE a config, então a divergência só aparece
  quando alguém builda local sem o `--bundles`;
- `.deb` sem `libwebkit2gtk-4.1-0`: o pacote instala "limpo" e o app não abre.

A validação contra o schema do Tauri usa a rede (o schema não é versionado
aqui). Sem rede, vira AVISO e o resto dos checks segue — um gate que reprova
por falta de internet treina a gente a ignorar o vermelho.

Uso: python3 scripts/check_bundle.py
"""
from __future__ import annotations

import json
import re
import os
import sys
import urllib.request
from pathlib import Path

CONF = Path("packages/app/api/tauri.conf.json")
WORKFLOW = Path(".github/workflows/ci.yml")
SCHEMA_URL = "https://schema.tauri.app/config/2"

# `app` não é um alvo declarável em `targets`: é o bundle cru, que o Tauri
# sempre produz. Logo, um token `--bundles` dele não é esperado na lista.
IMPLICIT_BUNDLES = {"app", "all"}

FAILURES: list[str] = []
WARNINGS: list[str] = []


def fail(msg: str) -> None:
    FAILURES.append(msg)


def warn(msg: str) -> None:
    WARNINGS.append(msg)


def fetch_schema() -> dict | None:
    req = urllib.request.Request(
        SCHEMA_URL, headers={"User-Agent": "gp100-check-bundle", "Accept": "application/json"}
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return json.load(r)
    except Exception as e:
        warn(f"schema do Tauri indisponivel ({type(e).__name__}) — validacao estrutural pulada")
        return None


def check_icons(conf: dict, root: Path) -> None:
    bundle = conf.get("bundle") or {}
    icons = bundle.get("icon") or []
    if not icons:
        fail("bundle.icon vazio — nenhum instalador sai sem ícone")
    for rel in icons:
        p = root / rel
        if not p.is_file():
            fail(f"bundle.icon aponta para arquivo inexistente: {rel}")

    # O `.ico` do Windows precisa de várias resoluções: o ícone que existia era
    # 32x32 único, o que produz instalador serrilhado (ou erro) em 128+.
    ico = root / "icons" / "icon.ico"
    if ico.is_file():
        data = ico.read_bytes()
        count = int.from_bytes(data[4:6], "little")
        sizes = set()
        for i in range(count):
            off = 6 + i * 16
            sizes.add((data[off] or 256, data[off + 1] or 256))
        required = {(16, 16), (32, 32), (48, 48), (256, 256)}
        missing = required - sizes
        if missing:
            fail(
                "icons/icon.ico sem as resolucoes "
                + ", ".join(f"{w}x{h}" for w, h in sorted(missing))
                + f" (tem {sorted(sizes)})"
            )

    for rel in ("icons/icon.icns", "icons/128x128.png", "icons/128x128@2x.png"):
        if not (root / rel).is_file():
            fail(f"falta {rel} — exigido pelo Tauri v2 (macOS/Linux/Windows)")

    # Integridade do PNG, em Python puro (o CI não tem Pillow).
    # Já aconteceu: uma passagem de "normalizar fim de linha" sobre os icones
    # trocou \r\n por \n DENTRO dos .png e os 16 arquivos viraram lixo — o
    # build do macOS só reclama no `dmg`, muito depois. Assinatura no começo e
    # o trailer do chunk IEND no fim fecham a conta.
    png_sig = b"\x89PNG\r\n\x1a\n"
    png_iend = b"\x00\x00\x00\x00IEND\xaeB`\x82"
    for p in sorted((root / "icons").glob("*.png")):
        data = p.read_bytes()
        if not data.startswith(png_sig):
            fail(f"{p.name} não tem a assinatura PNG — arquivo corrompido")
        elif not data.rstrip(b"\r\n").endswith(png_iend):
            fail(f"{p.name} não termina com o chunk IEND — arquivo corrompido (ou CRLF dentro do binário)")


def check_deb(conf: dict) -> None:
    deb = ((conf.get("bundle") or {}).get("linux") or {}).get("deb") or {}
    depends = deb.get("depends") or []
    # O Tauri tem default (libwebkit2gtk-4.1-0, libgtk-3-0), mas declarar
    # `depends` SUBSTITUI o default. Sem esta lista, o .deb sai sem as libs.
    for lib in ("libwebkit2gtk-4.1-0", "libgtk-3-0"):
        if not any(d.startswith(lib) for d in depends):
            fail(f"linux.deb.depends sem `{lib}` — o pacote instala e o app nao abre")


def check_targets_vs_workflow(conf: dict) -> None:
    targets = set((conf.get("bundle") or {}).get("targets") or [])
    if not targets:
        fail("bundle.targets vazio — `tauri build` local nao gera nada")
        return
    if not WORKFLOW.is_file():
        return
    text = WORKFLOW.read_text(encoding="utf-8")
    asked: set[str] = set()
    for m in re.finditer(r"--bundles\s+([a-z0-9_,]+)", text):
        asked |= {t.strip() for t in m.group(1).split(",") if t.strip()}
    for token in sorted(asked - IMPLICIT_BUNDLES):
        if token not in targets:
            fail(
                f"o job `Distribuição · instalador` pede `--bundles {token}` mas "
                f"bundle.targets = {sorted(targets)} — o `--bundles` sobrescreve a "
                "config, entao o build local sem o flag sai incompleto"
            )


def check_arch(conf: dict, root: Path) -> None:
    """O PKGBUILD (#29) mora fora do Tauri, então nada o liga à config por
    padrão: o nome do binário e o caminho dos ícones saem de lá e o daemon
    update do Arch aponta para `/usr/bin/<binário>` — divergir os dois produz
    um pacote que instala e não abre, ou um item de menu que não abre nada."""
    pkgbuild = Path("packaging/arch/PKGBUILD")
    if not pkgbuild.is_file():
        fail("packaging/arch/PKGBUILD ausente (#29 REL-ARCH)")
        return
    text = pkgbuild.read_text(encoding="utf-8")

    main_bin = conf.get("mainBinaryName") or ""
    m = re.search(r"^pkgname=(\S+)", text, re.M)
    pkg_name = m.group(1).strip("'\"") if m else ""
    if not pkg_name:
        fail("PKGBUILD sem `pkgname=`")
    elif main_bin and pkg_name != main_bin:
        fail(
            f"PKGBUILD `pkgname={pkg_name}` != `mainBinaryName` ({main_bin!r}) — "
            f"/usr/bin/{pkg_name} nao existe, porque o build do Tauri produz `{main_bin}`"
        )
    if main_bin and not re.search(rf"target/release/{re.escape(main_bin)}\b", text):
        fail(f"PKGBUILD nao compila/copia o binario `{main_bin}` do build do Tauri")
    desktop = Path("packaging/arch/gp100-nextgen-editor.desktop")
    if not desktop.is_file():
        fail("packaging/arch/gp100-nextgen-editor.desktop ausente — o PKGBUILD instala")
    elif main_bin:
        dtext = desktop.read_text(encoding="utf-8")
        if f"Exec={main_bin}" not in dtext:
            fail(f"o .desktop tem Exec= diferente de `mainBinaryName` ({main_bin})")

    # todo caminho de origem citado no package() precisa existir.
    # O PKGBUILD usa continuacao de linha (`install -Dm755 \\\n  origem \\\n  destino`),
    # entao o texto e normalizado para uma linha so ANTES do casamento: sem isso
    # o `[^\n]` parava no `\\` da primeira linha e tomava a BARRA como caminho
    # de origem — "PKGBUILD instala `\\`". Defeito real do gate, escondido
    # enquanto ele nao rodava (#71); a CI revelou no primeiro PR que o ligou.
    texto_plano = re.sub(r"\\\s*\n\s*", " ", text)
    for m in re.finditer(
        r"install -Dm\d+ (?:\S+ )*?(\S+) \"?\$\{pkgdir\}", texto_plano
    ):
        src = m.group(1)
        if src.startswith("$"):
            continue
        # O build do Tauri produz o binario; ele nao existe no repo ate compilar,
        # entao `target/release/<mainBinaryName>` e esperado, nao um erro.
        if src.startswith("target/release/"):
            continue
        if not (Path(src).is_file() or Path(src).is_dir()):
            fail(f"PKGBUILD instala `{src}`, que nao existe no repositorio")

    for lib in ("webkit2gtk-4.1", "gtk3"):
        if lib not in text:
            fail(f"PKGBUILD sem a dependencia `{lib}` — mesmo par do .deb")

    # ── #72: o INSRUMO do build() tem de existir e ser produzido por alguem ──
    # O `build()` compila de `gp100-nextgen-editor-<versão>.tar.gz`. Ate #72 esse
    # arquivo nao era publicado por NENHUM job, e o tarball do git nao tinha nem
    # o `frontendDist` nem o `packages/core` (dependencia de caminho do crate
    # do Tauri) — tres furos que deixavam o `makepkg` quebrar em silencio.
    fonte = next(
        (l for l in text.splitlines() if l.startswith("source=")),
        "",
    )
    if "$pkgname-$_pkgver.tar.gz" not in fonte:
        fail('PKGBUILD nao declara `source=("$pkgname-$_pkgver.tar.gz")`')

    sdist = Path("scripts/make_sdist.py")
    if not sdist.is_file():
        fail(
            "PKGBUILD consome um tarball que ninguem produz: "
            "`scripts/make_sdist.py` ausente (#72)"
        )
        return

    workflow = Path(".github/workflows/ci.yml").read_text(encoding="utf-8")
    if "make_sdist.py" not in workflow:
        fail(
            "nenhum job do CI invoca `scripts/make_sdist.py` — o tarball do "
            "PKGBUILD nunca chega a Release (#72)"
        )

    sd = sdist.read_text(encoding="utf-8")

    # O `frontendDist` do conf e RELATIVO ao diretorio do conf, entao
    # `../ui/dist` (em packages/app/api) resolve para `packages/app/ui/dist`.
    # Comparar o caminho resolvido — e nao a substring "packages/app/ui/dist" —
    # e o que torna o contrato honesto: uma substring casaria com o texto de um
    # comentario ou de uma mensagem de erro mesmo com o caminho errado.
    frontend_dist = (conf.get("build") or {}).get("frontendDist") or ""
    if frontend_dist:
        resolvido = os.path.normpath(
            os.path.join("packages/app/api", frontend_dist)
        ).replace(os.sep, "/")
        m_fr = re.search(r'^FRONT_DIST\s*=\s*"([^"]+)"', sd, re.M)
        no_sdist = m_fr.group(1) if m_fr else None
        if no_sdist != resolvido:
            fail(
                f"`frontendDist` do conf resolve para `{resolvido}` mas "
                f"`make_sdist.py` embute `{no_sdist}` — o build do shell entra "
                "em panic do `generate_context!` (#72)"
            )
    if '"packages/core"' not in sd:
        fail(
            '`make_sdist.py` nao embute `packages/core` — e dependencia de '
            'caminho do gp100-ui (path = "../../core"); sem ela o manifesto '
            "nem resolve (#72)"
        )



def check_schema(conf: dict) -> None:
    schema = fetch_schema()
    if not schema:
        return
    try:
        import jsonschema  # opcional
    except ImportError:
        warn("jsonschema nao instalado — validacao estrutural pulada (uv run --with jsonschema)")
        return
    errors = list(jsonschema.Draft7Validator(schema).iter_errors(conf))
    for e in errors[:10]:
        path = "/".join(map(str, e.absolute_path)) or "(raiz)"
        fail(f"tauri.conf.json invalido em /{path}: {e.message[:160]}")


def main() -> int:
    if not CONF.is_file():
        print(f"ERRO {CONF} nao existe")
        return 1
    root = CONF.parent
    conf = json.loads(CONF.read_text(encoding="utf-8"))

    check_icons(conf, root)
    check_deb(conf)
    check_targets_vs_workflow(conf)
    check_arch(conf, root)
    check_schema(conf)

    for w in WARNINGS:
        print(f"AVISO {w}")
    if FAILURES:
        print("\nFALHAS:")
        for f in FAILURES:
            print(f" - {f}")
        return 1
    targets = (conf.get("bundle") or {}).get("targets") or []
    print(
        f"tauri.conf.json + {len((conf.get('bundle') or {}).get('icon') or [])} icones "
        f"+ job de dist em acordo (targets: {', '.join(targets)})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
