# Tests

The test suite is primarily for testing the servers for API conformity, and only secondarily for
testing the JavaScript library.

The test currently builds and tests three versions of the server:

- Native server without authentication
- Native server with authentication
- Cloudflare worker without authentication

(TODO: Cloudflare worker with authentication)

# S3 Config

If _ALL FIVE_ of the following env vars are set, tests
with S3 as the storage backend will also be run.

- YSERVE_S3_ACCESS_KEY_ID
- YSERVE_S3_SECRET_KEY
- YSERVE_S3_REGION
- YSERVE_S3_BUCKET_PREFIX
- YSERVE_S3_BUCKET_NAME

