import { CONNECTION_STRING } from '@/lib/config'
import { Presence } from './Presence'
import { YDocProvider } from '@y-sweet/react'
import { getOrCreateDocAndToken } from '@y-sweet/sdk'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  const clientToken = await getOrCreateDocAndToken(CONNECTION_STRING, searchParams.doc)

  return (
    <YDocProvider clientToken={clientToken} setQueryParam="doc">
      <Presence />
    </YDocProvider>
  )
}
