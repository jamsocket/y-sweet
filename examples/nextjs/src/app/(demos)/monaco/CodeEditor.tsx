'use client'

import React, { useEffect, useState, useCallback, useMemo } from 'react'
import { Editor } from '@monaco-editor/react'
import { MonacoBinding } from 'y-monaco'
import { useYDoc, useAwareness, useYjsProvider } from '@y-sweet/react'
import { editor } from 'monaco-editor'
import Header from '@/components/Header'

export function CodeEditor() {
  const yDoc = useYDoc()
  const awareness = useAwareness()
  const [editorRef, setEditorRef] = useState<editor.IStandaloneCodeEditor | null>(null)
  const sharedText = useMemo(() => yDoc.getText('content'), [yDoc])

  useEffect(() => {
    if (!editorRef) return

    const binding = new MonacoBinding(
      sharedText,
      editorRef.getModel() as editor.ITextModel,
      new Set([editorRef]),
      awareness,
    )

    return () => binding.destroy()
  }, [editorRef, sharedText, awareness])

  const handleEditorMount = useCallback((editor: editor.IStandaloneCodeEditor) => {
    setEditorRef(editor)
  }, [])

  return (
    <div className="p-4 lg:p-8 space-y-4">
      <Header
        title="Monaco Code Editor"
        githubLink="https://github.com/jamsocket/y-sweet/tree/main/examples/nextjs/src/app/(demos)/monaco"
      />
      <div>
        <Editor
          defaultLanguage="javascript"
          defaultValue="// Start typing here..."
          theme="vs-dark"
          className="h-[600px] w-full"
          onMount={handleEditorMount}
          options={{
            tabSize: 2,
            automaticLayout: true,
            cursorStyle: 'line',
          }}
        />
      </div>
    </div>
  )
}
