# y-sweet

`y-sweet` is a suite of open-source packages for developing and productionizing Yjs applications.

It currently consists of:
- `y-sweet-server`, a standalone Yjs server
    - Persists data to S3-compatible storage, [modeled after Figma’s infrastructure](https://digest.browsertech.com/archive/browsertech-digest-figma-is-a-file-editor/).
    - Scales with a [session backend](https://driftingin.space/posts/session-lived-application-backends) model.
    - Optionally runs on Cloudflare Workers (with persistence to Cloudflare R2).
    - Written in Rust.
- `@y-sweet/js/sdk`, a TypeScript library for interacting with `y-sweet-server` from your application backend.
    - Create and manage documents.
    - Authorize document access and generate client tokens.
- `@y-sweet/js/react`, a React hooks library for building Yjs applications.
- `@y-sweet/js/client`, a TypeScript library with helper functions for non-React apps.

Y-sweet is MIT-licensed open source, and was created by [Drifting in Space](https://driftingin.space).
