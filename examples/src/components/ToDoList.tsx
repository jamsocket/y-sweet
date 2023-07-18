"use client"

import { useArray } from "@/lib/provider"
import { useCallback, useState } from "react"

type ToDoItem = {
    text: string
    done: boolean
}

export function ToDoInput(props: { onItem: (text: string) => void }) {
    const [text, setText] = useState('')
    const clickCallback = useCallback(() => {
        props.onItem(text)
        setText('')
    }, [props, text])

    const changeCallback = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
        setText(e.target.value)
    }, [setText])

    return (
        <div>
            <input type="text" value={text} onChange={changeCallback} />
            <button onClick={clickCallback}>Add Item</button>
        </div>
    )
}

export function ToDoList() {
    const items = useArray<ToDoItem>('todolist')

    return (
        <div>
            <h1>ToDo List</h1>
            {
                items && items.map((item, index) => {
                    return (
                        <div key={index}>
                            {item.text}: {item.done ? 'done' : 'not done'}
                        </div>
                    )
                })
            }

            <ToDoInput onItem={(text) => {
                items?.push([{
                    text,
                    done: false
                }])
            }} />
        </div>
    )
}
