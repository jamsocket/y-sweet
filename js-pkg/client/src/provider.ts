/**
 * Adapted from y-websocket
 *
 * https://raw.githubusercontent.com/yjs/y-websocket/master/src/y-websocket.js
 */

import * as Y from 'yjs' // eslint-disable-line
import type { ClientToken } from '@y-sweet/sdk'
import * as bc from 'lib0/broadcastchannel'
import * as time from 'lib0/time'
import * as encoding from 'lib0/encoding'
import * as decoding from 'lib0/decoding'
import * as syncProtocol from 'y-protocols/sync'
import * as authProtocol from 'y-protocols/auth'
import * as awarenessProtocol from 'y-protocols/awareness'
import { Observable } from 'lib0/observable'
import * as math from 'lib0/math'
import * as url from 'lib0/url'

export const messageSync = 0
export const messageQueryAwareness = 3
export const messageAwareness = 1
export const messageAuth = 2

export type HandlerFunction = (
  encoder: encoding.Encoder,
  decoder: decoding.Decoder,
  provider: YSweetProvider,
  emitSynced: boolean,
  messageType: number,
) => void

const messageHandlers: Array<HandlerFunction> = []

messageHandlers[messageSync] = (encoder, decoder, provider, emitSynced, _messageType) => {
  encoding.writeVarUint(encoder, messageSync)
  const syncMessageType = syncProtocol.readSyncMessage(decoder, encoder, provider.doc, provider)
  if (emitSynced && syncMessageType === syncProtocol.messageYjsSyncStep2 && !provider.synced) {
    provider.synced = true
  }
}

messageHandlers[messageQueryAwareness] = (
  encoder,
  _decoder,
  provider,
  _emitSynced,
  _messageType,
) => {
  encoding.writeVarUint(encoder, messageAwareness)
  encoding.writeVarUint8Array(
    encoder,
    awarenessProtocol.encodeAwarenessUpdate(
      provider.awareness,
      Array.from(provider.awareness.getStates().keys()),
    ),
  )
}

messageHandlers[messageAwareness] = (_encoder, decoder, provider, _emitSynced, _messageType) => {
  awarenessProtocol.applyAwarenessUpdate(
    provider.awareness,
    decoding.readVarUint8Array(decoder),
    provider,
  )
}

messageHandlers[messageAuth] = (_encoder, decoder, provider, _emitSynced, _messageType) => {
  authProtocol.readAuthMessage(decoder, provider.doc, (_ydoc, reason) =>
    permissionDeniedHandler(provider, reason),
  )
}

// @todo - this should depend on awareness.outdatedTime
const messageReconnectTimeout = 30000

// the number of times we try to reconnect before recreating the provider
const recreateThreshold = 5

const permissionDeniedHandler = (provider: YSweetProvider, reason: string) =>
  console.warn(`Permission denied to access ${provider.url}.\n${reason}`)

const readMessage = (
  provider: YSweetProvider,
  buf: Uint8Array,
  emitSynced: boolean,
): encoding.Encoder => {
  const decoder = decoding.createDecoder(buf)
  const encoder = encoding.createEncoder()
  const messageType = decoding.readVarUint(decoder)
  const messageHandler = provider.messageHandlers[messageType]
  if (/** @type {any} */ messageHandler) {
    messageHandler(encoder, decoder, provider, emitSynced, messageType)
  } else {
    console.error('Unable to compute message')
  }
  return encoder
}

