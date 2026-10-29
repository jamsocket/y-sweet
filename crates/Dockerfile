# google-cloud-storage requires rustc >= 1.88; aws-smithy-xml (locked) requires >= 1.91.1
FROM rust:1.93-slim-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /build

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --locked
# Distroless has no mkdir; pre-create the default data dir so it can be owned
# by the nonroot user (an empty named volume mounted here inherits it).
RUN mkdir -p /data

# Distroless runtime: no shell, package manager or coreutils, so an RCE can't
# spawn a shell or pull tools. cc-debian12 ships glibc, libgcc and the system CA
# trust store (the GCS/AWS SDK clients load TLS roots from it); TLS itself is
# rustls, so no OpenSSL is needed. :nonroot runs as UID/GID 65532.
# NOTE: no shell means shell-based healthchecks don't work; probe GET /ready
# over HTTP instead. For a debug shell, temporarily use :debug-nonroot.
FROM gcr.io/distroless/cc-debian12:nonroot

COPY --from=builder /build/target/release/y-sweet /usr/local/bin/y-sweet
COPY --from=builder --chown=65532:65532 /data /data

USER nonroot
# distroless :nonroot defaults to /home/nonroot; keep the old image's / so
# relative store paths (e.g. `serve ./data`) still resolve to the /data mount.
WORKDIR /

CMD ["y-sweet", "serve-doc", "--host=0.0.0.0"]
