ARG RUST_VERSION
FROM rust:${RUST_VERSION:-latest} AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM public.ecr.aws/lambda/provided:al2
COPY --from=builder /app/target/release/crud_api /bootstrap
CMD ["/bootstrap"]
