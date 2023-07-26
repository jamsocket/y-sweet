'use client'

import { useText, useAwareness } from '@/lib/provider'
import { useEffect, useRef } from 'react'
import { QuillBinding } from 'y-quill'

import 'quill/dist/quill.snow.css'

export function TextEditor() {
  const yText = useText('text')
  const awareness = useAwareness()
  const editorRef = useRef<HTMLDivElement | null>(null)
  const bindingRef = useRef<QuillBinding | null>(null)

  useEffect(() => {
    if (bindingRef.current !== null) {
      return
    }
    if (editorRef.current && awareness) {
      // These libraries are designed to work in the browser, and will cause warnings
      // if imported on the server. Nextjs renders components on both the server and
      // the client, so we import them lazily here when they are used on the client.
      const Quill = require('quill')
      const QuillCursors = require('quill-cursors')

      Quill.register('modules/cursors', QuillCursors)
      const quill = new Quill(editorRef.current, {
        theme: 'snow',
        modules: {
          cursors: true,
          toolbar: [
            [{ header: [1, 2, false] }],
            ['bold', 'italic', 'underline', 'strike'],
            [{ list: 'ordered' }, { list: 'bullet' }],
            ['link'],
          ],
        },
      })
      bindingRef.current = new QuillBinding(yText!, quill, awareness!)
    }
  }, [yText, awareness])

  return (
    <div>
      <h1>A Collaborative Text Editor</h1>
      <div ref={editorRef} />
    </div>
  )
}
