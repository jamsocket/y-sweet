import { ClientToken } from '@y-sweet/sdk'
import * as decoding from 'lib0/decoding'
import * as encoding from 'lib0/encoding'
import * as awarenessProtocol from 'y-protocols/awareness'
import * as syncProtocol from 'y-protocols/sync'
import * as Y from 'yjs'

export const messageSync = 0
export const messageQueryAwareness = 3
export const messageAwareness = 1
export const messageAuth = 2

/**
 * Note: this should always be a superset of y-websocket's valid events.
 * Ref: https://github.com/yjs/y-websocket/blob/aa4220407bda51ab6282d1291de6493c136c2089/README.md?plain=1#L119-L125
 */
type YSweetEvent = 'status' | 'sync' | 'connection-close' | 'connection-error'

/**
 * Note: this should always be a superset of y-websocket's valid statuses.
 * Ref: https://github.com/yjs/y-websocket/blob/aa4220407bda51ab6282d1291de6493c136c2089/README.md?plain=1#L121
 */
type YSweetStatus = {
  status: 'connected' | 'disconnected' | 'connecting'
}

type WebSocketPolyfillType = {
  new (url: string | URL): WebSocket
  prototype: WebSocket
  readonly CLOSED: number
  readonly CLOSING: number
  readonly CONNECTING: number
  readonly OPEN: number
}

export type AuthEndpoint = string | (() => Promise<ClientToken>)

export type YSweetProviderParams = {
  /** Whether to connect to the websocket on creation (otherwise use `connect()`) */
  connect?: boolean

  /** Awareness protocol instance */
  awareness?: awarenessProtocol.Awareness

  /** WebSocket constructor to use (defaults to `WebSocket`) */
  WebSocketPolyfill?: WebSocketPolyfillType

  /** Interval at which to resync */
  resyncInterval?: number

  /** Maximum backoff time when retrying */
  maxBackoffTime?: number

  /** An initial client token to use (skips the first auth request if provided.) */
  initialClientToken?: ClientToken
}

async function getClientToken(authEndpoint: AuthEndpoint, roomname: string): Promise<ClientToken> {
  if (typeof authEndpoint === 'function') {
    return await authEndpoint()
  }
  const body = JSON.stringify({ docId: roomname })
  const res = await fetch(authEndpoint, {
    method: 'POST',
    body,
    headers: { 'Content-Type': 'application/json' },
  })
  // TODO: handle errors
  const clientToken = await res.json()
  // TODO: check that clientToken.docId === this.roomname
  return clientToken
}

export class YSweetProvider extends EventTarget {
  private websocket: WebSocket | null = null
  public clientToken: ClientToken | null = null
  public synced: boolean = false
  private status: YSweetStatus = { status: 'disconnected' }
  public awareness: awarenessProtocol.Awareness
  private WebSocketPolyfill: WebSocketPolyfillType

  constructor(
    private authEndpoint: AuthEndpoint,
    private docId: string,
    private doc: Y.Doc,
    extraOptions: Partial<YSweetProviderParams> = {},
  ) {
    super()

    if (extraOptions.connect !== false) {
      this.connect()
    }

    if (extraOptions.initialClientToken) {
      this.clientToken = extraOptions.initialClientToken
    }

    this.awareness = extraOptions.awareness ?? new awarenessProtocol.Awareness(doc)
    this.WebSocketPolyfill = extraOptions.WebSocketPolyfill || WebSocket

    doc.on('update', this.update.bind(this))
  }

  private update(update: Uint8Array, origin: YSweetProvider) {
    if (!this.websocket) {
      console.warn('Websocket not connected')
      return
    }

    if (origin !== this) {
      const encoder = encoding.createEncoder()
      encoding.writeVarUint(encoder, messageSync)
      syncProtocol.writeUpdate(encoder, update)
      this.websocket.send(encoding.toUint8Array(encoder))
    }
  }

  private async ensureClientToken(): Promise<ClientToken> {
    if (this.clientToken) {
      return this.clientToken
    }
    this.clientToken = await getClientToken(this.authEndpoint, this.docId)
    return this.clientToken
  }

  private async connect() {
    let clientToken = await this.ensureClientToken()
    this.setupWs(clientToken)
  }

