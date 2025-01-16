'use client'

import Header from '@/components/Header'
import { usePresence, usePresenceSetter } from '@y-sweet/react'
import { useCallback, useRef, useState } from 'react'

type Presence = { x: number; y: number; color: string; rotation: number }

function Cursor(props: { presence: Presence }) {
  const { presence } = props

  return (
    <g
      transform={`translate(${presence.x} ${presence.y}) scale(1.7) rotate(${presence.rotation})`}
      style={{ transition: 'transform 0.05s' }}
    >
      <path
        d="M 0 0 L -12 6 L -10 0 L -12 -6 Z"
        fill={presence.color}
        stroke="#777"
        strokeWidth={2}
        strokeLinejoin="round"
      />
    </g>
  )
}

export function randomColor() {
  const hue = Math.random() * 360
  const value = Math.random() * 0.5 + 0.25
  return `hsl(${hue}, 75%, ${value * 100}%)`
}

export function Presence() {
  const [myColor, _] = useState(randomColor)
  const presence = usePresence<Presence>({ includeSelf: true })
  const setPresence = usePresenceSetter<Presence>()
  const lastRotation = useRef(0)

  const updatePresence = useCallback(
    (e: React.MouseEvent<SVGSVGElement>) => {
      let rect = e.currentTarget.getBoundingClientRect()
      let deltaX = e.movementX
      let deltaY = e.movementY

      if (deltaX === 0 && deltaY === 0) {
        return
      }

      let movementRotation = Math.atan2(deltaY, deltaX) * (180 / Math.PI)
      let difference = ((movementRotation - lastRotation.current + 180) % 360) - 180
      if (difference < -180) difference += 360
      movementRotation = lastRotation.current + difference

      const rotation = 0.9 * lastRotation.current + 0.1 * movementRotation
      lastRotation.current = rotation

      setPresence({
        x: e.clientX - rect.left,
        y: e.clientY - rect.top,
        color: myColor,
        rotation,
      })
    },
    [setPresence, myColor],
  )

  return (
    <div className="w-full h-full">
      <Header
        title="Live Cursors"
        githubLink="https://github.com/jamsocket/y-sweet/tree/main/examples/nextjs/src/app/(demos)/to-do-list"
      />
      <svg
        className="relative overflow-hidden w-full h-full cursor-none"
        onMouseMove={updatePresence}
      >
        {Array.from(presence.entries()).map(([key, value]) => (
          <Cursor key={key} presence={value} />
        ))}
      </svg>
    </div>
  )
}
