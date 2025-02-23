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
WORKDIR /app/audio_transcriber
RUN cargo build --release --bin audio_transcriber

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

# Install openai whisper
RUN pip3 install --break-system-packages openai-whisper

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

RUN mkdir /model
RUN chown ${USER}:${USER} /model

USER ${USER}:${USER}

COPY --from=builder --chown=${USER}:${USER} /app/audio_transcriber/download_model.py /app/download_model.py

RUN python3 /app/download_model.py

COPY --from=builder --chown=${USER}:${USER} /app/target/release/audio_transcriber /app/runtime

ENTRYPOINT [ "/app/runtime" ]