<img src="https://raw.githubusercontent.com/drifting-in-space/y-sweet/main/logo.svg" />

# y-sweet: a Yjs server with persistence and auth

**y-sweet** is an open-source server for building realtime applications on top of the [Yjs](https://github.com/yjs/yjs) CRDT library.

## Features

- Persists document data to a network filesystem or S3-compatible storage, [inspired by Figma’s infrastructure](https://digest.browsertech.com/archive/browsertech-digest-figma-is-a-file-editor/).
- Scales horizontally with a [session backend](https://driftingin.space/posts/session-lived-application-backends) model.
- Deploys as a native Linux process, or as a WebAssembly module on Cloudflare's edge.
- Provides document-level access control via client tokens.
- Written in Rust with a focus on stability and performance, building on the excellent [y-crdt](https://github.com/y-crdt/y-crdt/) library.
