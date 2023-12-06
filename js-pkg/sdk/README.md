<img src="https://raw.githubusercontent.com/drifting-in-space/y-sweet/main/logo.svg" />

# @y-sweet/sdk

[![GitHub Repo stars](https://img.shields.io/github/stars/drifting-in-space/y-sweet?style=social)](https://github.com/drifting-in-space/y-sweet)
[![Chat on Discord](https://img.shields.io/static/v1?label=chat&message=discord&color=404eed)](https://discord.gg/N5sEpsuhh9)
[![npm](https://img.shields.io/npm/v/@y-sweet/sdk)](https://www.npmjs.com/package/@y-sweet/sdk)

JavaScript/TypeScript backend SDK for building collaboration features with y-sweet.

## Installation
```
npm install @y-sweet/sdk
```

## Documentation
Read the [documentation](https://www.y-sweet.dev/) for guides and API references.

## Examples
Explore our [collaborative examples](https://github.com/drifting-in-space/y-sweet) to help you get started.

All examples are open source and live in this repository, within [/examples](https://github.com/drifting-in-space/y-sweet/tree/main/examples).

# Using @y-sweet/sdk

Here’s how access control works in y-sweet:

- Your server (i.e. your Next.js server component) connects to y-sweet using an API key and asks for a client token for a specific document.
- Your server then passes that client token to the client, often as props to a client-side React component.
- Your client then connects to y-sweet using the client token.

The client token contains all the information needed for the client to connect to a y-sweet document, so the client doesn’t need any configuration.
But you _do_ need to tell your server how to talk to y-sweet, by passing a **server token**.

A server token combines a URL and a secret key. It can be represented either as a JSON object with `url` and `token` as keys, or as a JSONified string
of the same. This makes it easy to store the server token in a secret store, and pass it to your server code as an environment variable.


``` tsx filename="Home.tsx"
import { YDocProvider } from '@y-sweet/react'
import { getOrCreateDocAndToken } from '@y-sweet/sdk'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
    // Point to a local or production y-sweet server.
    const connectionString = "ys://localhost:8080"

    const clientToken = await getOrCreateDocAndToken(connectionString, searchParams.doc)

    return (
        <YDocProvider clientToken={clientToken} setQueryParam="doc">
            // Call your collaborative interface here
        </YDocProvider>
    )
}
```
