FROM rust:1.83-slim-bookworm AS builder

WORKDIR /build

COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

COPY --from=builder /build/target/release/y-sweet /usr/local/bin/y-sweet

CMD ["y-sweet", "serve-doc", "--host=0.0.0.0"]
