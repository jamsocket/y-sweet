'use client'

import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'
import type { ReactNode } from 'react'
import { WebsocketProvider } from 'y-websocket'
import * as Y from 'yjs'
import type { Awareness } from 'y-protocols/awareness'
import { ClientToken } from '@y-sweet/sdk'
import { createYjsProvider } from './yjs-provider'
import { encodeClientToken } from '@y-sweet/sdk'

type YjsContextType = {
  doc: Y.Doc
  provider: WebsocketProvider
  clientToken: ClientToken
}

const YjsContext = createContext<YjsContextType | null>(null)

type YDocOptions = {
  hideDebuggerLink?: boolean
}

export function useYDoc(options?: YDocOptions): Y.Doc {
  const yjsCtx = useContext(YjsContext)

  useEffect(() => {
    if (!options?.hideDebuggerLink && yjsCtx) {
      const url = debuggerUrl(yjsCtx.clientToken)
      console.log(
        `%cOpen in Y-Sweet Debugger ⮕ ${url}`,
        'font-size: 1.5em; display: block; padding: 10px;',
      )
      console.log(
        '%cTo hide the debugger link, pass { hideDebuggerLink: true } to useYDoc',
        'font-style: italic;',
      )
    }
  }, [options?.hideDebuggerLink, yjsCtx])

  if (!yjsCtx) {
    throw new Error('Yjs hooks must be used within a YDocProvider')
  }
  return yjsCtx.doc
}

export function debuggerUrl(clientToken: ClientToken): string {
  const payload = encodeClientToken(clientToken)
  return `https://debugger.y-sweet.dev/?payload=${payload}`
}

export function useYSweetDebugUrl(): string {
  const yjsCtx = useContext(YjsContext)
  if (!yjsCtx) {
    throw new Error('Yjs hooks must be used within a YDocProvider')
  }
  return debuggerUrl(yjsCtx.clientToken)
}

export function useYjsProvider(): WebsocketProvider {
  const yjsCtx = useContext(YjsContext)
  if (!yjsCtx) {
    throw new Error('Yjs hooks must be used within a YDocProvider')
  }
  return yjsCtx.provider
}

export function useAwareness(): Awareness {
  const yjsCtx = useContext(YjsContext)
  if (!yjsCtx) {
    throw new Error('Yjs hooks must be used within a YDocProvider')
  }
  return yjsCtx.provider.awareness
}

type UsePresenceOptions = {
  includeSelf?: boolean
}

export function usePresenceSetter<T extends Record<string, any>>(): (presence: T) => void {
  const awareness = useAwareness()

  const setLocalPresence = useCallback(
    (localState: any) => {
      if (awareness) {
        awareness.setLocalState(localState)
      }
    },
    [awareness],
  )

  return setLocalPresence
}

export function usePresence<T extends Record<string, any>>(
  options?: UsePresenceOptions,
): Map<number, T> {
  const awareness = useAwareness()
  const [presence, setPresence] = useState<Map<number, T>>(new Map())

  const includeSelf = options?.includeSelf || false

  useEffect(() => {
    if (awareness) {
      const callback = () => {
        const map = new Map()
        awareness.getStates().forEach((state, clientID) => {
          if (!includeSelf && clientID === awareness.clientID) return

          if (Object.keys(state).length > 0) {
            map.set(clientID, state)
          }
        })

        setPresence(map)
      }
      awareness.on('change', callback)
      return () => {
        awareness.off('change', callback)
      }
    }
  }, [awareness])

  return presence
}

type YDocProviderProps = {
  children: ReactNode

  /** Response of a `getConnectionKey` call, passed from server to client. */
  clientToken: ClientToken

  /** If set to a string, the URL query parameter with this name
   * will be set to the doc id from connectionKey. */
  setQueryParam?: string
}

export function YDocProvider(props: YDocProviderProps) {
  const { children, clientToken } = props

  const [ctx, setCtx] = useState<YjsContextType | null>(null)

  useEffect(() => {
    const doc = new Y.Doc()
    const provider = createYjsProvider(doc, clientToken, {
      // TODO: this disables local cross-tab communication, which makes
      // debugging easier, but should be re-enabled eventually
      disableBc: true,
    })

    setCtx({ doc, provider, clientToken })

    return () => {
      provider.destroy()
      doc.destroy()
    }
  }, [clientToken.token, clientToken.url, clientToken.doc])

  useEffect(() => {
    if (props.setQueryParam) {
      const url = new URL(window.location.href)
      url.searchParams.set(props.setQueryParam, clientToken.doc)
      window.history.replaceState({}, '', url.toString())
    }
  }, [props.setQueryParam, clientToken.doc])

  if (ctx === null) return null

  return <YjsContext.Provider value={ctx}>{children}</YjsContext.Provider>
}

function useRedraw() {
  const [_, setRedraw] = useState(0)
  return useCallback(() => setRedraw((x) => x + 1), [setRedraw])
}

export type ObserverKind = 'deep' | 'shallow' | 'none'

export type ObjectOptions = {
  observe?: ObserverKind
}

export function useMap<T>(name: string, objectOptions?: ObjectOptions): Y.Map<T> {
  const doc = useYDoc()
  const map = useMemo(() => doc.getMap<T>(name), [doc, name])
  useObserve(map, objectOptions?.observe || 'deep')

  return map
}

export function useArray<T>(name: string, objectOptions?: ObjectOptions): Y.Array<T> {
  const doc = useYDoc()
  const array = useMemo(() => doc.getArray<T>(name), [doc, name])
  useObserve(array, objectOptions?.observe || 'deep')

  return array
}

export function useText(name: string, observerKind?: ObjectOptions): Y.Text {
  const doc = useYDoc()
  const text = useMemo(() => doc.getText(name), [doc, name])
  useObserve(text, observerKind?.observe || 'deep')

  return text
}

function useObserve(object: Y.AbstractType<any>, kind: ObserverKind) {
  const redraw = useRedraw()

  useEffect(() => {
    if (kind === 'deep') {
      object.observeDeep(redraw)
    } else if (kind === 'shallow') {
      object.observe(redraw)
    }

    return () => {
      if (kind === 'deep') {
        object.unobserveDeep(redraw)
      } else if (kind === 'shallow') {
        object.unobserve(redraw)
      }
    }
  })
}
