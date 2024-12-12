import { ClientToken, encodeClientToken } from '@y-sweet/sdk'
import * as Y from 'yjs'
import {
  type AuthEndpoint,
  EVENT_CONNECTION_STATUS,
  EVENT_LOCAL_CHANGES,
  STATUS_CONNECTED,
  STATUS_CONNECTING,
  STATUS_ERROR,
  STATUS_HANDSHAKING,
  STATUS_OFFLINE,
  YSweetProvider,
  type YSweetProviderParams,
  type YSweetStatus,
} from './provider'
export {
  AuthEndpoint,
  EVENT_CONNECTION_STATUS,
  EVENT_LOCAL_CHANGES,
  STATUS_CONNECTED,
  STATUS_CONNECTING,
  STATUS_ERROR,
  STATUS_HANDSHAKING,
  STATUS_OFFLINE,
  YSweetProvider,
  YSweetProviderParams,
  YSweetStatus,
}

/**
 * Given a docId and {@link AuthEndpoint}, create a {@link YSweetProvider} for it.
 *
 * @param doc
 * @param docId
 * @param authEndpoint
 * @param extraOptions
 * @returns
 */
export function createYjsProvider(
  doc: Y.Doc,
  docId: string,
  authEndpoint: AuthEndpoint,
  extraOptions: Partial<YSweetProviderParams> = {},
): YSweetProvider {
  return new YSweetProvider(authEndpoint, docId, doc, extraOptions)
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
