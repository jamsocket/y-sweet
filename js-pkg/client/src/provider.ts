import { encodeClientToken, type ClientToken } from '@y-sweet/sdk'
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
import { createIndexedDBProvider, IndexedDBProvider } from './indexeddb'

const MESSAGE_SYNC = 0
const MESSAGE_QUERY_AWARENESS = 3
const MESSAGE_AWARENESS = 1
const MESSAGE_SYNC_STATUS = 102

const RETRIES_BEFORE_TOKEN_REFRESH = 3
const DELAY_MS_BEFORE_RECONNECT = 500
const DELAY_MS_BEFORE_RETRY_TOKEN_REFRESH = 3_000

const BACKOFF_BASE = 1.1
const MAX_BACKOFF_COEFFICIENT = 10

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

  /**
   * If set, document state is stored locally for offline use and faster re-opens.
   * Defaults to `false`; set to `true` to enable.
   */
  offlineSupport?: boolean

  /** Whether to show the debugger link. Defaults to true. */
  showDebuggerLink?: boolean
}

function validateClientToken(clientToken: ClientToken, docId: string) {
  if (clientToken.docId !== docId) {
    throw new Error(
      `ClientToken docId does not match YSweetProvider docId: ${clientToken.docId} !== ${docId}`,
    )
  }
}

async function getClientToken(authEndpoint: AuthEndpoint, docId: string): Promise<ClientToken> {
  if (typeof authEndpoint === 'function') {
    const clientToken = await authEndpoint()
    validateClientToken(clientToken, docId)
    return clientToken
  }

  const body = JSON.stringify({ docId: docId })
  const res = await fetch(authEndpoint, {
    method: 'POST',
    body,
    headers: { 'Content-Type': 'application/json' },
  })

  if (!res.ok) {
    throw new Error(`Failed to get client token: ${res.status} ${res.statusText}`)
  }

  const clientToken = await res.json()
  validateClientToken(clientToken, docId)

  return clientToken
}

export class YSweetProvider {
  /** Awareness protocol instance. */
  public awareness: awarenessProtocol.Awareness

  /** Current client token. */
  public clientToken: ClientToken | null = null

  /** Connection status. */
  public status: YSweetStatus = STATUS_OFFLINE

  private websocket: WebSocket | null = null
  private WebSocketPolyfill: WebSocketPolyfillType
  private listeners: Map<YSweetEvent | YWebsocketEvent, Set<EventListener>> = new Map()

  private localVersion: number = 0
  private ackedVersion: number = -1

  /** Whether we are currently in the process of connecting. */
  private isConnecting: boolean = false

  private heartbeatHandle: ReturnType<typeof setTimeout> | null = null
  private connectionTimeoutHandle: ReturnType<typeof setTimeout> | null = null

  private reconnectSleeper: Sleeper | null = null
  private showDebuggerLink = true

  private indexedDBProvider: IndexedDBProvider | null = null

  private retries: number = 0

  /**
   * Older versions of the Y-Sweet server did not support the sync message, and would ignore it.
   * This may lead to the client thinking the server is offline, when really it just doesn't
   * know how to return a heartbeat.
   *
   * Eventually, we will build protocol version negotiation into the handshake. Until then, we
   * use a simple approach: until we receive the first sync message back, we assume the server
   * is an older version for the purpose of the heartbeat logic.
   */
  private receivedAtLeastOneSyncResponse: boolean = false

  /** @deprecated */
  get debugUrl() {
    if (!this.clientToken) return null

    const payload = encodeClientToken(this.clientToken)
    return `https://debugger.y-sweet.dev/?payload=${payload}`
  }

