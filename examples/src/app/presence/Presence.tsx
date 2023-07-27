'use client'

import { usePresence } from '@/lib/provider'
import { useCallback, useRef } from 'react'

const COLORS = [
    'bg-red-500',
    'bg-blue-500',
    'bg-green-500',
    'bg-purple-500',
    'bg-pink-500',
]

export function Presence() {
    const myColor = useRef(COLORS[Math.floor(Math.random() * COLORS.length)])
    const [presence, setPresence] = usePresence<{ x: number, y: number, color: string }>()
    const containerRef = useRef<HTMLDivElement>(null)

    const updatePresence = useCallback((e: any) => {
        setPresence({
            x: e.clientX - containerRef.current!.offsetLeft,
            y: e.clientY - containerRef.current!.offsetTop,
            color: myColor.current,
        })
    }, [setPresence])

    return (
        <div
            style={{ width: 500, height: 500 }}
            className="border-blue-400 border relative overflow-hidden"
            onMouseMove={updatePresence}
            ref={containerRef}
        >
            {Array.from(presence.entries()).map(([key, value]) =>
                <div key={key}
                    className={`absolute rounded-full ${value.color}`}
                    style={{ left: value.x - 6, top: value.y - 8, width: 10, height: 10 }} />
            )}
        </div>
    )
}
