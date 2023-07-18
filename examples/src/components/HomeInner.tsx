"use client"

import { useMap } from "@/lib/provider"

export function HomeInner() {
    const map = useMap('mymap')

    map?.set('hello', 'world')

    return (
        <div>
            <h1>Home</h1>
            {
                map && Array.from(map.entries()).map(([key, value]) => {
                    return (
                        <div key={key}>
                            {key}: {value}
                        </div>
                    )
                })
            }
        </div>
    )
}