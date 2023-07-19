"use client"

import { createContext, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { WebsocketProvider } from 'y-websocket';
import * as Y from 'yjs';

const YjsContext = createContext<Y.Doc | null>(null)

export function useYjs() {
    return useContext(YjsContext)
}

type YjsProviderProps = {
    children: React.ReactNode
    base_url: string
    doc_id: string
}

export function YjsProvider(props: YjsProviderProps) {
    const { children, base_url, doc_id } = props

    const docRef = useRef<Y.Doc | null>(null)
    if (docRef.current === null) {
        docRef.current = new Y.Doc()
    }

    useEffect(() => {
        new WebsocketProvider(
            base_url, doc_id, docRef.current!
        )
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

export function useObserve(object: Y.Map<any> | Y.Array<any>) {
    const redraw = useRedraw()

    useEffect(() => {
        object.observe(redraw)

        return () => {
            object.unobserve(redraw)
        }
    })
}
