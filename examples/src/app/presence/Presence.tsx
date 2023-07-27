'use client'

import { usePresence } from '@/lib/provider'
import { useCallback } from 'react'

export function Presence() {
    const [presence, setPresence] = usePresence()

    const updatePresence = useCallback((e: any) => {
        setPresence({
            x: e.clientX,
            y: e.clientY,
        })
    }, [setPresence])

    return (
        <div style={{ width: 500, height: 500 }} className="border-red-400 border" onMouseMove={updatePresence}>
            {Array.from(presence.entries()).map(([key, value]) =>
                <div key={key}
                    className="absolute"
                    style={{ left: value.x, top: value.y, width: 10, height: 10, backgroundColor: 'red' }} />
            )}
        </div>
    )
}
