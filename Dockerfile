# syntax=docker/dockerfile:1.7
# ============================================================================
# MawiGateway — single-image build.
#
# This file consolidates what used to be two separate images
# (mawi-api + mawi-web) into one. The Rust gateway serves the
# Next.js admin SPA from `/` and the API from `/v1/*` on the same
# port (8030).
#
# Stages:
#   web-deps   — npm ci once, cache node_modules
#   web-build  — `next build` produces a static `out/` dir
#   rust-base  — pinned rust toolchain + system deps + cargo-chef
#   planner    — extracts dep recipe (cheap, no compile)
#   cacher     — compiles ONLY the deps `gateway` needs
#   builder    — actual build, reusing the cached dep target dir
#   runtime    — minimal debian + docker CLI + binary + static files
#
# Build:
#   docker build -f Dockerfile -t mawigateway .
# Run:
#   docker run -p 8030:8030 -e MG_DATABASE_URL=... -e MG_MASTER_KEY=... mawigateway
# ============================================================================

ARG RUST_VERSION=1.92
ARG DEBIAN_RELEASE=bookworm
ARG NODE_VERSION=20

# ----------------------------------------------------------------------------
# Stage W1 — web-deps: cache npm install
# Re-runs only when frontend/package*.json change.
# ----------------------------------------------------------------------------
FROM node:${NODE_VERSION}-alpine AS web-deps
WORKDIR /web
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci --no-audit --no-fund

# ----------------------------------------------------------------------------
# Stage W2 — web-build: produce static export at /web/out
# Reuses node_modules from the deps stage.
# ----------------------------------------------------------------------------
FROM node:${NODE_VERSION}-alpine AS web-build
WORKDIR /web
COPY --from=web-deps /web/node_modules ./node_modules
COPY frontend/ ./
# next.config.js sets `output: 'export'` so this produces a static
# tree (HTML + JS + CSS + assets, no Node runtime needed).
RUN npm run build

# ----------------------------------------------------------------------------
# Stage R0 — rust-base: pinned toolchain + system deps + cargo-chef
# ----------------------------------------------------------------------------
FROM rust:${RUST_VERSION}-slim-${DEBIAN_RELEASE} AS rust-base
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked

# ----------------------------------------------------------------------------
# Stage R1 — planner: extract dep recipe
# ----------------------------------------------------------------------------
FROM rust-base AS planner
COPY backend/ /app/
RUN cargo chef prepare --recipe-path recipe.json

# ----------------------------------------------------------------------------
# Stage R2 — cacher: compile only the deps the gateway binary needs
# ----------------------------------------------------------------------------
FROM rust-base AS cacher
COPY --from=planner /app/recipe.json recipe.json
ENV CARGO_NET_RETRY=10 \
    CARGO_NET_GIT_FETCH_WITH_CLI=true \
    CARGO_HTTP_TIMEOUT=120 \
    CARGO_HTTP_MULTIPLEXING=false
RUN cargo chef cook --release --bin gateway --recipe-path recipe.json

# ----------------------------------------------------------------------------
# Stage R3 — builder: actual gateway build, reusing cached deps
# ----------------------------------------------------------------------------
FROM rust-base AS builder
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
COPY backend/ /app/
RUN cargo build --release --bin gateway

# ----------------------------------------------------------------------------
# Stage Final — runtime: minimal debian + docker CLI + binary + UI
# Docker CLI is required so the gateway can spawn MCP server containers
# via the mounted /var/run/docker.sock.
# ----------------------------------------------------------------------------
FROM debian:${DEBIAN_RELEASE}-slim AS runtime
ARG DEBIAN_RELEASE
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        libssl3 \
        ca-certificates \
        curl \
        gnupg \
    && install -m 0755 -d /etc/apt/keyrings \
    && curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc \
    && chmod a+r /etc/apt/keyrings/docker.asc \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian ${DEBIAN_RELEASE} stable" > /etc/apt/sources.list.d/docker.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends docker-ce-cli \
    && apt-get purge -y curl gnupg \
    && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/*

# Backend binary, migrations, and the static admin UI.
COPY --from=builder /app/target/release/gateway /app/mawi-gateway
COPY backend/migrations /app/migrations
COPY backend/.env.example /app/.env
COPY --from=web-build /web/out /app/static

# Tell the gateway where to serve the SPA from. main.rs reads this
# at boot and only mounts the static endpoint when the directory
# exists, so leaving it unset (e.g. in a dev cargo-run) is harmless.
ENV MG_STATIC_DIR=/app/static

# Non-root user is created but the gateway currently runs as root
# because it needs to talk to /var/run/docker.sock. Switch to `mawi`
# once the host docker group GID can be passed in at build time.
RUN useradd -m -u 1000 mawi

EXPOSE 8030

CMD ["./mawi-gateway"]
