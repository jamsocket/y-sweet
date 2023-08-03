import { WebsocketProvider } from 'y-websocket'
import * as Y from 'yjs'
import type { Awareness } from 'y-protocols/awareness'
import { ClientToken } from '@y-sweet/sdk'

export function createYjsProvider(
  doc: Y.Doc,
  clientToken: ClientToken,
  extraOptions: Partial<WebsocketParams> = {},
) {
  const params = clientToken.token ? { token: clientToken.token } : undefined

  const provider = new WebsocketProvider(clientToken.url, clientToken.doc, doc, {
    params,
    ...extraOptions,
  })

  return provider
}

// Taken from y-websocket.d.ts
type WebsocketParams = {
  connect?: boolean | undefined
  awareness?: Awareness | undefined
  params?:
    | {
        [x: string]: string
      }
    | undefined
  WebSocketPolyfill?:
    | {
        new (url: string | URL, protocols?: string | string[] | undefined): WebSocket
        prototype: WebSocket
        readonly CLOSED: number
        readonly CLOSING: number
        readonly CONNECTING: number
        readonly OPEN: number
      }
    | undefined
  resyncInterval?: number | undefined
  maxBackoffTime?: number | undefined
  disableBc?: boolean | undefined
}
