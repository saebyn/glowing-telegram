# Thanks BrainlessSociety!
################################################################################
# Base stage
################################################################################
ARG RUST_VERSION
FROM rust:${RUST_VERSION:-latest} AS base


################################################################################
# Chef stage
################################################################################
FROM base AS chef

# Install cargo-chef. Used to cache dependencies
RUN cargo install cargo-chef


################################################################################
# Planner stage
################################################################################
FROM chef AS planner

WORKDIR /app

COPY . .

RUN cargo chef prepare --recipe-path recipe.json


################################################################################
# Build stage
################################################################################
FROM chef AS builder

WORKDIR /app

COPY --from=planner /app/recipe.json recipe.json

# Build dependencies. This is the caching Docker layer
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .

# Run tests (If enabled)
ARG RUN_TESTS=false
RUN if [ "$RUN_TESTS" = "true" ]; then cargo test --release; fi

# Build application
RUN cargo build --release


################################################################################
# Final stage
################################################################################
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
  ca-certificates \
  libssl-dev \
  pkg-config \
  && rm -rf /var/lib/apt/lists/*

# Create a non-root user to run the app
ARG USER=user
ARG UID=10001

RUN adduser \
  --disabled-password \
  --gecos "" \
  --home "/nonexistent" \
  --shell "/sbin/nologin" \
  --no-create-home \
  --uid "${UID}" \
  "${USER}"

USER ${USER}:${USER}

WORKDIR /app

COPY --from=builder --chown=${USER}:${USER} /app/target/release/test_api /usr/local/bin

ENV CARGO_MANIFEST_DIR=/app

EXPOSE 3000

ENTRYPOINT ["test_api"]