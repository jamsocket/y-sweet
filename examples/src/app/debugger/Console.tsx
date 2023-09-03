'use client'

import { useYDoc } from '@y-sweet/react'
import { useCallback, useEffect, useMemo, useState } from 'react'
import * as Y from 'yjs'
import { Debuggable, DebuggableEntry, DebuggableYDoc } from './debuggable'

export function Console() {
  const doc: Y.Doc = useYDoc()
  const [_, setVersion] = useState(0)

  // TODO: move observers into items
  useEffect(() => {
    const callback = () => {
      setVersion(version => version + 1)
    }

    doc.on('update', callback)

    return () => {
      doc.off('update', callback)
    }
  },
    [setVersion]
  )

  return <DocEntryView doc={doc} />
}

type DocEntryViewProps = {
  doc: Y.Doc
}

function DocEntryView(props: DocEntryViewProps) {
  let debuggable = useMemo(() => new DebuggableYDoc(props.doc), [props.doc])

  return <div className="p-8">
    <DebuggableItems debuggable={debuggable} />
  </div>
}

function DebuggableItems(props: { debuggable: Debuggable }) {
  let { debuggable } = props

  return <div>
    {
      debuggable.entries().map(entry => <DebuggableItem entry={entry} key={entry.key} />)
    }
  </div>
}

function TypePill(props: {type?: string}) {
  if (!props.type) {
    return null
  }

  return <span className="text-xs bg-white p-1 rounded-md">{props.type}</span>
}

function DebuggableItem(props: { entry: DebuggableEntry }) {
  const { entry } = props
  const [expanded, setExpanded] = useState(true)
  
  const toggleExpanded = useCallback(() => {
    setExpanded(expanded => !expanded)
  }, [])

  if (entry.value.type === 'scalar') {
    return <div>
      <samp className="text-gray-500"><PrettyKey k={entry.key} />: <PrettyValue value={entry.value.value()} /></samp>
    </div>
  } else if (expanded) {
    return <div>
      <samp onClick={toggleExpanded} className="text-gray-500"><PrettyKey k={entry.key} />: <TypePill type={entry.value.typeName} /> {entry.value.type === 'list' ? '[' : '{'}</samp>
      <div className="pl-5">
        <DebuggableItems debuggable={entry.value} />
      </div>
      <samp className="text-gray-500">{entry.value.type === 'list' ? ']' : '}'}</samp>
    </div>
  } else {
    return <div>
      <samp className="text-gray-500" onClick={toggleExpanded}><PrettyKey k={entry.key} />: <TypePill type={entry.value.typeName} /> {entry.value.type === 'list' ? '[...]' : '{...}'}</samp>
    </div>
  }
}

function PrettyKey(props: { k: any }) {
  const { k } = props

  if (typeof k === 'string') {
    return <span className="text-blue-500">{k}</span>
  } else {
    return <span className="text-pink-500">{k}</span>
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
  } else if (typeof props.value === 'number') {
    return <span className="text-yellow-600">{props.value}</span>
  } else {
    console.log('unimplemented value type', typeof props.value)
    return <span>unknown type</span>
  }
}

function PrettyString(props: { value: string }) {
  let valueEscaped = JSON.stringify(props.value)
  valueEscaped = valueEscaped.slice(1, valueEscaped.length - 1)
  return (
    <span className="text-blue-300">
      {'"'}
      <span className="text-blue-600">{valueEscaped}</span>
      {'"'}
    </span>
  )
}
