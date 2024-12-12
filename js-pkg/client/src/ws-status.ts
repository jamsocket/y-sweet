/** Status API compatibility with y-websocket */

import {
  EVENT_CONNECTION_STATUS,
  STATUS_CONNECTED,
  STATUS_CONNECTING,
  STATUS_HANDSHAKING,
  YSweetProvider,
  YSweetStatus,
} from './provider'

export const EVENT_STATUS = 'status'
/** Fired when the _initial_ sync is complete. Only refired after that if there is a reconnection. */
export const EVENT_SYNC = 'sync'
export const EVENT_CONNECTION_CLOSE = 'connection-close'
export const EVENT_CONNECTION_ERROR = 'connection-error'

/** Alias of EVENT_SYNC for compatibility with y-websocket.
 * Ref: https://github.com/yjs/y-websocket/blob/aa4220407bda51ab6282d1291de6493c136c2089/src/y-websocket.js#L404
 */
export const EVENT_SYNCED = 'synced'

/**
 * Ref: https://github.com/yjs/y-websocket/blob/aa4220407bda51ab6282d1291de6493c136c2089/README.md?plain=1#L119-L125
 */
export type YWebsocketEvent =
  | typeof EVENT_STATUS
  | typeof EVENT_SYNC
  | typeof EVENT_CONNECTION_CLOSE
  | typeof EVENT_CONNECTION_ERROR
  | typeof EVENT_SYNCED

const WEBSOCKET_STATUS_CONNECTED = 'connected'
const WEBSOCKET_STATUS_DISCONNECTED = 'disconnected'
const WEBSOCKET_STATUS_CONNECTING = 'connecting'

export type YWebSocketStatus =
  | typeof WEBSOCKET_STATUS_CONNECTED
  | typeof WEBSOCKET_STATUS_DISCONNECTED
  | typeof WEBSOCKET_STATUS_CONNECTING

function translateStatus(status: YSweetStatus): YWebSocketStatus {
  if (status === STATUS_CONNECTED) {
    return WEBSOCKET_STATUS_CONNECTED
  } else if ([STATUS_CONNECTING, STATUS_HANDSHAKING].includes(status)) {
    return WEBSOCKET_STATUS_CONNECTING
  } else {
    return WEBSOCKET_STATUS_DISCONNECTED
  }
}

export class WebSocketCompatLayer {
  lastStatus: YWebSocketStatus = WEBSOCKET_STATUS_DISCONNECTED
  lastSyncStatus = false

  constructor(private provider: YSweetProvider) {
    this.provider.on(EVENT_CONNECTION_STATUS, this.updateStatus.bind(this))
  }

  updateStatus(status: YSweetStatus) {
    const newStatus = translateStatus(status)

    const syncStatus = status === STATUS_CONNECTED

    if (this.lastSyncStatus !== syncStatus) {
      this.lastSyncStatus = syncStatus

      // Yjs emits both `synced` and `sync` events for legacy reasons.
      // Ref: https://github.com/yjs/y-websocket/blob/aa4220407bda51ab6282d1291de6493c136c2089/src/y-websocket.js#L404
      this.provider.emit(EVENT_SYNC, syncStatus)
      this.provider.emit(EVENT_SYNCED, syncStatus)
    }

    if (this.lastStatus !== newStatus) {
      this.lastStatus = newStatus
      this.provider.emit(EVENT_STATUS, { status: newStatus })
    }
  }
}
