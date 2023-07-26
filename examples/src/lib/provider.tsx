"use client"

import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'
import { WebsocketProvider } from 'y-websocket'
import { IndexeddbPersistence } from 'y-indexeddb'
import * as Y from 'yjs'
import { ConnectionKey } from './yserv'
import type { Awareness } from 'y-protocols/awareness'
import { createYjsProvider } from './client';

type YjsContextType = {
    doc: Y.Doc
    wsProvider: WebsocketProvider
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
    return yjsCtx.wsProvider.awareness
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

        // const idxDbProvider = new IndexeddbPersistence(auth.doc_id, doc)
        // idxDbProvider.on('synced', () => {
        //     console.log('indexdb provider synced')
        //     // debugger
        // })

        const wsProvider = createYjsProvider(doc, auth, {
            // TODO: this disables cross-tab communication, which makes debugging easier, but should be re-enabled in prod
            disableBc: true,
        })

        wsProvider.on('sync', (isSynced: boolean) => {
            console.log('websocket provider synced', isSynced)
        })

        setCtx({ doc, wsProvider })

        return () => {
            wsProvider.destroy()
            doc.destroy()
            // idxDbProvider.destroy()
        }
    }, [auth.token, auth.base_url, auth.doc_id])

    useEffect(() => {
        if (props.setQueryParam) {
            const url = new URL(window.location.href)
            url.searchParams.set(props.setQueryParam, auth.doc_id)
            window.history.replaceState({}, '', url.toString())
        }
    }, [props.setQueryParam, auth.doc_id])

    if (!ctx) {
        return null
    }

    return (
        <YjsContext.Provider value={ctx}>{children}</YjsContext.Provider>
    )
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
