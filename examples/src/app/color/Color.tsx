"use client"

import { useMap } from "@/lib/provider";
import { useState } from "react";

const GRID_SIZE = 10
const COLORS = ['red', 'green', 'blue', 'purple', 'orange', 'pink', 'brown', null]
const DEFAULT_COLOR = '#eee'

export function ColorGrid() {
    const items = useMap<string>('colorgrid')
    const [color, setColor] = useState<string | null>('red')

    return <div className="space-y-3">
        <h1>Color Grid</h1>
        <div className="space-x-2 flex flex-row">
            {
                COLORS.map((color) => <div key={color} className="w-10 h-10" style={{backgroundColor: color ?? DEFAULT_COLOR}} onClick={() => setColor(color)}></div>)
            }
        </div>
        <table>
            <tbody>
                {
                    Array.from({length: GRID_SIZE}, (x, i) => <tr key={i}>
                        {
                            Array.from({length: GRID_SIZE}, (x, j) => {
                                const key = `${i},${j}`
                                const item = items!.get(key)
                                return <td key={key}>
                                    <div className="w-10 h-10" style={{backgroundColor: item || DEFAULT_COLOR}} onClick={() => {
                                        if (color === null) {
                                            items!.delete(key)
                                        } else {
                                            items!.set(key, color)
                                        }
                                    }}></div>
                                </td>
                            })
                        }
                    </tr>)
                }
            </tbody>
        </table>
    </div>
}
