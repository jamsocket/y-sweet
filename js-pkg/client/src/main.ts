import { YSweetProvider, type YSweetProviderParams } from './provider'
import * as Y from 'yjs'
import { ClientToken } from '@y-sweet/sdk'

export { YSweetProvider, YSweetProviderParams as WebsocketProviderParams, YSweetProviderParams }

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

  const provider = new YSweetProvider(clientToken.url, clientToken.doc, doc, {
    params,
    ...extraOptions,
  })

  return provider
}
