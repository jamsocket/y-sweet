'use client'

import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'
import { WebsocketProvider } from 'y-websocket'
import * as Y from 'yjs'
import { ConnectionKey } from './yserv'
import type { Awareness } from 'y-protocols/awareness'
import { createYjsProvider } from './client'

type YjsContextType = {
  doc: Y.Doc
  provider: WebsocketProvider
}

const YjsContext = createContext<YjsContextType | null>(null)

export function useYDoc(): Y.Doc {
  const yjsCtx = useContext(YjsContext)
  if (!yjsCtx) {
    throw new Error('Yjs hooks must be used within a YDocProvider')
  }
  return yjsCtx.doc
}

export function useAwareness(): Awareness {
  const yjsCtx = useContext(YjsContext)
  if (!yjsCtx) {
    throw new Error('Yjs hooks must be used within a YDocProvider')
  }
  return yjsCtx.provider.awareness
}

export function usePresence<T extends Record<string, any>>(): [
  Map<number, T>,
  (presence: T) => void,
] {
  const awareness = useAwareness()
  const [presence, setPresence] = useState<Map<number, T>>(new Map())

  useEffect(() => {
    if (awareness) {
      const callback = () => {
        const map = new Map()
        awareness.getStates().forEach((state, clientID) => {
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

  const setLocalPresence = useCallback(
    (localState: any) => {
      if (awareness) {
        awareness.setLocalState(localState)
      }
    },
    [awareness],
  )

  return [presence, setLocalPresence]
}

type YDocProviderProps = {
  children: React.ReactNode

  /** Response of a `getConnectionKey` call, passed from server to client. */
  connectionKey: ConnectionKey

  /** If set to a string, the URL query parameter with this name
   * will be set to the doc id from connectionKey. */
  setQueryParam?: string
}

export function YDocProvider(props: YDocProviderProps) {
  const { children, connectionKey: auth } = props

  const [ctx, setCtx] = useState<YjsContextType | null>(null)

  useEffect(() => {
    const doc = new Y.Doc()
    const provider = createYjsProvider(doc, auth, {
      // TODO: this disables cross-tab communication, which makes debugging easier, but should be re-enabled in prod
      disableBc: true,
    })

    setCtx({ doc, provider })

    return () => {
      provider.destroy()
      doc.destroy()
    }
  }, [auth.token, auth.base_url, auth.doc_id])

  useEffect(() => {
    if (props.setQueryParam) {
      const url = new URL(window.location.href)
      url.searchParams.set(props.setQueryParam, auth.doc_id)
      window.history.replaceState({}, '', url.toString())
    }
  }, [props.setQueryParam, auth.doc_id])

  if (ctx === null) return null

  return <YjsContext.Provider value={ctx}>{children}</YjsContext.Provider>
}

function useRedraw() {
  const [_, setRedraw] = useState(0)
  return useCallback(() => setRedraw((x) => x + 1), [setRedraw])
}

export function useMap<T>(name: string): Y.Map<T> {
  const doc = useYDoc()
  const map = useMemo(() => doc.getMap<T>(name), [doc, name])
  useObserve(map)

  return map
}

export function useArray<T>(name: string): Y.Array<T> {
  const doc = useYDoc()
  const array = useMemo(() => doc.getArray<T>(name), [doc, name])
  useObserve(array)

  return array
}

export function useText(name: string): Y.Text {
  const doc = useYDoc()
  const text = useMemo(() => doc.getText(name), [doc, name])
  useObserve(text)

  return text
}

function useObserve(object: Y.AbstractType<any>, deep = true) {
  const redraw = useRedraw()

  useEffect(() => {
    if (deep) {
      object.observeDeep(redraw)
    } else {
      object.observe(redraw)
    }

    return () => {
      if (deep) {
        object.unobserveDeep(redraw)
      } else {
        object.unobserve(redraw)
      }
    }
  })
}
