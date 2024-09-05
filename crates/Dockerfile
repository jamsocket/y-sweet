FROM rust:1.77-slim-buster as builder

WORKDIR /build

COPY . .
RUN cargo build --release

FROM debian:buster-slim

COPY --from=builder /build/target/release/y-sweet /usr/local/bin/y-sweet

CMD ["y-sweet", "serve-doc", "--host=0.0.0.0"]
