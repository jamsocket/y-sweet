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
        <div className="space-y-10">
            {
                keys.map((key) => {
                    const value = doc!.get(key)
                    return <DocEntryView key={key} name={key} value={value} />
                })
            }
        </div>
    )
}

function collectLinkedList(node: Y.Item): Y.Item[] {
    const result: Y.Item[] = []
    let current: Y.Item | null = node
    while (current !== null) {
        if (!(current.content instanceof Y.ContentDeleted)) {
            result.push(current)
        }
        current = current.right
    }
    return result
}

type DocEntryViewProps = {
    name: string,
    value: Y.AbstractType<any>
}

function DocEntryView(props: DocEntryViewProps) {
    return (
        <div className="space-y-5">
            <h2 className="font-mono text-xl font-bold">{props.name}</h2>
            {
                props.value._map?.size ? <MapView map={props.value._map} /> : null
            }
            {
                props.value._start ? <GeneralList list={props.value._start} /> : null
            }
        </div>
    )
}

function GeneralList(props: { list: Y.Item }) {
    const items = collectLinkedList(props.list)
    const [displayAsText, setDisplayAsText] = useState(false)

    let component
    if (displayAsText) {
        component = <TextView list={items} />
    } else {
        component = <ListView list={items} />
    }

    return <div>
        <label>
            <input type="checkbox" checked={displayAsText} onChange={(e) => setDisplayAsText(e.target.checked)} />
            <span className="ml-2">Display as text</span>
        </label>

        {component}
    </div>
}

type ListViewProps = {
    list: Y.Item[]
}

function ListView(props: ListViewProps) {
    return <div className="font-mono text-gray-400">
        <span>[</span>
        <div className="pl-10">
            {
                props.list.map((item, i) => {
                    return <div key={i}>
                        <ItemView item={item} />
                    </div>
                })
            }
        </div>
        <span>]</span>
    </div>
}

function TextView(props: ListViewProps) {
    return <pre className="bg-gray-100 rounded-md p-5">
        {props.list.map((item) => {
            if (item.content instanceof Y.ContentDeleted) {
                return 'deleted'
            } else if (item.content instanceof Y.ContentString) {
                return item.content.str
            } else {
                console.log('unimplemented item type', item)
                return 'unknown'
            }
        })
        }
    </pre>
}

type MapViewProps = {
    map: Map<string, Y.Item>
}

function MapView(props: MapViewProps) {
    return <div className="font-mono text-gray-400">
        <span>{"{"}</span>
        <div className="pl-10">
            {
                Array.from(props.map.entries()).filter(v => !(v[1].content instanceof Y.ContentDeleted)).map(([key, item]) => {
                    return <div key={key}>
                        <PrettyKeyString value={key} />
                        <span>: </span>
                        <ItemView item={item} />
                    </div>
                })
            }
        </div>
        <span>{"}"}</span>
    </div>
}

type ItemViewProps = {
    item: Y.Item
}

function ItemView(props: ItemViewProps) {
    if (props.item.content instanceof Y.ContentDeleted) {
        return <samp>deleted</samp>
    } else if (props.item.content instanceof Y.ContentAny) {
        return <PrettyValue value={props.item.content.arr[0]} />
    } else if (props.item.content instanceof Y.ContentType) {
        const content: Y.ContentType = props.item.content

        if (content.type instanceof Y.Map) {
            return <MapView map={content.type._map} />
        }

        return <span>unknown content type...</span>
    } else if (props.item.content instanceof Y.ContentString) {
        return <PrettyString value={props.item.content.str} />
    } else {
        console.log('unimplemented item type', props.item)
        return <samp>unknown</samp>
    }
}

function PrettyValue(props: { value: any }) {
    if (typeof props.value === 'string') {
        return <PrettyString value={props.value} />
    } else if (typeof props.value === 'boolean') {
        if (props.value) {
            return <span className="text-green-600">true</span>
        } else {
            return <span className="text-purple-600">false</span>
        }
    } else {
        console.log('unimplemented value type', typeof props.value)
        return <span>unknown type</span>
    }
}

function PrettyString(props: { value: string }) {
    let valueEscaped = JSON.stringify(props.value)
    valueEscaped = valueEscaped.slice(1, valueEscaped.length - 1)
    return <span className="text-blue-300">"<span className="text-blue-600">{valueEscaped}</span>"</span>
}

function PrettyKeyString(props: { value: string }) {
    const valueEscaped = JSON.stringify(props.value).slice(1, props.value.length + 1)
    return <span className="text-red-300">"<span className="text-red-600">{valueEscaped}</span>"</span>
}