import { YSweetProvider, type YSweetProviderParams } from './provider'
import * as Y from 'yjs'
import { ClientToken, encodeClientToken } from '@y-sweet/sdk'
export { YSweetProvider, YSweetProviderParams }

/**
 * Given a {@link ClientToken}, create a {@link YSweetProvider} for it.
 *
 * @param doc
 * @param clientToken
 * @param extraOptions
 * @returns
 */
export function createYjsProvider(
  doc: Y.Doc,
  clientToken: ClientToken,
  extraOptions: Partial<YSweetProviderParams> = {},
): YSweetProvider {
  const params = clientToken.token ? { token: clientToken.token } : undefined

  const provider = new YSweetProvider(clientToken.url, clientToken.docId, doc, {
    params,
    ...extraOptions,
  })

  return provider
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
