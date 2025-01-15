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