const setupWS = (provider: YSweetProvider) => {
  if (provider.shouldConnect && provider.ws === null) {
    const websocket = new provider._WS(provider.url)
    websocket.binaryType = 'arraybuffer'
    provider.ws = websocket
    provider.wsconnecting = true
    provider.wsconnected = false
    provider.synced = false

    websocket.onmessage = (event) => {
      provider.wsLastMessageReceived = time.getUnixTime()
      const encoder = readMessage(provider, new Uint8Array(event.data), true)
      if (encoding.length(encoder) > 1) {
        websocket.send(encoding.toUint8Array(encoder))
      }
    }
    websocket.onerror = (event) => {
      provider.observable.emit('connection-error', [event, provider])
    }
    websocket.onclose = (event) => {
      provider.observable.emit('connection-close', [event, provider])
      provider.ws = null
      provider.wsconnecting = false
      if (provider.wsconnected) {
        provider.wsconnected = false
        provider.synced = false
        // update awareness (all users except local left)
        awarenessProtocol.removeAwarenessStates(
          provider.awareness,
          Array.from(provider.awareness.getStates().keys()).filter(
            (client) => client !== provider.doc.clientID,
          ),
          provider,
        )
        provider.observable.emit('status', [
          {
            status: 'disconnected',
          },
        ])
      } else {
        provider.wsUnsuccessfulReconnects++
      }

      if (provider.wsUnsuccessfulReconnects > recreateThreshold) {
        provider.destroy()
        while (provider.onFailureHandlers.length > 0) {
          provider.onFailureHandlers.pop()!()
        }
        return
      }

      // Start with no reconnect timeout and increase timeout by
      // using exponential backoff starting with 100ms
      setTimeout(
        setupWS,
        math.min(math.pow(2, provider.wsUnsuccessfulReconnects) * 100, provider.maxBackoffTime),
        provider,
      )
    }
    websocket.onopen = () => {
      provider.wsLastMessageReceived = time.getUnixTime()
      provider.wsconnecting = false
      provider.wsconnected = true
      provider.wsUnsuccessfulReconnects = 0
      provider.observable.emit('status', [
        {
          status: 'connected',
        },
      ])
      // always send sync step 1 when connected
      const encoder = encoding.createEncoder()
      encoding.writeVarUint(encoder, messageSync)
      syncProtocol.writeSyncStep1(encoder, provider.doc)
      websocket.send(encoding.toUint8Array(encoder))
      // broadcast local awareness state
      if (provider.awareness.getLocalState() !== null) {
        const encoderAwarenessState = encoding.createEncoder()
        encoding.writeVarUint(encoderAwarenessState, messageAwareness)
        encoding.writeVarUint8Array(
          encoderAwarenessState,
          awarenessProtocol.encodeAwarenessUpdate(provider.awareness, [provider.doc.clientID]),
        )
        websocket.send(encoding.toUint8Array(encoderAwarenessState))
      }
    }
    provider.observable.emit('status', [
      {
        status: 'connecting',
      },
    ])
  }
}

const broadcastMessage = (provider: YSweetProvider, buf: ArrayBuffer) => {
  const ws = provider.ws
  if (provider.wsconnected && ws && ws.readyState === ws.OPEN) {
    ws.send(buf)
  }
  if (provider.bcconnected) {
    bc.publish(provider.bcChannel, buf, provider)
  }
}

type WebSocketPolyfillType = {
  new (url: string | URL, protocols?: string | string[] | undefined): WebSocket
  prototype: WebSocket
  readonly CLOSED: number
  readonly CLOSING: number
  readonly CONNECTING: number
  readonly OPEN: number
}

export type AuthEndpoint = string | (() => Promise<ClientToken>)
export type YSweetProviderWithClientToken = YSweetProvider & {
  clientToken: ClientToken
}

export type YSweetProviderParams = {
  connect?: boolean
  awareness?: awarenessProtocol.Awareness
  params?: {
    [x: string]: string
  }
  WebSocketPolyfill?: WebSocketPolyfillType
  resyncInterval?: number
  maxBackoffTime?: number
  disableBc?: boolean
  observable?: Observable<string>
}

/**
 * Websocket Provider for Yjs. Creates a websocket connection to sync the shared document.
 * The document name is attached to the provided url. I.e. the following example
 * creates a websocket connection to http://localhost:1234/my-document-name
 *
 * @example
 * import * as Y from 'yjs'
 * import { YSweetProvider } from 'y-websocket'
 * const doc = new Y.Doc()
 * const provider = new YSweetProvider('http://localhost:1234', 'my-document-name', doc)
 */
