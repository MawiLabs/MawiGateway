# syntax=docker/dockerfile:1.7
# ============================================================================
# Backend image — multi-stage with cargo-chef for dep caching.
#
# Stages:
#   base    — pinned rust toolchain + system deps + cargo-chef + chef binary
#   planner — extracts a dep recipe from Cargo.toml/lock (cheap, no compile)
#   cacher  — compiles JUST the deps the `gateway` binary needs (cached
#             until recipe.json changes)
#   builder — copies cached deps + source, compiles the gateway binary
#   runtime — minimal debian + docker CLI (for MCP container spawning)
# ============================================================================

# Pin the toolchain for reproducible builds and stable cache hits.
ARG RUST_VERSION=1.92
ARG DEBIAN_RELEASE=bookworm

FROM rust:${RUST_VERSION}-slim-${DEBIAN_RELEASE} AS base
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked

# ----------------------------------------------------------------------------
# Stage 1 — planner: extract dep recipe (cheap, no compile)
# ----------------------------------------------------------------------------
FROM base AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ----------------------------------------------------------------------------
# Stage 2 — cacher: compile only the deps the gateway binary actually needs.
# `--bin gateway` skips dev-only / test-only deps and any deps used only by
# other workspace members, cutting cold-build time meaningfully.
# This layer is reused until Cargo.toml / Cargo.lock change.
# ----------------------------------------------------------------------------
FROM base AS cacher
COPY --from=planner /app/recipe.json recipe.json
ENV CARGO_NET_RETRY=10 \
    CARGO_NET_GIT_FETCH_WITH_CLI=true \
    CARGO_HTTP_TIMEOUT=120 \
    CARGO_HTTP_MULTIPLEXING=false
RUN cargo chef cook --release --bin gateway --recipe-path recipe.json

# ----------------------------------------------------------------------------
# Stage 3 — builder: actual build, reusing the cached dep target dir.
# ----------------------------------------------------------------------------
FROM base AS builder
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
COPY . .
RUN cargo build --release --bin gateway

# ----------------------------------------------------------------------------
# Stage 4 — runtime: minimal debian + docker CLI.
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

COPY --from=builder /app/target/release/gateway /app/mawi-gateway
COPY migrations /app/migrations
COPY .env.example /app/.env

# Non-root user is created but the gateway currently runs as root because
# it needs to talk to /var/run/docker.sock. Switch to the `mawi` user once
# the host docker group GID can be passed in at build time.
RUN useradd -m -u 1000 mawi

EXPOSE 8030

CMD ["./mawi-gateway"]
