import { WebsocketProvider, type WebsocketProviderParams } from './websocket'
import * as Y from 'yjs'
import { ClientToken } from '@y-sweet/sdk'

export { WebsocketProvider, WebsocketProviderParams }

/**
 * Given a {@link ClientToken}, create a {@link WebsocketProvider} for it.
 *
 * @param doc
 * @param clientToken
 * @param extraOptions
 * @returns
 */
export function createYjsProvider(
  doc: Y.Doc,
  clientToken: ClientToken,
  extraOptions: Partial<WebsocketProviderParams> = {},
): WebsocketProvider {
  const params = clientToken.token ? { token: clientToken.token } : undefined

  const provider = new WebsocketProvider(clientToken.url, clientToken.doc, doc, {
    params,
    ...extraOptions,
  })

  return provider
}
