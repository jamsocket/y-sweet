'use client'

import { useArray } from '@y-sweet/react'
import { useCallback, useState } from 'react'
import * as Y from 'yjs'
import Title from '@/components/Title'
import CopyLink from '@/components/CopyLink'

type ToDoItem = {
  text: string
  done: boolean
}

export function ToDoInput(props: { onItem: (text: string) => void }) {
  const [text, setText] = useState('')
  const onSubmit = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault()
      props.onItem(text)
      setText('')
    },
    [props, text],
  )

  const changeCallback = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      setText(e.target.value)
    },
    [setText],
  )

  return (
    <form onSubmit={onSubmit} className="flex flex-row space-x-2 max-w-2xl">
      <input
        type="text"
        value={text}
        onChange={changeCallback}
        className="bg-white flex-1 block ring-pink-900 rounded-md border-0 px-3.5 py-2 text-gray-900 shadow-sm ring-1 ring-inset
                 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-pink-900
                sm:text-sm sm:leading-6"
      />
      <button
        type="submit"
        className="block rounded-md bg-pink-900 px-3.5 py-2.5 text-center text-sm font-semibold
                text-white shadow-sm hover:bg-pink-900 focus-visible:outline focus-visible:outline-2
                focus-visible:outline-offset-2 focus-visible:outline-pink-900"
      >
        Add
      </button>
    </form>
  )
}

type ToDoItemProps = {
  item: Y.Map<any>
}

export function ToDoItem({ item }: ToDoItemProps) {
  const clickCallback = useCallback(() => {
    item.set('done', !item.get('done'))
  }, [item])

  return (
    <div>
      <label className="flex flex-row space-x-2 items-center">
        <input
          type="checkbox"
          className="w-6 h-6 cursor-pointer"
          checked={item.get('done')}
          onChange={clickCallback}
        />
        <input
          className="bg-transparent p-1 rounded text-pink-950 text-lg focus:bg-white"
          value={item.get('text')}
          onChange={(e) => item.set('text', e.target.value)}
        />
      </label>
    </div>
  )
}

export function ToDoList() {
  const items = useArray<Y.Map<any>>('todolist')

  const pushItem = useCallback(
    (text: string) => {
      let item = new Y.Map([
        ['text', text],
        ['done', false],
      ] as [string, any][])

      items?.push([item])
    },
    [items],
  )

  const clearCompleted = useCallback(() => {
    let indexOffset = 0
    items?.forEach((item, index) => {
      if (item.get('done')) {
        items.delete(index - indexOffset, 1)
        indexOffset += 1
      }
    })
  }, [items])

  return (
    <div className="space-y-4 p-4 lg:p-8">
      <Title>To-do List</Title>
      <div className="space-y-1">
        {items && items.map((item, index) => <ToDoItem key={index} item={item} />)}
      </div>
      <ToDoInput onItem={pushItem} />
      <button
        onClick={clearCompleted}
        className="block rounded-md bg-pink-900 px-3.5 py-2.5 text-center text-sm
                font-semibold text-white shadow-sm hover:bg-pink-900 focus-visible:outline
                focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-pink-900"
      >
        Clear Completed
      </button>
      <CopyLink />
    </div>
  )
}
