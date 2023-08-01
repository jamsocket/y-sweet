<img src="logo.svg" />

**y-sweet** is an open-source stack for building realtime applications on top of the [Yjs](https://github.com/yjs/yjs) CRDT library.

The core component of y-sweet is a standalone Yjs server which:

- Persists document data to a network filesystem or S3-compatible storage, [modeled after Figma’s infrastructure](https://digest.browsertech.com/archive/browsertech-digest-figma-is-a-file-editor/).
- Scales horizontally with a [session backend](https://driftingin.space/posts/session-lived-application-backends) model.
- Deploys as a native Linux process, or as a WebAssembly module on Cloudflare's edge.
- Provides document-level access control via client tokens.
- Is written in Rust and built on the excellent [y-crdt](https://github.com/y-crdt/y-crdt/).

The y-sweet server can be used by itself, or with the rest of the y-sweet stack:

- `@y-sweet/sdk`, a TypeScript library for interacting with `y-sweet-server` from your application backend.
- `@y-sweet/react`, a React hooks library for building Yjs applications.
- A debugger for exploring Yjs document and presence state (WIP).

The goal of y-sweet is to give developers the performance and end-to-end developer ergonomics they would expect from a proprietary state-sync platform, without the lock-in.

y-sweet is MIT-licensed, and was created by [Drifting in Space](https://driftingin.space).
