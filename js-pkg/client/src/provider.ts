import { ClientToken } from '@y-sweet/sdk'
import * as decoding from 'lib0/decoding'
import * as encoding from 'lib0/encoding'
import * as awarenessProtocol from 'y-protocols/awareness'
import * as syncProtocol from 'y-protocols/sync'
import * as Y from 'yjs'

const MESSAGE_SYNC = 0
const MESSAGE_QUERY_AWARENESS = 3
const MESSAGE_AWARENESS = 1
const MESSAGE_SYNC_STATUS = 102

const EVENT_STATUS = 'status'

/** Fired when the _initial_ sync is complete. Only refired after that if there is a reconnection. */
const EVENT_SYNC = 'sync'
const EVENT_CONNECTION_CLOSE = 'connection-close'
const EVENT_CONNECTION_ERROR = 'connection-error'
/** Fired every time the sync status changes. */
const EVENT_SYNC_STATUS = 'sync-status'

/**
 * Note: this should always be a superset of y-websocket's valid events.
 * Ref: https://github.com/yjs/y-websocket/blob/aa4220407bda51ab6282d1291de6493c136c2089/README.md?plain=1#L119-L125
 */
type YSweetEvent =
  | typeof EVENT_STATUS
  | typeof EVENT_SYNC
  | typeof EVENT_CONNECTION_CLOSE
  | typeof EVENT_CONNECTION_ERROR
  | typeof EVENT_SYNC_STATUS

const STATUS_CONNECTED = 'connected'
const STATUS_DISCONNECTED = 'disconnected'
const STATUS_CONNECTING = 'connecting'

const RETRIES_BEFORE_TOKEN_REFRESH = 3
const DELAY_MS_BEFORE_RECONNECT = 500
const DELAY_MS_BEFORE_RETRY_TOKEN_REFRESH = 3000

/**
 * Note: this should always be a superset of y-websocket's valid statuses.
 * Ref: https://github.com/yjs/y-websocket/blob/aa4220407bda51ab6282d1291de6493c136c2089/README.md?plain=1#L121
 */
type YSweetStatus = {
  status: typeof STATUS_CONNECTED | typeof STATUS_DISCONNECTED | typeof STATUS_CONNECTING
}

type WebSocketPolyfillType = {
  new(url: string | URL): WebSocket
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

  /** Maximum backoff time when retrying */
  maxBackoffTime?: number

  /** An initial client token to use (skips the first auth request if provided.) */
  initialClientToken?: ClientToken
}

async function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms))
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

  if (!res.ok) {
    throw new Error(`Failed to get client token: ${res.status} ${res.statusText}`)
  }

  const clientToken = await res.json()

  if (clientToken.docId !== roomname) {
    throw new Error(
      `Client token docId does not match roomname: ${clientToken.docId} !== ${roomname}`,
    )
  }

  return clientToken
}

export class YSweetProvider {
  private websocket: WebSocket | null = null
  public clientToken: ClientToken | null = null
  public synced: boolean = false
  private status: YSweetStatus = { status: STATUS_DISCONNECTED }
  public awareness: awarenessProtocol.Awareness
  private WebSocketPolyfill: WebSocketPolyfillType
  private listeners: Map<YSweetEvent, Set<EventListener>> = new Map()

  private lastSyncSent: number = 0
  private lastSyncAcked: number = 0

  /** Whether we should attempt to connect if we are in a disconnected state. */
  private shouldConnect: boolean

  /** Whether we are currently in the process of connecting. */
  private isConnecting: boolean = false

  constructor(
    private authEndpoint: AuthEndpoint,
    private docId: string,
    private doc: Y.Doc,
    extraOptions: Partial<YSweetProviderParams> = {},
  ) {
    this.shouldConnect = extraOptions.connect !== false

    if (extraOptions.initialClientToken) {
      this.clientToken = extraOptions.initialClientToken
    }

    this.awareness = extraOptions.awareness ?? new awarenessProtocol.Awareness(doc)
    this.awareness.on('update', this.handleAwarenessUpdate.bind(this))
    this.WebSocketPolyfill = extraOptions.WebSocketPolyfill || WebSocket

    doc.on('update', this.update.bind(this))

    if (this.shouldConnect) {
      this.connect()
    }
  }

  private updateSyncedState() {
    if (this.lastSyncAcked === this.lastSyncSent) {
      this.synced = true
      this.emit(EVENT_SYNC_STATUS, true)
    } else {
      this.synced = false
      this.emit(EVENT_SYNC_STATUS, false)
    }
  }

