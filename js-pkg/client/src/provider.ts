import { ClientToken } from '@y-sweet/sdk'
import * as decoding from 'lib0/decoding'
import * as encoding from 'lib0/encoding'
import * as awarenessProtocol from 'y-protocols/awareness'
import * as syncProtocol from 'y-protocols/sync'
import * as Y from 'yjs'
import { Sleeper } from './sleeper'
import {
  EVENT_CONNECTION_CLOSE,
  EVENT_CONNECTION_ERROR,
  WebSocketCompatLayer,
  YWebsocketEvent,
} from './ws-status'

const MESSAGE_SYNC = 0
const MESSAGE_QUERY_AWARENESS = 3
const MESSAGE_AWARENESS = 1
const MESSAGE_SYNC_STATUS = 102

const RETRIES_BEFORE_TOKEN_REFRESH = 3
const DELAY_MS_BEFORE_RECONNECT = 500
const DELAY_MS_BEFORE_RETRY_TOKEN_REFRESH = 3_000

/** Amount of time without receiving any message that we should send a MESSAGE_SYNC_STATUS message. */
const MAX_TIMEOUT_BETWEEN_HEARTBEATS = 2_000

/**
 * Amount of time after sending a MESSAGE_SYNC_STATUS message that we should close the connection
 * unless any message has been received.
 **/
const MAX_TIMEOUT_WITHOUT_RECEIVING_HEARTBEAT = 3_000

// Note: These should not conflict with y-websocket's events, defined in `ws-status.ts`.
export const EVENT_LOCAL_CHANGES = 'local-changes'
export const EVENT_CONNECTION_STATUS = 'connection-status'

type YSweetEvent = typeof EVENT_LOCAL_CHANGES | typeof EVENT_CONNECTION_STATUS

/** The provider is offline because it has not been asked to connect or has been disconnected by the application. */
export const STATUS_OFFLINE = 'offline'

/** The provider is attempting to connect. */
export const STATUS_CONNECTING = 'connecting'

/** The provider is in an error state and will attempt to reconnect after a delay. */
export const STATUS_ERROR = 'error'

/** The provider is connected but has not yet completed the handshake. */
export const STATUS_HANDSHAKING = 'handshaking'

/** The provider is connected and has completed the handshake. */
export const STATUS_CONNECTED = 'connected'

