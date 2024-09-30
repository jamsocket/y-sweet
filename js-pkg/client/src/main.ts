import {
  YSweetProvider,
  ySweetProviderWrapper,
  type YSweetProviderParams,
  type AuthEndpoint,
  type YSweetProviderWithClientToken,
} from './provider'
import * as Y from 'yjs'
import { ClientToken, encodeClientToken } from '@y-sweet/sdk'
export { YSweetProvider, YSweetProviderParams, AuthEndpoint, YSweetProviderWithClientToken }

/**
 * Given a docId and {@link AuthEndpoint}, create a {@link YSweetProvider} for it.
 *
 * @param doc
 * @param docId
 * @param authEndpoint
 * @param extraOptions
 * @returns
 */
export async function createYjsProvider(
  doc: Y.Doc,
  docId: string,
  authEndpoint: AuthEndpoint,
  extraOptions: Partial<YSweetProviderParams> = {},
): Promise<YSweetProviderWithClientToken> {
  return ySweetProviderWrapper(authEndpoint, docId, doc, extraOptions)
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
