import { YSweetProvider, type YSweetProviderParams } from './provider'
import * as Y from 'yjs'
import { ClientToken, encodeClientToken } from '@y-sweet/sdk'
export { YSweetProvider, YSweetProviderParams }

type RelaxedClientToken = ClientToken | (() => ClientToken) | (() => Promise<ClientToken>)

async function resolveClientToken(relaxedClientToken: RelaxedClientToken): Promise<ClientToken> {
  if (typeof relaxedClientToken === 'function') {
    let result: ClientToken | Promise<ClientToken> = relaxedClientToken()
    if (result instanceof Promise) {
      return await result
    } else {
      return result
    }
  } else {
    return relaxedClientToken
  }
}

/**
 * Given a {@link ClientToken}, create a (wrapped) {@link YSweetProvider} for it.
 *
 * Like {@link YSweetProvider}, the wrapped provider supports `destroy()` to clean up resources.
 *
 * Instead of passing a {@link ClientToken} directly, you can also pass a function that
 * returns a {@link ClientToken}, or returns a {@link Promise} that resolves to a {@link ClientToken}.
 *
 * @param doc
 * @param clientToken
 * @param extraOptions
 * @returns
 */
export function createYjsProvider(
  doc: Y.Doc,
  clientToken: RelaxedClientToken,
  extraOptions: Partial<YSweetProviderParams> = {},
): LazyProvider {
  return new LazyProvider(doc, clientToken, extraOptions)
}

class LazyProvider {
  provider: Promise<YSweetProvider>

  constructor(
    doc: Y.Doc,
    relaxedClientToken: RelaxedClientToken,
    extraOptions: Partial<YSweetProviderParams>,
  ) {
    this.provider = new Promise(async (resolve) => {
      let clientToken = await resolveClientToken(relaxedClientToken)

      const params = clientToken.token ? { token: clientToken.token } : undefined
      const provider = new YSweetProvider(clientToken.url, clientToken.docId, doc, {
        params,
        ...extraOptions,
      })

      resolve(provider)
    })
  }

  destroy() {
    this.provider.then((provider) => provider.destroy())
  }
}

/**
 * Get a URL to open the Y-Sweet Debugger for the given client token.
 *
 * @param clientToken The client token to open the debugger for.
 * @returns A debugger URL as a string.
 */
export function debuggerUrl(clientToken: ClientToken): string {
  const payload = encodeClientToken(clientToken)
  return `https://debugger.y-sweet.dev/?payload=${payload}`
}
