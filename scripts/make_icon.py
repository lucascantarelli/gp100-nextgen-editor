#!/usr/bin/env python3
"""
Gera a FONTE 1024x1024 do ícone do app, para o `tauri icon` derivar o set
completo (PNG/ICO/ICNS/Store logos).

POR QUE UM SCRIPT E NÃO UM ARQUIVO SOLTO: o set que o Tauri exige é derivado de
uma única fonte. Guardar só o `.png` derivado é perder a procedência — ninguém
sabe de onde saiu a cor, e a próxima pessoa redesenha do zero. As cores aqui
saem dos TOKENS do próprio projeto (`packages/app/ui/src/design/design.css`),
então o ícone não pode divergir do tema: é a paleta "obsidiana violeta" e o
cromo do pedal.

O desenho: pedal de efeito visto de cima — o chassi escuro com gradiente de
superfície, o anel metálico do footswitch e o núcleo violeta aceso. Um pedal
lê a 32x32; o número "100" ou as letras "GP" somem antes.

Uso: python3 scripts/make_icon.py [saida.png]
"""
from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

SIZE = 1024
OUT = Path("packages/app/api/icons/icon-source.png")

# ── paleta: espelha --surface-top/--surface-bottom, --bg, --accent*, --silver
BG_TOP = (36, 28, 54)      # --surface-top  #241c36
BG_BOTTOM = (20, 16, 31)   # --surface-bottom #14101f
BORDER = (46, 36, 66)      # --border #2e2442
VOID = (12, 9, 16)         # --bg #0c0910
ACCENT = (124, 58, 237)    # --accent #7c3aed
GLOW = (147, 51, 234)      # --accent-glow #9333ea
TEXT_ON_ACCENT = (249, 250, 251)  # --on-accent #f9fafb
SILVER = (209, 213, 219)   # --silver #d1d5db
STEEL = (156, 163, 175)    # --steel #9ca3af


def vertical_gradient(w: int, h: int, top: tuple, bottom: tuple) -> Image.Image:
    """Gradiente vertical. Feito por linha (1024 linhas) em vez de por pixel:
    1M de operações de putpixel em Python puro demora ~2s por chamada."""
    img = Image.new("RGB", (1, h))
    px = img.load()
    for y in range(h):
        t = y / max(h - 1, 1)
        px[0, y] = tuple(round(top[i] + (bottom[i] - top[i]) * t) for i in range(3))
    return img.resize((w, h), Image.BILINEAR)


def build() -> Image.Image:
    # chassi: cantos arredondados (--radius-xl = 16/1024), com folga para a
    # sombra não ser cortada pelo limite do PNG
    margin = 64
    body = SIZE - margin * 2
    radius = 180

    face = vertical_gradient(body, body, BG_TOP, BG_BOTTOM).convert("RGBA")

    # luz de cima/esquerda, sombra embaixo/direita (a regra única do projeto, §LUZ)
    sheen = Image.new("L", (body, body), 0)
    ImageDraw.Draw(sheen).ellipse((-body // 2, -body // 3, body, body // 2), fill=110)
    sheen = sheen.filter(ImageFilter.GaussianBlur(120))
    face = Image.composite(
        Image.new("RGBA", (body, body), (255, 255, 255, 255)),
        face,
        sheen.point(lambda v: int(v * 0.28)),
    )
    face.putalpha(
        Image.new("L", (body, body), 0)
    )  # placeholder; a máscara de cantos entra abaixo

    # máscara de cantos arredondados (o gradiente é retangular)
    mask = Image.new("L", (body, body), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, body - 1, body - 1), radius=radius, fill=255)

    # aro/borda de 6px com --border
    ring = Image.new("RGBA", (body, body), BORDER + (255,))
    ring.putalpha(mask)
    inner = Image.new("L", (body, body), 0)
    ImageDraw.Draw(inner).rounded_rectangle(
        (6, 6, body - 7, body - 7), radius=radius - 6, fill=255
    )
    ring.putalpha(Image.composite(Image.new("L", (body, body), 255), mask, inner))
    face.putalpha(inner)

    d = ImageDraw.Draw(face)

    # ── footswitch: aro cromado + cavidade escura + núcleo violeta aceso ──
    c = body // 2
    outer_r = int(body * 0.315)
    ring_w = int(body * 0.052)

    # sombra do footswitch (desce para baixo/direita)
    sh = Image.new("RGBA", (body, body), (0, 0, 0, 0))
    ImageDraw.Draw(sh).ellipse(
        (c - outer_r + 14, c - outer_r + 22, c + outer_r + 14, c + outer_r + 22),
        fill=(0, 0, 0, 120),
    )
    sh = sh.filter(ImageFilter.GaussianBlur(34))
    face.alpha_composite(sh)

    # aro metálico: gradiente diagonal cromo (luz em cima/esquerda)
    metal = vertical_gradient(body, body, SILVER, STEEL).rotate(0)
    metal = metal.convert("RGBA")
    aro = Image.new("L", (body, body), 0)
    ImageDraw.Draw(aro).ellipse(
        (c - outer_r, c - outer_r, c + outer_r, c + outer_r), fill=255
    )
    hole = Image.new("L", (body, body), 0)
    ImageDraw.Draw(hole).ellipse(
        (
            c - outer_r + ring_w,
            c - outer_r + ring_w,
            c + outer_r - ring_w,
            c + outer_r - ring_w,
        ),
        fill=255,
    )
    aro.putalpha(Image.composite(Image.new("L", (body, body), 0), aro, hole))
    face.alpha_composite(Image.composite(metal, Image.new("RGBA", (body, body)), aro))

    # cavidade interna --bg
    inner_r = outer_r - ring_w
    d.ellipse(
        (c - inner_r, c - inner_r, c + inner_r, c + inner_r), fill=VOID + (255,)
    )

    # núcleo violeta aceso (o LED)
    core_r = int(body * 0.145)
    halo = Image.new("RGBA", (body, body), (0, 0, 0, 0))
    ImageDraw.Draw(halo).ellipse(
        (c - core_r * 2, c - core_r * 2, c + core_r * 2, c + core_r * 2),
        fill=GLOW + (150,),
    )
    halo = halo.filter(ImageFilter.GaussianBlur(60))
    face.alpha_composite(halo)

    core = Image.new("RGBA", (body, body), (0, 0, 0, 0))
    ImageDraw.Draw(core).ellipse(
        (c - core_r, c - core_r, c + core_r, c + core_r), fill=ACCENT + (255,)
    )
    face.alpha_composite(core)

    # specular do núcleo (vidro do LED) — 22% de --light-specular, cima/esquerda
    spec = Image.new("RGBA", (body, body), (0, 0, 0, 0))
    ImageDraw.Draw(spec).ellipse(
        (
            c - int(core_r * 0.62),
            c - int(core_r * 0.62),
            c + int(core_r * 0.05),
            c + int(core_r * 0.15),
        ),
        fill=TEXT_ON_ACCENT + (90,),
    )
    spec = spec.filter(ImageFilter.GaussianBlur(14))
    face.alpha_composite(spec)

    # assentar em fundo transparente
    out = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    out.alpha_composite(face, (margin, margin))
    return out


def main(argv: list[str]) -> int:
    out = Path(argv[0]) if argv else OUT
    out.parent.mkdir(parents=True, exist_ok=True)
    img = build()
    img.save(out, "PNG", optimize=True)
    print(f"{out} {img.size[0]}x{img.size[1]} ({out.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