  private bindWebsocket(websocket: WebSocket) {
    if (this.websocket) {
      this.websocket.close()
      this.websocket.onopen = null
      this.websocket.onmessage = null
      this.websocket.onclose = null
      this.websocket.onerror = null
    }

    this.websocket = websocket
    this.websocket.binaryType = 'arraybuffer'
    this.websocket.onopen = this.websocketOpen.bind(this)
    this.websocket.onmessage = this.receiveMessage.bind(this)
    this.websocket.onclose = this.websocketClose.bind(this)
    this.websocket.onerror = this.websocketError.bind(this)
  }

  private setupWs(clientToken: ClientToken) {
    let url = clientToken.url + `/${clientToken.docId}`
    if (clientToken.token) {
      url = url + `?token=${clientToken.token}`
    }

    this.setStatus({ status: 'connecting' })
    const websocket = new (this.WebSocketPolyfill || WebSocket)(url)
    this.bindWebsocket(websocket)
  }

  private syncStep1() {
    const encoder = encoding.createEncoder()
    encoding.writeVarUint(encoder, messageSync)
    syncProtocol.writeSyncStep1(encoder, this.doc)
    this.websocket?.send(encoding.toUint8Array(encoder))
  }

  private receiveSyncMessage(decoder: decoding.Decoder) {
    const encoder = encoding.createEncoder()
    encoding.writeVarUint(encoder, messageSync)
    const syncMessageType = syncProtocol.readSyncMessage(decoder, encoder, this.doc, this)
    if (syncMessageType === syncProtocol.messageYjsSyncStep2) {
      this.setSynced(true)
    }

    if (encoding.length(encoder) > 1) {
      this.websocket?.send(encoding.toUint8Array(encoder))
    }
  }

  private websocketOpen() {
    this.setStatus({ status: 'connected' })
    this.syncStep1()
  }

  private receiveMessage(event: MessageEvent) {
    let message: Uint8Array = new Uint8Array(event.data)
    const decoder = decoding.createDecoder(message)
    const messageType = decoding.readVarUint(decoder)
    switch (messageType) {
      case messageSync:
        this.receiveSyncMessage(decoder)
        break
      default:
        break
    }
  }

  private websocketClose(event: CloseEvent) {
    this.setSynced(false)
    this.setStatus({ status: 'disconnected' })
  }

  private websocketError(event: Event) {
    // trigger a reconnect
    console.error('websocket error', event)
  }

  addEventListener(
    type: YSweetEvent,
    listener: EventListener,
    options?: boolean | AddEventListenerOptions,
  ): void {
    super.addEventListener(type, listener, options)
  }

  removeEventListener(
    type: YSweetEvent,
    listener: EventListener,
    options?: boolean | EventListenerOptions,
  ): void {
    super.removeEventListener(type, listener, options)
  }

  protected emit(eventName: YSweetEvent, data: any = null): void {
    this.dispatchEvent(new CustomEvent(eventName, { detail: data }))
  }

  private setSynced(state: boolean) {
    if (this.synced !== state) {
      this.synced = state
      this.emit('sync', state)
    }
  }

  private setStatus(status: YSweetStatus) {
    if (this.status.status !== status.status) {
      this.status = status
      this.emit('status', status)
    }
  }

  public destroy() {
    if (this.websocket) {
      this.websocket.close()
    }
  }

  /** Aliases for compatibility with y-websocket provider. */

  private _on(type: YSweetEvent, listener: (d: any) => void, once?: boolean): void {
    if ((type as any) === 'synced') {
      // Yjs emits both 'sync' and 'synced' events for legacy reasons. We only emit 'sync', but if
      // we attempt to subscribe to 'synced', we rewrite it to 'sync' to maintain compatibility.
      // Ref: https://github.com/yjs/y-websocket/blob/aa4220407bda51ab6282d1291de6493c136c2089/src/y-websocket.js#L404
      type = 'sync'
    }
    this.addEventListener(type, (d: Event) => listener((d as CustomEvent).detail), { once })
  }

  on(type: YSweetEvent, listener: (d: any) => void): void {
    this._on(type, listener)
  }

  once(type: YSweetEvent, listener: (d: any) => void): void {
    this._on(type, listener, true)
  }

  off(type: YSweetEvent, listener: (d: any) => void): void {
    this.removeEventListener(type, listener)
  }
}
