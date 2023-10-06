'use client'

import { withYjs, YjsEditor } from '@slate-yjs/core'
import { useYDoc, useYjsProvider } from '@y-sweet/react'
import { useEffect, useMemo, useState } from 'react'
import { Editor, Transforms, createEditor } from 'slate'
import { Slate, Editable, withReact } from 'slate-react'
import * as Y from 'yjs'

const initialValue = [
  {
    type: 'paragraph',
    children: [{ text: 'A line of text in a paragraph.' }],
  },
]

export function SlateEditor() {
  const [connected, setConnected] = useState(false)

  const yDoc = useYDoc()
  const yProvider = useYjsProvider()

  const sharedType = useMemo(() => {
    return yDoc.get('content', Y.XmlText) as Y.XmlText
  }, [yDoc])

  useEffect(() => {
    yProvider.on('sync', setConnected)
    return () => yProvider.off('sync', setConnected)
  }, [yProvider])

  if (!connected) return 'Loading...'

  return <SlateConnectedEditor sharedType={sharedType} />
}

function SlateConnectedEditor({ sharedType }: { sharedType: Y.XmlText }) {
  const editor = useMemo(() => {
    const e = withReact(withYjs(createEditor(), sharedType))

    const { normalizeNode } = e
    e.normalizeNode = (entry) => {
      const [node] = entry
      if (!Editor.isEditor(node) || node.children.length > 0) {
        return normalizeNode(entry)
      }

      Transforms.insertNodes(e, initialValue, { at: [0] })
    }

    return e
  }, [sharedType])

  useEffect(() => {
    YjsEditor.connect(editor)
    return () => YjsEditor.disconnect(editor)
  }, [editor])

  return (
    <Slate editor={editor} initialValue={initialValue}>
      <Editable />
    </Slate>
  )
}
