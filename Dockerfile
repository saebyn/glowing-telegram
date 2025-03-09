# Thanks BrainlessSociety!
################################################################################
# Base stages
################################################################################
ARG RUST_VERSION
ARG DEBIAN_VERSION

# Use the official rust image as the base image for our rust build pipeline
FROM rust:${RUST_VERSION:-latest} AS rust_base

# Use the official debian image as the base image for our runtime image
FROM debian:${DEBIAN_VERSION:-latest} AS runtime_base

FROM runtime_base AS runtime
# Create a non-root user to run the app
ARG USER=user
ARG UID=10001

WORKDIR /app

RUN apt-get update \
  && apt-get -y upgrade

RUN adduser \
  --disabled-password \
  --gecos "" \
  --home "/nonexistent" \
  --shell "/sbin/nologin" \
  --no-create-home \
  --uid "${UID}" \
  "${USER}"

################################################################################
# Chef stage
################################################################################
FROM rust_base AS rust_chef

# Install cargo-chef. Used to cache dependencies
RUN cargo install cargo-chef


################################################################################
# Planner stage
################################################################################
FROM rust_chef AS rust_planner

WORKDIR /app

COPY . .

RUN cargo chef prepare --recipe-path recipe.json


################################################################################
# Build stage
################################################################################
FROM rust_chef AS rust_builder

COPY --from=rust_planner /app/recipe.json recipe.json

# Build dependencies. This is the caching Docker layer
RUN cargo chef cook --release --recipe-path recipe.json

WORKDIR /app

COPY . .

# Run tests (If enabled)
ARG RUN_TESTS=false
RUN if [ "$RUN_TESTS" = "true" ]; then cargo test --release; fi

# Build all the subcrates
RUN cargo build --release

################################################################################
# Target stages for each binary
################################################################################

# ai_chat_lambda
FROM runtime_base AS ai_chat_lambda
COPY --from=rust_builder /app/target/release/ai_chat_lambda /bootstrap
CMD ["/bootstrap"]

# audio_transcriber
FROM runtime AS audio_transcriber

RUN apt-get install -y --no-install-recommends \
  ca-certificates \
  curl \
  ffmpeg \
  libssl-dev \
  pkg-config \
  python3 \
  python3-pip \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

RUN pip3 install --break-system-packages openai-whisper

RUN mkdir /model
RUN chown ${USER}:${USER} /model

USER ${USER}:${USER}

COPY --from=rust_builder --chown=${USER}:${USER} /app/audio_transcriber/download_model.py /app/download_model.py

RUN python3 /app/download_model.py

COPY --from=rust_builder --chown=${USER}:${USER} /app/target/release/audio_transcriber /app/audio_transcriber

ENTRYPOINT [ "/app/audio_transcriber" ]

# crud_api
FROM runtime_base AS crud_api
COPY --from=rust_builder /app/target/release/crud_api /bootstrap
CMD ["/bootstrap"]

# media_lambda
FROM public.ecr.aws/lambda/python:3 AS media_lambda
COPY media_lambda/main.py ${LAMBDA_TASK_ROOT}
CMD [ "main.handler" ]

# render_job
FROM runtime AS render_job

RUN apt-get install -y --no-install-recommends \
  ca-certificates \
  ffmpeg \
  libssl-dev \
  pkg-config \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

COPY --from=rust_builder --chown=${USER}:${USER} /app/target/release/render_job /app/render_job
ENTRYPOINT [ "/app/render_job" ]

# summarize_transcription
FROM runtime_base AS summarize_transcription
COPY --from=rust_builder /app/target/release/summarize_transcription /bootstrap
CMD ["/bootstrap"]

# twitch_lambda
FROM runtime_base AS twitch_lambda
COPY --from=rust_builder /app/target/release/twitch_lambda /bootstrap
CMD ["/bootstrap"]

# upload_video
FROM runtime_base AS upload_video
COPY --from=rust_builder /app/target/release/upload_video /bootstrap
CMD ["/bootstrap"]

# video_ingestor
FROM runtime AS video_ingestor

RUN apt-get install -y --no-install-recommends \
  ca-certificates \
  curl \
  libssl-dev \
  pkg-config \
  ffmpeg \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

COPY --from=rust_builder --chown=${USER}:${USER} /app/target/release/video_ingestor /app/video_ingestor
ENTRYPOINT [ "/app/video_ingestor" ]

# youtube_lambda
FROM runtime_base AS youtube_lambda
COPY --from=rust_builder /app/target/release/youtube_lambda /bootstrap
CMD ["/bootstrap"]