export class YSweetProvider {
  onFailureHandlers: Array<() => void> = []
  maxBackoffTime: number
  bcChannel: string
  url: string
  roomname: string
  doc: Y.Doc
  observable: Observable<string>
  _WS: WebSocketPolyfillType
  awareness: awarenessProtocol.Awareness
  wsconnected: boolean
  wsconnecting: boolean
  bcconnected: boolean
  disableBc: boolean
  wsUnsuccessfulReconnects: number
  messageHandlers: Array<HandlerFunction>
  _synced: boolean
  ws: WebSocket | null
  wsLastMessageReceived: number
  shouldConnect: boolean
  _resyncInterval: ReturnType<typeof setInterval> | number // TODO: is setting this to 0 used as null?
  _bcSubscriber: Function
  _updateHandler: (arg0: Uint8Array, arg1: any, arg2: Y.Doc, arg3: Y.Transaction) => void
  _awarenessUpdateHandler: Function
  _unloadHandler: Function
  _checkInterval: ReturnType<typeof setInterval> | number

  /**
   * @param serverUrl - server url
   * @param roomname - room name
   * @param doc - Y.Doc instance
   * @param opts - options
   * @param opts.connect - connect option
   * @param opts.awareness - awareness protocol instance
   * @param opts.params - parameters
   * @param opts.WebSocketPolyfill - WebSocket polyfill
   * @param opts.resyncInterval - resync interval
   * @param opts.maxBackoffTime - maximum backoff time
   * @param opts.disableBc - disable broadcast channel
   * @param opts.observable - an observable instance to emit events on
   */
  constructor(
    serverUrl: string,
    roomname: string,
    doc: Y.Doc,
    {
      connect = true,
      awareness = new awarenessProtocol.Awareness(doc),
      params = {},
      WebSocketPolyfill = WebSocket,
      resyncInterval = -1,
      maxBackoffTime = 2500,
      disableBc = false,
      observable = new Observable<string>(),
    }: YSweetProviderParams = {},
  ) {
    // ensure that url is always ends with /
    while (serverUrl[serverUrl.length - 1] === '/') {
      serverUrl = serverUrl.slice(0, serverUrl.length - 1)
    }
    const encodedParams = url.encodeQueryParams(params)
    this.observable = observable
    this.maxBackoffTime = maxBackoffTime
    this.bcChannel = serverUrl + '/' + roomname
    this.url = serverUrl + '/' + roomname + (encodedParams.length === 0 ? '' : '?' + encodedParams)
    this.roomname = roomname
    this.doc = doc
    this._WS = WebSocketPolyfill
    this.awareness = awareness
    this.wsconnected = false
    this.wsconnecting = false
    this.bcconnected = false
    this.disableBc = disableBc
    this.wsUnsuccessfulReconnects = 0
    this.messageHandlers = messageHandlers.slice()
    this._synced = false
    this.ws = null
    this.wsLastMessageReceived = 0
    this.shouldConnect = connect

    this._resyncInterval = 0
    if (resyncInterval > 0) {
      this._resyncInterval = setInterval(() => {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
          // resend sync step 1
          const encoder = encoding.createEncoder()
          encoding.writeVarUint(encoder, messageSync)
          syncProtocol.writeSyncStep1(encoder, doc)
          this.ws.send(encoding.toUint8Array(encoder))
        }
      }, resyncInterval)
    }

    this._bcSubscriber = (data: ArrayBuffer, origin: any) => {
      if (origin !== this) {
        const encoder = readMessage(this, new Uint8Array(data), false)
        if (encoding.length(encoder) > 1) {
          bc.publish(this.bcChannel, encoding.toUint8Array(encoder), this)
        }
      }
    }

    /**
     * Listens to Yjs updates and sends them to remote peers (ws and broadcastchannel)
     */
    this._updateHandler = (update: Uint8Array, origin: any) => {
      if (origin !== this) {
        const encoder = encoding.createEncoder()
        encoding.writeVarUint(encoder, messageSync)
        syncProtocol.writeUpdate(encoder, update)
        broadcastMessage(this, encoding.toUint8Array(encoder))
      }
    }

