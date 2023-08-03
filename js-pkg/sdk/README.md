# @y-sweet/sdk

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
import { getOrCreateDoc } from '@y-sweet/sdk'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
    // In production, you can put the entire `apiToken` JSON string into a secret
    // store and pass it directly to getOrCreateDoc, as in:
    //     getOrCreateDoc(myDocId, process.env.Y_SWEET_CONFIG)
    const apiToken = {
        "endpoint": "https://y-sweet.net/project/EMwBNR17f4cn1SN1uFERi/",
        "token": "AAAgBQcOc66DXGOCecN17S4VciBCb9eR/GGsFH4H8M9hhY4="
    }

    const clientToken = await getOrCreateDoc(searchParams.doc, apiToken)

    return (
        <YDocProvider clientToken={clientToken} setQueryParam="doc">
            // Call your collaborative interface here
        </YDocProvider>
    )
}
```
