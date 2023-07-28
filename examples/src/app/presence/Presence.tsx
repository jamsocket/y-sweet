'use client'

import { usePresence } from '@y-sweet/js/react'
import { useCallback, useRef } from 'react'

const COLORS = ['bg-red-500', 'bg-blue-500', 'bg-green-500', 'bg-purple-500', 'bg-pink-500']

export function Presence() {
  const myColor = useRef(COLORS[Math.floor(Math.random() * COLORS.length)])
  const [presence, setPresence] = usePresence<{ x: number; y: number; color: string }>()

  const updatePresence = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    setPresence({
      x: e.clientX - e.currentTarget.offsetLeft,
      y: e.clientY - e.currentTarget.offsetTop,
      color: myColor.current,
    })
  }, [])

  return (
    <div
      className="border-blue-400 border relative overflow-hidden w-[500px] h-[500px]"
      onMouseMove={updatePresence}
    >
      {Array.from(presence.entries()).map(([key, value]) => (
        <div
          key={key}
          className={`absolute rounded-full ${value.color}`}
          style={{ left: value.x - 6, top: value.y - 8, width: 10, height: 10 }}
        />
      ))}
    </div>
  )
}