export type YSweetStatus =
  | typeof STATUS_OFFLINE
  | typeof STATUS_CONNECTED
  | typeof STATUS_CONNECTING
  | typeof STATUS_ERROR
  | typeof STATUS_HANDSHAKING

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
  /** Awareness protocol instance. */
  public awareness: awarenessProtocol.Awareness

  /** Current client token. */
  public clientToken: ClientToken | null = null

  /** Whether the local document has unsynced changes. */
  public hasLocalChanges: boolean = true

  /** Connection status. */
  public status: YSweetStatus = STATUS_OFFLINE

  private websocket: WebSocket | null = null
  private WebSocketPolyfill: WebSocketPolyfillType
  private listeners: Map<YSweetEvent | YWebsocketEvent, Set<EventListener>> = new Map()

  private localVersion: number = 0
  private ackedVersion: number = -1

  /** Whether a (re)connect loop is currently running. This acts as a lock to prevent two concurrent connect loops. */
  private isConnecting: boolean = false

  private heartbeatHandle: ReturnType<typeof setTimeout> | null = null
  private connectionTimeoutHandle: ReturnType<typeof setTimeout> | null = null

  private reconnectSleeper: Sleeper | null = null

  constructor(
    private authEndpoint: AuthEndpoint,
    private docId: string,
    private doc: Y.Doc,
    extraOptions: Partial<YSweetProviderParams> = {},
  ) {
    if (extraOptions.initialClientToken) {
      this.clientToken = extraOptions.initialClientToken
    }

    // Sets up some event handlers for y-websocket compatibility.
    new WebSocketCompatLayer(this)

    this.awareness = extraOptions.awareness ?? new awarenessProtocol.Awareness(doc)
    this.awareness.on('update', this.handleAwarenessUpdate.bind(this))
    this.WebSocketPolyfill = extraOptions.WebSocketPolyfill || WebSocket

    this.online = this.online.bind(this)
    this.offline = this.offline.bind(this)
    if (typeof window !== 'undefined') {
      window.addEventListener('offline', this.offline)
      window.addEventListener('online', this.online)
    }

    doc.on('update', this.update.bind(this))

    if (extraOptions.connect !== false) {
      this.connect()
    }
  }

  private offline() {
    // When the browser indicates that we are offline, we immediately
    // probe the connection status.
    // This accelerates the process of discovering we are offline, but
    // doesn't mean we entirely trust the browser, since it can be wrong
    // (e.g. in the case that the connection is over localhost).
    this.checkSync()
  }

  private online() {
    if (this.reconnectSleeper) {
      this.reconnectSleeper.wake()
    }
  }

  private clearHeartbeat() {
    if (this.heartbeatHandle) {
      clearTimeout(this.heartbeatHandle)
      this.heartbeatHandle = null
    }
  }

  private setHeartbeat() {
    if (this.heartbeatHandle) {
      return
    }
    this.heartbeatHandle = setTimeout(() => {
      this.checkSync()
      this.heartbeatHandle = null
    }, MAX_TIMEOUT_BETWEEN_HEARTBEATS)
  }

  private clearConnectionTimeout() {
    if (this.connectionTimeoutHandle) {
      clearTimeout(this.connectionTimeoutHandle)
      this.connectionTimeoutHandle = null
    }
  }

  private setConnectionTimeout() {
    if (this.connectionTimeoutHandle) {
      return
    }
    this.connectionTimeoutHandle = setTimeout(() => {
      if (this.websocket) {
        this.websocket.close()
        this.setStatus(STATUS_ERROR)
        this.connect()
      }
      this.connectionTimeoutHandle = null
    }, MAX_TIMEOUT_WITHOUT_RECEIVING_HEARTBEAT)
  }

  private send(message: Uint8Array) {
    if (this.websocket?.readyState === this.WebSocketPolyfill.OPEN) {
      this.websocket.send(message)
    }
  }

  private updateSyncedState() {
    let hasLocalChanges = this.ackedVersion !== this.localVersion
    if (hasLocalChanges === this.hasLocalChanges) {
      return
    }

    this.hasLocalChanges = hasLocalChanges
    this.emit(EVENT_LOCAL_CHANGES, hasLocalChanges)
  }

  private setStatus(status: YSweetStatus) {
    if (this.status === status) {
      return
    }

    this.status = status
    this.emit(EVENT_CONNECTION_STATUS, status)
  }

  private update(update: Uint8Array, origin: YSweetProvider) {
    if (origin !== this) {
      const encoder = encoding.createEncoder()
      encoding.writeVarUint(encoder, MESSAGE_SYNC)
      syncProtocol.writeUpdate(encoder, update)
      this.send(encoding.toUint8Array(encoder))

      this.localVersion += 1
      this.checkSync()
    }
  }

  private checkSync() {
    const encoder = encoding.createEncoder()
    encoding.writeVarUint(encoder, MESSAGE_SYNC_STATUS)

    const versionEncoder = encoding.createEncoder()
    encoding.writeVarUint(versionEncoder, this.localVersion)
    encoding.writeVarUint8Array(encoder, encoding.toUint8Array(versionEncoder))

    this.send(encoding.toUint8Array(encoder))

    this.updateSyncedState()
    this.setConnectionTimeout()
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

  /**
   * Attempts to connect to the websocket.
   * Returns a promise that resolves to true if the connection was successful, or false if the connection failed.
   */
  private attemptToConnect(clientToken: ClientToken): Promise<boolean> {
    let promise = new Promise<boolean>((resolve) => {
      let statusListener = (event: YSweetStatus) => {
        if (event === STATUS_CONNECTED) {
          this.off(EVENT_CONNECTION_STATUS, statusListener)
          resolve(true)
        } else if (event === STATUS_ERROR) {
          this.off(EVENT_CONNECTION_STATUS, statusListener)
          resolve(false)
        }
      }

      this.on(EVENT_CONNECTION_STATUS, statusListener)
    })

    let url = this.generateUrl(clientToken)
    this.setStatus(STATUS_CONNECTING)
    const websocket = new (this.WebSocketPolyfill || WebSocket)(url)
    this.bindWebsocket(websocket)

    return promise
  }

  public async connect(): Promise<void> {
    if (this.isConnecting) {
      console.warn('connect() called while a connect loop is already running.')
      return
    }

    this.isConnecting = true
    this.setStatus(STATUS_CONNECTING)

    while (![STATUS_OFFLINE, STATUS_CONNECTED].includes(this.status)) {
      this.setStatus(STATUS_CONNECTING)
      let clientToken
      try {
        clientToken = await this.ensureClientToken()
      } catch (e) {
        console.warn('Failed to get client token', e)
        this.setStatus(STATUS_ERROR)
        this.reconnectSleeper = new Sleeper(DELAY_MS_BEFORE_RETRY_TOKEN_REFRESH)
        await this.reconnectSleeper.sleep()
        continue
      }

      for (let i = 0; i < RETRIES_BEFORE_TOKEN_REFRESH; i++) {
        if (await this.attemptToConnect(clientToken)) {
          break
        }

        this.reconnectSleeper = new Sleeper(DELAY_MS_BEFORE_RECONNECT)
        await this.reconnectSleeper.sleep()
      }

      // Delete the current client token to force a token refresh on the next attempt.
      this.clientToken = null
    }

    this.isConnecting = false
  }

  public disconnect() {
    if (this.websocket) {
      this.websocket.close()
    }

    this.setStatus(STATUS_OFFLINE)
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

  generateUrl(clientToken: ClientToken) {
    const url = clientToken.url + `/${clientToken.docId}`
    if (clientToken.token) {
      return `${url}?token=${clientToken.token}`
    }
    return url
  }

  private syncStep1() {
    const encoder = encoding.createEncoder()
    encoding.writeVarUint(encoder, MESSAGE_SYNC)
    syncProtocol.writeSyncStep1(encoder, this.doc)
    this.send(encoding.toUint8Array(encoder))
  }

  private receiveSyncMessage(decoder: decoding.Decoder) {
    const encoder = encoding.createEncoder()
    encoding.writeVarUint(encoder, MESSAGE_SYNC)
    const syncMessageType = syncProtocol.readSyncMessage(decoder, encoder, this.doc, this)
    if (syncMessageType === syncProtocol.messageYjsSyncStep2) {
      this.setStatus(STATUS_CONNECTED)
    }

    if (encoding.length(encoder) > 1) {
      this.send(encoding.toUint8Array(encoder))
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

    this.send(encoding.toUint8Array(encoder))
  }

  private broadcastAwareness() {
    if (this.awareness.getLocalState() !== null) {
      const encoderAwarenessState = encoding.createEncoder()
      encoding.writeVarUint(encoderAwarenessState, MESSAGE_AWARENESS)
      encoding.writeVarUint8Array(
        encoderAwarenessState,
        awarenessProtocol.encodeAwarenessUpdate(this.awareness, [this.doc.clientID]),
      )
      this.send(encoding.toUint8Array(encoderAwarenessState))
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
    this.setStatus(STATUS_HANDSHAKING)
    this.syncStep1()
    this.checkSync()
    this.broadcastAwareness()
    this.setHeartbeat()
  }

  private receiveMessage(event: MessageEvent) {
    this.clearConnectionTimeout()
    this.clearHeartbeat()
    this.setHeartbeat()

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
        let lastSyncBytes = decoding.readVarUint8Array(decoder)
        let d2 = decoding.createDecoder(lastSyncBytes)
        let ackedVersion = decoding.readVarUint(d2)
        this.ackedVersion = Math.max(this.ackedVersion, ackedVersion)
        this.updateSyncedState()
        break
      default:
        break
    }
  }

  private websocketClose(event: CloseEvent) {
    this.emit(EVENT_CONNECTION_CLOSE, event)
    this.setStatus(STATUS_ERROR)
    this.connect()

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
    this.emit(EVENT_CONNECTION_ERROR, event)
    this.setStatus(STATUS_ERROR)

    this.connect()
  }

  public emit(eventName: YSweetEvent | YWebsocketEvent, data: any = null): void {
    const listeners = this.listeners.get(eventName) || new Set()
    for (const listener of listeners) {
      listener(data)
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

    this.send(encoding.toUint8Array(encoder))
  }

  public destroy() {
    if (this.websocket) {
      this.websocket.close()
    }

    awarenessProtocol.removeAwarenessStates(this.awareness, [this.doc.clientID], 'window unload')

    if (typeof window !== 'undefined') {
      window.removeEventListener('offline', this.offline)
      window.removeEventListener('online', this.online)
    }
  }

  private _on(
    type: YSweetEvent | YWebsocketEvent,
    listener: (d: any) => void,
    once?: boolean,
  ): void {
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

  public on(type: YSweetEvent | YWebsocketEvent, listener: (d: any) => void): void {
    this._on(type, listener)
  }

  public once(type: YSweetEvent | YWebsocketEvent, listener: (d: any) => void): void {
    this._on(type, listener, true)
  }

  public off(type: YSweetEvent | YWebsocketEvent, listener: (d: any) => void): void {
    const listeners = this.listeners.get(type)
    if (listeners) {
      listeners.delete(listener)
    }
  }

  /**
   * Whether the provider should attempt to connect.
   *
   * @deprecated use provider.status !== 'offline' instead, or call `provider.connect()` / `provider.disconnect()` to set.
   */
  get shouldConnect(): boolean {
    return this.status !== STATUS_OFFLINE
  }

  /**
   * Whether the underlying websocket is connected.
   *
   * @deprecated use provider.status === 'connected' || provider.status === 'handshaking' instead.
   */
  get wsconnected() {
    return this.status === STATUS_CONNECTED || this.status === STATUS_HANDSHAKING
  }

  /**
   * Whether the underlying websocket is connecting.
   *
   * @deprecated use provider.status === 'connecting' instead.
   */
  get wsconnecting() {
    return this.status === STATUS_CONNECTING
  }

  /**
   * Whether the document is synced. (For compatibility with y-websocket.)
   *
   * @deprecated use provider.status === 'connected' instead.
   * */
  get synced() {
    return this.status === STATUS_CONNECTED
  }
}
