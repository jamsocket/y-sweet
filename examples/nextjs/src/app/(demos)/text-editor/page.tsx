'use client'

import { YDocProvider } from '@y-sweet/react'
import { useSearchParams } from 'next/navigation'
import { TextEditor } from './TextEditor'
import { randomId } from '@/lib/utils'

export default function Home() {
  const searchParams = useSearchParams()
  const docId = searchParams.get('doc') ?? randomId()
  return (
    <YDocProvider docId={docId} setQueryParam="doc" authEndpoint="/api/auth">
      <TextEditor />
    </YDocProvider>
  )
}
