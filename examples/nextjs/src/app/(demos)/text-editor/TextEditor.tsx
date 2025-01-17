'use client'

import { useText, useAwareness } from '@y-sweet/react'
import { useEffect, useRef } from 'react'
import { QuillBinding } from 'y-quill'
import Header from '@/components/Header'

import 'quill/dist/quill.snow.css'

export function TextEditor() {
  const yText = useText('text', { observe: 'none' })
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
        placeholder: 'Start collaborating...',
        modules: {
          cursors: true,
          toolbar: [
            [{ header: [1, 2, false] }],
            ['bold', 'italic', 'underline'],
            [{ list: 'ordered' }],
            ['link'],
          ],
        },
      })
      bindingRef.current = new QuillBinding(yText!, quill, awareness!)
    }
  }, [yText, awareness])

  return (
    <div className="p-4 sm:p-8 space-y-3">
      <Header
        title="Quill Text Editor"
        githubLink="https://github.com/jamsocket/y-sweet/tree/main/examples/nextjs/src/app/(demos)/text-editor"
      />
      <div className="bg-white/90">
        <div ref={editorRef} />
      </div>
    </div>
  )
}
