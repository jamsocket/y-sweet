# ---------- Build Stage ----------
FROM rust:1.88 as builder

WORKDIR /app
COPY crates/ ./

RUN apt-get update && apt-get install -y pkg-config libssl-dev ca-certificates && \
  update-ca-certificates && \
  cargo build --release

# ---------- Runtime Stage ----------
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y pkg-config libssl-dev ca-certificates && \
  update-ca-certificates

WORKDIR /app

COPY --from=builder /app/target/release/y-sweet /usr/local/bin/y-sweet

COPY docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

CMD ["/entrypoint.sh"]