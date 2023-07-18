"use client"

import { createContext, useContext, useEffect, useMemo, useRef } from 'react';
import { WebsocketProvider } from 'y-websocket';
import * as Y from 'yjs';

const YjsContext = createContext<Y.Doc | null>(null)

export function useYjs() {
    return useContext(YjsContext)
}

type YjsProviderProps = {
    children: React.ReactNode
    url: string
}

export function YjsProvider(props: YjsProviderProps) {
    const { children, url } = props

    const docRef = useRef<Y.Doc | null>(null)
    if (docRef.current === null) {
        docRef.current = new Y.Doc()
    }

    useEffect(() => {
        new WebsocketProvider(
            url, 'my-demo', docRef.current!
        )
    })

    return (
        <YjsContext.Provider value={docRef.current}>{children}</YjsContext.Provider>
    )
}

export function useMap(name: string): Y.Map<any> | undefined {
    let doc = useYjs()
    let map = useMemo(() => doc?.getMap(name), [doc, name])
    return map
}
