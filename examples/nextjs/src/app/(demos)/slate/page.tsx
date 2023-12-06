import { CONNECTION_STRING } from '@/lib/config'
import { SlateEditor } from './SlateEditor'
import { YDocProvider } from '@y-sweet/react'
import { getOrCreateDocAndToken } from '@y-sweet/sdk'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  const clientToken = await getOrCreateDocAndToken(CONNECTION_STRING, searchParams.doc)

  return (
    <YDocProvider clientToken={clientToken} setQueryParam="doc">
      <SlateEditor />
    </YDocProvider>
  )
}
