import { YDocProvider } from '@/lib/provider'
import { getConnectionKey } from '@/lib/yserv'
import { Console } from './Console'
import { ENV_CONFIG } from '@/lib/config'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  const doc = searchParams.doc

  if (!doc) {
    return <p>Missing doc param</p>
  }

  const connectionKey = await getConnectionKey(searchParams.doc, {}, ENV_CONFIG)

  return (
    <YDocProvider connectionKey={connectionKey}>
      <Console />
    </YDocProvider>
  )
}
