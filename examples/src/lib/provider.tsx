"use client"

import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { WebsocketProvider } from 'y-websocket';
import * as Y from 'yjs';
import { ConnectionKey } from './yserv';
import type { Awareness } from 'y-protocols/awareness'

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
        const params = auth.token ? { token: auth.token } : undefined
        
        const provider = new WebsocketProvider(
            auth.base_url, auth.doc_id, ctx.doc, { params }
        )

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

    return (
        <YjsContext.Provider value={ctx}>{children}</YjsContext.Provider>
    )
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

function useObserve(object: Y.Map<any> | Y.Array<any> | Y.Text, deep = true) {
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
