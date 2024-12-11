'use client'

import { useAwareness, useYDoc, useYjsProvider } from '@y-sweet/react'
import { useCallback, useEffect, useMemo, useState } from 'react'
import * as Y from 'yjs'
import { Debuggable, DebuggableEntry } from '../lib/debuggable'
import { DebuggableYDoc } from '@/lib/debuggable/ydoc'
import { Awareness } from 'y-protocols/awareness'
import { DebuggableAwareness } from '@/lib/debuggable/yawareness'

export function Debugger() {
  const doc: Y.Doc = useYDoc()
  const awareness = useAwareness()

  return (
    <div>
      <div className="flex flex-row">
        <div className="flex-1">
          <h2 className="text-2xl font-bold mb-4">State</h2>
          <DocEntryView doc={doc} />
        </div>
        <div className="flex-1">
          <h2 className="text-2xl font-bold mb-4">Awareness</h2>
          <AwarenessView awareness={awareness} />
        </div>
      </div>
    </div>
  )
}

type DocEntryViewProps = {
  doc: Y.Doc
}

function AwarenessView(props: { awareness: Awareness }) {
  const debuggable = useMemo(() => new DebuggableAwareness(props.awareness), [props.awareness])

  let [_, setRenderVersion] = useState(0)

  useEffect(() => {
    const clear = debuggable.listen(() => {
      setRenderVersion((v) => v + 1)
    })

    return clear
  }, [debuggable])

  return (
    <div className="cursor-default">
      <DebuggableItems debuggable={debuggable} />
    </div>
  )
}

function DocEntryView(props: DocEntryViewProps) {
  let debuggable = useMemo(() => new DebuggableYDoc(props.doc), [props.doc])

  let [_, setRenderVersion] = useState(0)

  // NB This listen only actually matters when going from 0 to 1 entries.
  useEffect(() => {
    const clear = debuggable.listen(() => {
      setRenderVersion((v) => v + 1)
    })

    return clear
  }, [debuggable])

  const len = debuggable.entries().length

  if (len === 0) {
    return <div>No entries.</div>
  }

  return (
    <div className="cursor-default">
      <DebuggableItems debuggable={debuggable} />
    </div>
  )
}

function DebuggableItems(props: { debuggable: Debuggable }) {
  let { debuggable } = props

  let [_, setRenderVersion] = useState(0)

  useEffect(() => {
    const clear = debuggable.listen(() => {
      setRenderVersion((v) => v + 1)
    })

    return clear
  }, [debuggable])

  return (
    <div>
      {debuggable.entries().map((entry) => (
        <DebuggableItem entry={entry} key={entry.key} />
      ))}
    </div>
  )
}

function TypePill(props: { type?: string; onClick?: (e: React.MouseEvent) => void }) {
  if (!props.type) {
    return null
  }

  return (
    <span
      className="text-xs bg-slate-600 text-slate-200 p-1 rounded-md"
      onClickCapture={props.onClick}
    >
      {props.type}
    </span>
  )
}

function DebuggableItem(props: { entry: DebuggableEntry }) {
  const { entry } = props
  const [expanded, setExpanded] = useState(true)
  let [_, setRenderVersion] = useState(0)

  const toggleExpanded = useCallback((e: React.MouseEvent) => {
    setExpanded((expanded) => !expanded)
  }, [])

  const toggleType = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation()
      e.preventDefault()
      entry.value.toggleType?.()
      setRenderVersion((v) => v + 1)
    },
    [entry.value],
  )

  if (entry.value.type() === 'scalar') {
    return (
      <div>
        <samp className="text-gray-500">
          <PrettyKey k={entry.key} />: <PrettyValue value={entry.value.value()} />
        </samp>
      </div>
    )
  } else if (entry.value.type() === 'text') {
    return (
      <div>
        <samp className="text-gray-500">
          <PrettyKey k={entry.key} />:{' '}
          <TypePill type={entry.value.typeName?.()} onClick={toggleType} />{' '}
          <TextView value={entry.value} />
        </samp>
      </div>
    )
  } else if (expanded) {
    return (
      <div>
        <samp onClick={toggleExpanded} className="text-gray-500 select-none">
          <PrettyKey k={entry.key} />:{' '}
          <TypePill type={entry.value.typeName?.()} onClick={toggleType} />{' '}
          {entry.value.type() === 'list' ? '[' : '{'}
        </samp>
        <div className="pl-5">
          <DebuggableItems debuggable={entry.value} />
        </div>
        <samp className="text-gray-500">{entry.value.type() === 'list' ? ']' : '}'}</samp>
      </div>
    )
  } else {
    return (
      <div>
        <samp className="text-gray-500 select-none" onClick={toggleExpanded}>
          <PrettyKey k={entry.key} />:{' '}
          <TypePill onClick={toggleType} type={entry.value.typeName?.()} />{' '}
          {entry.value.type() === 'list' ? '[...]' : '{...}'}
        </samp>
      </div>
    )
  }
}

function TextView(props: { value: Debuggable }) {
  let [_, setRenderVersion] = useState(0)

  useEffect(() => {
    const clear = props.value.listen(() => {
      setRenderVersion((v) => v + 1)
    })

    return clear
  }, [props.value])

  return (
    <div>
      {props.value.value().map((d: Y.Item, i: number) => {
        if (d.content instanceof Y.ContentString) {
          return (
            <span className="text-gray-300 whitespace-pre-wrap" key={i}>
              {d.content.str}
            </span>
          )
        }

        if (d.content instanceof Y.ContentFormat) {
          let tag = d.content.key
          let start = d.content.value !== null
          let value = d.content.value === true ? '' : `=${JSON.stringify(d.content.value)}`

          if (start) {
            return <span className="text-orange-300" key={i}>{`<${tag}${value}>`}</span>
          } else {
            return <span className="text-orange-300" key={i}>{`</${tag}>`}</span>
          }
        }

        console.warn('unhandled text item', d)
        return null
      })}
    </div>
  )
}

function PrettyKey(props: { k: any }) {
  const { k } = props

  let color = 'text-pink-300'
  if (typeof k === 'string') {
    color = 'text-blue-300'
  }

  return <span className={`${color} select-none`}>{k}</span>
}

function PrettyValue(props: { value: any }) {
  if (typeof props.value === 'string') {
    return <PrettyString value={props.value} />
  } else if (typeof props.value === 'boolean') {
    if (props.value) {
      return <span className="text-green-300">true</span>
    } else {
      return <span className="text-purple-300">false</span>
    }
  } else if (typeof props.value === 'number') {
    return <span className="text-yellow-300">{props.value}</span>
  } else {
    console.log('unimplemented value type', typeof props.value)
    return <span>unknown type</span>
  }
}

function PrettyString(props: { value: string }) {
  let valueEscaped = JSON.stringify(props.value)
  valueEscaped = valueEscaped.slice(1, valueEscaped.length - 1)
  return (
    <span className="text-gray-500">
      {'"'}
      <span className="text-neutral-200">{valueEscaped}</span>
      {'"'}
    </span>
  )
}
