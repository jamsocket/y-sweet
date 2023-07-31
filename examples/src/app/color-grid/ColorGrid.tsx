'use client'

import { useMap } from '@y-sweet/react'
import { useState } from 'react'
import Title from '@/components/Title'

const GRID_SIZE = 10
const COLORS = ['#500724', '#831843', '#9d174d', '#be185d', '#db2777', '#f472b6', '#f9a8d4', null]
const DEFAULT_COLOR = 'white'

export function ColorGrid() {
  const items = useMap<string>('colorgrid')
  const [color, setColor] = useState<string | null>(COLORS[0])

  return (
    <div className="space-y-3 m-10">
      <Title>Color Grid</Title>
      <div className="space-x-2 flex flex-row">
        {COLORS.map((color) => (
          <div
            key={color}
            className="w-10 h-10 cursor-pointer"
            style={{ backgroundColor: color ?? DEFAULT_COLOR }}
            onClick={() => setColor(color)}
          ></div>
        ))}
      </div>
      <table>
        <tbody>
          {Array.from({ length: GRID_SIZE }, (x, i) => (
            <tr key={i}>
              {Array.from({ length: GRID_SIZE }, (x, j) => {
                const key = `${i},${j}`
                const item = items!.get(key)
                return (
                  <td key={key}>
                    <div
                      className="w-10 h-10 cursor-pointer"
                      style={{ backgroundColor: item || DEFAULT_COLOR }}
                      onClick={() => {
                        if (color === null) {
                          items!.delete(key)
                        } else {
                          items!.set(key, color)
                        }
                      }}
                    ></div>
                  </td>
                )
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
