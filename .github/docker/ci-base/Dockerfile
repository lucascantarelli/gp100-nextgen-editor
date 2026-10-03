# syntax=docker/dockerfile:1
# ═══════════════════════════════════════════════════════════════════════════
# ci-base — imagem de CI para os jobs SEM GUI (issue #68).
#
# POR QUE EXISTE: `ci-linux` carrega WebKitGTK/GTK/xvfb/mesa por causa do
# shell Tauri. Tudo isso é peso morto para os jobs que nunca abrem uma
# janela — pytest da especificação, auditoria de dependência e validação dos
# workflows. Esses jobs pagavam um pull de >1,5 GB para rodar um interpretador
# Python e um `pnpm audit`.
#
# POR QUE DEBIAN SLIM E NÃO ALPINE (alpine era a sugestão padrão do pedido):
# `alsa-sys` e boa parte das dependências com código C/linkam contra glibc; em
# musl o build do `gp100-cli` não fecha sem patch. O ganho do Alpine aqui
# seria ~50 MB numa imagem que já é pequena — e o custo (toolchain musl
# própria, cache de crates separado, divergência de `libasound`) é real.
#
# CONTÉM:
#   - Rust stable + clippy + rustfmt;
#   - uv (Python do projeto de análise/RE);
#   - Node 22 + pnpm 11 (o `pnpm audit` roda aqui; o store NÃO é aquecido —
#     o job de auditoria não instala dependências, só lê o lockfile);
#   - ca-certificates, git, jq, pkg-config, libasound2-dev.
#
# NÃO CONTÉM: WebKit/GTK/xvfb/mesa (ficam na ci-linux) e Chromium.
# Os jobs de browser ficam nos runners hospedados com cache do
# ~/.cache/ms-playwright — medido no issue #41: o pull da imagem custaria mais
# que o install do browser.
#
# `GP100_CI_IMAGE=1` é o marcador que as composite actions leem para saber que
# Node/pnpm/Rust já estão prontos e não devem ser instalados de novo.
#
# TAGS: `:1` (estável, a que os jobs referenciam) + `sha-<curto>` (auditoria).
# ═══════════════════════════════════════════════════════════════════════════

ARG DEBIAN_VERSION=bookworm-slim

FROM rust:${DEBIAN_VERSION}

ARG NODE_VERSION=22
ARG PNPM_VERSION=11

ENV DEBIAN_FRONTEND=noninteractive \
    GP100_CI_IMAGE=1 \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:/usr/local/bin:$PATH \
    CARGO_TERM_COLOR=always

RUN apt-get update -y \
 && apt-get install -y --no-install-recommends \
      ca-certificates curl git jq pkg-config libasound2-dev \
 && rm -rf /var/lib/apt/lists/*

# uv pelo instalador oficial: o `rust:bookworm-slim` não traz Python gerenciado,
# e o `pip install uv` cairia no PEP 668 (ambiente externo).
COPY --from=ghcr.io/astral-sh/uv:0.9.7 /uv /uvx /usr/local/bin/

# Node + pnpm: mesma versão do resto do CI (a composite setup-node-pnpm aborta
# se a major divergir — o guard de deriva da imagem).
RUN case "$(dpkg --print-architecture)" in \
      amd64) NODE_ARCH=x64 ;; \
      arm64) NODE_ARCH=arm64 ;; \
      *) echo "arquitetura não suportada" >&2; exit 1 ;; \
    esac \
 && NODE_TARBALL="$(curl -fsSL "https://nodejs.org/dist/latest-v${NODE_VERSION}.x/" \
      | grep -o "node-v${NODE_VERSION}\.[0-9.]*-linux-${NODE_ARCH}\.tar\.xz" | head -1)" \
 && curl -fsSL "https://nodejs.org/dist/latest-v${NODE_VERSION}.x/${NODE_TARBALL}" -o /tmp/node.tar.xz \
 && tar -xJf /tmp/node.tar.xz -C /usr/local --strip-components=1 \
 && rm -f /tmp/node.tar.xz \
 && npm install -g "pnpm@${PNPM_VERSION}" --no-fund --no-audit

# ── Sanidade: se faltar ferramenta, o BUILD falha (não o job na CI) ──
RUN set -eux \
 && for bin in rustc cargo rustfmt cargo-clippy uv node pnpm git jq pkg-config; do \
      command -v "$bin"; \
    done \
 && rustc --version && cargo --version && uv --version \
 && node -e 'const major = process.versions.node.split(".")[0]; if (major !== "22") throw new Error("node " + major);' \
 && pnpm --version \
 && cargo fmt --version