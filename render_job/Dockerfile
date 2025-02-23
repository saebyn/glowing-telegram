# Thanks BrainlessSociety!
################################################################################
# Base stage
################################################################################
ARG RUST_VERSION
ARG DEBIAN_VERSION
FROM rust:${RUST_VERSION:-latest} AS base

################################################################################
# Chef stage
################################################################################
FROM base AS chef

# Install cargo-chef. Used to cache dependencies
RUN cargo install --version 0.1.68 cargo-chef


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

COPY --from=planner /app/recipe.json recipe.json

# Build dependencies. This is the caching Docker layer
RUN cargo chef cook --release --recipe-path recipe.json

WORKDIR /app

COPY . .

# Run tests (If enabled)
ARG RUN_TESTS=false
RUN if [ "$RUN_TESTS" = "true" ]; then cargo test --release; fi

# Build application
WORKDIR /app/render_job
RUN cargo build --release --bin render_job

################################################################################
# Final stage
################################################################################
FROM debian:${DEBIAN_VERSION:-latest} AS runtime

RUN apt-get update \
  && apt-get -y upgrade \
  && apt-get install -y --no-install-recommends \
  ca-certificates \
  curl \
  ffmpeg \
  libssl-dev \
  pkg-config \
  python3 \
  python3-pip \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*


# Create a non-root user to run the app
ARG USER=user
ARG UID=10001

WORKDIR /app

RUN adduser \
  --disabled-password \
  --gecos "" \
  --home "/nonexistent" \
  --shell "/sbin/nologin" \
  --no-create-home \
  --uid "${UID}" \
  "${USER}"

USER ${USER}:${USER}

COPY --from=builder --chown=${USER}:${USER} /app/target/release/render_job /app/render_job

ENTRYPOINT [ "/app/render_job" ]