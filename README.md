<img src="logo.svg" />

`y-sweet` is an open-source stack for building realtime applications on top of the [Yjs](https://github.com/yjs/yjs) CRDT library.

The core component of `y-sweet` is a standalone Yjs server with some unique features:

- Persist to a network filesystem or S3-compatible storage, [modeled after Figma’s infrastructure](https://digest.browsertech.com/archive/browsertech-digest-figma-is-a-file-editor/).
- Scales horizontally with a [session backend](https://driftingin.space/posts/session-lived-application-backends) model.
- Deploys as a native Linux process, or as a WebAssembly module on Cloudflare's edge.
- Written in Rust and built on [y-crdt](https://github.com/y-crdt/y-crdt/).

Additionally, the `y-sweet` stack includes:
- `@y-sweet/sdk`, a TypeScript library for interacting with `y-sweet-server` from your application backend.
    - Create and manage documents.
    - Authorize document access and generate client tokens.
- `@y-sweet/react`, a React hooks library for building Yjs applications.
- A debugger for exploring Yjs document and presence state (WIP).

The goal of y-sweet is to give developers the end-to-end developer ergonomics they would expect from a proprietary commercial solution, on top of the excellent open-source technology of Yjs, without the lock-in.

y-sweet is MIT-licensed, and was created by [Drifting in Space](https://driftingin.space).
