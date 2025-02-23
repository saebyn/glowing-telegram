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

WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .

RUN cargo build --release --bin summarize_transcription

################################################################################
# Final stage
################################################################################
FROM base AS final
COPY --from=builder /app/target/release/summarize_transcription /bootstrap

CMD ["/bootstrap"]