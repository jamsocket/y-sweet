'use client'

import { useYDoc, useYjsProvider } from '@y-sweet/react'
import { useCreateBlockNote } from '@blocknote/react'
import { BlockNoteView } from '@blocknote/mantine'
import '@blocknote/mantine/style.css'
import Header from '@/components/Header'

export function BlockNote() {
  const provider = useYjsProvider()
  const doc = useYDoc()

  const editor = useCreateBlockNote({
    collaboration: {
      provider,
      fragment: doc.getXmlFragment('blocknote'),
      user: { name: 'Your Username', color: '#ff0000' },
    },
  })

  return (
    <div className="p-4 sm:p-8 flex flex-col gap-y-3 h-full">
      <Header
        title="BlockNote Editor"
        githubLink="https://github.com/jamsocket/y-sweet/tree/main/examples/nextjs/src/app/(demos)/blocknote"
      />
      <div className="flex-1 bg-white px-2 py-4 rounded-lg">
        <BlockNoteView editor={editor} theme="light" />
      </div>
    </div>
  )
}
