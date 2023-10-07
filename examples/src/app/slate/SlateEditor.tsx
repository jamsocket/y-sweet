'use client'

import { useYDoc, useYjsProvider } from '@y-sweet/react'
import { useEffect, useMemo, useState } from 'react'
import * as Y from 'yjs'

import RichtextSlateEditor from './RichtextSlateEditor'

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

  return (
    <div className="p-8">
      <div className="bg-white rounded-lg">
        <RichtextSlateEditor sharedType={sharedType} />
      </div>
    </div>
  )
}
