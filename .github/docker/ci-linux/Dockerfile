# syntax=docker/dockerfile:1
# ═══════════════════════════════════════════════════════════════════════════
# ci-linux — imagem de CI do GP-100 NextGen Editor (publicada no ghcr.io).
#
# POR QUE EXISTE (issue #41): o caminho quente do CI Linux pagava, a CADA run,
# `apt-get` do WebKitGTK/GTK (45–57s), `cargo install tauri-driver` (17s) e o
# rustup do runner (8–10s). Isso é FIXO — não depende do código — logo pertence
# à imagem, não ao job. Migraram para cá: o build/test do shell Tauri em Linux
# e o smoke do webview. Os jobs SEM GUI usam a `ci-base` (slim, sem WebKit).
#
# CONTÉM (versões em build-arg; a fonte única das ferramentas é este arquivo):
#   - Rust stable + clippy + rustfmt (RUSTUP_HOME/CARGO_HOME em /opt para
#     sobreviverem ao HOME=/github/home dos jobs de container);
#   - tauri-driver COMPILADO do crates.io (--locked) em /usr/local/bin;
#   - WebKitGTK 4.1 + GTK3 + appindicator + rsvg (dev) — build do gp100-ui;
#   - ALSA (libasound2-dev + pkg-config) — build do gp100-cli no Linux;
#   - webkit2gtk-driver + xvfb + mesa (renderização por software) — smoke;
#   - Node 22 + pnpm 11 com o store AQUECIDO a partir do pnpm-lock.yaml do
#     repo (o `pnpm install` dos jobs fica praticamente offline).
#
# NÃO CONTÉM Chromium/Playwright de propósito: os jobs de browser (`3 test ·
# e2e shell`, `3 test · e2e visual`) ficam nos runners hospedados com cache do
# ~/.cache/ms-playwright. MEDIDO (issue #41): o pull da imagem custa mais do que
# o install do Chromium que ele eliminaria — o container só entrou onde o pull
# se paga (lição CI-11 no knowledge.md).
#
# COMO USAR: o job `Infra · imagens de container` do ci.yml publica `:1` (estável, pull
# pelos jobs) + `sha-<curto>` (auditoria). Os jobs `3 test · e2e webview` e
# `2/3 build|test · rust` referenciam a tag em `container.image`; a política:
# README ↔ revisão humana no PR.
# ═══════════════════════════════════════════════════════════════════════════

ARG UBUNTU_VERSION=24.04
FROM ubuntu:${UBUNTU_VERSION}

ARG RUST_TOOLCHAIN=stable
ARG NODE_VERSION=22
ARG PNPM_VERSION=11

# GP100_CI_IMAGE marca os jobs de container para as composite actions
# (setup-rust/setup-node-pnpm pulam instalações que já estão na imagem).
ENV DEBIAN_FRONTEND=noninteractive \
    GP100_CI_IMAGE=1 \
    RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:/usr/local/bin:$PATH \
    CARGO_TERM_COLOR=always \
    # Store do pnpm fora do HOME do job (o runner usa /github/home). O pnpm lê
    # configuração por env com o prefixo npm_config_*.
    npm_config_store_dir=/opt/pnpm-store

# ── Sistema: build do Tauri (webkit/gtk), do cli (ALSA) e smoke (xvfb) ──
RUN apt-get update -y \
 && apt-get install -y --no-install-recommends \
      ca-certificates curl git jq build-essential pkg-config \
      libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
      libasound2-dev \
      webkit2gtk-driver xvfb libgl1-mesa-dri libegl-mesa0 \
 && rm -rf /var/lib/apt/lists/*

# ── Rust stable + clippy + rustfmt ──
RUN curl -fsSL https://sh.rustup.rs -o /tmp/rustup.sh \
 && sh /tmp/rustup.sh -y --no-modify-path --profile minimal \
      --default-toolchain "${RUST_TOOLCHAIN}" \
      --component clippy --component rustfmt \
 && rm -f /tmp/rustup.sh \
 && rustc --version && cargo --version \
 && cargo clippy --version && cargo fmt --version

# ── tauri-driver COMPILADO: o smoke do shell não instala mais por run ──
# O binário vai para /usr/local/bin (PATH global; o job roda com HOME=/github/home).
RUN cargo install tauri-driver --locked \
 && cp "$CARGO_HOME/bin/tauri-driver" /usr/local/bin/ \
 # --help (não --version): o tauri-driver 2.x não tem --version e sai 1.
 && tauri-driver --help > /dev/null

# ── Node + pnpm: jobs de front dentro do container não baixam Node ──
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
 && npm install -g "pnpm@${PNPM_VERSION}" --no-fund --no-audit \
 && node --version && pnpm --version

# ── Store do pnpm aquecido com o lockfile do repo ──
# O job `Infra · imagens de container` rebuilda quando o lockfile muda, então o store nasce
# coerente com o commit; se o lockfile avançar, o `pnpm install
# --frozen-lockfile` do job baixa só o delta.
COPY packages/app/ui/package.json packages/app/ui/pnpm-lock.yaml /tmp/warm/
RUN cd /tmp/warm \
 && pnpm fetch --store-dir /opt/pnpm-store \
 && rm -rf /tmp/warm \
 && du -sh /opt/pnpm-store

# ── Sanidade: se faltar ferramenta, o BUILD falha (não o job na CI) ──
RUN set -eux \
 && for bin in rustc cargo rustfmt tauri-driver WebKitWebDriver xvfb-run node pnpm jq pkg-config; do \
      command -v "$bin"; \
    done \
 && node -e 'const major = process.versions.node.split(".")[0]; if (major !== "22") throw new Error("node " + major);' \
 && pnpm --version \
 && cargo fmt --version

CMD ["/bin/bash"]
