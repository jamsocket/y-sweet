import { YDocProvider } from '@y-sweet/react'
import { getOrCreateDoc } from '@y-sweet/sdk'
import { ColorGrid } from './ColorGrid'
import { CONNECTION_STRING } from '@/lib/config'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  const clientToken = await getOrCreateDoc(searchParams.doc, CONNECTION_STRING)

  return (
    <YDocProvider clientToken={clientToken} setQueryParam="doc">
      <ColorGrid />
    </YDocProvider>
  )
}
