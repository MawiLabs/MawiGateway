# Base stage with common dependencies and cargo-chef
FROM rust:slim-bookworm as base
WORKDIR /app
# Install system dependencies required for building Rust crates (e.g. openssl)
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
# Install cargo-chef
RUN cargo install cargo-chef

# Stage 1: Planner - Compute the recipe
FROM base as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Cacher - Build dependencies using the recipe
FROM base as cacher
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this layer is cached until Cargo.toml/lock changes
# Configure Cargo for better network resilience
ENV CARGO_NET_RETRY=10 \
    CARGO_NET_GIT_FETCH_WITH_CLI=true \
    CARGO_HTTP_TIMEOUT=120 \
    CARGO_HTTP_MULTIPLEXING=false
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 3: Builder - Build the actual application
FROM base as builder
# Copy compiled dependencies from cacher
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
# Copy source code
COPY . .
# Build the binary
RUN cargo build --release --bin gateway

# Stage 4: Runtime - Minimal runtime image
FROM debian:bookworm-slim
WORKDIR /app

# Install runtime dependencies including Docker CLI for MCP server spawning
RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    curl \
    gnupg \
    && install -m 0755 -d /etc/apt/keyrings \
    && curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc \
    && chmod a+r /etc/apt/keyrings/docker.asc \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian bookworm stable" > /etc/apt/sources.list.d/docker.list \
    && apt-get update \
    && apt-get install -y docker-ce-cli \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/gateway /app/mawi-gateway

# Copy migrations and config
COPY migrations /app/migrations
COPY .env.example /app/.env

# Create non-root user
RUN useradd -m -u 1000 mawi
# Note: Running as root for now to access Docker socket
# In production, add mawi user to docker group instead
# USER mawi

EXPOSE 8030

CMD ["./mawi-gateway"]
