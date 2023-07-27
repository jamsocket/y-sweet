'use client'

import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'
import { WebsocketProvider } from 'y-websocket'
import * as Y from 'yjs'
import { ConnectionKey } from './yserv'
import type { Awareness } from 'y-protocols/awareness'
import { createYjsProvider } from './client'

type YjsContextType = {
  doc: Y.Doc
  provider?: WebsocketProvider
}

const YjsContext = createContext<YjsContextType | null>(null)

export function useYDoc(): Y.Doc | null {
  return useContext(YjsContext)?.doc ?? null
}

export function useAwareness(): Awareness | null {
  return useContext(YjsContext)?.provider?.awareness ?? null
}

export function usePresence(): [Map<number, any>, (e: any) => void] {
  const awareness = useAwareness()
  const [presence, setPresence] = useState<Map<number, any>>(new Map())
  
  useEffect(() => {
    if (awareness) {
      const callback = () => {
        setPresence(new Map(awareness.getStates()))
      }
      awareness.on('change', callback)
      return () => {
        awareness.off('change', callback)
      }
    }
  }, [awareness])

  const setLocalPresence = useCallback((localState: any) => {
    if (awareness) {
      awareness.setLocalState(localState)
    }
  }, [awareness])

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

  const [ctx, setCtx] = useState<YjsContextType>(() => {
    const doc = new Y.Doc()
    return {
      doc,
    }
  })

  useEffect(() => {
    const provider = createYjsProvider(ctx.doc, auth, {
      // TODO: this disables cross-tab communication, which makes debugging easier, but should be re-enabled in prod
      disableBc: true,
    })

    setCtx({
      doc: ctx.doc,
      provider,
    })

    if (props.setQueryParam) {
      const url = new URL(window.location.href)
      url.searchParams.set(props.setQueryParam, auth.doc_id)
      window.history.replaceState({}, '', url.toString())
    }
  }, [])

  return <YjsContext.Provider value={ctx}>{children}</YjsContext.Provider>
}

function useRedraw() {
  const [_, setRedraw] = useState(0)
  return useCallback(() => setRedraw((x) => x + 1), [setRedraw])
}

export function useMap<T>(name: string): Y.Map<T> | undefined {
  const doc = useYDoc()
  const map = useMemo(() => doc?.getMap(name), [doc, name])
  useObserve(map!)

  return map as Y.Map<T>
}

export function useArray<T>(name: string): Y.Array<T> | undefined {
  const doc = useYDoc()
  const array = useMemo(() => doc?.getArray(name), [doc, name])
  useObserve(array!)

  return array as Y.Array<T>
}

export function useText(name: string): Y.Text | undefined {
  const doc = useYDoc()
  const text = useMemo(() => doc?.getText(name), [doc, name])
  useObserve(text!)

  return text as Y.Text
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