  constructor(
    private authEndpoint: AuthEndpoint,
    private docId: string,
    private doc: Y.Doc,
    extraOptions: Partial<YSweetProviderParams> = {},
  ) {
    if (extraOptions.initialClientToken) {
      this.clientToken = extraOptions.initialClientToken
      validateClientToken(this.clientToken, this.docId)
    }

    this.showDebuggerLink = extraOptions.showDebuggerLink !== false

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

    if (extraOptions.offlineSupport === true && typeof indexedDB !== 'undefined') {
      ;(async () => {
        this.indexedDBProvider = await createIndexedDBProvider(doc, docId)
      })()
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

  private resetHeartbeat() {
    this.clearHeartbeat()
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

    if (!this.receivedAtLeastOneSyncResponse) {
      // Until we receive the first sync response on the connection, we assume
      // the server is an older version.
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

  private incrementLocalVersion() {
    // We need to increment the local version before we emit, so that event
    // listeners see the right hasLocalChanges value.
    let emit = !this.hasLocalChanges
    this.localVersion += 1

    if (emit) {
      this.emit(EVENT_LOCAL_CHANGES, true)
    }
  }

  private updateAckedVersion(version: number) {
    // The version _should_ never go backwards, but we guard for that in case it does.
    version = Math.max(version, this.ackedVersion)

    // We need to increment the local version before we emit, so that event
    // listeners see the right hasLocalChanges value.
    let emit = this.hasLocalChanges && version === this.localVersion
    this.ackedVersion = version

    if (emit) {
      this.emit(EVENT_LOCAL_CHANGES, false)
    }

    this.receivedAtLeastOneSyncResponse = true
  }

  private setStatus(status: YSweetStatus) {
    if (this.status === status) {
      return
    }

    this.status = status
    this.emit(EVENT_CONNECTION_STATUS, status)
  }

  private update(update: Uint8Array, origin: YSweetProvider | IndexedDBProvider) {
    if (origin === this) {
      // Ignore updates that came through the Y-Sweet Provider (i.e. not local changes)
      return
    }

    if (this.indexedDBProvider && origin === this.indexedDBProvider) {
      // Ignore updates from our own IndexedDB provider.
      return
    }

    // If we made it here, the update came from local changes.
    // Warn if the client holds a read-only token.
    const authorization = this.clientToken?.authorization
    if (authorization === 'read-only') {
      console.warn(
        'Client with read-only authorization attempted to write to the Yjs document. These changes may appear locally, but they will not be applied to the shared document.',
      )
    }

    const encoder = encoding.createEncoder()
    encoding.writeVarUint(encoder, MESSAGE_SYNC)
    syncProtocol.writeUpdate(encoder, update)
    this.send(encoding.toUint8Array(encoder))

    this.incrementLocalVersion()
    this.checkSync()
  }

  private checkSync() {
    const encoder = encoding.createEncoder()
    encoding.writeVarUint(encoder, MESSAGE_SYNC_STATUS)

    const versionEncoder = encoding.createEncoder()
    encoding.writeVarUint(versionEncoder, this.localVersion)
    encoding.writeVarUint8Array(encoder, encoding.toUint8Array(versionEncoder))

    this.send(encoding.toUint8Array(encoder))
    this.setConnectionTimeout()
  }

  private async ensureClientToken(): Promise<ClientToken> {
    if (this.clientToken === null) {
      this.clientToken = await getClientToken(this.authEndpoint, this.docId)
    }
    return this.clientToken
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

    const lastDebugUrl = this.debugUrl

    connecting: while (![STATUS_OFFLINE, STATUS_CONNECTED].includes(this.status)) {
      this.setStatus(STATUS_CONNECTING)
      let clientToken
      try {
        clientToken = await this.ensureClientToken()
      } catch (e) {
        console.warn('Failed to get client token', e)
        this.setStatus(STATUS_ERROR)
        let timeout =
          DELAY_MS_BEFORE_RETRY_TOKEN_REFRESH *
          Math.min(MAX_BACKOFF_COEFFICIENT, Math.pow(BACKOFF_BASE, this.retries))
        this.retries += 1
        this.reconnectSleeper = new Sleeper(timeout)
        await this.reconnectSleeper.sleep()
        continue
      }

      for (let i = 0; i < RETRIES_BEFORE_TOKEN_REFRESH; i++) {
        if (await this.attemptToConnect(clientToken)) {
          this.retries = 0
          break connecting
        }

        let timeout =
          DELAY_MS_BEFORE_RECONNECT *
          Math.min(MAX_BACKOFF_COEFFICIENT, Math.pow(BACKOFF_BASE, this.retries))
        this.retries += 1
        this.reconnectSleeper = new Sleeper(timeout)
        await this.reconnectSleeper.sleep()
      }

      // Delete the current client token to force a token refresh on the next attempt.
      this.clientToken = null
    }

    this.isConnecting = false

    if (this.showDebuggerLink && lastDebugUrl !== this.debugUrl) {
      console.log(
        `%cOpen this in Y-Sweet Debugger ⮕ ${this.debugUrl}`,
        'font-size: 1.5em; display: block; padding: 10px;',
      )
      console.log(
        '%cTo hide the debugger link, set the showDebuggerLink option to false when creating the provider',
        'font-style: italic;',
      )
    }
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
    this.resetHeartbeat()

    this.receivedAtLeastOneSyncResponse = false
  }

  private receiveMessage(event: MessageEvent) {
    this.clearConnectionTimeout()
    this.resetHeartbeat()

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
        this.updateAckedVersion(ackedVersion)
        break
      default:
        break
    }
  }

  private websocketClose(event: CloseEvent) {
    this.emit(EVENT_CONNECTION_CLOSE, event)
    this.setStatus(STATUS_ERROR)
    this.clearHeartbeat()
    this.clearConnectionTimeout()
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
    this.clearHeartbeat()
    this.clearConnectionTimeout()

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

    if (this.indexedDBProvider) {
      this.indexedDBProvider.destroy()
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
   * Whether the document has local changes.
   */
  get hasLocalChanges() {
    return this.ackedVersion !== this.localVersion
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
