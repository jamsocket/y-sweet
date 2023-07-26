import { WebsocketProvider } from 'y-websocket'
import * as Y from 'yjs'
import { ConnectionKey } from './yserv'
import type { Awareness } from 'y-protocols/awareness'

export function createYjsProvider(
  doc: Y.Doc,
  connectionKey: ConnectionKey,
  extraOptions: Partial<WebsocketParams> = {},
) {
  const params = connectionKey.token ? { token: connectionKey.token } : undefined

  const provider = new WebsocketProvider(connectionKey.base_url, connectionKey.doc_id, doc, {
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
