"use client"

import { useYDoc } from '@/lib/provider';
import { useEffect, useState } from 'react';
import * as Y from 'yjs';

export function Console() {
    const doc = useYDoc()
    const [keys, setKeys] = useState<string[]>([])

    useEffect(() => {
        // TODO: the goal here is to avoid calling setKeys when the keys haven't changed.
        // But if feels a bit gross, is there a better way?
        let lastKeys: string[] = []

        doc?.on('update', () => {
            const keys = Array.from(doc!.share.keys() ?? [])

            if (lastKeys.length !== keys.length || !lastKeys.every((key, i) => key === keys[i])) {
                setKeys(keys)
                lastKeys = keys
            }
        })
    })

    return (
        <div>
            <h1>Console</h1>
            {
                keys.map((key) => {
                    const value = doc!.get(key)
                    return <DocEntryView key={key} name={key} value={value} />
                })
            }
        </div>
    )
}

type DocEntryViewProps = {
    name: string,
    value: Y.AbstractType<any>
}

function DocEntryView(props: DocEntryViewProps) {
    return (
        <div>
            <h2 className="font-mono text-xl font-bold">{props.name}</h2>
            {
                props.value._map ? <MapView map={props.value._map} /> : null
            }
        </div>
    )
}

type MapViewProps = {
    map: Map<string, Y.Item>
}

function MapView(props: MapViewProps) {
    return <div>
        {
            Array.from(props.map.entries()).map(([key, item]) => {
                return <div key={key}>
                    <samp>{JSON.stringify(key)}</samp>:&nbsp;
                    <ItemView item={item} />
                </div>
            })
        }
    </div>
}

type ItemViewProps = {
    item: Y.Item
}

function ItemView(props: ItemViewProps) {
    if (props.item.content instanceof Y.ContentDeleted) {
        return <samp>deleted</samp>
    } else if (props.item.content instanceof Y.ContentAny) {
        return <samp>{JSON.stringify(props.item.content.arr[0])}</samp>
    } else {
        console.log('unimplemented item type', props.item)
        return <samp>unknown</samp>
    }
}