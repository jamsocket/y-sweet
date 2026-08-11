# google-cloud-storage requires rustc >= 1.88; its deps use edition 2024 (cargo >= 1.85)
FROM rust:1.90-slim-bookworm AS builder

WORKDIR /build

COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

COPY --from=builder /build/target/release/y-sweet /usr/local/bin/y-sweet

CMD ["y-sweet", "serve-doc", "--host=0.0.0.0"]
