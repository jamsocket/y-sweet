"use client"

import { createContext, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { WebsocketProvider } from 'y-websocket';
import * as Y from 'yjs';
import { ConnectionKey } from './yserv';

const YjsContext = createContext<Y.Doc | null>(null)

export function useYjs() {
    return useContext(YjsContext)
}

type YjsProviderProps = {
    children: React.ReactNode
    connectionKey: ConnectionKey
    setQueryParam?: string
}

export function YjsProvider(props: YjsProviderProps) {
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
    const doc = useYjs()
    const map = useMemo(() => doc?.getMap(name), [doc, name])
    useObserve(map!)

    return map as Y.Map<T>
}

export function useArray<T>(name: string): Y.Array<T> | undefined {
    const doc = useYjs()
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
