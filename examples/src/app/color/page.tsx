import { YDocProvider } from '@/lib/provider'
import { getOrCreateDoc } from '@/lib/yserv'
import { ColorGrid } from './Color'
import { ENV_CONFIG } from '@/lib/config'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  const connectionKey = await getOrCreateDoc(searchParams.doc, ENV_CONFIG)

  return (
    <YDocProvider connectionKey={connectionKey} setQueryParam='doc'>
      <ColorGrid />
    </YDocProvider>
  )
}