  private update(update: Uint8Array, origin: YSweetProvider) {
    if (!this.websocket) {
      console.warn('Websocket not connected')
      return
    }

    if (origin !== this) {
      const encoder = encoding.createEncoder()
      encoding.writeVarUint(encoder, MESSAGE_SYNC)
      syncProtocol.writeUpdate(encoder, update)
      this.websocket.send(encoding.toUint8Array(encoder))

      this.checkSync()
    }
  }

  private checkSync() {
    if (!this.websocket) {
      console.warn('Websocket not connected')
      return
    }

    this.lastSyncSent += 1
    const encoder = encoding.createEncoder()
    encoding.writeVarUint(encoder, MESSAGE_SYNC_STATUS)

    const versionEncoder = encoding.createEncoder()
    encoding.writeVarUint(versionEncoder, this.lastSyncSent)

    encoding.writeVarUint8Array(encoder, encoding.toUint8Array(versionEncoder))
    this.websocket.send(encoding.toUint8Array(encoder))

    this.updateSyncedState()
  }

  private async ensureClientToken(): Promise<ClientToken> {
    if (this.clientToken) {
      return this.clientToken
    }
    if (typeof this.authEndpoint === 'string') {
      this.clientToken = await getClientToken(this.authEndpoint, this.docId)
      return this.clientToken
    } else {
      this.clientToken = await this.authEndpoint()
      return this.clientToken
    }
  }

  async connect() {
    if (this.isConnecting) {
      return
    }

    this.shouldConnect = true
    this.isConnecting = true

    while (this.shouldConnect) {
      let clientToken
      try {
        clientToken = await this.ensureClientToken()
      } catch (e) {
        console.warn('Failed to get client token', e)
        await sleep(DELAY_MS_BEFORE_RETRY_TOKEN_REFRESH)
        continue
      }

      for (let i = 0; i < RETRIES_BEFORE_TOKEN_REFRESH && this.shouldConnect; i++) {
        let resolve_: (value: boolean) => void

        let promise = new Promise((resolve) => {
          resolve_ = resolve
        })

        let statusListener = (event: YSweetStatus) => {
          if (event.status === STATUS_CONNECTED) {
            resolve_(true)
          } else if (event.status === STATUS_DISCONNECTED) {
            resolve_(false)
          }
        }

        let errorListener = () => {
          resolve_(false)
        }

        this.on(EVENT_STATUS, statusListener)
        this.on(EVENT_CONNECTION_ERROR, errorListener)

        this.setupWs(clientToken)

        if ((await promise) === true) {
          this.isConnecting = false
          return
        }

        this.off(EVENT_STATUS, statusListener)
        this.off(EVENT_CONNECTION_ERROR, errorListener)

        await sleep(DELAY_MS_BEFORE_RECONNECT)
      }

      // Delete the current client token to force a token refresh on the next attempt.
      this.clientToken = null
    }

    this.isConnecting = false
  }

  disconnect() {
    if (this.websocket) {
      this.websocket.close()
    }
    this.shouldConnect = false
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

    this.setStatus({ status: STATUS_CONNECTING })
    const websocket = new (this.WebSocketPolyfill || WebSocket)(url)
    this.bindWebsocket(websocket)
  }

  private syncStep1() {
    const encoder = encoding.createEncoder()
    encoding.writeVarUint(encoder, MESSAGE_SYNC)
    syncProtocol.writeSyncStep1(encoder, this.doc)
    this.websocket?.send(encoding.toUint8Array(encoder))
  }

  private receiveSyncMessage(decoder: decoding.Decoder) {
    const encoder = encoding.createEncoder()
    encoding.writeVarUint(encoder, MESSAGE_SYNC)
    const syncMessageType = syncProtocol.readSyncMessage(decoder, encoder, this.doc, this)
    if (syncMessageType === syncProtocol.messageYjsSyncStep2) {
      this.setSynced(true)
    }

    if (encoding.length(encoder) > 1) {
      this.websocket?.send(encoding.toUint8Array(encoder))
    }
  }

  private queryAwareness() {
    const encoder = encoding.createEncoder()
    encoding.writeVarUint(encoder, MESSAGE_QUERY_AWARENESS)
    encoding.writeVarUint8Array(
      encoder,
      awarenessProtocol.encodeAwarenessUpdate(
        this.awareness,
        Array.from(this.awareness.getStates().keys()),
      ),
    )

    this.websocket?.send(encoding.toUint8Array(encoder))
  }

  private broadcastAwareness() {
    if (this.awareness.getLocalState() !== null) {
      const encoderAwarenessState = encoding.createEncoder()
      encoding.writeVarUint(encoderAwarenessState, MESSAGE_AWARENESS)
      encoding.writeVarUint8Array(
        encoderAwarenessState,
        awarenessProtocol.encodeAwarenessUpdate(this.awareness, [this.doc.clientID]),
      )
      this.websocket?.send(encoding.toUint8Array(encoderAwarenessState))
    }
  }

