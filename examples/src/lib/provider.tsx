"use client"

import { createContext, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { WebsocketProvider } from 'y-websocket';
import * as Y from 'yjs';
import { ConnectionKey } from './yserv';

const YjsContext = createContext<Y.Doc | null>(null)

export function useYDoc() {
    return useContext(YjsContext)
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

    const docRef = useRef<Y.Doc | null>(null)
    if (docRef.current === null) {
        docRef.current = new Y.Doc()
    }

    useEffect(() => {
        new WebsocketProvider(
            auth.base_url, auth.doc_id, docRef.current!
        )

        if (props.setQueryParam) {
            const url = new URL(window.location.href)
            url.searchParams.set(props.setQueryParam, auth.doc_id)
            window.history.replaceState({}, '', url.toString())
        }
    })

    return (
        <YjsContext.Provider value={docRef.current}>{children}</YjsContext.Provider>
    )
}

function useRedraw() {
    const [_, setRedraw] = useState(0)
    return () => setRedraw((x) => x + 1)
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

function useObserve(object: Y.Map<any> | Y.Array<any>, deep = true) {
    const redraw = useRedraw()

    useEffect(() => {
        if (deep) {
            object.observeDeep(redraw)
        } else {
            object.observe(redraw)
        }
        
        return () => {
            object.unobserve(redraw)
        }
    })
}
