'use client'

import { useYDoc } from '@y-sweet/react'
import { useCallback, useEffect, useMemo, useState } from 'react'
import * as Y from 'yjs'
import { Debuggable, DebuggableEntry, DebuggableYDoc } from './debuggable'

export function Debugger() {
  const doc: Y.Doc = useYDoc()
  const [_, setVersion] = useState(0)

  // TODO: move observers into items
  useEffect(() => {
    const callback = () => {
      setVersion((version) => version + 1)
    }

    doc.on('update', callback)

    return () => {
      doc.off('update', callback)
    }
  }, [setVersion])

  return <DocEntryView doc={doc} />
}

type DocEntryViewProps = {
  doc: Y.Doc
}

function DocEntryView(props: DocEntryViewProps) {
  let debuggable = useMemo(() => new DebuggableYDoc(props.doc), [props.doc])

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

  return (
    <div>
      {debuggable.entries().map((entry) => (
        <DebuggableItem entry={entry} key={entry.key} />
      ))}
    </div>
  )
}

function TypePill(props: { type?: string }) {
  if (!props.type) {
    return null
  }

  return <span className="text-xs bg-slate-600 text-slate-200 p-1 rounded-md">{props.type}</span>
}

function DebuggableItem(props: { entry: DebuggableEntry }) {
  const { entry } = props
  const [expanded, setExpanded] = useState(true)

  const toggleExpanded = useCallback((e: React.MouseEvent) => {
    setExpanded((expanded) => !expanded)
  }, [])

  if (entry.value.type === 'scalar') {
    return (
      <div>
        <samp className="text-gray-500">
          <PrettyKey k={entry.key} />: <PrettyValue value={entry.value.value()} />
        </samp>
      </div>
    )
  } else if (expanded) {
    return (
      <div>
        <samp onClickCapture={toggleExpanded} className="text-gray-500 select-none">
          <PrettyKey k={entry.key} />: <TypePill type={entry.value.typeName} />{' '}
          {entry.value.type === 'list' ? '[' : '{'}
        </samp>
        <div className="pl-5">
          <DebuggableItems debuggable={entry.value} />
        </div>
        <samp className="text-gray-500">{entry.value.type === 'list' ? ']' : '}'}</samp>
      </div>
    )
  } else {
    return (
      <div>
        <samp className="text-gray-500 select-none" onClickCapture={toggleExpanded}>
          <PrettyKey k={entry.key} />: <TypePill type={entry.value.typeName} />{' '}
          {entry.value.type === 'list' ? '[...]' : '{...}'}
        </samp>
      </div>
    )
  }
}

function PrettyKey(props: { k: any }) {
  const { k } = props

  if (typeof k === 'string') {
    return <span className="text-blue-300">{k}</span>
  } else {
    return <span className="text-pink-300">{k}</span>
  }
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