  private updateAwareness(decoder: decoding.Decoder) {
    awarenessProtocol.applyAwarenessUpdate(
      this.awareness,
      decoding.readVarUint8Array(decoder),
      this,
    )
  }

  private websocketOpen() {
    this.setStatus({ status: STATUS_CONNECTED })
    this.syncStep1()
    this.broadcastAwareness()
  }

  private receiveMessage(event: MessageEvent) {
    let message: Uint8Array = new Uint8Array(event.data)
    const decoder = decoding.createDecoder(message)
    const messageType = decoding.readVarUint(decoder)
    switch (messageType) {
      case MESSAGE_SYNC:
        this.receiveSyncMessage(decoder)
        break
      case MESSAGE_AWARENESS:
        this.updateAwareness(decoder)
        break
      case MESSAGE_QUERY_AWARENESS:
        this.queryAwareness()
        break
      case MESSAGE_SYNC_STATUS:
        this.lastSyncAcked = decoding.readVarUint(decoder)
        this.updateSyncedState()
        break
      default:
        break
    }
  }

  private websocketClose(event: CloseEvent) {
    this.emit(EVENT_CONNECTION_CLOSE, event)
    this.setSynced(false)
    this.setStatus({ status: STATUS_DISCONNECTED })

    if (this.shouldConnect && !this.isConnecting) {
      this.connect()
    }

    // Remove all awareness states except for our own.
    awarenessProtocol.removeAwarenessStates(
      this.awareness,
      Array.from(this.awareness.getStates().keys()).filter(
        (client) => client !== this.doc.clientID,
      ),
      this,
    )
  }

  private websocketError(event: Event) {
    this.emit('connection-error', event)

    if (this.shouldConnect && !this.isConnecting) {
      this.connect()
    }
  }

  protected emit(eventName: YSweetEvent, data: any = null): void {
    const listeners = this.listeners.get(eventName) || new Set()
    for (const listener of listeners) {
      listener(data)
    }
  }

  private setSynced(state: boolean) {
    if (this.synced !== state) {
      this.synced = state
      this.emit(EVENT_SYNC, state)
    }
  }

  private setStatus(status: YSweetStatus) {
    if (this.status.status !== status.status) {
      this.status = status
      this.emit('status', status)
    }
  }

  private handleAwarenessUpdate(
    { added, updated, removed }: { added: Array<any>; updated: Array<any>; removed: Array<any> },
    _origin: any,
  ) {
    const changedClients = added.concat(updated).concat(removed)
    const encoder = encoding.createEncoder()
    encoding.writeVarUint(encoder, MESSAGE_AWARENESS)
    encoding.writeVarUint8Array(
      encoder,
      awarenessProtocol.encodeAwarenessUpdate(this.awareness, changedClients),
    )

    this.websocket?.send(encoding.toUint8Array(encoder))
  }

  public destroy() {
    if (this.websocket) {
      this.websocket.close()
    }

    awarenessProtocol.removeAwarenessStates(this.awareness, [this.doc.clientID], 'window unload')
  }

  /** Aliases for compatibility with y-websocket provider. */

  private _on(type: YSweetEvent, listener: (d: any) => void, once?: boolean): void {
    if ((type as any) === 'synced') {
      // Yjs emits both 'sync' and 'synced' events for legacy reasons. We only emit 'sync', but if
      // we attempt to subscribe to 'synced', we rewrite it to 'sync' to maintain compatibility.
      // Ref: https://github.com/yjs/y-websocket/blob/aa4220407bda51ab6282d1291de6493c136c2089/src/y-websocket.js#L404
      type = EVENT_SYNC
    }

    if (!this.listeners.has(type)) {
      this.listeners.set(type, new Set())
    }
    if (once) {
      let listenerOnce = (d: any) => {
        listener(d)
        this.listeners.get(type)?.delete(listenerOnce)
      }
      this.listeners.get(type)?.add(listenerOnce)
    } else {
      this.listeners.get(type)?.add(listener)
    }
  }

  on(type: YSweetEvent, listener: (d: any) => void): void {
    this._on(type, listener)
  }

  once(type: YSweetEvent, listener: (d: any) => void): void {
    this._on(type, listener, true)
  }

  off(type: YSweetEvent, listener: (d: any) => void): void {
    const listeners = this.listeners.get(type)
    if (listeners) {
      listeners.delete(listener)
    }
  }
}
