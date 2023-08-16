import { Y_SWEET_CONFIG } from '@/lib/config'
import { TextEditor } from './TextEditor'
import { YDocProvider } from '@y-sweet/react'
import { getOrCreateDoc } from '@y-sweet/sdk'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  const clientToken = await getOrCreateDoc(searchParams.doc, Y_SWEET_CONFIG)

  return (
    <YDocProvider clientToken={clientToken} setQueryParam="doc">
      <TextEditor />
    </YDocProvider>
  )
}
