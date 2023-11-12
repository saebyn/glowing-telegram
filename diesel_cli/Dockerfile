# Set up a build environment for rust to run `cargo install diesel_cli`
# Then copy the binary into a new container

FROM rust 
# as builder

RUN cargo install diesel_cli --no-default-features --features postgres

#FROM alpine:latest

#RUN apk --no-cache add ca-certificates postgresql-client

#COPY --from=builder /usr/local/cargo/bin/diesel /usr/local/bin/diesel

WORKDIR /app
ENTRYPOINT ["diesel"]
CMD ["--help"]
