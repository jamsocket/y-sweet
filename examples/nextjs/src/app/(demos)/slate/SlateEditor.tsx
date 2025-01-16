'use client'

import { useYDoc, useYjsProvider, useAwareness } from '@y-sweet/react'
import { useEffect, useMemo, useState } from 'react'
import Header from '@/components/Header'
import * as Y from 'yjs'

import RichtextSlateEditor from './RichtextSlateEditor'

export function SlateEditor() {
  const [connected, setConnected] = useState(false)

  const yDoc = useYDoc()
  const provider = useYjsProvider()
  const awareness = useAwareness()

  const sharedType = useMemo(() => {
    return yDoc.get('content', Y.XmlText) as Y.XmlText
  }, [yDoc])

  useEffect(() => {
    provider.on('sync', setConnected)
    return () => provider.off('sync', setConnected)
  }, [provider])

  if (!connected) return 'Loading...'

  return (
    <div className="p-4 lg:p-8 space-y-3">
      <Header
        title="Slate Rich Text Editor"
        githubLink="https://github.com/jamsocket/y-sweet/tree/main/examples/nextjs/src/app/(demos)/slate"
      />
      <div className="bg-white rounded-lg">
        <RichtextSlateEditor sharedType={sharedType} awareness={awareness} />
      </div>
    </div>
  )
}
