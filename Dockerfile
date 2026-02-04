# syntax=docker/dockerfile:1
FROM lukemathwalker/cargo-chef:latest-rust-1.93-slim-bookworm AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

RUN --mount=type=cache,id=s/99538def-79df-456c-ad70-cfc331c7f14e-/usr/local/cargo/registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=s/99538def-79df-456c-ad70-cfc331c7f14e-/app/target,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json

COPY . .

RUN --mount=type=cache,id=s/99538def-79df-456c-ad70-cfc331c7f14e-/usr/local/cargo/registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=s/99538def-79df-456c-ad70-cfc331c7f14e-/app/target,target=/app/target \
    cargo build --release --bin worldstate_server && \
    cp /app/target/release/worldstate_server /app/worldstate_server

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/worldstate_server worldstate_server

EXPOSE 3000

ENTRYPOINT ["./worldstate_server"]