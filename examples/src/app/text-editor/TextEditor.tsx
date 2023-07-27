'use client'

import { useText, useAwareness } from '../../../../js/src/react'
import { useEffect, useRef } from 'react'
import { QuillBinding } from 'y-quill'
import Title from '@/components/Title'

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
        placeholder: 'start collaborating or debating about barbie vs oppenheimer...',
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
    <div className="m-10 space-y-3">
      <Title>A Collaborative Text Editor</Title>
      <div ref={editorRef} />
    </div>
  )
}
