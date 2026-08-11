# syntax=docker/dockerfile:1

# --- Build: cache compiled dependencies with cargo-chef ---
# Pre-built image ships both Rust and cargo-chef (layer cached by gha cache).
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

# rust-toolchain.toml pins nightly-2026-08-08; install it once (cached layer)
RUN rustup toolchain install --profile minimal \
    --component rustc-dev --component rust-src \
    nightly-2026-08-08 \
    && rustup override set nightly-2026-08-08

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# Dependencies only change when Cargo.toml/Cargo.lock do -> this layer is cached
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
# GIT_SHA comes from the workflow as a build-arg; set it after cook so a new
# sha does not invalidate the cached dependency layer
ARG GIT_SHA
ENV GIT_SHA=${GIT_SHA}
# Real sources; only the backend crate itself gets recompiled from here on
COPY . .
RUN cargo build --release -p backend

# --- Runtime stage: slim Debian, non-root user, just the binary ---
FROM debian:trixie-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system sapa \
    && useradd --system --gid sapa --create-home sapa

WORKDIR /app

COPY --from=builder /app/target/release/backend /app/backend

USER sapa

EXPOSE 3000

CMD ["/app/backend"]
