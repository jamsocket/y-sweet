import { WebsocketProvider, type WebsocketProviderParams } from './websocket'
import * as Y from 'yjs'
import { type ClientToken } from '@y-sweet/sdk'

export { WebsocketProvider, WebsocketProviderParams, ClientToken }

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

function stringToBase64(input: string) {
  if (typeof window !== 'undefined' && window.btoa) {
    // Browser
    return window.btoa(input)
  } else if (typeof Buffer !== 'undefined') {
    // Node.js
    return Buffer.from(input).toString('base64')
  } else {
    throw new Error('Unable to encode to Base64')
  }
}

function base64ToString(input: string) {
  if (typeof window !== 'undefined' && window.atob) {
    // Browser
    return window.atob(input)
  } else if (typeof Buffer !== 'undefined') {
    // Node.js
    return Buffer.from(input, 'base64').toString()
  } else {
    throw new Error('Unable to decode from Base64')
  }
}

export function encodeClientToken(token: ClientToken): string {
  const jsonString = JSON.stringify(token)
  let base64 = stringToBase64(jsonString)
  base64 = base64.replace('+', '-').replace('/', '_').replace(/=+$/, '')
  return base64
}

export function decodeClientToken(token: string): ClientToken {
  let base64 = token.replace('-', '+').replace('_', '/')
  while (base64.length % 4) {
    base64 += '='
  }
  const jsonString = base64ToString(base64)
  return JSON.parse(jsonString)
}
