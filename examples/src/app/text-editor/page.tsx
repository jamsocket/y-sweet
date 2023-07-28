import { ENV_CONFIG } from '@/lib/config'
import { TextEditor } from './TextEditor'
import { YDocProvider } from '@y-sweet/js/react'
import { getOrCreateDoc } from '@y-sweet/js/sdk'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  const connectionKey = await getOrCreateDoc(searchParams.doc, ENV_CONFIG)

  return (
    <YDocProvider connectionKey={connectionKey} setQueryParam="doc">
      <TextEditor />
    </YDocProvider>
  )
}
