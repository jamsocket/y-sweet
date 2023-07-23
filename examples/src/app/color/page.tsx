import Debugger from '@/components/DocDebugger'
import { YDocProvider } from '@/lib/provider'
import { getOrCreateDoc } from '@/lib/yserv'
import { ColorGrid } from './Color'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  const connectionKey = await getOrCreateDoc(searchParams.doc, {token: 'foobar'})

  return (
    <YDocProvider connectionKey={connectionKey} setQueryParam='doc'>
      <ColorGrid />
    </YDocProvider>
  )
}
