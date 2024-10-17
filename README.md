<img src="https://raw.githubusercontent.com/jamsocket/y-sweet/main/logo.svg" />

# Y-Sweet: a realtime CRDT-based document database, backed by object storage

[![GitHub Repo stars](https://img.shields.io/github/stars/jamsocket/y-sweet?style=social)](https://github.com/jamsocket/y-sweet)
[![Chat on Discord](https://img.shields.io/discord/939641163265232947?color=404eed&label=discord)](https://discord.gg/N5sEpsuhh9)

**Y-Sweet** is an open-source server for building realtime applications on top of the [Yjs](https://github.com/yjs/yjs) CRDT library.

## Features

- Persists document data to a network filesystem or S3-compatible storage, [inspired by Figma’s infrastructure](https://digest.browsertech.com/archive/browsertech-digest-figma-is-a-file-editor/).
- Scales horizontally with a [session backend](https://jamsocket.com/blog/session-backends) model.
- Deploys as a native Linux process.
- Provides document-level access control via client tokens.
- Written in Rust with a focus on stability and performance, building on the [blazing fast](https://github.com/dmonad/crdt-benchmarks) [y-crdt](https://github.com/y-crdt/y-crdt/) library.

## Y-Sweet stack

The Y-Sweet server can be used by any Yjs app, or you can use our opinionated stack to integrate Yjs and Y-Sweet into a Next.js app.

- `@y-sweet/sdk`, a TypeScript library for interacting with `y-sweet-server` from your application backend.
- `@y-sweet/client`, a TypeScript library for syncing documents from a client to a Y-Sweet server.
- `@y-sweet/react`, a library of React hooks for connecting to a Y-Sweet server and manipulating Yjs docs.
- A [debugger](https://docs.jamsocket.com/y-sweet/advanced/debugger) for exploring Yjs document and presence state.

The goal of the Y-Sweet stack is to give developers the end-to-end developer ergonomics they would expect from a proprietary state-sync platform, **without the lock-in**.

Y-Sweet is MIT-licensed, and was created by [Jamsocket](https://jamsocket.com).

## Docs

- [API docs](https://docs.jamsocket.com/y-sweet)
    - [TypeScript client](https://docs.jamsocket.com/y-sweet/reference/client)
    - [React hooks](https://docs.jamsocket.com/y-sweet/reference/react)
    - [Document management SDK](https://docs.jamsocket.com/y-sweet/reference/sdk)
- [Y-Sweet on Jamsocket (managed service) docs](https://docs.jamsocket.com/y-sweet/quickstart)
- [Self Hosting](https://github.com/jamsocket/y-sweet/blob/main/docs/running.md)

## Examples

Explore our [collaborative examples](https://github.com/jamsocket/y-sweet) to help you get started.

All examples are open source and live in this repository, within [/examples](https://github.com/jamsocket/y-sweet/tree/main/examples).

## Usage

Check the [vanilla js example](/examples/vanilla/) for more details.

### On the server
``` js
import { DocumentManager } from '@y-sweet/sdk';

// Pass in a CONNECTION_STRING, which you can get from a Y-Sweet service in the Jamsocket dashboard or from running npx y-sweet@latest serve locally
const manager = new DocumentManager(CONNECTION_STRING);

// create an endpoint that auths your user and returns a Y-Sweet client token
export async function POST(request) {
  // in a production app, you'd want to authenticate the user
  // and make sure they have access to the given doc
  const body = await request.json();
  const docId = body.docId;
  const clientToken = await manager.getOrCreateDocAndToken(docId);
  return Response.json(clientToken);
}
```

### On the client
``` js
import * as Y from 'yjs';
import { createYjsProvider } from '@y-sweet/client';

// Create the Yjs doc and link it to the Y-Sweet server:
const doc = new Y.Doc();
const docId = 'my-doc-id';
createYjsProvider(doc, docId, '/api/my-auth-endpoint');

// Now use the doc like a normal Yjs doc!
let mySharedMap = doc.getMap('thing');
mySharedMap.set("foo", 123);

// Update your UI based on `mySharedMap` changes like this, for example:
mySharedMap.observe((event) => {
  event.keysChanged.forEach((key) => {
    // do whatever you want based on the detected change:
    yourUpdateFunction(key, mySharedMap.get(key));
  });
});
```

## Packages

### Server

| Package Manager | Name | Version | Path |
| --- | ---- | ---- | ---- |
| npm | `y-sweet` | [![npm](https://img.shields.io/npm/v/y-sweet)](https://www.npmjs.com/package/y-sweet) | `js-pkg/server`
| crates.io | `y-sweet` | [![crates.io](https://img.shields.io/crates/v/y-sweet.svg)](https://crates.io/crates/y-sweet) | `crates/y-sweet` |
| crates.io | `y-sweet-core` | [![crates.io](https://img.shields.io/crates/v/y-sweet-core.svg)](https://crates.io/crates/y-sweet-core) | `crates/y-sweet-core` |

### Client

| Package Manager | Name | Version | Path |
| --- | ---- | ---- | ---- |
| npm | `@y-sweet/sdk` | [![npm](https://img.shields.io/npm/v/@y-sweet/sdk)](https://www.npmjs.com/package/@y-sweet/sdk) | `js-pkg/sdk` |
| npm | `@y-sweet/client` | [![npm](https://img.shields.io/npm/v/@y-sweet/client)](https://www.npmjs.com/package/@y-sweet/client) | `js-pkg/client` |
| npm | `@y-sweet/react` | [![npm](https://img.shields.io/npm/v/@y-sweet/react)](https://www.npmjs.com/package/@y-sweet/react) | `js-pkg/react` |
| pypi | `y-sweet-sdk` | [![pypi](https://img.shields.io/pypi/v/y-sweet-sdk)](https://pypi.org/project/y-sweet-sdk/) | `python/y_sweet_sdk` |

## Hosted Y-Sweet on Jamsocket

If you were to use the open source Y-Sweet server alone, you would still have to set up the infrastructure for self hosting it. With [Jamsocket](https://jamsocket.com/y-sweet), Y-Sweet scales effortlessly, and every document is automatically persisted when you link your own S3 storage.

You can try it out for free today by following our [quickstart](https://docs.jamsocket.com/y-sweet/quickstart) guide.

If you are interested in being a build partner for early access to new features, please [reach out](mailto:hi@jamsocket.com).

We are committed to growing Y-Sweet as an open-source-first project, wherever you decide to host it.