    this.doc.on('update', this._updateHandler as any)

    // TODO: I think we can get more specific with the array types.
    // They are not documented here so we need to do some digging.
    // https://docs.yjs.dev/api/about-awareness
    this._awarenessUpdateHandler = (
      { added, updated, removed }: { added: Array<any>; updated: Array<any>; removed: Array<any> },
      _origin: any,
    ) => {
      const changedClients = added.concat(updated).concat(removed)
      const encoder = encoding.createEncoder()
      encoding.writeVarUint(encoder, messageAwareness)
      encoding.writeVarUint8Array(
        encoder,
        awarenessProtocol.encodeAwarenessUpdate(awareness, changedClients),
      )
      broadcastMessage(this, encoding.toUint8Array(encoder))
    }

    this._unloadHandler = () => {
      awarenessProtocol.removeAwarenessStates(this.awareness, [doc.clientID], 'window unload')
    }

    if (typeof window !== 'undefined') {
      window.addEventListener('unload', this._unloadHandler as any)
    } else if (typeof process !== 'undefined') {
      process.on('exit', this._unloadHandler as any)
    }

    awareness.on('update', this._awarenessUpdateHandler)
    this._checkInterval = setInterval(() => {
      if (
        this.wsconnected &&
        messageReconnectTimeout < time.getUnixTime() - this.wsLastMessageReceived
      ) {
        // no message received in a long time - not even your own awareness
        // updates (which are updated every 15 seconds)
        this.ws?.close()
      }
    }, messageReconnectTimeout / 10)
    if (connect) {
      this.connect()
    }
  }

  /**
   * @type {boolean}
   */
  get synced() {
    return this._synced
  }

  set synced(state) {
    if (this._synced !== state) {
      this._synced = state
      this.observable.emit('synced', [state])
      this.observable.emit('sync', [state])
    }
  }

  destroy() {
    if (this._resyncInterval !== 0) {
      clearInterval(this._resyncInterval)
    }
    clearInterval(this._checkInterval)
    this.disconnect()
    if (typeof window !== 'undefined') {
      window.removeEventListener('unload', this._unloadHandler as any)
    } else if (typeof process !== 'undefined') {
      process.off('exit', this._unloadHandler as any)
    }
    this.awareness.off('update', this._awarenessUpdateHandler)
    this.doc.off('update', this._updateHandler)
  }

  connectBc() {
    if (this.disableBc) {
      return
    }
    if (!this.bcconnected) {
      bc.subscribe(this.bcChannel, this._bcSubscriber as any)
      this.bcconnected = true
    }
    // send sync step1 to bc
    // write sync step 1
    const encoderSync = encoding.createEncoder()
    encoding.writeVarUint(encoderSync, messageSync)
    syncProtocol.writeSyncStep1(encoderSync, this.doc)
    bc.publish(this.bcChannel, encoding.toUint8Array(encoderSync), this)
    // broadcast local state
    const encoderState = encoding.createEncoder()
    encoding.writeVarUint(encoderState, messageSync)
    syncProtocol.writeSyncStep2(encoderState, this.doc)
    bc.publish(this.bcChannel, encoding.toUint8Array(encoderState), this)
    // write queryAwareness
    const encoderAwarenessQuery = encoding.createEncoder()
    encoding.writeVarUint(encoderAwarenessQuery, messageQueryAwareness)
    bc.publish(this.bcChannel, encoding.toUint8Array(encoderAwarenessQuery), this)
    // broadcast local awareness state
    const encoderAwarenessState = encoding.createEncoder()
    encoding.writeVarUint(encoderAwarenessState, messageAwareness)
    encoding.writeVarUint8Array(
      encoderAwarenessState,
      awarenessProtocol.encodeAwarenessUpdate(this.awareness, [this.doc.clientID]),
    )
    bc.publish(this.bcChannel, encoding.toUint8Array(encoderAwarenessState), this)
  }

  disconnectBc() {
    // broadcast message with local awareness state set to null (indicating disconnect)
    const encoder = encoding.createEncoder()
    encoding.writeVarUint(encoder, messageAwareness)
    encoding.writeVarUint8Array(
      encoder,
      awarenessProtocol.encodeAwarenessUpdate(this.awareness, [this.doc.clientID], new Map()),
    )
    broadcastMessage(this, encoding.toUint8Array(encoder))
    if (this.bcconnected) {
      bc.unsubscribe(this.bcChannel, this._bcSubscriber as any)
      this.bcconnected = false
    }
  }

  disconnect() {
    this.shouldConnect = false
    this.disconnectBc()
    if (this.ws !== null) {
      this.ws.close()
    }
  }

  connect() {
    this.shouldConnect = true
    if (!this.wsconnected && this.ws === null) {
      setupWS(this)
      this.connectBc()
    }
  }

  addOnFailureHandler(handler: () => void): () => void {
    this.onFailureHandlers.push(handler)
    return () => {
      this.onFailureHandlers = this.onFailureHandlers.filter((h) => h !== handler)
    }
  }
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

