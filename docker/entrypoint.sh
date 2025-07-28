#!/bin/sh
if [ "${ENV:-}" = "prod" ]; then
  exec y-sweet serve "${DATA_DIR:-}" --host 0.0.0.0 --url-prefix "${SERVER_URL:-}" --auth "${PRIVATE_KEY:-}" --prod
else
  exec y-sweet serve "${DATA_DIR:-}" --host 0.0.0.0 --url-prefix "${SERVER_URL:-}" --auth "${PRIVATE_KEY:-}"
fi