FROM rust:1.88-slim-bookworm AS builder

WORKDIR /build

COPY . .
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

COPY --from=builder /build/target/release/y-sweet /usr/local/bin/y-sweet

COPY <<EOF /entrypoint.sh
#!/bin/sh
if [ "\${ENV:-}" = "prod" ]; then
  exec y-sweet serve "\${DATA_DIR:-}" --host 0.0.0.0 --url-prefix "\${SERVER_URL:-}" --auth "\${PRIVATE_KEY:-}" --prod
else
  exec y-sweet serve "\${DATA_DIR:-}" --host 0.0.0.0 --url-prefix "\${SERVER_URL:-}" --auth "\${PRIVATE_KEY:-}"
fi
EOF

RUN chmod +x /entrypoint.sh

CMD ["/entrypoint.sh"]