function updateProviderParams(
  providerParams: YSweetProviderParams,
  clientToken: ClientToken,
): YSweetProviderParams {
  // if the clientToken has a token, add it to the provider params
  const connectParams = providerParams.params ? { ...providerParams.params } : {}
  if (clientToken.token) connectParams.token = clientToken.token
  return { ...providerParams, params: connectParams }
}

export async function ySweetProviderWrapper(
  authEndpoint: AuthEndpoint,
  roomname: string,
  doc: Y.Doc,
  providerParams: YSweetProviderParams = {},
): Promise<YSweetProviderWithClientToken> {
  const observable = providerParams.observable ?? new Observable<string>()
  providerParams = { ...providerParams, observable }

  let _clientToken = await getClientToken(authEndpoint, roomname)
  let _provider = new YSweetProvider(_clientToken.url, roomname, doc, {
    ...updateProviderParams(providerParams, _clientToken),
  })
  _provider.addOnFailureHandler(recreateProvider)

  async function recreateProvider() {
    _clientToken = await getClientToken(authEndpoint, roomname)
    _provider = new YSweetProvider(_clientToken.url, roomname, doc, {
      ...updateProviderParams(providerParams, _clientToken),
      connect: true,
    })
    _provider.addOnFailureHandler(recreateProvider)
  }

  return {
    observable,
    get clientToken() {
      return _clientToken
    },
    destroy() {
      _provider.destroy()
    },
    connectBc() {
      _provider.connectBc()
    },
    disconnectBc() {
      _provider.disconnectBc()
    },
    disconnect() {
      _provider.disconnect()
    },
    connect() {
      _provider.connect()
    },
    addOnFailureHandler(handler: () => void): () => void {
      return _provider.addOnFailureHandler(handler)
    },
    get synced() {
      return _provider.synced
    },
    get maxBackoffTime() {
      return _provider.maxBackoffTime
    },
    get bcChannel() {
      return _provider.bcChannel
    },
    get url() {
      return _provider.url
    },
    get roomname() {
      return _provider.roomname
    },
    get doc() {
      return _provider.doc
    },
    get awareness() {
      return _provider.awareness
    },
    get wsconnected() {
      return _provider.wsconnected
    },
    get wsconnecting() {
      return _provider.wsconnecting
    },
    get bcconnected() {
      return _provider.bcconnected
    },
    get disableBc() {
      return _provider.disableBc
    },
    get wsUnsuccessfulReconnects() {
      return _provider.wsUnsuccessfulReconnects
    },
    get messageHandlers() {
      return _provider.messageHandlers
    },
    get ws() {
      return _provider.ws
    },
    get wsLastMessageReceived() {
      return _provider.wsLastMessageReceived
    },
    get shouldConnect() {
      return _provider.shouldConnect
    },
  } as YSweetProviderWithClientToken
}
