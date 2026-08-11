# google-cloud-storage requires rustc >= 1.88; aws-smithy-xml (locked) requires >= 1.91.1
FROM rust:1.93-slim-bookworm AS builder

WORKDIR /build

COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

# The GCS/AWS SDK clients load TLS roots from the system trust store.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/y-sweet /usr/local/bin/y-sweet

CMD ["y-sweet", "serve-doc", "--host=0.0.0.0"]
