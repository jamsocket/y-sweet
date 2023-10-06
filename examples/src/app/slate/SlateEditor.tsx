'use client'

import { withYjs, slateNodesToInsertDelta, YjsEditor } from '@slate-yjs/core'
import { useYDoc } from '@y-sweet/react'
import { useEffect, useMemo, useState } from 'react'
import { Editor, Transforms, createEditor } from 'slate'
import { Slate, Editable, withReact } from 'slate-react'
import { Transform } from 'stream'
import * as Y from 'yjs'
import { YXmlText } from 'yjs/dist/src/internals'

const initialValue = [
  {
    type: 'paragraph',
    children: [{ text: 'A line of text in a paragraph.' }],
  },
]

export function SlateEditor() {
  const [value, setValue] = useState([])

  const yDoc = useYDoc()
  const sharedType = useMemo(() => {
    const sharedType = yDoc.get('content', Y.XmlText) as YXmlText

    sharedType.applyDelta(slateNodesToInsertDelta(initialValue))

    return sharedType
  }, [])

  const editor = useMemo(() => withReact(withYjs(createEditor(), sharedType)), [])
  const { normalizeNode } = editor
  editor.normalizeNode = (entry) => {
    const [node] = entry
    if (!Editor.isEditor(node) || node.children.length > 0) {
      return normalizeNode(entry)
    }
  }

  useEffect(() => {
    YjsEditor.connect(editor)

    return () => YjsEditor.disconnect(editor)
  }, [editor])

  return (
    <Slate editor={editor} initialValue={initialValue} onChange={setValue as any}>
      <Editable />
    </Slate>
  )
}
